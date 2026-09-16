-- =====================================================================
-- 001_identity_and_access.sql
-- odyssey1e — identity, tenancy, and the access model
-- Revision 2 (review fixes; see REVIEW NOTES at the foot of this file)
-- =====================================================================
--
-- SCOPE: this migration establishes WHO can see WHAT. It deliberately
-- contains almost no game content. Reference data (skills, spells, dice
-- art, narrative lines) and the 5e detail tables come in 002.
--
-- WHY THIS ORDER: tenancy columns and RLS policies are the single most
-- expensive thing to retrofit. Adding game_id to a populated schema
-- means backfilling every row and rewriting every policy and query that
-- touches it. Everything else can be added incrementally; this cannot.
--
-- THE MODEL
--   profiles      one row per authenticated user (id = auth.uid())
--   games         a campaign. Owned by exactly one DM.
--   game_members  who is in which game, and in what role
--   characters    belongs to a game, owned by a player
--   rolls         the roll log, scoped to a game
--
-- REFERENCE DATA IS NOT TENANTED — a standing decision for 002.
--   The 360 narrative lines, the 80 dice faces and the skill/spell
--   catalogues are global content shared by every game. Giving them a
--   game_id would duplicate 360 rows per campaign and make seeding a
--   per-game chore.
--   BUT each reference table gets a NULLABLE game_id: null means global,
--   set means "this campaign only". Lookups take the game-specific row
--   when one exists and fall back to global. That is the same precedence
--   the Apps Script system already used for dice — ActorDice, then the
--   script property, then the default — and it is what lets a DM add
--   house narration without forking content everyone shares.
--
-- ON RLS: these are real policies, not the open dev pattern
-- (USING (true) WITH CHECK (true)). Open policies mean every client sees
-- every row, which makes a DM-vs-player access test prove nothing. If
-- the policies are not the thing under test, the access model is not
-- being tested.
-- =====================================================================


-- =====================================================================
-- PROFILES
-- =====================================================================

create table public.profiles (
  id            uuid primary key references auth.users(id) on delete cascade,
  display_name  text not null default 'Adventurer',
  created_at    timestamptz not null default now()
);

comment on table  public.profiles is
  'One row per authenticated user. id is auth.uid(); every ownership column in this schema points here.';
comment on column public.profiles.display_name is
  'Shown at the table and on roll cards. Not unique — two players may share a name.';

alter table public.profiles enable row level security;

create policy "profiles: read own"
  on public.profiles for select
  using (id = auth.uid());

create policy "profiles: update own"
  on public.profiles for update
  using (id = auth.uid())
  with check (id = auth.uid());

create policy "profiles: insert own"
  on public.profiles for insert
  with check (id = auth.uid());


-- Create the profile row automatically on signup, so no client code has
-- to remember to. Without this a new user authenticates successfully and
-- then has no profile, which surfaces later as a confusing null join.
create function public.handle_new_user()
returns trigger
language plpgsql
security definer set search_path = ''
as $$
begin
  insert into public.profiles (id, display_name)
  values (new.id, coalesce(new.raw_user_meta_data->>'display_name', 'Adventurer'));
  return new;
end;
$$;

create trigger on_auth_user_created
  after insert on auth.users
  for each row execute function public.handle_new_user();


-- =====================================================================
-- SHARED HELPERS
-- =====================================================================

-- Keeps updated_at honest without every client remembering to set it.
create function public.touch_updated_at()
returns trigger
language plpgsql
as $$
begin
  new.updated_at = now();
  return new;
end;
$$;


-- Join codes are generated here rather than chosen, so entropy is not a
-- matter of whoever names the game. The alphabet omits 0/O/1/I/L — these
-- get read aloud across a table and mis-heard characters are the whole
-- failure mode. 8 chars from 31 symbols is ~2^40 of space; see the
-- brute-force note in REVIEW NOTES.
create function public.gen_join_code()
returns text
language plpgsql
volatile
as $$
declare
  alphabet constant text := 'ABCDEFGHJKMNPQRSTUVWXYZ23456789';
  out text := '';
  i int;
