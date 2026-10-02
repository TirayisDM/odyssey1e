-- 087. WHAT A CLASS GIVES YOU, AND WHEN.
--
-- 055 gave a class a hit die, saves and proficiencies - everything a
-- character gets at level one and nothing they get afterwards. A
-- Fighter 5 and a Fighter 1 differed only in hit points.
--
-- ---------------------------------------------------------------------
-- DERIVED, NEVER ACCUMULATED
-- ---------------------------------------------------------------------
--
-- A character's features are not granted and stored - they are WHAT
-- THEIR CLASS ROWS SAY AT THEIR CURRENT LEVELS, computed on every read.
-- Nothing to hand out at level-up and nothing to forget; dropping a
-- level takes its features with it without a reversal step, and a DM
-- correcting the catalogue corrects every character at once.
--
-- The one thing that cannot be derived is a CHOICE somebody made, and
-- that is the only thing `character_choices` stores.
--
-- ---------------------------------------------------------------------
-- MOST FEATURES ARE NOT A CHOICE
-- ---------------------------------------------------------------------
--
-- Second Wind, Action Surge, Sneak Attack, Rage - they arrive. If every
-- feature were a dropdown a player would click OK through a career. The
-- PHB makes you decide in four places and `choose_from` names which:
--
--   ability          an Ability Score Improvement, 2 picks of +1
--   skill            Expertise, 2 picks
--   fighting_style   Fighter 1, Ranger 2, Paladin 2
--   subclass         one level per class
--
-- NULL means it simply happens, which is most rows.
--
-- FEATS ARE OUT, by instruction - they are a PHB OPTIONAL rule and the
-- default is ability scores only. That is why an ASI has no "or a feat"
-- branch here: adding one later is a value in `choose_from` and a list
-- to choose from, not a change to this shape.
--
-- SUBCLASS FEATURES ARE OUT OF THIS PASS. The choice POINT is seeded so
-- a sheet says "you pick an archetype at 3", with `choose_from` left
-- NULL until there are archetypes to offer - a dropdown with nothing in
-- it is worse than a sentence.

create table if not exists class_features (
  id          uuid primary key default gen_random_uuid(),
  game_id     uuid references games(id) on delete cascade,
  class_key   text not null,
  level       integer not null,
  key         text not null,
  name        text not null,
  text        text,
  choose_from text,
  picks       integer not null default 1,
  constraint class_features_level_range check (level between 1 and 20),
  constraint class_features_choose_from_check
    check (choose_from is null or choose_from in
      ('ability', 'skill', 'fighting_style', 'subclass')),
  constraint class_features_picks_positive check (picks >= 1)
);

comment on table class_features is
  'What a class grants, by level. Reference data, tenanted like classes and items: game_id NULL is the catalogue every game reads. A character''s features are DERIVED from these and their class levels on every read rather than granted and stored - so dropping a level takes its features with it and a corrected catalogue corrects everybody.';
comment on column class_features.key is
  'Stable identifier for this feature, unique within a class. What character_choices points at, which is why it must not be renamed once somebody has chosen against it.';
comment on column class_features.text is
  'A short functional summary IN OUR OWN WORDS. Deliberately not the book''s wording - this says what the feature does so a player at the table does not have to look it up, and nothing more.';
comment on column class_features.choose_from is
  'What list this feature makes you choose from: ability, skill, fighting_style, subclass. NULL means it simply happens, which is most features - 5e only makes you decide in a handful of places and a dropdown on every row would be a career of clicking OK.';
comment on column class_features.picks is
  'How many to choose. An Ability Score Improvement is 2 picks of +1, which is exactly 5e''s "one score by 2 or two scores by 1". Expertise is 2 skills. Ignored when choose_from is NULL.';

create unique index if not exists class_features_global_key
  on class_features (class_key, key) where game_id is null;
create unique index if not exists class_features_game_key
  on class_features (game_id, class_key, key) where game_id is not null;
create index if not exists class_features_by_class
  on class_features (class_key, level);

alter table class_features enable row level security;

drop policy if exists "class_features: read global or own game" on class_features;
create policy "class_features: read global or own game" on class_features
  for select using (game_id is null or is_game_member(game_id));

drop policy if exists "class_features: dm writes own game" on class_features;
create policy "class_features: dm writes own game" on class_features
  for insert with check (game_id is not null and is_game_dm(game_id));

drop policy if exists "class_features: dm updates own game" on class_features;
create policy "class_features: dm updates own game" on class_features
  for update using (game_id is not null and is_game_dm(game_id))
  with check (game_id is not null and is_game_dm(game_id));

drop policy if exists "class_features: dm deletes own game" on class_features;
create policy "class_features: dm deletes own game" on class_features
  for delete using (game_id is not null and is_game_dm(game_id));

-- WHAT SOMEBODY DECIDED. The only part of a feature that is not
-- derivable, and therefore the only part stored.
--
-- LOCKED ONCE SET, which is 5e: retraining is at the DM's say-so rather
-- than a player's. The lock is in the command, not here - a check
-- constraint cannot tell a player from a DM, and the policy already
-- can.
create table if not exists character_choices (
  id           uuid primary key default gen_random_uuid(),
  character_id uuid not null references characters(id) on delete cascade,
  class_key    text not null,
  feature_key  text not null,
  -- WHICH ONE OF THE PICKS. An ASI is two separate +1s and they may
  -- land on different abilities, so a feature can hold more than one
  -- row and `pick` keeps them apart.
  pick         integer not null default 1,
  choice       text not null,
  chosen_at    timestamptz not null default now(),
  unique (character_id, class_key, feature_key, pick)
);

comment on table character_choices is
  'What a character chose where a feature made them choose. The only part of a feature that is not derived from the catalogue and their level, and so the only part stored. Locked once set - 5e retrains at the DM''s say-so, and the command enforces that rather than a constraint, because a constraint cannot tell a player from a DM.';
comment on column character_choices.pick is
  'Which of the feature''s picks this is. An Ability Score Improvement is two +1s that may land on different abilities, so one feature holds two rows.';
comment on column character_choices.choice is
  'The chosen value, by key: an ability code, a skill key, a fighting style key, a subclass key. By value and with no FK, for 055''s reason - the partial unique indexes that carry tenancy cannot back one.';

alter table character_choices enable row level security;

drop policy if exists "character_choices: read with character" on character_choices;
create policy "character_choices: read with character" on character_choices
  for select using (exists (
    select 1 from characters c
    where c.id = character_choices.character_id and is_game_member(c.game_id)));

drop policy if exists "character_choices: owner or dm writes" on character_choices;
create policy "character_choices: owner or dm writes" on character_choices
  for insert with check (exists (
    select 1 from characters c
    where c.id = character_choices.character_id
      and (c.owner_uid = auth.uid() or is_game_dm(c.game_id))));

drop policy if exists "character_choices: dm updates" on character_choices;
create policy "character_choices: dm updates" on character_choices
  for update using (exists (
    select 1 from characters c
    where c.id = character_choices.character_id and is_game_dm(c.game_id)))
  with check (exists (
    select 1 from characters c
    where c.id = character_choices.character_id and is_game_dm(c.game_id)));

drop policy if exists "character_choices: dm deletes" on character_choices;
create policy "character_choices: dm deletes" on character_choices
  for delete using (exists (
    select 1 from characters c
    where c.id = character_choices.character_id and is_game_dm(c.game_id)));
