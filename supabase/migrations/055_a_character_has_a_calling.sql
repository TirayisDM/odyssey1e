-- =====================================================================
-- 055_a_character_has_a_calling.sql
-- odyssey1e — the twelve classes, and the hit die that comes with them
-- =====================================================================
--
-- `create_character` takes a name and nothing else. Everything a
-- character IS arrives later or never, and "never" is the common case:
-- Snot and Unnamed were made through that command and stood for weeks
-- with no size, therefore no hit die, therefore NO HIT POINTS AT ALL.
-- Not zero - null, which the sheet renders as an empty space where a
-- character's life should be.
--
-- 029 said LEVEL IS HIT DICE and made that true for monsters, where the
-- die comes from size because the Monster Manual writes it that way.
-- vitality.rs has carried the other half as a comment ever since:
--
--     WHAT THIS IS NOT. A player character's hit points are not this.
--     5e maxes a PC's first hit die and rolls the rest, and the die
--     comes from class rather than size.
--
-- This is the table that comment was waiting for. A class is where a
-- PC's die comes from, and once a character has one, hit points are
-- derived rather than stated - the same move 029 made for monsters,
-- which is what turned `hp_max` from a magic number into something a
-- level button could recompute.
--
-- ---------------------------------------------------------------------
-- A CATALOGUE, ON THE PATTERN THE OTHERS USE
-- ---------------------------------------------------------------------
--
-- Nullable tenancy, exactly as `items` and `skills` have it since 004:
-- `game_id IS NULL` is the SRD row everyone sees, a row with a game_id
-- is that table's own version, and two partial unique indexes keep one
-- key unique in each space. A DM who wants a Fighter with a d12 writes
-- their own `fighter` row and their game sees it instead.
--
-- `characters.class_key` therefore references BY VALUE and carries no
-- foreign key, for the reason 004 recorded and 027 repeated: a partial
-- unique index cannot back an FK, and the two partial indexes are the
-- point of the design rather than an accident of it.
--
-- ---------------------------------------------------------------------
-- THE VOCABULARY IS THE ONE `is_proficient` ALREADY READS
-- ---------------------------------------------------------------------
--
-- This is the part that would have silently failed, so it was checked
-- against the live table rather than assumed. equipment::is_proficient
-- matches a weapon on `weapon_class` reduced to a prefix - `sim` or
-- `mar`, never "simple" - or on an exact item key, and armour on
-- `armor_category`, which is `lgt`/`med`/`hvy`/`shl`. Skills are the
-- three-letter keys in `skills.key`.
--
-- Seeding "simple" or "light" here would have produced a Fighter who
-- is proficient with nothing, with no error anywhere - the defect class
-- STATUS.md names as this codebase's most expensive, and the same shape
-- as the numeric trap that fired three times before it was cornered.
--
-- ---------------------------------------------------------------------
-- WHAT IS DELIBERATELY NOT HERE
-- ---------------------------------------------------------------------
--
-- NO SUBCLASSES. Champion, Thief, Life Domain. They arrive at level 3
-- for most classes and each one is a bundle of features, so they want
-- their own table keyed to this one - not twelve more rows pretending
-- to be classes.
--
-- NO CLASS FEATURES. Second Wind, Sneak Attack, Rage. Every one is a
-- rule with its own resource and its own timing, and a `features text[]`
-- column would be a list of words no code could act on. 054 made this
-- argument about action economy and it holds here: one honest column
-- beats a dozen decorative ones.
--
-- NO SPELLCASTING. Six of the twelve cast, with three different
-- progressions, and there is no spell table to point at yet.
--
-- NO MULTICLASSING. `class_key` is one value. 5e multiclassing needs
-- levels PER class, which is a join table, and building it before a
-- single character has one class would be inventing a problem.
--
-- NO ABILITY SCORE IMPROVEMENTS, and no starting equipment: the first
-- is a level-up event and the second wants the inventory to exist for
-- new characters, which is 026 through 049's work meeting this one.
--
-- What this gives is the thing that was missing: a character who knows
-- what they are, and therefore how much life they have.
-- =====================================================================

