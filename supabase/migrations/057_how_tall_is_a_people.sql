-- =====================================================================
-- 057_how_tall_is_a_people.sql
-- odyssey1e — height, playability, and the Unt'gar
-- =====================================================================
--
-- THIS CAMPAIGN RUNS FROM TWO FEET TO TWENTY-FIVE. A rodent people at
-- 2', the Unt'gar at 4.5', the Felligar at 5.5', the Unt'garoth near
-- 8', the Jotun at 18' and the Imiear at nearly 25'. 056 gave a species
-- a size CATEGORY and stopped there, which in a world shaped like this
-- throws away most of what is interesting about it.
--
-- HEIGHT IS THE FACT; THE CATEGORY IS A CONSEQUENCE. So both are
-- stored: the feet, because that is what a species IS, and the rung,
-- because the rules need discrete steps to hang grappling and squeezing
-- on. `size::for_height` derives one from the other and the two are
-- checked against each other rather than one being trusted blindly.
--
-- ---------------------------------------------------------------------
-- WHERE THE ROUNDING BITES, SAID PLAINLY
-- ---------------------------------------------------------------------
--
-- The Jotun at 18 feet and the Imiear at nearly 25 are BOTH Huge,
-- because 5e's Huge band runs from 16 to 32. Seven feet apart and not
-- one number between them differs - same reach, same space, same
-- carrying multiplier.
--
-- That is recorded rather than quietly fixed, and there is a test in
-- size.rs asserting it so it cannot be forgotten. If it turns out to be
-- wrong for this campaign the change is to scale the CONTINUOUS facts -
-- reach, space, carrying - off height, and keep the category only for
-- the discrete rules. `height_min_ft` and `height_max_ft` are what such
-- a change would read, which is half the reason they are here.
--
-- ---------------------------------------------------------------------
-- NOT EVERY PEOPLE IS A PLAYER CHARACTER
-- ---------------------------------------------------------------------
--
-- The Imiear are semi-intelligent and rage-driven and Dave has said
-- they are not PCs. That is a fact about the species rather than a
-- permission check, so it lives on the row: `playable` keeps them out
-- of the creation picker while leaving them a full species everywhere
-- else - statblocks, the viewer, and whatever a DM wants to do with
-- them.
--
-- ---------------------------------------------------------------------
-- THE UNT'GAR
-- ---------------------------------------------------------------------
--
-- A sub-species of the Hågenfolk, 4 to 5 feet, subterranean, miners and
-- traders. Their document gives a full mechanical block and it is
-- seeded whole.
--
-- STONECUNNING IS CONDITIONAL EXPERTISE and gets the same treatment as
-- the Unt'garoth's Enduring Might: History is granted outright at
-- ordinary proficiency, and the DOUBLING applies only to stonework,
-- which nothing here can detect. Granting expertise flat would hand out
-- double proficiency on every History check in the game, including the
-- ones about kings. The trait text says whose call it is.
--
-- Darkvision, Mineral Sense, Trade Savvy and Environmental Resilience
-- are all written down and none of them are applied: there is no vision
-- system, no short-rest system, and advantage on the roll screen is a
-- human choice with nothing to fire it. Same honest line 056 drew.
--
-- WHAT IS NOT SEEDED: the Felligar, the Jotun, the Imiear and the
-- rodent people. Their heights are known and nothing else is - no
-- ability bonuses, no traits, no prose. A row with a name and six
-- defaults is worse than no row, because it is pickable and gives a
-- character nothing. They land when their documents do; the framework
-- is what this migration is for.
-- =====================================================================

alter table public.species
  add column if not exists height_min_ft numeric,
  add column if not exists height_max_ft numeric,
  add column if not exists playable boolean not null default true;

comment on column public.species.height_min_ft is
  'Shortest typical adult, in feet. THE FACT the size category is derived from - see size::for_height. Stored because a campaign spanning 2 to 25 feet loses too much to six rungs alone.';
comment on column public.species.height_max_ft is
  'Tallest typical adult, in feet. The pair is a range rather than an average because species documents state one.';
comment on column public.species.playable is
  'Whether this people appears in the character creation picker. False for the Imiear, who are semi-intelligent - a fact about the species, not a permission check, so it lives here rather than in a policy.';

