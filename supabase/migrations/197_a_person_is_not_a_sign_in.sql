-- 197. A PERSON IS NOT A SIGN-IN.
--
-- Dave: "the log in operation is critical and what we are using is
-- mostly just place-keeper - this will have to hold up to public release
-- at some point - it will likely need to be updated regardless of what we
-- build but we do need to compartmentalize it."
--
-- THE PROBLEM IS ONE EQUALS SIGN. `profiles.id` references
-- `auth.users(id)`, so a profile is not merely linked to a sign-in, it
-- IS one. 42 policies then compare an owner column against `auth.uid()`.
-- Two consequences, and the second is the bad one:
--
--   1. ONE PERSON CANNOT HAVE TWO SIGN-INS. Sign in with Google tomorrow
--      having signed up with email today and you are a different human
--      being as far as every row in this database is concerned.
--
--   2. THE LOGIN SYSTEM CANNOT BE REPLACED. `auth.users.id` is the
--      primary key of the game's data, 42 policies deep, and `auth` is
--      a schema we do not own. Changing authentication later would mean
--      rewriting ownership of every row.
--
-- The second is why this is worth doing NOW rather than when it is
-- needed. Today it is provably a no-op - six users, one sign-in each,
-- `profiles.id = auth.users.id` for all of them - so the whole change
-- can be verified by showing that nothing moved. With a thousand users
-- and two providers it would be a data migration with no safe moment.
--
-- ---------------------------------------------------------------------
-- THE SHAPE: THREE WORDS THAT WERE ONE
-- ---------------------------------------------------------------------
--
--   a PERSON    `profiles.id` - what the game's data references. Never
--               changes, never comes from `auth`.
--   a SIGN-IN   `auth.users.id` - a way of proving you are that person.
--               There may be several, and they are replaceable.
--   the BRIDGE  `identities` - which sign-in belongs to which person.
--
-- `auth.uid()` answers "which sign-in"; the policies were asking it
-- "which person". `current_profile()` is the translation, and it is the
-- ONLY place that knows the two are related.
--
-- NOTHING IN THE GAME SCHEMA MOVES. `owner_uid`, `dm_uid` and
-- `game_members.profile_id` already hold profile ids and still do. This
-- migration changes what fills them in and what they are compared
-- against, not what they mean.
--
-- THIS FILE IS WHY 196 EXISTS. Its first attempt was refused by its own
-- final gate - `the wall has 2 hole(s)` - because the two functions it
-- creates were born executable by `anon`. 196 fixed how functions are
-- born; this is the same migration, applied after it.

create table public.identities (
  -- THE SIGN-IN IS THE KEY, and that is the load-bearing constraint. One
  -- sign-in belongs to exactly one person, so `current_profile()` can
  -- never be ambiguous. Without this the resolver would need an
  -- `order by` and would be picking a human being arbitrarily.
  auth_uid   uuid primary key references auth.users(id) on delete cascade,
  profile_id uuid not null references public.profiles(id) on delete cascade,

  -- WHICH DOOR THEY CAME IN BY. Not a closed list and not checked: the
  -- providers are Supabase's to add, and a check constraint here would
  -- be a second copy of their list that nothing keeps in step - the trap
  -- 021 paid for. It is recorded so a person can be shown "email and
  -- Google", and so an identity can be revoked by provider.
  provider   text not null check (btrim(provider) <> ''),
  linked_at  timestamptz not null default now()
);

-- "every sign-in I own", which is the lookup an account screen needs.
create index identities_by_profile on public.identities (profile_id);

comment on table public.identities is
  '197. Which sign-in belongs to which person. The only place that knows '
  'auth.users and profiles are related.';

