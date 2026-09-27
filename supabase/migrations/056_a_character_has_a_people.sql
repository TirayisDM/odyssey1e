-- =====================================================================
-- 056_a_character_has_a_people.sql
-- odyssey1e — species, and the first of them
-- =====================================================================
--
-- THIS CAMPAIGN IS ALL CUSTOM SPECIES. Not "5e's list plus some", not
-- homebrew bolted onto a shipped table - the whole roster is Dave's, and
-- the Unt'garoth are the first. That decision is why this table looks
-- the way it does: there is no SRD species seed to sit under a game's
-- overrides, so the catalogue is a catalogue of one, and the tenancy is
-- here because 004's pattern is what every other catalogue uses and a
-- second campaign will want its own.
--
-- 055 gave a character a class and therefore a hit die. This gives them
-- a PEOPLE, and with it the other half of what a character sheet is
-- made of: the scores they started from, how big they are, and what
-- their body can do that nobody else's can.
--
-- ---------------------------------------------------------------------
-- WHAT IS APPLIED AND WHAT IS ONLY WRITTEN DOWN
-- ---------------------------------------------------------------------
--
-- This is the important column in this file and the reason `traits`
-- carries an `applied` flag. A species sheet is a mix of things the
-- engine can act on and things it cannot, and the ones it cannot are
-- not lesser - they are the DM's to adjudicate. What is dishonest is a
-- screen that shows them identically.
--
-- APPLIED, because there is somewhere real for them to land:
--
--   ability_bonuses     every modifier downstream, in one place - see
--                       character.rs, where the map is built
--   ability_maxima      a cap the score editor enforces
--   size                carry.rs already multiplies capacity by size
--   carry_size_steps    Powerful Build, as one step up that same ladder
--   skill_profs         character_skills, at creation
--   unarmored_ac_*      equipment::armor_class's unarmoured branch
--
-- WRITTEN DOWN ONLY, because the system they need does not exist:
--
--   speed               THERE IS NO MOVEMENT SYSTEM. 40 feet is stored
--                       and shown and nothing consumes it. When
--                       movement arrives this column is already here.
--   damage_resistances  THERE IS NO RESISTANCE SYSTEM. Damage has types
--                       on items since 027 and nothing halves anything.
--   the advantage traits  The roll screen's Normal/Adv/Dis is a human
--                       choice. A trait saying "advantage on CON saves
--                       against exhaustion" has no hook to fire from,
--                       and inventing one would mean inventing the
--                       conditions too.
--
-- 054 made this argument about action economy and refused to guess at
-- 193 rows. The same call: one honest column beats a decorative one,
-- and a trait marked `applied: false` tells a DM exactly where their
-- own judgement is still required.
--
-- ---------------------------------------------------------------------
-- BASE AND EFFECTIVE ARE DIFFERENT FACTS
-- ---------------------------------------------------------------------
--
-- A species bonus is NOT added into `character_abilities.score`. The
-- stored score stays the number somebody rolled or bought, and the
-- bonus is applied on the way to the sheet.
--
-- Writing +2 into the row would destroy the base, so changing species
-- later would double-count or silently under-count, and nothing could
-- tell a Strength of 18 apart from a 16 with a species behind it. This
-- is 049's tri-state argument in another costume, and 001's: a stored
-- value is a record of what somebody decided, and a derived one is a
-- consequence. Keep them apart and both stay true.
--
-- ---------------------------------------------------------------------
-- WHY TRAITS ARE jsonb AND NOT A CHILD TABLE
-- ---------------------------------------------------------------------
--
-- Techniques got their own table in 043 because things JOIN to them -
-- an object has them, an attack rolls them. Nothing joins to a trait.
-- It is an ordered list of prose belonging to exactly one species, read
-- only by a screen, and a child table would need its own two partial
-- unique indexes to carry the same tenancy for no benefit anybody can
-- name. If a trait ever becomes something a rule fires from, it earns
-- its table then, and it will want columns this list does not have.
--
-- ---------------------------------------------------------------------
-- ONE CONTRADICTION IN THE SOURCE, RESOLVED AND FLAGGED
-- ---------------------------------------------------------------------
--
-- The species document says both "their strength to extend to 20
-- naturally" and, in the section headed Exceptional Strength / Natural
-- Maximum Ability Score, "Maximum of 21: Natural Strength Ability score
-- achievable is 21". Those cannot both be true.
--
-- 21 is seeded, because the later passage is the specific one - it has
-- a heading, a mechanic and a worked explanation, against a clause in a
-- summary sentence. This is a DESIGN QUESTION rather than a data entry
-- one, so it is written here where it can be found and changed with one
-- update rather than buried.
-- =====================================================================

