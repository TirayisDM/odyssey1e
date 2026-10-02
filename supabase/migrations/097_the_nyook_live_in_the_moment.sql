-- =====================================================================
-- 097_the_nyook_live_in_the_moment.sql
-- odyssey1e — the fourth people, and the first with a ceiling
-- =====================================================================
--
-- The Ny'ook, a Hågenfolk sub-species like the Unt'gar: three to four
-- feet, fast, charming, and resistant to magic in a way that costs
-- them magic of their own.
--
-- ---------------------------------------------------------------------
-- WHAT IS APPLIED
-- ---------------------------------------------------------------------
--
--   DEX +2, CHA +2       the only people so far with two bonuses of
--                        the same size and no third
--   Small, 3-4 ft        midpoint 3.5, and the sm band is 2 to 4, so
--                        the stated size and the height agree
--   STRENGTH CEILING 13  see below - the first ability_maxima entry
--                        that lowers a ceiling rather than raising one
--   Acrobatics granted   Quick Reflexes
--   speed 30             small and still quick, which their document
--                        calls out as the point
--
-- ---------------------------------------------------------------------
-- THE STRENGTH CEILING IS ENFORCED IN ONE PLACE AND NOT THE OTHER
-- ---------------------------------------------------------------------
--
-- "No matter how much they train, their Strength score cannot exceed
-- 13." `ability_maxima` is the right column and 056 built it for the
-- opposite case - the Unt'garoth's RAISED ceiling of 21 - so it is
-- worth being exact about what a LOWERED one does today:
--
--   species bonus    `effective_score` will not add past it. The
--                    Ny'ook get no Strength bonus, so this is moot
--                    for them and correct for anybody who does.
--   an ASI           `apply_bumps` does `(score + value).min(ceiling)`.
--                    A Ny'ook spending Ability Score Improvements on
--                    Strength stops dead at 13. ENFORCED.
--   a rolled score   NOT enforced. `effective_score` returns a base at
--                    or above the maximum unchanged - deliberately,
--                    because that is how a raised ceiling has to
--                    behave - so a Ny'ook ASSIGNED a 16 at creation
--                    keeps the 16.
--
-- The trait is marked applied because the engine honours the ceiling
-- everywhere it does the adding. The gap is a missing refusal at the
-- point of writing a score, not a wrong number: chargen and
-- `set_ability` would both have to ask the species before accepting
-- one. Written down here rather than quietly half-done.
--
-- ---------------------------------------------------------------------
-- THE SOURCE CONTRADICTS ITSELF ABOUT THE RESISTANCE
-- ---------------------------------------------------------------------
--
-- Three statements, two mechanics:
--
--   "+2 on Spell Saving Throws"                     (Abilities list)
--   "a +2 bonus on saving throws against spells"    (Magical Resistance)
--   "Advantage on saving throws against spells and poisons"
--                                                   (Playable Race)
--
-- +2 and advantage are not the same thing and the second document
-- hedges with "Advantage (or a +2 bonus)". NEITHER IS SEEDED AS
-- APPLIED, which costs nothing today - there is no save-bonus system
-- and advantage is a choice per roll - and the trait text carries both
-- readings so the decision is made once, by Dave, when something can
-- act on it. 056 did the same with the Unt'garoth's 20-against-21.
--
-- ALSO NOT DAMAGE RESISTANCE. `damage_resistances` stays empty for the
-- same reason it did for the Fjell'gar's cold: a save bonus is not
-- resistance, and putting it in that column would halve real damage on
-- a rule nobody wrote.
--
-- ---------------------------------------------------------------------
-- NON-MAGICAL IS A DRAWBACK, AND THE FIRST ONE THAT SHAPES A BUILD
-- ---------------------------------------------------------------------
--
-- They cannot cast innately. Their document is explicit that this
-- steers them to Cleric, Druid and Bard - ritual and divine rather
-- than arcane. Marked `drawback` and NOT applied: there is no spell
-- system to forbid anything in, and `classes` has no gate on who may
-- take what. A Ny'ook Wizard is refused by the table, not by this
-- schema.
--
-- MAX STRENGTH IS ALSO MARKED A DRAWBACK while being applied - the two
-- fields are orthogonal and this is the row that proves it. A ceiling
-- is a cost even when the engine is the thing enforcing it.
-- =====================================================================

