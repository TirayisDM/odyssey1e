-- =====================================================================
-- 011_encounters.sql
-- odyssey1e — the things a roll can be aimed at
-- =====================================================================
--
-- 009 gave a roll a target and left the number to a keyboard. This is
-- where the number comes from instead.
--
-- THE INTERFACE ALREADY EXISTS. A roll takes exactly three things:
-- target_value, target_kind, target_label. An actor supplies an `ac`, a
-- challenge supplies a `dc`, and nothing in the roll path changes.
-- "Target or attempt" is a distinction the UI draws; attacking a goblin
-- and picking a lock are the same operation against different numbers.
--
-- FOUR TABLES.
--
-- encounters            the container. One per fight, scene or room.
-- npcs                  statblock catalogue. Reusable and duplicatable:
--                       one goblin, three instances.
-- encounter_actors      who is present - a player character OR an NPC
--                       instance. One table, see below.
-- encounter_challenges  the DM's authored difficulties: a lock, a save,
--                       a check.
--
-- ONE TABLE FOR ACTORS, NOT TWO.
--
-- An actor is a character or an NPC, never both, enforced by an XOR
-- check. Two junction tables would have meant every target list being a
-- UNION and every consumer handling two shapes; one table keeps the
-- list a list. The cost is two nullable columns, which the check makes
-- honest.
--
-- NO AC OR HP COPIED ONTO AN ACTOR.
--
-- A character's AC is computed from what they are wearing and moves
-- when they change armour - 010 exists precisely so that number is
-- never stored. An NPC's comes from its statblock. The actor row holds
-- identity and per-instance OVERRIDES, and the rest is derived at read
-- time. A goblin that picked up a shield gets an ac_override; the other
-- two do not.
--
-- npc_key IS BY VALUE, NOT A FOREIGN KEY. Same reason as
-- character_items.item_key and dice_faces.set_key: npcs carries a
-- nullable game_id so a campaign can shadow a global statblock, which
-- means key is unique only within each partition, which means a partial
-- unique index, which cannot back an FK. See 008's header.
--
-- ONE LIVE ENCOUNTER PER GAME, enforced by a partial unique index on
-- status - the same mechanism that guarantees one equipped dice set.
-- Drafts and ended encounters are unlimited.
--
-- TWO COLUMNS NOTHING READS YET, DELIBERATELY.
--
-- `initiative` on an actor, and `reset_at` on a challenge. Both are
-- single nullable columns on tables being created anyway, and both
-- encode a decision already taken rather than a guess: initiative order
-- is part of encounters and players will be prompted to roll it on
-- enrollment; challenge state is DERIVED from the rolls aimed at it,
-- with the DM able to reset.
--
-- reset_at is how "derived" and "resettable" coexist. A challenge is
-- passed if a successful roll targeted it AFTER reset_at, so resetting
-- is moving a watermark rather than editing history - the rolls stay,
-- the lock is simply open for business again. Nothing computes that
-- until rolls can reference a challenge, which is the next migration.
--
-- This is a narrower judgement than it looks: two columns for settled
-- decisions is not the thirty-column dossier that 010 refused, which
-- served a narrator that does not exist.
-- =====================================================================

create table public.encounters (
  id          uuid primary key default gen_random_uuid(),
  game_id     uuid not null references public.games(id) on delete cascade,
  name        text not null,
  status      text not null default 'draft'
                check (status in ('draft', 'active', 'ended')),
  created_at  timestamptz not null default now()
);

comment on table public.encounters is
  'A fight, a scene, a room - whatever collection of targets is in front of the table right now. The container; the things in it are actors and challenges.';
comment on column public.encounters.status is
  'draft while the DM builds it, active while it is in front of players, ended afterwards. At most one active per game, by partial unique index - the same mechanism that keeps one dice set equipped.';

create unique index encounters_one_active_per_game_idx
  on public.encounters(game_id) where status = 'active';
create index encounters_game_idx on public.encounters(game_id, created_at desc);

-- ---------------------------------------------------------------------

