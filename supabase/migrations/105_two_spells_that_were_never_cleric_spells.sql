-- 105. TWO SPELLS THAT WERE NEVER CLERIC SPELLS.
--
-- 102 to 104 seeded the cleric list and left two of 006's export rows
-- with no class at all: Fireball and Aura of Life. Neither is a cleric
-- spell and neither was a mistake in the seeding - they came off ONE
-- CHARACTER'S sheet, which is what 006's header warned the whole table
-- was.
--
-- Fireball is sorcerer and wizard. Aura of Life is paladin. Tagging
-- them is better than deleting them: they are correct rows on the
-- wrong list, and the class query is what keeps them off the cleric
-- tab now.
--
-- THEIR FIELDS ARE STILL THIN - no casting time, no components, and
-- descriptions off the export rather than written here. They are not
-- on anybody's tab yet, so that is a gap rather than a fault, and the
-- pass that seeds the wizard or paladin list fills them in properly.

update spells set classes = array['sorcerer', 'wizard']
 where game_id is null and key = 'sp_fireball';

update spells set classes = array['paladin']
 where game_id is null and key = 'sp_auraoflife';