-- ---------------------------------------------------------------------
-- SEED THE SIX, SO THE FALLBACK IS NEVER LOAD-BEARING
-- ---------------------------------------------------------------------
--
-- Every existing profile gets the sign-in it was created from, which is
-- its own id. This is the whole data migration, and it is why this is
-- the right moment: the mapping is the identity function today, so the
-- table can be built without deciding anything.
insert into public.identities (auth_uid, profile_id, provider)
select u.id, p.id, coalesce(u.raw_app_meta_data ->> 'provider', 'email')
  from public.profiles p
  join auth.users u on u.id = p.id
on conflict (auth_uid) do nothing;

-- ---------------------------------------------------------------------
-- THE TRANSLATION
-- ---------------------------------------------------------------------
--
-- THE COALESCE IS THE BRIDGE AND IT IS DELIBERATE: a sign-in with no
-- row in `identities` resolves to itself, which is exactly today's
-- behaviour. So this function is a no-op the moment it is created, a
-- brand-new sign-up works before anything has written an identity row,
-- and the trigger below then fills it in. Nothing has to happen in the
-- right order.
--
-- It is not a hole. Falling back yields a profile id equal to a sign-in
-- id that owns nothing and is a member of no game - the same nothing a
-- stranger sees today.
create or replace function public.current_profile()
returns uuid
language sql
stable
security definer
set search_path to ''
as $fn$
  select coalesce(
    (select i.profile_id from public.identities i where i.auth_uid = auth.uid()),
    auth.uid()
  );
$fn$;

comment on function public.current_profile() is
  '197. The person behind the current sign-in. Policies ask this, not '
  'auth.uid(). Falls back to auth.uid() for a sign-in not yet bridged.';

grant execute on function public.current_profile() to authenticated;

-- A SIGN-IN CAN SEE WHICH PERSON IT IS, AND NOTHING ELSE. Reading the
-- table is how an account screen lists "email, Google"; writing to it is
-- how one sign-in would claim another person, so it is not granted. A
-- real linking flow has to prove possession of both sign-ins, which is
-- a verified round trip and not an INSERT policy.
alter table public.identities enable row level security;

create policy "identities: read own"
  on public.identities for select to authenticated
  using (profile_id = (select public.current_profile()));

-- ---------------------------------------------------------------------
-- KEEP IT COMPLETE WITHOUT ANYONE REMEMBERING TO
-- ---------------------------------------------------------------------
--
-- 193 and 196's lesson applied immediately: a step that has to be
-- remembered is a step that will be forgotten. Sign-up inserts a profile
-- whose id is the sign-in's id (001), so the first identity row is
-- derivable and nothing in the app needs teaching.
create or replace function public.identity_follows_profile()
returns trigger
language plpgsql
security definer
set search_path to ''
as $trg$
begin
  insert into public.identities (auth_uid, profile_id, provider)
  select new.id, new.id,
         coalesce((select u.raw_app_meta_data ->> 'provider'
                     from auth.users u where u.id = new.id), 'email')
   where exists (select 1 from auth.users u where u.id = new.id)
  on conflict (auth_uid) do nothing;
  return new;
end;
$trg$;

-- The `where exists` matters: once the FK below is dropped a profile may
-- be created that no sign-in corresponds to yet - a player the DM adds
-- before they have an account - and that must not fail on the FK here.
create trigger profiles_get_an_identity
  after insert on public.profiles
  for each row execute function public.identity_follows_profile();

-- ---------------------------------------------------------------------
-- THE WALL ITSELF
-- ---------------------------------------------------------------------
--
-- THIS IS THE COMPARTMENTALIZATION, and it is one line. While
-- `profiles.id` references `auth.users(id) ON DELETE CASCADE`, a person
-- is a sign-in and deleting a login takes the human being with it - and
-- on 001's cascades, their characters.
--
-- After this, `auth.users` cascades into `identities` and stops there: a
-- deleted sign-in removes a way of proving who you are and nothing else.
-- A person with two sign-ins who drops one still exists.
--
-- WHAT THIS NOW LEAVES UNDONE, written down rather than discovered:
-- deleting a PERSON is no longer anything the database does by itself.
-- A real account-deletion path - which a commercial release needs for
-- its own legal reasons - has to delete the profile deliberately and
-- decide what happens to a campaign's rows. That is a feature, and it
-- does not exist.
alter table public.profiles drop constraint profiles_id_fkey;