create table if not exists public.species (
  id            uuid primary key default gen_random_uuid(),
  game_id       uuid references public.games(id) on delete cascade,
  key           text not null,
  name          text not null,

  -- APPLIED -------------------------------------------------------
  ability_bonuses    jsonb   not null default '{}'::jsonb,
  ability_maxima     jsonb   not null default '{}'::jsonb,
  size               text    not null default 'med',
  carry_size_steps   integer not null default 0,
  skill_profs        text[]  not null default '{}',
  unarmored_ac_base  integer,
  unarmored_ac_ability text,

  -- WRITTEN DOWN ONLY ---------------------------------------------
  speed              integer,
  damage_resistances text[]  not null default '{}',
  languages          text[]  not null default '{}',

  -- PROSE, for the viewer -----------------------------------------
  summary       text,
  appearance    text,
  culture       text,
  history       text,
  roleplaying   text,
  age_note      text,
  alignment_note text,
  traits        jsonb not null default '[]'::jsonb,

  created_at    timestamptz not null default now(),

  constraint species_size_is_a_size
    check (size in ('tiny','sm','med','lg','huge','grg')),
  -- One step is Powerful Build. Two would be a species that carries
  -- like something three sizes up, which is a thing somebody might
  -- write; six is a typo.
  constraint species_carry_steps_are_sane
    check (carry_size_steps between 0 and 3),
  -- An unarmoured base without the ability to add is half a rule.
  constraint species_unarmored_ac_is_whole
    check (
      (unarmored_ac_base is null and unarmored_ac_ability is null)
      or (unarmored_ac_base is not null and unarmored_ac_ability is not null)
    ),
  constraint species_unarmored_ability_is_an_ability
    check (unarmored_ac_ability is null
           or unarmored_ac_ability in ('str','dex','con','int','wis','cha')),
  constraint species_traits_is_a_list
    check (jsonb_typeof(traits) = 'array'),
  constraint species_bonuses_are_objects
    check (jsonb_typeof(ability_bonuses) = 'object'
           and jsonb_typeof(ability_maxima) = 'object')
);

comment on table public.species is
  'What a character''s people are. Nullable tenancy like every other catalogue; this campaign has no SRD layer because every species in it is custom.';
comment on column public.species.ability_bonuses is
  'Ability code -> bonus, e.g. {"str":2,"con":1}. APPLIED ON THE WAY TO THE SHEET and never written into character_abilities.score, so the stored score stays the number somebody rolled.';
comment on column public.species.ability_maxima is
  'Ability code -> the natural ceiling for this people, e.g. {"str":21}. Absent means 20, which is 5e''s default.';
comment on column public.species.size is
  'Feeds carry.rs''s capacity multiplier and anything else that reads size. Set on the character at creation.';
comment on column public.species.carry_size_steps is
  'How many sizes UP this people carries, on top of their own. 1 is Powerful Build. Affects only carrying, pushing, dragging and lifting - not reach, not cover, not the hit die.';
comment on column public.species.skill_profs is
  'Skills granted outright, as three-letter skills.key values. Written into character_skills at creation.';
comment on column public.species.unarmored_ac_base is
  'The base AC when wearing no body armour, replacing the usual 10. NULL means the ordinary rule.';
comment on column public.species.unarmored_ac_ability is
  'Which modifier is added to unarmored_ac_base. Unyielding Defense is 12 + CON rather than the usual 10 + DEX.';
comment on column public.species.speed is
  'Feet per turn. STORED AND SHOWN, CONSUMED BY NOTHING - there is no movement system yet. Here so that when one arrives the number is already right.';
comment on column public.species.damage_resistances is
  'Damage types halved. STORED AND SHOWN, APPLIED BY NOTHING - there is no resistance system. A DM halves it by hand and the sheet says they should.';
comment on column public.species.traits is
  'Ordered list of {name, text, applied}. `applied` false means the engine does not act on it and a DM must - which is a fact worth showing rather than hiding.';

create unique index if not exists species_global_key_idx
  on public.species (key) where game_id is null;
create unique index if not exists species_game_key_idx
  on public.species (key, game_id) where game_id is not null;

alter table public.species enable row level security;

create policy "species: read global or own game" on public.species
  for select using (game_id is null or is_game_member(game_id));
create policy "species: dm writes own game" on public.species
  for insert with check (game_id is not null and is_game_dm(game_id));
create policy "species: dm updates own game" on public.species
  for update using (game_id is not null and is_game_dm(game_id))
          with check (game_id is not null and is_game_dm(game_id));
create policy "species: dm deletes own game" on public.species
  for delete using (game_id is not null and is_game_dm(game_id));