create table public.npcs (
  id         uuid primary key default gen_random_uuid(),
  key        text not null,
  game_id    uuid references public.games(id) on delete cascade,
  name       text not null,
  ac         integer not null check (ac >= 0),
  hp_max     integer not null check (hp_max >= 1),
  size       text check (size is null or size in ('tiny','sm','med','lg','huge','grg')),
  notes      text,
  image_url  text
);

comment on table public.npcs is
  'Statblock catalogue. Reusable and duplicatable: one goblin row, three instances in an encounter, each with its own hit points. Same catalogue-plus-instance shape as items and character_items.';
comment on column public.npcs.key is
  'Stable slug, referenced by value from encounter_actors.npc_key. Not an FK - a global statblock and a campaign override share a key, so key is unique only within each partition. See 008.';
comment on column public.npcs.game_id is
  'NULL means global. Set means this campaign only.';
comment on column public.npcs.ac is
  'Stored, unlike a character''s. An NPC has no equipment model to compute from, and inventing one to hold a goblin''s 15 would be ceremony. If NPCs ever wear real armour this becomes the same computed question 010 answered.';
comment on column public.npcs.hp_max is
  'Maximum hit points for one instance. Current HP is this less the damage events aimed at that instance, so two goblins from one statblock take damage independently.';

create unique index npcs_global_key_idx on public.npcs(key) where game_id is null;
create unique index npcs_game_key_idx   on public.npcs(key, game_id) where game_id is not null;

-- ---------------------------------------------------------------------

create table public.encounter_actors (
  id            uuid primary key default gen_random_uuid(),
  encounter_id  uuid not null references public.encounters(id) on delete cascade,
  character_id  uuid references public.characters(id) on delete cascade,
  npc_key       text,
  label         text not null,
  initiative    integer,
  ac_override   integer,
  hp_override   integer,
  active        boolean not null default true,
  enrolled_at   timestamptz not null default now(),

  -- Exactly one of the two, never both, never neither. The whole reason
  -- one table can hold both kinds of participant.
  constraint encounter_actors_one_kind_check
    check ((character_id is null) <> (npc_key is null))
);

comment on table public.encounter_actors is
  'Who is in the encounter. A player character or an NPC instance, never both - see the XOR check. Enrollment is the DM''s call, not automatic: a character off scouting is not in the fight.';
comment on column public.encounter_actors.npc_key is
  'References npcs.key by value. See npcs.key for why not an FK. NULL when this actor is a player character.';
comment on column public.encounter_actors.label is
  'What the players see and what a roll snapshots: Goblin 1, Goblin 2. The per-instance name, which is what makes one statblock duplicatable.';
comment on column public.encounter_actors.initiative is
  'Turn order. NOTHING READS THIS YET. Initiative is part of encounters and players will be prompted to roll on enrollment; the column is here because it is one column on a table being created anyway and the decision is already taken. NULL means not yet rolled, which is not the same as rolling zero.';
comment on column public.encounter_actors.ac_override is
  'Per-instance AC, when this one goblin differs. NULL means derive: a character''s computed AC from 010, or the statblock''s. AC is NOT copied here on enrollment - a character who changes armour mid-fight should change what it takes to hit them.';
comment on column public.encounter_actors.hp_override is
  'Per-instance maximum HP, for the ogre that is nearly dead when the scene opens. NULL derives from the statblock or the character.';
comment on column public.encounter_actors.active is
  'False removes an actor from the target list without deleting the row, so the rolls that named it still make sense.';

create index encounter_actors_encounter_idx
  on public.encounter_actors(encounter_id) where active;
create index encounter_actors_character_idx
  on public.encounter_actors(character_id) where character_id is not null;

-- ---------------------------------------------------------------------

create table public.encounter_challenges (
  id            uuid primary key default gen_random_uuid(),
  encounter_id  uuid not null references public.encounters(id) on delete cascade,
  label         text not null,
  dc            integer not null check (dc >= 1),
  skill_key     text,
  active        boolean not null default true,
  reset_at      timestamptz,
  created_at    timestamptz not null default now()
);

comment on table public.encounter_challenges is
  'The DM''s authored difficulties: a lock, a save, a check. A challenge supplies a dc exactly as an actor supplies an ac, and the roll path cannot tell them apart.';
