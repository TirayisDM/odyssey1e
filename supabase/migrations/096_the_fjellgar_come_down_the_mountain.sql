-- =====================================================================
-- 096_the_fjellgar_come_down_the_mountain.sql
-- odyssey1e — the third people, from Dave's two documents
-- =====================================================================
--
-- 057 left four peoples unseeded with their heights known and nothing
-- else, and said why: a row with a name and six defaults is pickable
-- and gives a character nothing. The Fjell'gar now have their
-- documents, so they get a row.
--
-- THE NAME IS FJELL'GAR. STATUS.md has called them the "Felligar"
-- since 057, which is how it was heard rather than how it is written -
-- corrected there in this commit. The key drops the apostrophe like
-- untgar and untgaroth do.
--
-- ---------------------------------------------------------------------
-- WHAT IS APPLIED
-- ---------------------------------------------------------------------
--
--   DEX +2, WIS +1       agility and spatial awareness
--   Medium, 4.5-5.5 ft   midpoint 5.0, which is the med band (4-8), so
--                        the stated size and the height agree
--   Athletics granted    Climber's Grace
--   AC 12 + DEX unarmoured   Stone Resilience, and a shield still
--                        counts on top - which armor_class already
--                        does for the Unt'garoth's 12 + CON
--   speed 30             the ordinary walking speed
--
-- ---------------------------------------------------------------------
-- WHAT IS WRITTEN DOWN AND NOT APPLIED, and why each one
-- ---------------------------------------------------------------------
--
--   Mountain Born        advantage on saves against cold and thin air.
--                        ADVANTAGE IS A CHOICE AT THE TABLE - the
--                        engine takes adv/dis per roll and nothing
--                        grants it standing.
--   Rock Grip            advantage against being dislodged, and Dodge
--                        while climbing. No climbing state exists.
--   Spatial Awareness    cannot become lost. No travel system.
--   Guerrilla Tactics    weapon proficiencies and a Help bonus from
--                        higher ground. `species` has no weapon_profs
--                        column - only `characters` does - and there
--                        is no high ground.
--   Climbing Speed       30 ft climbing. `speed` holds walking, and a
--                        second movement rate has nowhere to go until
--                        there is a movement system at all.
--
-- A NOTE ON THE COLD. The prose calls them resistant to cold and the
-- mechanic is ADVANTAGE ON SAVES, which is not damage resistance.
-- `damage_resistances` stays empty - the Unt'garoth's cold and fire are
-- in that column because their document grants the resistance itself.
-- Reading one as the other would have halved real damage on a rule
-- nobody wrote.
--
-- CLIMBER'S GRACE GETS THE STONECUNNING TREATMENT. The proficiency is
-- granted outright; the DOUBLE proficiency on a climb is a DM call,
-- because nothing here can tell a climbing Athletics check from a
-- shoving one. Same call 057 made for the Unt'gar's stonework.
--
-- NO DRAWBACK IS MARKED, and that is a reading rather than an
-- omission. The Unt'gar earned one in 059 because 25 feet against a
-- 30-foot norm is a real cost. The Fjell'gar walk at 30 and their
-- documents state nothing they give up; inventing one to balance them
-- would be designing Dave's game for him.
--
-- `history` IS NULL. Both documents cover appearance, society, values,
-- abilities, diet, reproduction and relations - and no history. An
-- empty section is the honest answer, and the sheet omits what is null.
--
-- NOT IN THE CATALOGUE: climbing picks (light, 1d6 slashing) and
-- grappling spears (javelin-equivalent, 30/120, 1d8 piercing) are named
-- in Guerrilla Tactics and do not exist in `items`. They are a seed of
-- their own, in 042's shape, whenever somebody wants to carry one.
-- =====================================================================

insert into public.species
  (game_id, key, name, ability_bonuses, ability_maxima, size,
   height_min_ft, height_max_ft, playable,
   carry_size_steps, skill_profs, unarmored_ac_base, unarmored_ac_ability,
   speed, damage_resistances, tongues,
   summary, appearance, culture, history, roleplaying,
   age_note, alignment_note, traits)