-- ---------------------------------------------------------------------
-- THE CATALOGUE
-- ---------------------------------------------------------------------

create table if not exists public.classes (
  id             uuid primary key default gen_random_uuid(),
  game_id        uuid references public.games(id) on delete cascade,
  key            text not null,
  name           text not null,
  hit_die        integer not null,
  primary_abilities text[] not null default '{}',
  saving_throws  text[]   not null default '{}',
  armor_profs    text[]   not null default '{}',
  weapon_profs   text[]   not null default '{}',
  skill_choices  integer  not null default 2,
  skill_options  text[]   not null default '{}',
  description    text,
  created_at     timestamptz not null default now(),

  -- The six dice a creature can be made of. Same ladder as 029's
  -- sizes, stated again because a class picks from it freely: a
  -- Barbarian is d12 at any size.
  constraint classes_hit_die_is_a_die
    check (hit_die in (4, 6, 8, 10, 12, 20)),

  -- Two saves, as every class in the book has. Not enforced as
  -- exactly two - a homebrew class with three is a DM's business -
  -- but a class with none is a row somebody forgot to finish.
  constraint classes_skill_choices_are_sane
    check (skill_choices between 0 and 18)
);

comment on table  public.classes is
  'What a player character is, and therefore which die their hit points come from. Nullable tenancy: game_id NULL is the SRD row, a game_id is that table''s own version.';
comment on column public.classes.game_id is
  'NULL = the global SRD row every game sees. Set = this game''s own version, which wins over the global one of the same key.';
comment on column public.classes.key is
  'Stable identifier - fighter, rogue. Referenced by value from characters.class_key, because a partial unique index cannot back a foreign key.';
comment on column public.classes.hit_die is
  'The die this class rolls for hit points. THE PC RULE: maximum at first level, average thereafter - see vitality.rs, which has carried this as a comment since 029.';
comment on column public.classes.primary_abilities is
  'What the class runs on. Advisory - nothing derives from it yet; it is here so a creation screen can say which scores matter before somebody rolls them.';
comment on column public.classes.saving_throws is
  'The two abilities this class is proficient in for saves. Ability codes: str/dex/con/int/wis/cha.';
comment on column public.classes.armor_profs is
  'Armour categories, in the vocabulary equipment::is_proficient actually checks: lgt, med, hvy, shl. NOT "light" - that would match nothing and report nothing.';
comment on column public.classes.weapon_profs is
  'Weapon classes as prefixes - sim, mar - or exact item keys for the restricted classes. The same two forms characters.weapon_profs already holds.';
comment on column public.classes.skill_choices is
  'How many of skill_options the character picks. Rogue takes four, Bard and Ranger three, everyone else two.';
comment on column public.classes.skill_options is
  'Three-letter keys from skills.key. An empty list means any skill, which is how the Bard is written.';

-- One key per space, the 004 pattern: unique among the globals, and
-- unique within each game that overrides one.
create unique index if not exists classes_global_key_idx
  on public.classes (key) where game_id is null;
create unique index if not exists classes_game_key_idx
  on public.classes (key, game_id) where game_id is not null;

alter table public.classes enable row level security;

create policy "classes: read global or own game" on public.classes
  for select using (game_id is null or is_game_member(game_id));
create policy "classes: dm writes own game" on public.classes
  for insert with check (game_id is not null and is_game_dm(game_id));
create policy "classes: dm updates own game" on public.classes
  for update using (game_id is not null and is_game_dm(game_id))
          with check (game_id is not null and is_game_dm(game_id));
create policy "classes: dm deletes own game" on public.classes
  for delete using (game_id is not null and is_game_dm(game_id));

-- ---------------------------------------------------------------------
-- THE CHARACTER'S SIDE OF IT
-- ---------------------------------------------------------------------

alter table public.characters
  add column if not exists class_key text;

comment on column public.characters.class_key is
  'Which class, by value into classes.key - no FK, because the two partial unique indexes that make tenancy work cannot back one. NULL is honest: every character made before 055, and every monster, has no class.';

