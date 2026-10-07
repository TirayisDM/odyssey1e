-- 160. ONE PROFICIENCY SCALE FOR THE WHOLE CATALOGUE.
--
-- 147 gave its 77 new creatures 5e's proficiency by CR and deliberately
-- left the existing 203 on 127's two-step rule, writing down why: "a
-- balance change arriving as a side effect of a data migration is
-- exactly what 132 had to reverse". The catalogue has had two scales
-- since, and that header said deriving the old ones from CR was "a
-- one-line change whenever Dave wants it".
--
-- Dave wants it. This is that change, taken on purpose rather than as a
-- side effect - which is the whole difference from 129.
--
-- ---------------------------------------------------------------------
-- WHY THIS COULD NOT BE DERIVED FROM `level`
-- ---------------------------------------------------------------------
--
-- The tempting one-liner is `prof_bonus = f(level)`. It is wrong, and
-- it is wrong in the way 151 was nearly wrong about caster level:
-- `npcs.level` is 127's HAND-SET ENCOUNTER WEIGHT for everything 130
-- seeded, and the challenge rating only for what 147 added. Deriving
-- from it would give the Lich - level 18, CR 21 - a proficiency of 6
-- where the book says 7, and the Red Dragon Wyrmling - level 6, CR 4 -
-- a 3 where the book says 2.
--
-- So the CR was READ, one page at a time, off the same published SRD
-- that 146, 147 and 148 were checked against. 261 of 280 creatures have
-- a page. `cr` is now a column, because it is the fact everything else
-- here is derived from and a derivation nobody can check is not much
-- better than a guess.
--
-- ---------------------------------------------------------------------
-- WHAT MOVES: FIFTY CREATURES, AND ALMOST ALL OF THEM UP
-- ---------------------------------------------------------------------
--
--   +4   1   Lich 3 -> 7
--   +3   3   Pit Fiend, Balor, Adult Red Dragon 3 -> 6
--   +2  11   Adult Blue/Green/Black/White Dragon, Iron Golem,
--            Marilith, Nalfeshnee, Storm Giant, Vampire 3 -> 5;
--            Bone Devil, Glabrezu 2 -> 4
--   +1  32   Erinyes, Horned Devil, Roc, Deva, Stone Golem, Clay
--            Golem, Cloud Giant, Fire Giant, Treant, Young Red and
--            Blue Dragon 3 -> 4, and 21 others 2 -> 3
--   -1   3   Black Pudding, Chuul, Red Dragon Wyrmling 3 -> 2
--
-- THE TOP END WAS THE WORST AFFECTED, which makes sense: 127's rule
-- stopped at 3 and the book keeps going to 9. Everything legendary has
-- been swinging and saving like a mid-level monster.
--
-- THE THREE THAT DROP are 127's rule being too generous at the bottom -
-- a Red Dragon Wyrmling is CR 4 and 127 put it at level 6.
--
-- 211 creatures do not move at all, which is the quiet evidence that
-- 147's 77 were right: they were seeded from CR in the first place.
--
-- ---------------------------------------------------------------------
-- NINETEEN ARE LEFT ALONE, AND THEY ARE OURS
-- ---------------------------------------------------------------------
--
-- Acolyte, Archer, Banshee, Carrion Crawler, Dire Boar, Druid, Gas
-- Spore, Goblin Boss, Guard Captain, Harpy Matriarch, Helmed Horror,
-- Hook Horror, Intellect Devourer, Kobold Dragonshield, Orc War Chief,
-- Pixie, Scarecrow, Twig Blight and Yuan-ti Pureblood have no page on
-- that SRD - six we invented and thirteen that were written from memory
-- of a book they are not in, which 147's header already recorded.
--
-- They keep 127's rule and their `cr` stays NULL. Inventing a CR for
-- them to derive a proficiency from would be two guesses stacked, and
-- NULL says "nobody has rated this" where a number would not.
--
-- ---------------------------------------------------------------------
-- WHAT THIS DOES NOT FIX
-- ---------------------------------------------------------------------
--
-- `npcs.level` IS STILL TWO SCALES. 130's creatures carry 127's hand-set
-- weight and 147's carry CR, and this migration does not touch it -
-- changing what a creature's level says would move the technique gates
-- 132 had to put back, and level is load-bearing in ways proficiency is
-- not. Now that `cr` is stored, `level` can be reconciled against it
-- whenever that is worth doing, and the two can be compared instead of
-- confused.
--
-- AND PROFICIENCY IS STILL STORED RATHER THAN DERIVED. The honest shape
-- is `cr` stored and `prof_bonus` computed from it in Rust where it can
-- be tested - 001's "derive what can be derived". That is a change to
-- `Sheet::proficiency_bonus` and its callers rather than to the data,
-- and it belongs in its own commit.