begin
  for i in 1..8 loop
    out := out || substr(alphabet, 1 + floor(random() * length(alphabet))::int, 1);
  end loop;
  return out;
end;
$$;


-- =====================================================================
-- GAMES
-- =====================================================================

create table public.games (
  id          uuid primary key default gen_random_uuid(),
  name        text not null,
  dm_uid      uuid not null references public.profiles(id) on delete restrict,
  join_code   text not null unique default public.gen_join_code(),
  is_open     boolean not null default true,
  created_at  timestamptz not null default now()
);

comment on table  public.games is
  'A campaign. Exactly one DM owns it; players join via join_code.';
comment on column public.games.dm_uid is
  'The owning DM. on delete restrict — deleting a profile must not silently orphan a campaign.';
comment on column public.games.join_code is
  'Short shared string a player enters to join. Generated, unique, and never selectable by non-members — join_game() reads it as definer.';
comment on column public.games.is_open is
  'False closes the game to new joins without invalidating the code.';

create index games_dm_uid_idx on public.games(dm_uid);

alter table public.games enable row level security;


-- =====================================================================
-- GAME MEMBERSHIP
-- =====================================================================

create type public.game_role as enum ('dm', 'player');

create table public.game_members (
  game_id     uuid not null references public.games(id) on delete cascade,
  profile_id  uuid not null references public.profiles(id) on delete cascade,
  role        public.game_role not null default 'player',
  joined_at   timestamptz not null default now(),
  primary key (game_id, profile_id)
);

comment on table  public.game_members is
  'Who is in which game, and in what role. The DM gets a row here too, so membership checks never special-case them.';
comment on column public.game_members.role is
  'dm or player. A game has exactly one dm row, enforced by the partial unique index below.';

create unique index game_members_one_dm_idx
  on public.game_members(game_id)
  where role = 'dm';

create index game_members_profile_idx on public.game_members(profile_id);

alter table public.game_members enable row level security;


-- =====================================================================
-- MEMBERSHIP HELPERS
-- =====================================================================
--
-- !! THESE MUST BE SECURITY DEFINER. !!
--
-- A policy on game_members that queries game_members recurses
-- infinitely — Postgres applies the table's own RLS to the subquery,
-- which invokes the policy, which runs the subquery. The error is
-- "infinite recursion detected in policy for relation game_members" and
-- it is the most common way a Supabase multi-tenant schema fails on
-- first run.
--
-- A SECURITY DEFINER function runs as its owner, which owns these tables
-- and therefore bypasses their RLS, breaking the cycle. search_path is
-- pinned empty so a caller cannot shadow public with their own schema.
-- =====================================================================

create function public.is_game_member(p_game_id uuid)
returns boolean
language sql
stable
security definer set search_path = ''
as $$
  select exists (
    select 1 from public.game_members
    where game_id = p_game_id and profile_id = auth.uid()
  );
$$;

comment on function public.is_game_member(uuid) is
  'True when the caller belongs to the game. SECURITY DEFINER — see the header above; do not convert to SECURITY INVOKER.';

create function public.is_game_dm(p_game_id uuid)
returns boolean
language sql
stable
security definer set search_path = ''
as $$
  select exists (
    select 1 from public.game_members
    where game_id = p_game_id and profile_id = auth.uid() and role = 'dm'
  );
$$;

comment on function public.is_game_dm(uuid) is
  'True when the caller is the DM of the game. SECURITY DEFINER for the same reason as is_game_member.';

revoke all on function public.is_game_member(uuid) from public;
revoke all on function public.is_game_dm(uuid)     from public;
grant execute on function public.is_game_member(uuid) to authenticated;
grant execute on function public.is_game_dm(uuid)     to authenticated;


-- ---- games policies ----

create policy "games: members read"
  on public.games for select
  using (public.is_game_member(id));

create policy "games: dm creates"
  on public.games for insert
  with check (dm_uid = auth.uid());