-- ---------------------------------------------------------------------
-- WHAT A NEW ROW'S OWNER IS
-- ---------------------------------------------------------------------
alter table public.actions
  alter column owner_uid set default public.current_profile();

-- ---------------------------------------------------------------------
-- THE TWO PREDICATES EVERY POLICY LEANS ON
-- ---------------------------------------------------------------------
--
-- Re-emitted verbatim but asking the right question. These two are why
-- the policy rewrite below is 42 expressions and not 142: most of the
-- schema reaches membership through them.
create or replace function public.is_game_dm(p_game_id uuid)
returns boolean
language sql
stable
security definer
set search_path to ''
as $fn$
  select exists (select 1 from public.game_members
                 where game_id = p_game_id
                   and profile_id = public.current_profile()
                   and role = 'dm');
$fn$;

create or replace function public.is_game_member(p_game_id uuid)
returns boolean
language sql
stable
security definer
set search_path to ''
as $fn$
  select exists (select 1 from public.game_members
                 where game_id = p_game_id
                   and profile_id = public.current_profile());
$fn$;

create or replace function public.join_game(p_code text)
returns uuid
language plpgsql
security definer
set search_path to ''
as $fn$
declare v_game public.games%rowtype;
begin
  -- STILL `auth.uid()`, and this is the one place that is correct:
  -- the question here is "is anyone signed in", which is a fact about
  -- the SIGN-IN. Which person they are is the next line's business.
  if auth.uid() is null then raise exception 'not authenticated'; end if;
  select * into v_game from public.games where join_code = upper(btrim(p_code));
  if not found then raise exception 'no game with that code'; end if;
  if not v_game.is_open then raise exception 'that game is closed to new players'; end if;
  insert into public.game_members (game_id, profile_id, role)
  values (v_game.id, public.current_profile(), 'player')
  on conflict (game_id, profile_id) do nothing;
  return v_game.id;
end; $fn$;

-- ---------------------------------------------------------------------
-- AND THE 42 POLICIES, GENERATED RATHER THAN TRANSCRIBED
-- ---------------------------------------------------------------------
--
-- THE RECURRING TRAP OF THIS PROJECT IS A SECOND COPY OF SOMETHING THAT
-- NOTHING CHECKS - 021's explicit column list, five times over. Hand-
-- retyping 42 policy expressions to change one function call in each
-- would be that trap at its largest, and a single mistyped predicate is
-- a silent authorization hole rather than a failed build.
--
-- So the expressions are not retyped. Each policy is read back from the
-- catalogue, `auth.uid()` is replaced in the text Postgres itself
-- produced, and the policy is recreated with its own command, roles and
-- permissive flag. The only thing this loop can get wrong is something
-- it would get wrong for all 42 at once, loudly.
--
-- `(select ...)` AROUND THE CALL, NOT A BARE CALL. In a policy a bare
-- function is evaluated per row; wrapped in a scalar subquery the planner
-- hoists it and runs it once per statement. Since `current_profile()`
-- reads a table where `auth.uid()` read a setting, a bare call would
-- have made every policy a per-row lookup - on `objects` that is
-- hundreds. This way it is one.
--
-- AND THE COUNT IS ASSERTED, which earned its place immediately: this
-- migration first went in saying 44, a number taken from reading a
-- listing rather than counting it. The loop found 42, the assertion
-- failed, and the whole thing rolled back instead of half-rewriting the
-- authorization model. The real count is 42, from the catalogue.
do $rw$
declare
  r     record;
  stmt  text;
  n     int := 0;