-- A range that runs backwards is a typo, and a negative height is not
-- a people.
alter table public.species
  drop constraint if exists species_height_is_a_range;
alter table public.species
  add constraint species_height_is_a_range check (
    (height_min_ft is null and height_max_ft is null)
    or (height_min_ft > 0 and height_max_ft >= height_min_ft)
  );

-- The Unt'garoth's own document: 7 to 10 feet, and Large.
update public.species
   set height_min_ft = 7, height_max_ft = 10
 where key = 'untgaroth' and game_id is null;

insert into public.species
  (game_id, key, name, ability_bonuses, ability_maxima, size,
   height_min_ft, height_max_ft, playable,
   carry_size_steps, skill_profs, unarmored_ac_base, unarmored_ac_ability,
   speed, damage_resistances, languages,
   summary, appearance, culture, history, roleplaying,
   age_note, alignment_note, traits)
values (
  null, 'untgar', 'Unt''gar',
  '{"con": 2, "int": 1}'::jsonb,
  '{}'::jsonb,
  'med', 4, 5, true,
  0, '{his}', null, null,
  25, '{}', '{Common,"Hågenfolk","one other"}',

  'A sub-species of the Hågenfolk, attuned to what lies beneath the mountains of Tirayis. Exceptional miners and shrewd traders - the lifeblood of subterranean commerce and the keepers of the planet''s deepest secrets.',

  'Short and stocky, four to five feet, with musculature built by generations of shaping the underground. Skin runs from deep umber to coal black, tougher than it looks and adapted to low light. Reflective irises like a nocturnal animal''s. Rugged faces, pronounced brows, strong jaws, and hair worn long and braided - as much for the work as for the custom.',

  'Their underground cities are carved into bedrock and lit by luminescent fungi and crystal, as beautiful as they are functional; small surface outposts handle trade and diplomacy. They are sought-after partners, known for fairness and for goods that last. Family units run to several generations and kinship outranks nearly everything. They hold that the relationship with the planet is symbiotic - take what is needed, respect the balance - and honour past generations through story and ritual while still embracing what is new.',

  'Miners and traders of the deep mountains, with networks reaching far beyond their own cities. The Deep Song, a ritual communication of rhythm and vibration, is believed to resonate with the planet''s own heartbeat. The Rite of the Hammer marks coming of age: a young Unt''gar crafts their first tool or weapon, and it is their first contribution to the community.',

  'Pragmatism is the through-line. An Unt''gar negotiates rather than fights, manages resources rather than hoards them, and measures a stranger by what they do over time rather than what they say at the outset. Consider what drew this one to the surface, and what they intend to bring home.',

  'Mature at 25 and live up to 500 years.',
  'Tend towards lawful neutral, valuing order and fairness in trade and society.',

  '[
    {"name": "Ability Score Increase",
     "text": "Constitution +2, Intelligence +1.",
     "applied": true},
    {"name": "Stonecunning",
     "text": "Proficiency in History. On checks relating to STONEWORK the proficiency bonus is DOUBLED - the doubling is a DM call, since nothing here can tell a question about stonework from one about kings.",
     "applied": true},
    {"name": "Darkvision",
     "text": "See in dim and dark conditions up to 120 feet. NOT APPLIED: there is no vision or light system yet.",
     "applied": false},
    {"name": "Mineral Sense",
     "text": "After a short rest, detect minerals within 60 feet, through up to 10 feet of solid rock or metal. NOT APPLIED: there is no rest system and no map to detect across.",
     "applied": false},
    {"name": "Trade Savvy",
     "text": "Advantage on Persuasion checks in trade negotiations - which the Trade tab is full of. NOT APPLIED: choose Adv on the roll screen.",
     "applied": false},
    {"name": "Environmental Resilience",
     "text": "Advantage on saving throws against toxic gases and underground hazards. NOT APPLIED: choose Adv on the roll screen.",
     "applied": false},
    {"name": "Deep Community",
     "text": "Multi-generational households and kinship that outranks nearly every other loyalty. Roleplay rather than rule.",
     "applied": false}
  ]'::jsonb
)
on conflict do nothing;