values (
  null, 'fjellgar', 'Fjell''gar',
  '{"dex": 2, "wis": 1}'::jsonb,
  '{}'::jsonb,
  'med', 4.5, 5.5, true,
  0, '{ath}', 12, 'dex',
  30, '{}',
  '[{"name": "Common", "spoken": true, "written": true},
    {"name": "Fjell''gar", "spoken": true, "written": true}]'::jsonb,

  'A hardy, resourceful people of the high mountains, shaped by thin air and vertical stone. Unparalleled climbers, calm under pressure, and as steadfast as the peaks they come from.',

  'Compact and muscular, four and a half to five and a half feet, built for endurance and deceptively agile. Skin runs from ashen grey to earthy brown, often mottled like the rock itself, thick and slightly textured against cold wind and sharp edges. Large low-light eyes in amber, green or steel grey, with a second membrane against dust. Hands and feet adapted for climbing - elongated fingers, strong prehensile toes - and some carry ridge-like protrusions along the spine or forearms that help on a cliff and turn a blow in a fight.',

  'They live in close-knit settlements built into caves, cliffsides and high plateaus, designed to disappear into the landscape. Standing is earned rather than inherited: a Fjell''gar is measured by what they contribute, particularly in climbing, hunting and engineering. Deeply spiritual, they hold to a balance between earth and sky and honour the spirits of mountains, storms and endurance at cliffside altars. Self-reliance is taught early - younglings are expected to fend for themselves long before they are grown - and younglings are raised communally by a tribe that is fiercely protective of them.',

  null,

  'Consider what took this one off the mountain: an explorer after what lies beyond it, an emissary building alliances, or an outcast travelling by necessity. They are cautious with outsiders - remote country makes for few visitors - and trade readily with anyone who respects their territory, usually in rare minerals and high-altitude herbs. They fight on their own terms where they can, using terrain and ambush rather than meeting a line head-on. Physical quirks are worth deciding on: ridged spines and speckled patterns are cosmetic and say where in the range a character comes from.',

  'Mature by 20 and live up to 150 years.',
  'Lean neutral, caring more for survival and balance than for law or chaos; many tend good, out of reverence for harmony in nature.',

  '[
    {"name": "Ability Score Increase",
     "text": "Dexterity +2, Wisdom +1.",
     "applied": true},

    {"name": "Climber''s Grace",
     "text": "Athletics proficiency, granted here. On a Strength (Athletics) check to CLIMB the proficiency bonus counts twice - a DM call, because nothing can tell a climb from a shove.",
     "applied": true},

    {"name": "Stone Resilience",
     "text": "Unarmoured AC is 12 + Dexterity, and a shield still counts on top.",
     "applied": true},

    {"name": "Mountain Born",
     "text": "Adapted to altitude above 20,000 feet and to cold. Advantage on saving throws and ability checks against extreme cold and thin air.",
     "applied": false},

    {"name": "Rock Grip",
     "text": "Advantage on checks and saves against being knocked prone or dislodged while climbing. Taking the Dodge action while climbing gives attackers disadvantage.",
     "applied": false},

    {"name": "Spatial Awareness",
     "text": "Cannot become lost except by magic. Advantage on Wisdom (Perception) checks to spot natural hazards - unstable rock, hidden crevasses.",
     "applied": false},

    {"name": "Guerrilla Tactics",
     "text": "Proficiency with handaxes, climbing picks (light, 1d6 slashing) and grappling spears (javelin-equivalent, 30/120, 1d8 piercing). Using Help in combat gives the ally +2 to the attack if it is made from higher ground.",
     "applied": false},

    {"name": "Climbing Speed",
     "text": "A climbing speed of 30 feet, alongside the 30-foot walk.",
     "applied": false}
  ]'::jsonb
);