insert into public.species
  (game_id, key, name, ability_bonuses, ability_maxima, size,
   height_min_ft, height_max_ft, playable,
   carry_size_steps, skill_profs, unarmored_ac_base, unarmored_ac_ability,
   speed, damage_resistances, tongues,
   summary, appearance, culture, history, roleplaying,
   age_note, alignment_note, traits)
values (
  null, 'nyook', 'Ny''ook',
  '{"dex": 2, "cha": 2}'::jsonb,
  '{"str": 13}'::jsonb,
  'sm', 3, 4, true,
  0, '{acr}', null, null,
  30, '{}',
  '[{"name": "Common", "spoken": true, "written": true},
    {"name": "Ny''ook", "spoken": true, "written": true}]'::jsonb,

  'A Hågenfolk offshoot of halfling stature, built for speed and charm, and naturally resistant to magic and poison. Short lives at a high metabolism have made a culture that prizes the present: they celebrate often, decide fast, and rarely plan far.',

  'Three to four feet, slender and lithe, with lean muscle built for quick bursts rather than long labour. Skin runs from light tan to deep brown with a faint glow to it, a side effect of the metabolism that drives them. Bright, sharp eyes - green, hazel, warm brown - usually with a spark in them. Ears are slightly pointed and larger than a Fjell''gar''s, an adaptation for hearing what they cannot see far enough to spot. Fine hair from light brown to black, worn short or tied back so it stays out of the way.',

  'They live in close clusters around open courtyards that serve as market, meeting place and dance floor by turns. Homes are modular and rebuilt as families grow or projects demand. Leadership rotates - a chieftain or elder holds the role while they are the right one for it, and handing it on is unremarkable rather than a crisis. Titles and lineage count for little against what somebody can actually do. Festivals come constantly, for the turn of a season, a finished job, or no reason at all, and their hospitality to outsiders is famous. Coming of age is the Toxin Trial: a young Ny''ook handles or neutralises a mild poison to show they are ready.',

  'They split from the main Hågenfolk line early in Tirayis''s history, into country that rewarded what they already were. The resistance to magic and toxins reads as a survival adaptation - generations in places thick with magical energy or poisonous flora - and the focus on the immediate followed from lives of a hundred and fifty to two hundred years among neighbours who live far longer.',

  'Play the speed of decision rather than the speed of foot: a Ny''ook acts while others are still weighing it up, and is already somewhere else by the time it goes wrong. Warm on first meeting and easy to like, though the connection may not outlast the week. Little patience for long plans and real attention for what is in front of them. They are stewards of balance in their own telling - complementing an ally''s magic rather than wielding any - and they tend to be the one sent to talk to the people nobody else can talk to.',

  'Mature around 15 and rarely pass 200 years.',
  'Often chaotic good, guided by freedom, spontaneity and goodwill.',

  '[
    {"name": "Ability Score Increase",
     "text": "Dexterity +2, Charisma +2.",
     "applied": true},

    {"name": "Max Strength",
     "text": "Strength cannot exceed 13, however much they train. The engine will not add past it - no species bonus and no Ability Score Improvement can - but it does not reach back and lower a score assigned above it at creation.",
     "applied": true,
     "kind": "drawback"},

    {"name": "Quick Reflexes",
     "text": "Acrobatics proficiency, granted here. Their own document offers Sleight of Hand as the alternative; Acrobatics is what both copies name first.",
     "applied": true},

    {"name": "Magic and Toxin Resistance",
     "text": "Resistant to spells and poisons. The source says +2 on saving throws in two places and advantage in a third - a decision still to be made, and nothing turns on it until saves can carry either.",
     "applied": false},

    {"name": "Nimble Agility",
     "text": "Can move through the space of any creature at least one size larger.",
     "applied": false},

    {"name": "Sharp Senses",
     "text": "Heightened hearing and sight, for noticing danger and reading a complicated space. No number is stated.",
     "applied": false},

    {"name": "High Metabolism",
     "text": "Energy at the cost of years: they burn hot and live 150 to 200 where other Hågenfolk live far longer.",
     "applied": false},

    {"name": "Non-Magical",
     "text": "No innate spellcasting - their resistance disrupts arcane flow through their own bodies. Ritual, divine and natural practice are open to them, which steers them toward Cleric, Druid and Bard. Nothing in this schema forbids a Ny''ook Wizard; the table does.",
     "applied": false,
     "kind": "drawback"}
  ]'::jsonb
);