create policy "games: dm updates"
  on public.games for update
  using (public.is_game_dm(id))
  with check (public.is_game_dm(id));

create policy "games: dm deletes"
  on public.games for delete
  using (public.is_game_dm(id));


-- ---- game_members policies ----
--
-- NOTE THE ABSENCE OF AN INSERT POLICY. That is deliberate — see
-- join_game() below. Revision 1 had a self-join INSERT policy whose
-- WITH CHECK read the games table; because games has RLS requiring
-- membership, a prospective member could never see the row, the
-- subquery always returned nothing, and joining was impossible. The
-- lesson generalizes: a policy that reads another RLS-protected table
-- is evaluated under that table's RLS too.

create policy "game_members: members read roster"
  on public.game_members for select
  using (public.is_game_member(game_id));

create policy "game_members: dm manages"
  on public.game_members for update
  using (public.is_game_dm(game_id))
  with check (public.is_game_dm(game_id));

create policy "game_members: dm removes, or self leaves"
  on public.game_members for delete
  using (public.is_game_dm(game_id) or profile_id = auth.uid());


-- Seat the DM when a game is created. Without this the creator owns a
-- game they are not a member of, so is_game_member() is false and their
-- own select policy hides it from them.
create function public.seat_game_dm()
returns trigger
language plpgsql
security definer set search_path = ''
as $$
begin
  insert into public.game_members (game_id, profile_id, role)
  values (new.id, new.dm_uid, 'dm');
  return new;
end;
$$;

create trigger on_game_created
  after insert on public.games
  for each row execute function public.seat_game_dm();


-- =====================================================================
-- JOINING
-- =====================================================================
--
-- The only way into a game. A function rather than a policy because the
-- prospective member cannot see the game row yet — that is precisely
-- what they are asking to change. As definer it reads games directly,
-- which also means join_code never has to be selectable by anyone.
--
-- Returns the game id on success; raises otherwise. The three failure
-- messages are deliberately distinguishable, because "wrong code" and
-- "game closed" need different responses at the table.
-- =====================================================================

create function public.join_game(p_code text)
returns uuid
language plpgsql
security definer set search_path = ''
as $$
declare
  v_game public.games%rowtype;
begin
  if auth.uid() is null then
    raise exception 'not authenticated';
  end if;

  select * into v_game from public.games
   where join_code = upper(btrim(p_code));

  if not found then
    raise exception 'no game with that code';
  end if;

  if not v_game.is_open then
    raise exception 'that game is closed to new players';
  end if;

  insert into public.game_members (game_id, profile_id, role)
  values (v_game.id, auth.uid(), 'player')
  on conflict (game_id, profile_id) do nothing;

  return v_game.id;
end;
$$;

comment on function public.join_game(text) is
  'Join a game by code. The ONLY path into game_members for a player — there is no INSERT policy on that table. Case- and whitespace-insensitive on the code.';

revoke all on function public.join_game(text) from public;
grant execute on function public.join_game(text) to authenticated;


-- =====================================================================
-- CHARACTERS
-- =====================================================================
--
-- Deliberately thin. The 5e detail — abilities, skills, inventory,
-- spells, features — lands in 002 as child tables keyed on character_id.
-- What matters here is that a character carries BOTH game_id and
-- owner_uid, because they answer different questions: which campaign it
-- belongs to, and who at the table controls it.
-- =====================================================================

create table public.characters (
  id            uuid primary key default gen_random_uuid(),
  game_id       uuid not null references public.games(id) on delete cascade,
  owner_uid     uuid not null references public.profiles(id) on delete restrict,
  name          text not null,
  token_name    text,
  portrait_url  text,
  is_active     boolean not null default true,
  created_at    timestamptz not null default now(),
  updated_at    timestamptz not null default now()
);

comment on table  public.characters is
  'A player character in a game. game_id says which campaign; owner_uid says who controls it. Both are required.';
comment on column public.characters.owner_uid is
  'The controlling player. The DM may reassign this — that is the recovery path when someone loses a device or hands a character on.';