alter table public.npcs
  add column if not exists cr integer;

alter table public.npcs
  drop constraint if exists npcs_cr_check;
alter table public.npcs
  add constraint npcs_cr_check check (cr is null or (cr >= 0 and cr <= 30));

comment on column public.npcs.cr is
  'The challenge rating printed in the book, read off the published SRD (160). NULL for the nineteen creatures that have no SRD page, which are ours. This is NOT `level`: level is 127''s hand-set encounter weight for everything 130 seeded and the CR only for what 147 added, and the two differ by as much as 3. prof_bonus is derived from this column by 5e''s table - 2 up to CR 4, then 3, 4, 5, 6, 7, 8, 9 every four ratings - and is stored rather than computed only because that derivation has not moved into Rust yet.';

-- Fractional ratings land on 0: the table gives +2 for everything up to
-- CR 4 regardless, so 1/8 and 1/4 and 0 are the same answer here and
-- storing 0 avoids a numeric column that has to hold "1/4".

update public.npcs n
   set cr = v.cr, prof_bonus = v.prof
  from (values
('aarakocra',0,2),('aboleth',10,4),('adult_black_dragon',14,5),('adult_blue_dragon',16,5),('adult_brass_dragon',13,5),('adult_bronze_dragon',15,5),('adult_copper_dragon',14,5),('adult_gold_dragon',17,6),('adult_green_dragon',15,5),('adult_red_dragon',17,6),('adult_silver_dragon',16,5),('adult_white_dragon',13,5),('air_elemental',5,3),('allosaurus',2,2),('ancient_black_dragon',21,7),('ancient_blue_dragon',23,7),('ancient_brass_dragon',20,6),('ancient_bronze_dragon',22,7),('ancient_copper_dragon',21,7),('ancient_gold_dragon',24,7),('ancient_green_dragon',22,7),('ancient_red_dragon',24,7),('ancient_silver_dragon',23,7),('ancient_white_dragon',20,6),('androsphinx',17,6),('animated_armor',1,2),('ankheg',2,2),('ankylosaurus',3,2),('ape',0,2),('assassin',8,3),('awakened_tree',2,2),('azer',2,2),('balor',19,6),('bandit',0,2),('bandit_captain',2,2),('barbed_devil',5,3),('basilisk',3,2),('bearded_devil',3,2),('behir',11,4),('berserker',2,2),('black_bear',0,2),('black_dragon_wyrmling',2,2),('black_pudding',4,2),('blink_dog',0,2),('blue_dragon_wyrmling',3,2),('boar',0,2),('bone_devil',9,4),('brass_dragon_wyrmling',1,2),('bronze_dragon_wyrmling',2,2),('brown_bear',1,2),('bugbear',1,2),('bulette',5,3),('centaur',2,2),('chain_devil',8,3),('chimera',6,3),('chuul',4,2),('clay_golem',9,4),('cloaker',8,3),('cloud_giant',9,4),('cockatrice',0,2),('commoner',0,2),('copper_dragon_wyrmling',1,2),('couatl',4,2),('crocodile',0,2),('cult_fanatic',2,2),('cultist',0,2),('darkmantle',0,2),('deva',10,4),('dire_wolf',1,2),('djinni',11,4),('doppelganger',3,2),('dragon_turtle',17,6),('dretch',0,2),('drider',6,3),('drow',0,2),('dryad',1,2),('duergar',1,2),('dust_mephit',0,2),('earth_elemental',5,3),('efreeti',11,4),('elephant',4,2),('erinyes',12,4),('ettercap',2,2),('ettin',4,2),('fire_elemental',5,3),('fire_giant',9,4),('flesh_golem',5,3),('flying_sword',0,2),('frost_giant',8,3),('gargoyle',2,2),('gelatinous_cube',2,2),('ghast',2,2),('ghost',4,2),('ghoul',1,2),('giant_ape',7,3),('giant_bat',0,2),('giant_boar',2,2),('giant_centipede',0,2),('giant_crocodile',5,3),('giant_eagle',1,2),('giant_elk',2,2),('giant_hyena',1,2),('giant_octopus',1,2),('giant_rat',0,2),('giant_scorpion',3,2),('giant_shark',5,3),('giant_snake',2,2),('giant_spider',1,2),('giant_toad',1,2),('giant_wasp',0,2),('giant_wolf_spider',0,2),('gibbering_mouther',2,2),('glabrezu',9,4),('gladiator',5,3),('gnoll',0,2),('goblin',0,2),('gold_dragon_wyrmling',3,2),('gorgon',5,3),('gray_ooze',0,2),('green_dragon_wyrmling',2,2),('green_hag',3,2),('grick',2,2),('griffon',2,2),('grimlock',0,2),('guard',0,2),('guardian_naga',10,4),('gynosphinx',11,4),('harpy',1,2),('hell_hound',3,2),('hezrou',8,3),('hill_giant',5,3),('hippogriff',1,2),('hobgoblin',0,2),('homunculus',0,2),('horned_devil',11,4),('hydra',8,3),('ice_devil',14,5),('ice_mephit',0,2),('imp',1,2),('invisible_stalker',6,3),('iron_golem',16,5),('killer_whale',3,2),('knight',3,2),('kobold',0,2),('kraken',23,7),('lamia',4,2),('lemure',0,2),('lich',21,7),('lion',1,2),('lizardfolk',0,2),('mage',6,3),('magma_mephit',0,2),('magmin',0,2),('mammoth',6,3),('manticore',3,2),('marilith',16,5),('mastiff',0,2),('medusa',6,3),('merfolk',0,2),('merrow',2,2),('mimic',2,2),('minotaur',3,2),('minotaur_skeleton',2,2),('mummy',3,2),('mummy_lord',15,5),('nalfeshnee',13,5),('night_hag',5,3),('nightmare',3,2),('noble',0,2),('ochre_jelly',2,2),('ogre',2,2),('ogre_zombie',2,2),('oni',7,3),('orc',0,2),('otyugh',5,3),('owlbear',3,2),('panther',0,2),('pegasus',2,2),('pit_fiend',20,6),('planetar',16,5),('plesiosaurus',2,2),('polar_bear',2,2),('priest',2,2),('pseudodragon',0,2),('pteranodon',0,2),('purple_worm',15,5),('quasit',1,2),('rakshasa',13,5),('red_dragon_wyrmling',4,2),('remorhaz',11,4),('rhinoceros',2,2),('riding_horse',0,2),('roc',11,4),('roper',5,3),('rug_of_smothering',2,2),('rust_monster',0,2),('saber_toothed_tiger',2,2),('sahuagin',0,2),('salamander',5,3),('satyr',0,2),('scout',0,2),('sea_hag',2,2),('shadow',0,2),('shambling_mound',5,3),('shield_guardian',7,3),('shrieker',0,2),('silver_dragon_wyrmling',2,2),('skeleton',0,2),('skeleton_warhorse',0,2),('solar',21,7),('specter',1,2),('spirit_naga',8,3),('sprite',0,2),('spy',1,2),('steam_mephit',0,2),('stirge',0,2),('stone_giant',7,3),('stone_golem',10,4),('storm_giant',13,5),('succubus',4,2),('swarm_of_insects',0,2),('swarm_of_rats',0,2),('tarrasque',30,9),('thug',0,2),('tiger',1,2),('treant',9,4),('tribal_warrior',0,2),('triceratops',5,3),('troll',5,3),('tyrannosaurus_rex',8,3),('unicorn',5,3),('vampire',13,5),('vampire_spawn',5,3),('veteran',3,2),('violet_fungus',0,2),('vrock',6,3),('warhorse',0,2),('water_elemental',5,3),('werebear',5,3),('wereboar',4,2),('wererat',2,2),('weretiger',4,2),('werewolf',3,2),('white_dragon_wyrmling',2,2),('wight',3,2),('will_o_wisp',2,2),('wolf',0,2),('wraith',5,3),('wyvern',6,3),('xorn',5,3),('young_black_dragon',7,3),('young_blue_dragon',9,4),('young_brass_dragon',6,3),('young_bronze_dragon',8,3),('young_copper_dragon',7,3),('young_gold_dragon',10,4),('young_green_dragon',8,3),('young_red_dragon',10,4),('young_silver_dragon',9,4),('young_white_dragon',6,3),('zombie',0,2)
  ) as v(key, cr, prof)
 where n.game_id is null
   and n.key = v.key;