begin
  for r in
    select c.relname                                  as tbl,
           p.polname,
           p.polcmd,
           p.polpermissive,
           coalesce(nullif(
             (select string_agg(quote_ident(g.rolname), ', ' order by g.rolname)
                from pg_catalog.pg_roles g where g.oid = any(p.polroles)), ''),
             'public')                                as roles,
           pg_catalog.pg_get_expr(p.polqual, p.polrelid)      as q_using,
           pg_catalog.pg_get_expr(p.polwithcheck, p.polrelid) as q_check
      from pg_catalog.pg_policy p
      join pg_catalog.pg_class c on c.oid = p.polrelid
     where c.relnamespace = 'public'::regnamespace
       and (coalesce(pg_catalog.pg_get_expr(p.polqual, p.polrelid), '')
            || coalesce(pg_catalog.pg_get_expr(p.polwithcheck, p.polrelid), ''))
           like '%auth.uid()%'
     order by c.relname, p.polname
  loop
    execute format('drop policy %I on public.%I', r.polname, r.tbl);

    stmt := format('create policy %I on public.%I as %s for %s to %s',
                   r.polname, r.tbl,
                   case when r.polpermissive then 'permissive' else 'restrictive' end,
                   case r.polcmd when 'r' then 'select'
                                 when 'a' then 'insert'
                                 when 'w' then 'update'
                                 when 'd' then 'delete'
                                 else 'all' end,
                   r.roles);

    if r.q_using is not null then
      stmt := stmt || format(' using (%s)',
        replace(r.q_using, 'auth.uid()', '(select public.current_profile())'));
    end if;
    if r.q_check is not null then
      stmt := stmt || format(' with check (%s)',
        replace(r.q_check, 'auth.uid()', '(select public.current_profile())'));
    end if;

    execute stmt;
    n := n + 1;
  end loop;

  raise notice '197: rewrote % policies', n;

  if n <> 42 then
    raise exception '197: expected 42 policies to rewrite, found %', n;
  end if;
end
$rw$;

-- ---------------------------------------------------------------------
-- PROVE IT MOVED NOTHING
-- ---------------------------------------------------------------------
do $gate$
declare
  n int;
begin
  -- 1. The bridge is the identity function today, which is what makes
  --    this migration a no-op rather than a change of ownership.
  select count(*) into n from public.identities where profile_id <> auth_uid;
  if n > 0 then
    raise exception '197: % identity rows are not self-mapping; this was '
                    'supposed to be a no-op', n;
  end if;

  select count(*) into n from public.identities;
  if n <> (select count(*) from public.profiles) then
    raise exception '197: % identities for % profiles', n,
                    (select count(*) from public.profiles);
  end if;

  -- 2. Nothing in the schema still asks "which sign-in" when it means
  --    "which person".
  select count(*) into n
    from pg_catalog.pg_policy p
    join pg_catalog.pg_class c on c.oid = p.polrelid
   where c.relnamespace = 'public'::regnamespace
     and (coalesce(pg_catalog.pg_get_expr(p.polqual, p.polrelid), '')
          || coalesce(pg_catalog.pg_get_expr(p.polwithcheck, p.polrelid), ''))
         like '%auth.uid()%';
  if n > 0 then
    raise exception '197: % policies still compare against auth.uid()', n;
  end if;

  -- 3. And they all ask the new question.
  select count(*) into n
    from pg_catalog.pg_policy p
    join pg_catalog.pg_class c on c.oid = p.polrelid
   where c.relnamespace = 'public'::regnamespace
     and (coalesce(pg_catalog.pg_get_expr(p.polqual, p.polrelid), '')
          || coalesce(pg_catalog.pg_get_expr(p.polwithcheck, p.polrelid), ''))
         like '%current_profile()%';
  raise notice '197: % policies now resolve the person', n;

  -- 4. 195's check, which this migration has just added a function and
  --    a table to.
  select count(*) into n from public.security_doors();
  if n > 0 then
    raise exception '197: the wall has % hole(s)', n;
  end if;
end
$gate$;