create index if not exists characters_class_idx
  on public.characters (class_key) where class_key is not null;

-- ---------------------------------------------------------------------
-- THE TWELVE
-- ---------------------------------------------------------------------
--
-- Hit dice and saving throws are the two facts anything derives from;
-- the rest is what a creation screen needs to stop asking the player to
-- know the book. Descriptions are one line and written here rather than
-- copied, because a table of contents is not a rulebook.

insert into public.classes
  (game_id, key, name, hit_die, primary_abilities, saving_throws,
   armor_profs, weapon_profs, skill_choices, skill_options, description)
values
  (null, 'barbarian', 'Barbarian', 12, '{str}', '{str,con}',
   '{lgt,med,shl}', '{sim,mar}', 2,
   '{ani,ath,itm,nat,prc,sur}',
   'A fury that armour would only slow down.'),

  (null, 'bard', 'Bard', 8, '{cha}', '{dex,cha}',
   '{lgt}', '{sim,crossbow_hand,longsword,rapier,shortsword}', 3,
   '{}',
   'Magic worked through performance, and a little of everything else.'),

  (null, 'cleric', 'Cleric', 8, '{wis}', '{wis,cha}',
   '{lgt,med,shl}', '{sim}', 2,
   '{his,ins,med,per,rel}',
   'Divine power, lent rather than owned.'),

  (null, 'druid', 'Druid', 8, '{wis}', '{int,wis}',
   '{lgt,med,shl}',
   '{club,dagger,dart,javelin,mace,quarterstaff,scimitar,sickle,sling,spear}', 2,
   '{arc,ani,ins,med,nat,prc,rel,sur}',
   'The old power of the wild, and the shapes it wears.'),

  (null, 'fighter', 'Fighter', 10, '{str,dex}', '{str,con}',
   '{lgt,med,hvy,shl}', '{sim,mar}', 2,
   '{acr,ani,ath,his,ins,itm,prc,sur}',
   'Every weapon, every armour, and the training to use them.'),

  (null, 'monk', 'Monk', 8, '{dex,wis}', '{str,dex}',
   '{}', '{sim,shortsword}', 2,
   '{acr,ath,his,ins,rel,ste}',
   'The body as the weapon, and stillness as the armour.'),

  (null, 'paladin', 'Paladin', 10, '{str,cha}', '{wis,cha}',
   '{lgt,med,hvy,shl}', '{sim,mar}', 2,
   '{ath,ins,itm,med,per,rel}',
   'An oath, and the power that keeping it grants.'),

  (null, 'ranger', 'Ranger', 10, '{dex,wis}', '{str,dex}',
   '{lgt,med,shl}', '{sim,mar}', 3,
   '{ani,ath,ins,inv,nat,prc,ste,sur}',
   'The borderlands, and what has to be hunted there.'),

  (null, 'rogue', 'Rogue', 8, '{dex}', '{dex,int}',
   '{lgt}', '{sim,crossbow_hand,longsword,rapier,shortsword}', 4,
   '{acr,ath,dec,ins,itm,inv,prc,prf,per,slt,ste}',
   'Precision where a fight is not fair, and the skills to keep it that way.'),

  (null, 'sorcerer', 'Sorcerer', 6, '{cha}', '{con,cha}',
   '{}', '{dagger,dart,sling,quarterstaff,crossbow_light}', 2,
   '{arc,dec,ins,itm,per,rel}',
   'Magic in the blood, never studied and not quite controlled.'),

  (null, 'warlock', 'Warlock', 8, '{cha}', '{wis,cha}',
   '{lgt}', '{sim}', 2,
   '{arc,dec,his,itm,inv,nat,rel}',
   'A bargain with something that had power to lend.'),

  (null, 'wizard', 'Wizard', 6, '{int}', '{int,wis}',
   '{}', '{dagger,dart,sling,quarterstaff,crossbow_light}', 2,
   '{arc,his,ins,inv,med,rel}',
   'Magic as a discipline, written down and carried in a book.')
on conflict do nothing;