comment on column public.characters.token_name is
  'Short name for roll cards and tokens. "Rodnar" where name is "Rodnar Shieldcrest".';
comment on column public.characters.is_active is
  'False retires a character without deleting its roll history.';

create index characters_game_idx  on public.characters(game_id);
create index characters_owner_idx on public.characters(owner_uid);

create trigger characters_touch_updated_at
  before update on public.characters
  for each row execute function public.touch_updated_at();

alter table public.characters enable row level security;

create policy "characters: members read"
  on public.characters for select
  using (public.is_game_member(game_id));

create policy "characters: owner creates own"
  on public.characters for insert
  with check (owner_uid = auth.uid() and public.is_game_member(game_id));

create policy "characters: owner or dm updates"
  on public.characters for update
  using (owner_uid = auth.uid() or public.is_game_dm(game_id))
  with check (public.is_game_dm(game_id) or owner_uid = auth.uid());

create policy "characters: dm deletes"
  on public.characters for delete
  using (public.is_game_dm(game_id));


-- =====================================================================
-- ROLLS
-- =====================================================================
--
-- The RollLog, ported. Three things carried over deliberately from the
-- Apps Script system because they were earned the hard way:
--
--   1. STATUS IS A MACHINE VOCABULARY, EXACT. Write the result first
--      with a pessimistic status, promote it only once the side effect
--      lands. A crash mid-delivery leaves a recoverable row, never a
--      lie.
--
--   2. THE DIE IS PART OF THE RECORD. die_image_url and dice_set_id are
--      stored, never derived at read time. In the old system they were
--      computed on every sync, so historical rolls silently changed
--      their die art whenever the player equipped a different set — a
--      record that rewrote itself. Snapshot at resolve time.
--
--   3. TWO-PHASE DELIVERY. The dice go out immediately; the narrative is
--      patched in when it arrives, because a slow enrichment must never
--      gate a fast result. The owner-update policy below exists solely
--      to permit that patch — see REVIEW NOTES.
-- =====================================================================

create type public.roll_status as enum (
  'pending',    -- created, not yet resolved
  'resolved',   -- dice are final; narrative may still be landing
  'delivered',  -- everyone who should see it has seen it. Immutable.
  'failed',     -- resolved, but delivery did not land. Recoverable by resend.
  'error'       -- resolution itself failed
);

create table public.rolls (
  id             uuid primary key default gen_random_uuid(),
  game_id        uuid not null references public.games(id) on delete cascade,
  character_id   uuid references public.characters(id) on delete set null,
  owner_uid      uuid not null references public.profiles(id) on delete restrict,
  created_at     timestamptz not null default now(),

  character_name text not null default 'Someone',
  roller_name    text not null default 'Adventurer',
  label          text,
  request        text not null,
  mode           text not null default 'normal',
  formula        text,
  detail         text,
  total          integer,
  natural_roll   integer,
  status         public.roll_status not null default 'pending',
  flavor         text,
  narrative      text,

  die_image_url  text,
  dice_set_id    text,

  adj_bonus      integer not null default 0,
  adj_penalty    integer not null default 0
);

comment on table  public.rolls is
  'The roll log, scoped to a game. Immutable to players once delivered; only the DM may amend after that.';
comment on column public.rolls.character_name is
  'WHOSE ACTION THIS WAS — the character, snapshotted at insert. This is what the card shows ("Rodnar rolls Insight"), and it is the column the old RollLog.Roller maps to. Not read through character_id at display time: that FK is ON DELETE SET NULL and follows renames, so an old roll would lose or change its subject.';
comment on column public.rolls.roller_name is
  'WHO AT THE TABLE MADE IT — the player, snapshotted at insert. Different question from character_name: the card names the character, but a DM scanning the log wants the person. Survives a rename, a handover, or the player leaving.';
comment on column public.rolls.request is
  'What was asked for, in engine vocabulary: a skill key, "death save", "short rest".';
comment on column public.rolls.mode is
  'normal | adv | dis.';