alter table public.characters
  add column if not exists species_key text;

comment on column public.characters.species_key is
  'Which people, by value into species.key - no FK, for 004''s reason: the two partial unique indexes that carry tenancy cannot back one. NULL is honest for every character made before 056.';

create index if not exists characters_species_idx
  on public.characters (species_key) where species_key is not null;

-- ---------------------------------------------------------------------
-- THE FIRST PEOPLE
-- ---------------------------------------------------------------------

insert into public.species
  (game_id, key, name, ability_bonuses, ability_maxima, size,
   carry_size_steps, skill_profs, unarmored_ac_base, unarmored_ac_ability,
   speed, damage_resistances, languages,
   summary, appearance, culture, history, roleplaying,
   age_note, alignment_note, traits)
values (
  null, 'untgaroth', 'Unt''garoth',
  '{"str": 2, "con": 1}'::jsonb,
  '{"str": 21}'::jsonb,
  'lg', 1, '{ath}', 12, 'con',
  40, '{cold,fire}', '{Common,"Unt''garoth Dialect"}',

  'Towering humanoids forged by the Mithar''itra as labourers and warriors, now a fiercely independent people of the mountains and tundra. Their culture values survival, honour and community.',

  'Seven to ten feet tall and four to nine hundred pounds, built of dense bone and heavy muscle. Skin runs from ashen grey through earthy brown to icy blue, often marked with natural patterning. Deep-set eyes of dark brown, grey or ice. Hair short or tightly braided. Pronounced brows, strong jaws, and on some, ridges of bone.',

  'Tight-knit and self-reliant, settled deep in mountains and across remote tundra in places built to survive the weather. Leadership is meritocratic - chieftains and elders chosen for strength, wisdom and judgement. History passes by mouth: ancestral triumphs, survival, and what the land taught. Strength and bravery are shown in ritual combat at rites of passage. Honour and loyalty are the pillars; a person is judged by their deeds and what they give the community.',

  'Crafted by the Mithar''itra to build planetary homes and to fight. After uncontrolled events on Tirayis they migrated to the remote regions and became self-sufficient, developing their own culture apart from their makers. Isolation made them independent and wary; outsiders earn trust by showing respect, and rarely otherwise.',

  'Their size makes them the focal point of any room, and it is worth playing both halves of that - the physical presence, and the stoic, methodical nature underneath it. Consider what their community means to them, and whether this one is a protector of the homeland or has left it seeking knowledge, honour or redemption.',

  'Mature at about 10 years and live up to 50.',
  'Typically lawful neutral, driven by traditions of honour and community.',

  '[
    {"name": "Powerful Build",
     "text": "Count as one size larger when determining carrying capacity and the weight you can push, drag or lift.",
     "applied": true},
    {"name": "Imposing Stature",
     "text": "Large. Dense bone and heavy muscle put an Unt''garoth well beyond most other peoples for raw strength.",
     "applied": true},
    {"name": "Exceptional Strength",
     "text": "Strength rises naturally to 21 rather than stopping at 20, with no magic or class feature needed to reach it.",
     "applied": true},
    {"name": "Enduring Might",
     "text": "Proficiency in Athletics. Add DOUBLE your proficiency bonus to Strength (Athletics) checks for climbing, lifting or grappling - the doubling is a DM call, since the engine cannot tell which Athletics check you are making.",
     "applied": true},
    {"name": "Unyielding Defense",
     "text": "While wearing no body armour your AC is 12 + your Constitution modifier, rather than 10 + Dexterity.",
     "applied": true},
    {"name": "Elemental Resilience",
     "text": "Resistance to fire and cold damage - half damage from either. NOT APPLIED BY THE ENGINE: there is no resistance system yet, so halve it at the table.",
     "applied": false},
    {"name": "Temperate Endurance",
     "text": "Advantage on saving throws against extreme heat and extreme cold. NOT APPLIED: choose Adv on the roll screen.",
     "applied": false},
    {"name": "Mountain''s Resolve",
     "text": "Advantage on Constitution saving throws against exhaustion and environmental effects. NOT APPLIED: choose Adv on the roll screen.",
     "applied": false},
    {"name": "Mountain Born",
     "text": "Accustomed to high altitude, including above 20,000 feet, and naturally adapted to cold climates.",
     "applied": false},
    {"name": "Dense Mass",
     "text": "An Unt''garoth cannot swim. Their density does not permit it.",
     "applied": false},
    {"name": "Hardy Constitution",
     "text": "A robust immune system turns aside most disease and many toxins.",
     "applied": false}
  ]'::jsonb
)
on conflict do nothing;
