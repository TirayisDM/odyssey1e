-- =====================================================================
-- 005_character_sheet.sql
-- odyssey1e — the minimum a named roll needs to resolve
-- =====================================================================
--
-- "insight" becomes a formula only when the engine can find four things:
-- which ability the skill keys off, that ability's score, whether the
-- character is proficient, and the proficiency bonus. Three tables.
--
-- PROFICIENCY BONUS IS NOT STORED. It is floor((level-1)/4)+2, a pure
-- function of level, computed in Rust with the rest of the rules. The
-- database holds facts; the engine holds arithmetic. A generated column
-- would put one rule in a second place, and the two would drift.
--
-- Same reasoning for ability modifiers: floor((score-10)/2) lives in the
-- engine, not the schema.
--
-- FIRST TENANTED-NULLABLE REFERENCE TABLE. public.skills establishes the
-- pattern declared in 001: game_id NULL is global content every game
-- shares, game_id SET is this campaign only. A lookup prefers the
-- game-scoped row and falls back to global, so a DM can add a house
-- skill without forking the 18 rows everyone else uses.
--
-- !! THE WRINKLE IN THAT PATTERN — every reference table from 006 on
-- will hit it. A PRIMARY KEY CANNOT CONTAIN A NULLABLE COLUMN.
-- `primary key (key, game_id)` silently forces game_id NOT NULL and
-- destroys the design; the first attempt at this migration failed
-- exactly there. Hence a surrogate id, plus TWO partial unique indexes
-- to enforce what the composite key was supposed to.
-- The second half of the same trap: NULL is not equal to NULL in a
-- unique constraint, so without the partial index two global rows could
-- share a key and neither would be rejected.
-- =====================================================================

create type public.ability_code as enum ('str','dex','con','int','wis','cha');

comment on type public.ability_code is
  'Machine vocabulary, exact and lowercase. Matches ABIL_NAMES in the Apps Script engine so ported rules need no translation layer.';


-- =====================================================================
-- SKILL CATALOGUE  (reference data — see the tenancy note above)
-- =====================================================================

create table public.skills (
  id          uuid primary key default gen_random_uuid(),
  key         text not null,
  game_id     uuid references public.games(id) on delete cascade,
  name        text not null,
  ability     public.ability_code not null,
  sort_order  integer not null default 0
);

comment on table public.skills is
  'Skill catalogue. game_id NULL is global content shared by every game; a row with game_id set overrides it for that campaign only. The surrogate id exists because a primary key cannot contain a nullable column — see the migration header.';
comment on column public.skills.key is
  'Three-letter code, exact: acr ani arc ath dec his ins itm inv med nat prc prf per rel slt ste sur. Carried over from SKILL_NAMES in diceroller.js so old data and new agree.';
comment on column public.skills.game_id is
  'NULL means global. Set means this campaign only, shadowing the global row with the same key.';
comment on column public.skills.ability is
  'The ability this skill keys off. A character does not override it — that is a house rule, and a house rule is a game-scoped row here.';

-- One global row per key. A plain UNIQUE would not do this: NULL is not
-- equal to NULL, so every global row would look distinct.
create unique index skills_global_key_idx
  on public.skills(key) where game_id is null;

-- One override per key per game.
create unique index skills_game_key_idx
  on public.skills(key, game_id) where game_id is not null;

create index skills_game_idx on public.skills(game_id);

alter table public.skills enable row level security;

create policy "skills: read global or own game"
  on public.skills for select
  using (game_id is null or public.is_game_member(game_id));

create policy "skills: dm writes own game"
  on public.skills for insert
  with check (game_id is not null and public.is_game_dm(game_id));

create policy "skills: dm updates own game"
  on public.skills for update
  using (game_id is not null and public.is_game_dm(game_id))
  with check (game_id is not null and public.is_game_dm(game_id));

create policy "skills: dm deletes own game"
  on public.skills for delete
  using (game_id is not null and public.is_game_dm(game_id));

-- Note there is no path to writing a GLOBAL row. Shared content is
-- seeded by migration, never by a client. A DM who wants a variant adds
-- a game-scoped row that shadows it.


-- =====================================================================
-- CHARACTER LEVEL
-- =====================================================================

alter table public.characters
  add column level integer not null default 1
    check (level between 1 and 20);

comment on column public.characters.level is
  'Total class level, 1-20. The engine derives proficiency bonus from it: floor((level-1)/4)+2. Deliberately not stored or generated — see the migration header.';


-- =====================================================================
-- ABILITY SCORES
-- =====================================================================

create table public.character_abilities (
  character_id  uuid not null references public.characters(id) on delete cascade,
  ability       public.ability_code not null,
  score         integer not null default 10 check (score between 1 and 30),
  save_prof     boolean not null default false,
  primary key (character_id, ability)
);

comment on table public.character_abilities is
  'Six rows per character, seeded on creation. The modifier is floor((score-10)/2), computed in the engine rather than stored — same reasoning as proficiency bonus.';
comment on column public.character_abilities.save_prof is
  'Proficient in this saving throw. A save adds the full proficiency bonus or nothing; 5e has no half-proficiency on saves.';

alter table public.character_abilities enable row level security;

-- Visibility follows the parent character rather than repeating the
-- membership test. characters is itself protected, so a character you
-- cannot see yields no ability rows.
create policy "character_abilities: read with character"
  on public.character_abilities for select
  using (exists (select 1 from public.characters c
                 where c.id = character_id and public.is_game_member(c.game_id)));