comment on column public.rolls.natural_roll is
  'The raw d20 face, and ONLY when exactly one d20 term was rolled. Null for multi-d20 and utility results — crit and fumble detection depend on that distinction.';
comment on column public.rolls.status is
  'pending → resolved → delivered. failed means resolved but delivery did not land, and is recoverable by resend. error means resolution itself failed.';
comment on column public.rolls.die_image_url is
  'The die face this roll was MADE with, snapshotted at resolve time. Never derived at read time — see the header above.';
comment on column public.rolls.dice_set_id is
  'The dice set equipped at the moment of the roll. Text, not a FK, so retiring a set never rewrites history.';
comment on column public.rolls.adj_penalty is
  'DM penalty, stored positive. Sign-agnostic on input: 2 and -2 both mean a 2-point penalty.';

create index rolls_game_created_idx on public.rolls(game_id, created_at desc);
create index rolls_character_idx    on public.rolls(character_id);
create index rolls_owner_idx        on public.rolls(owner_uid);

alter table public.rolls enable row level security;

create policy "rolls: members read"
  on public.rolls for select
  using (public.is_game_member(game_id));

create policy "rolls: owner inserts own"
  on public.rolls for insert
  with check (owner_uid = auth.uid() and public.is_game_member(game_id));

-- The narrative patch. An owner may amend their own roll while it is
-- still settling, and not after: once status is 'delivered', 'failed' or
-- 'error' this policy stops matching and the row is history.
create policy "rolls: owner finalizes own"
  on public.rolls for update
  using (owner_uid = auth.uid() and status in ('pending', 'resolved'))
  with check (owner_uid = auth.uid());

create policy "rolls: dm updates"
  on public.rolls for update
  using (public.is_game_dm(game_id))
  with check (public.is_game_dm(game_id));

create policy "rolls: dm deletes"
  on public.rolls for delete
  using (public.is_game_dm(game_id));


-- Freeze BOTH names onto the row at insert, unless the client supplied
-- them. Doing it here rather than client-side means the snapshot cannot
-- be forgotten. SECURITY DEFINER because profiles is readable only by
-- its owner — an invoker-rights function would find nothing.
create function public.snapshot_roll_names()
returns trigger
language plpgsql
security definer set search_path = ''
as $$
declare
  v text;
begin
  if new.character_name is null or btrim(new.character_name) in ('', 'Someone') then
    if new.character_id is not null then
      select c.name into v from public.characters c where c.id = new.character_id;
      new.character_name := coalesce(nullif(btrim(v), ''), 'Someone');
    end if;
  end if;

  if new.roller_name is null or btrim(new.roller_name) in ('', 'Adventurer') then
    select p.display_name into v from public.profiles p where p.id = new.owner_uid;
    new.roller_name := coalesce(nullif(btrim(v), ''), 'Adventurer');
  end if;

  return new;
end;
$$;

create trigger rolls_snapshot_names
  before insert on public.rolls
  for each row execute function public.snapshot_roll_names();


-- A WITH CHECK cannot see the old row, so it cannot say "this column did
-- not change". Without this trigger a player could re-parent their own
-- roll into another game they belong to, or hand it to another
-- character. Freeze the identity columns; everything else is fair game
-- for the phase-two patch.
create function public.freeze_roll_identity()
returns trigger
language plpgsql
as $$
begin
  if new.game_id is distinct from old.game_id
     or new.owner_uid is distinct from old.owner_uid
     or new.roller_name is distinct from old.roller_name
     or new.character_name is distinct from old.character_name then
    raise exception 'game_id, owner_uid and the two snapshotted names are immutable on rolls';
  end if;
  return new;
end;
$$;

create trigger rolls_freeze_identity
  before update on public.rolls
  for each row execute function public.freeze_roll_identity();