comment on column public.encounter_challenges.label is
  'What the players see and what a roll snapshots: "The iron lock".';
comment on column public.encounter_challenges.skill_key is
  'The check this is meant to invite, as a skills.key - slt for a lock, ath for a stuck door. Guidance for the card, not a restriction: the DM may allow anything, and nothing enforces it.';
comment on column public.encounter_challenges.reset_at is
  'THE DM''S WATERMARK, AND NOTHING READS IT YET. Whether a challenge has been passed is DERIVED from the rolls aimed at it rather than stored, so there is no bookkeeping to forget. Resetting moves this timestamp instead of editing history: only successes AFTER it count, the rolls all stay, and the lock is simply open for business again. The derivation needs rolls to reference a challenge, which is the next migration.';

create index encounter_challenges_encounter_idx
  on public.encounter_challenges(encounter_id) where active;

-- ---------------------------------------------------------------------
-- RLS. Members read what is in their game; the DM writes it.
-- ---------------------------------------------------------------------

alter table public.encounters           enable row level security;
alter table public.npcs                 enable row level security;
alter table public.encounter_actors     enable row level security;
alter table public.encounter_challenges enable row level security;

create policy "encounters: members read"
  on public.encounters for select using (public.is_game_member(game_id));
create policy "encounters: dm writes"
  on public.encounters for insert with check (public.is_game_dm(game_id));
create policy "encounters: dm updates"
  on public.encounters for update using (public.is_game_dm(game_id))
                        with check (public.is_game_dm(game_id));
create policy "encounters: dm deletes"
  on public.encounters for delete using (public.is_game_dm(game_id));

-- Same shape as items: global rows are seeded by migration and nothing
-- in a client can write one.
create policy "npcs: read global or own game"
  on public.npcs for select
  using (game_id is null or public.is_game_member(game_id));
create policy "npcs: dm writes own game"
  on public.npcs for insert
  with check (game_id is not null and public.is_game_dm(game_id));
create policy "npcs: dm updates own game"
  on public.npcs for update
  using (game_id is not null and public.is_game_dm(game_id))
  with check (game_id is not null and public.is_game_dm(game_id));
create policy "npcs: dm deletes own game"
  on public.npcs for delete
  using (game_id is not null and public.is_game_dm(game_id));

create policy "encounter_actors: read with encounter"
  on public.encounter_actors for select
  using (exists (select 1 from public.encounters e
                 where e.id = encounter_id and public.is_game_member(e.game_id)));
create policy "encounter_actors: dm writes"
  on public.encounter_actors for insert
  with check (exists (select 1 from public.encounters e
                      where e.id = encounter_id and public.is_game_dm(e.game_id)));
create policy "encounter_actors: dm updates"
  on public.encounter_actors for update
  using (exists (select 1 from public.encounters e
                 where e.id = encounter_id and public.is_game_dm(e.game_id)))
  with check (exists (select 1 from public.encounters e
                      where e.id = encounter_id and public.is_game_dm(e.game_id)));
create policy "encounter_actors: dm deletes"
  on public.encounter_actors for delete
  using (exists (select 1 from public.encounters e
                 where e.id = encounter_id and public.is_game_dm(e.game_id)));

create policy "encounter_challenges: read with encounter"
  on public.encounter_challenges for select
  using (exists (select 1 from public.encounters e
                 where e.id = encounter_id and public.is_game_member(e.game_id)));
create policy "encounter_challenges: dm writes"
  on public.encounter_challenges for insert
  with check (exists (select 1 from public.encounters e
                      where e.id = encounter_id and public.is_game_dm(e.game_id)));
create policy "encounter_challenges: dm updates"
  on public.encounter_challenges for update
  using (exists (select 1 from public.encounters e
                 where e.id = encounter_id and public.is_game_dm(e.game_id)))
  with check (exists (select 1 from public.encounters e
                      where e.id = encounter_id and public.is_game_dm(e.game_id)));
create policy "encounter_challenges: dm deletes"
  on public.encounter_challenges for delete
  using (exists (select 1 from public.encounters e
                 where e.id = encounter_id and public.is_game_dm(e.game_id)));