create policy "character_abilities: owner or dm writes"
  on public.character_abilities for insert
  with check (exists (select 1 from public.characters c
                      where c.id = character_id
                        and (c.owner_uid = auth.uid() or public.is_game_dm(c.game_id))));

create policy "character_abilities: owner or dm updates"
  on public.character_abilities for update
  using (exists (select 1 from public.characters c
                 where c.id = character_id
                   and (c.owner_uid = auth.uid() or public.is_game_dm(c.game_id))))
  with check (exists (select 1 from public.characters c
                      where c.id = character_id
                        and (c.owner_uid = auth.uid() or public.is_game_dm(c.game_id))));

create policy "character_abilities: owner or dm deletes"
  on public.character_abilities for delete
  using (exists (select 1 from public.characters c
                 where c.id = character_id
                   and (c.owner_uid = auth.uid() or public.is_game_dm(c.game_id))));


-- =====================================================================
-- SKILL PROFICIENCY
-- =====================================================================

create table public.character_skills (
  character_id  uuid not null references public.characters(id) on delete cascade,
  skill_key     text not null,
  prof          numeric(2,1) not null default 0 check (prof in (0, 0.5, 1, 2)),
  primary key (character_id, skill_key)
);

comment on table public.character_skills is
  'Proficiency per skill. A row is only needed where prof is non-zero — an absent row means untrained, which is the common case and not worth storing 18 rows of nothing.';
comment on column public.character_skills.prof is
  'Multiplier on the proficiency bonus: 0 untrained, 0.5 half (Jack of All Trades), 1 proficient, 2 expertise. The engine floors the product, matching Math.floor(s.prof * ctx.pb) in the original.';
comment on column public.character_skills.skill_key is
  'References skills.key by value, not by foreign key — a global skill and a game override share a key, so an FK could not point at one of them unambiguously.';

alter table public.character_skills enable row level security;

create policy "character_skills: read with character"
  on public.character_skills for select
  using (exists (select 1 from public.characters c
                 where c.id = character_id and public.is_game_member(c.game_id)));

create policy "character_skills: owner or dm writes"
  on public.character_skills for insert
  with check (exists (select 1 from public.characters c
                      where c.id = character_id
                        and (c.owner_uid = auth.uid() or public.is_game_dm(c.game_id))));

create policy "character_skills: owner or dm updates"
  on public.character_skills for update
  using (exists (select 1 from public.characters c
                 where c.id = character_id
                   and (c.owner_uid = auth.uid() or public.is_game_dm(c.game_id))))
  with check (exists (select 1 from public.characters c
                      where c.id = character_id
                        and (c.owner_uid = auth.uid() or public.is_game_dm(c.game_id))));

create policy "character_skills: owner or dm deletes"
  on public.character_skills for delete
  using (exists (select 1 from public.characters c
                 where c.id = character_id
                   and (c.owner_uid = auth.uid() or public.is_game_dm(c.game_id))));


-- =====================================================================
-- SEED: the 18 standard skills, as global content
-- =====================================================================
-- Keys and default abilities lifted directly from SKILL_NAMES in
-- diceroller.js. Idempotent — the conflict target is the partial index.

insert into public.skills (key, game_id, name, ability, sort_order) values
  ('acr', null, 'Acrobatics',      'dex',  1),
  ('ani', null, 'Animal Handling', 'wis',  2),
  ('arc', null, 'Arcana',          'int',  3),
  ('ath', null, 'Athletics',       'str',  4),
  ('dec', null, 'Deception',       'cha',  5),
  ('his', null, 'History',         'int',  6),
  ('ins', null, 'Insight',         'wis',  7),
  ('itm', null, 'Intimidation',    'cha',  8),
  ('inv', null, 'Investigation',   'int',  9),
  ('med', null, 'Medicine',        'wis', 10),
  ('nat', null, 'Nature',          'int', 11),
  ('prc', null, 'Perception',      'wis', 12),
  ('prf', null, 'Performance',     'cha', 13),
  ('per', null, 'Persuasion',      'cha', 14),
  ('rel', null, 'Religion',        'int', 15),
  ('slt', null, 'Sleight of Hand', 'dex', 16),
  ('ste', null, 'Stealth',         'dex', 17),
  ('sur', null, 'Survival',        'wis', 18)
on conflict (key) where game_id is null do nothing;


-- =====================================================================
-- SEED A CHARACTER'S ABILITIES
-- =====================================================================
-- Six rows at 10 each the moment a character is created, so the engine
-- never meets a half-built sheet. A missing ability row would read as a
-- modifier of zero — the same answer a score of 10 gives, but by
-- coincidence rather than by design, and coincidence is a bad thing to
-- build crit detection on.

create function public.seed_character_abilities()
returns trigger
language plpgsql
security definer set search_path = ''
as $$
begin
  insert into public.character_abilities (character_id, ability)
  select new.id, a
  from unnest(enum_range(null::public.ability_code)) as a
  on conflict do nothing;
  return new;
end;
$$;

create trigger characters_seed_abilities
  after insert on public.characters
  for each row execute function public.seed_character_abilities();

revoke all on function public.seed_character_abilities() from public, anon, authenticated;


-- =====================================================================
-- BACKFILL (run once, after applying, for characters created earlier)
-- =====================================================================
-- Not part of the migration proper — the trigger only fires on new
-- rows. Safe to re-run.
--
--   insert into public.character_abilities (character_id, ability)
--   select c.id, a
--   from public.characters c
--   cross join unnest(enum_range(null::public.ability_code)) as a
--   on conflict do nothing;
-- =====================================================================