-- =====================================================================
-- VERIFICATION
-- =====================================================================
--
-- RLS on everywhere. Any false is a table the policies do not protect.
--
--   select relname, relrowsecurity
--   from pg_class
--   where relnamespace = 'public'::regnamespace and relkind = 'r'
--   order by relname;
--
-- The policy inventory:
--
--   select tablename, policyname, cmd
--   from pg_policies
--   where schemaname = 'public'
--   order by tablename, policyname;
--
-- THE TEST THAT MATTERS — run it before building anything on top.
-- Four plus-addressed accounts; email confirmation off while testing.
--
--   1. dm@  — sign in, create a game, note the generated join_code.
--   2. p1@  — select public.join_game('<code>'); create a character;
--             insert a roll.
--   3. p2@  — join the same game.
--             SHOULD see p1's character and roll (same table).
--             SHOULD NOT be able to update p1's character.
--   4. p3@  — sign in and do NOT join.
--             SHOULD see nothing: no game, no characters, no rolls.
--             select * from public.games;      -> 0 rows
--             select * from public.characters; -> 0 rows
--             select * from public.rolls;      -> 0 rows
--   5. p1@  — update your own roll's narrative while status='resolved'
--             (allowed), then set status='delivered' and try again
--             (denied). That is two-phase delivery working as designed.
--
-- Step 4 is the one that proves the model. If p3 sees anything, a policy
-- is wrong and everything built above it inherits the hole.
--
-- =====================================================================
-- REVIEW NOTES — what changed from revision 1, and what is still open
-- =====================================================================
--
-- FIXED · joining was impossible. Revision 1 gated self-join with an
--   INSERT policy whose WITH CHECK selected from games. games has RLS
--   requiring membership, so a prospective member's subquery always
--   returned zero rows and every join was denied. Replaced with
--   join_game(), and the INSERT policy on game_members removed
--   entirely. Generalized lesson, worth keeping: a policy that reads
--   another RLS-protected table is evaluated under that table's RLS.
--
-- FIXED · two-phase delivery was blocked. Revision 1 allowed updates
--   only by the DM, which forbade the narrative patch the whole
--   two-phase design depends on. Added "rolls: owner finalizes own",
--   scoped to pending/resolved, plus freeze_roll_identity() to stop the
--   patch being used to re-parent a row.
--
-- FIXED · character vs player was conflated. Revision 3 snapshotted only
--   the player's display name and called it roller_name — but the old
--   RollLog.Roller held ctx.name, the CHARACTER, and that is what the
--   card shows. Both are now frozen on the row and they answer different
--   questions: character_name is whose action it was, roller_name is who
--   at the table made it. When migrating old rows, RollLog.Roller maps to
--   character_name, NOT roller_name.
--
-- ADDED · rolls.roller_name, snapshotted at insert by trigger. The
--   alternative was a policy letting players read each other's profiles
--   so a card could join for the name at read time. Snapshotting is
--   better for the same reason it is better for die art: a roll is a
--   record. The card stays correct after a rename, a character handover
--   or a player leaving, and no cross-profile read policy is needed at
--   all. The name is frozen on update alongside game_id and owner_uid.
--
--   Consequence to know: profiles stay private to their owner, so a
--   roster view joining game_members → profiles still renders UUIDs.
--   When you build that view, add the shared-games select policy — it is
--   a one-liner and nothing here depends on its absence.
--
-- ADDED · generated join codes with an unambiguous alphabet, so entropy
--   does not depend on what the DM types and a code read aloud across a
--   table is not mis-heard.
--
-- ADDED · updated_at is maintained by trigger rather than by client
--   discipline.
--
-- OPEN · join codes are brute-forceable in principle. 8 chars from 31
--   symbols is ~2^40, and any authenticated user may call join_game()
--   repeatedly. For a private table this is theoretical, but if this
--   ever faces strangers, add rate limiting on the function and
--   consider expiring codes. Noted rather than solved, deliberately.
--
-- OPEN · account deletion. characters.owner_uid is ON DELETE RESTRICT,
--   so a profile owning characters cannot be deleted until the DM
--   reassigns them. That is the safe default, but it means "delete my
--   account" is a workflow, not a DELETE.
-- =====================================================================
