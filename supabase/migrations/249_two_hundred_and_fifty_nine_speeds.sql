-- 249. TWO HUNDRED AND FIFTY-NINE SPEEDS, TRANSCRIBED.
--
-- Dave: *"seed properly from srd"*, then *"fetch them from the srd
-- api"*. 248 added `npcs.speed` and left every row null; this fills in
-- what the SRD actually says.
--
-- Fetched from `dnd5eapi.co`, one request per creature, 334 of them,
-- on 2026-10-25. **Transcribed, not remembered** - README warns the
-- existing statblocks are "SRD-derived FROM MEMORY rather than
-- transcribed", and writing 259 speeds from memory would have
-- repeated exactly the fault that warning exists for.
--
-- SOURCE AND LICENCE. System Reference Document 5.1, (c) Wizards of
-- the Coast LLC, under the Creative Commons Attribution 4.0
-- International Licence. The API is a redistribution of it.
--
-- SPOT-CHECKED BEFORE WRITING. Eighteen well-known speeds agreed with
-- the fetched data: wolf 40, dire wolf 50, zombie 20, giant eagle 10,
-- pegasus 60, roc 20, tarrasque 40, gelatinous cube 15, stirge 10,
-- mimic 15, shrieker 0, ghost 0, and six more.
--
-- 259 OF 280, AND THE 21 STAY NULL. Null is not a failure here; it is
-- the answer 248 built. Two groups, neither to be guessed at:
--
--   NOT IN THE SRD - ours or from elsewhere: archer, dire_boar,
--   goblin_boss, guard_captain, harpy_matriarch, kobold_dragonshield,
--   orc_war_chief.
--
--   IN THE 5.1 SRD BUT NOT IN THIS API's 2014 DATASET: aarakocra,
--   allosaurus, ankylosaurus, banshee, carrion_crawler, gas_spore,
--   helmed_horror, hook_horror, intellect_devourer, pixie,
--   pteranodon, scarecrow, twig_blight, yuan_ti_pureblood.
--
-- The second group is a gap in the SOURCE rather than in the SRD, and
-- is listed so whoever fills it in knows they are transcribing from a
-- book rather than correcting a mistake.
--
-- A SHAPECHANGER IS ONE ROW HERE AND SEVERAL THERE. **The only
-- judgement in this file.** The API splits a werewolf into human,
-- hybrid and wolf forms; we have one row, and the HUMANOID form was
-- taken because that is what the statblock's own Speed line says - 5e
-- prints "Speed 30 ft. (40 ft. in wolf form)", so the base is the
-- humanoid one and the rest is the parenthetical. Seven rows:
-- vampire, succubus, werebear, wereboar, wererat, weretiger,
-- werewolf. A creature that changes shape and changes speed wants a
-- rule, and there is no rule for shape yet.
--
-- NINE HAVE A WALKING SPEED OF ZERO. Six because the SRD says `0 ft.`
-- - a shrieker and a ghost do not walk - and three because the SRD
-- gives them **no walking speed at all**: an air elemental flies, a
-- giant shark and a killer whale swim.
--
-- Zero is the honest transcription of "cannot walk", and 248 made 0
-- distinct from null for this. It is also TEMPORARILY MISLEADING, and
-- that is worth saying: a shark is not immobile, it is in the wrong
-- medium. This column is the WALKING speed, `stride` spends only
-- walking today, and `terrain.rs` already has `Mode::Swim` and
-- `Mode::Fly` waiting for the columns that say which creatures have
-- them. When those land, these three gain a swim or fly speed beside
-- a walk of zero and nothing here changes.

update public.npcs n
   set speed = v.feet
  from (values
('aboleth',10),('acolyte',30),('adult_black_dragon',40),('adult_blue_dragon',40),('adult_brass_dragon',40),('adult_bronze_dragon',40),('adult_copper_dragon',40),('adult_gold_dragon',40),('adult_green_dragon',40),('adult_red_dragon',40),('adult_silver_dragon',40),('adult_white_dragon',40),('air_elemental',0),('ancient_black_dragon',40),('ancient_blue_dragon',40),('ancient_brass_dragon',40),('ancient_bronze_dragon',40),('ancient_copper_dragon',40),('ancient_gold_dragon',40),('ancient_green_dragon',40),('ancient_red_dragon',40),('ancient_silver_dragon',40),('ancient_white_dragon',40),('androsphinx',40),('animated_armor',25),('ankheg',30),('ape',30),('assassin',30),('awakened_tree',20),('azer',30),('balor',40),('bandit',30),('bandit_captain',30),('barbed_devil',30),('basilisk',20),('bearded_devil',30),('behir',50),('berserker',30),('black_bear',40),('black_dragon_wyrmling',30),('black_pudding',20),('blink_dog',40),('blue_dragon_wyrmling',30),('boar',40),('bone_devil',40),('brass_dragon_wyrmling',30),('bronze_dragon_wyrmling',30),('brown_bear',40),('bugbear',30),('bulette',40),('centaur',50),('chain_devil',30),('chimera',30),('chuul',30),('clay_golem',20),('cloaker',10),('cloud_giant',40),('cockatrice',20),('commoner',30),('copper_dragon_wyrmling',30),('couatl',30),('crocodile',20),('cult_fanatic',30),('cultist',30),('darkmantle',10),('deva',30),('dire_wolf',50),('djinni',30),('doppelganger',30),('dragon_turtle',20),('dretch',20),('drider',30),('drow',30),('druid_npc',30),('dryad',30),('duergar',25),('dust_mephit',30),('earth_elemental',30),('efreeti',40),('elephant',40),('erinyes',30),('ettercap',30),('ettin',40),('fire_elemental',50),('fire_giant',30),('flesh_golem',30),('flying_sword',0),('frost_giant',40),('gargoyle',30),('gelatinous_cube',15),('ghast',30),('ghost',0),('ghoul',30),('giant_ape',40),('giant_bat',10),('giant_boar',40),('giant_centipede',30),('giant_crocodile',30),('giant_eagle',10),('giant_elk',60),('giant_hyena',50),('giant_octopus',10),('giant_rat',30),('giant_scorpion',40),('giant_shark',0),('giant_snake',30),('giant_spider',30),('giant_toad',20),('giant_wasp',10),('giant_wolf_spider',40),('gibbering_mouther',10),('glabrezu',40),('gladiator',30),('gnoll',30),('goblin',30),('gold_dragon_wyrmling',30),('gorgon',40),('gray_ooze',10),('green_dragon_wyrmling',30),('green_hag',30),('grick',30),('griffon',30),('grimlock',30),('guard',30),('guardian_naga',40),('gynosphinx',40),('harpy',20),('hell_hound',50),('hezrou',30),('hill_giant',40),('hippogriff',40),('hobgoblin',30),('homunculus',20),('horned_devil',20),('hydra',30),('ice_devil',40),('ice_mephit',30),('imp',20),('invisible_stalker',50),('iron_golem',30),('killer_whale',0),('knight',30),('kobold',30),('kraken',20),('lamia',30),('lemure',15),('lich',30),('lion',50),('lizardfolk',30),('mage',30),('magma_mephit',30),('magmin',30),('mammoth',40),('manticore',30),('marilith',40),('mastiff',40),('medusa',30),('merfolk',10),('merrow',10),('mimic',15),('minotaur',40),('minotaur_skeleton',40),('mummy',20),('mummy_lord',20),('nalfeshnee',20),('night_hag',30),('nightmare',60),('noble',30),('ochre_jelly',10),('ogre',40),('ogre_zombie',30),('oni',30),('orc',30),('otyugh',30),('owlbear',40),('panther',50),('pegasus',60),('pit_fiend',30),('planetar',40),('plesiosaurus',20),('polar_bear',40),('priest',25),('pseudodragon',15),('purple_worm',50),('quasit',40),('rakshasa',40),('red_dragon_wyrmling',30),('remorhaz',30),('rhinoceros',40),('riding_horse',60),('roc',20),('roper',10),('rug_of_smothering',10),('rust_monster',40),('saber_toothed_tiger',40),('sahuagin',30),('salamander',30),('satyr',40),('scout',30),('sea_hag',30),('shadow',40),('shambling_mound',20),('shield_guardian',30),('shrieker',0),('silver_dragon_wyrmling',30),('skeleton',30),('skeleton_warhorse',60),('solar',50),('specter',0),('spirit_naga',40),('sprite',10),('spy',30),('steam_mephit',30),('stirge',10),('stone_giant',40),('stone_golem',30),('storm_giant',50),('succubus',30),('swarm_of_insects',20),('swarm_of_rats',30),('tarrasque',40),('thug',30),('tiger',40),('treant',30),('tribal_warrior',30),('triceratops',50),('troll',30),('tyrannosaurus_rex',50),('unicorn',50),('vampire',30),('vampire_spawn',30),('veteran',30),('violet_fungus',5),('vrock',40),('warhorse',60),('water_elemental',30),('werebear',30),('wereboar',30),('wererat',30),('weretiger',30),('werewolf',30),('white_dragon_wyrmling',30),('wight',30),('will_o_wisp',0),('wolf',40),('wraith',0),('wyvern',20),('xorn',20),('young_black_dragon',40),('young_blue_dragon',40),('young_brass_dragon',40),('young_bronze_dragon',40),('young_copper_dragon',40),('young_gold_dragon',40),('young_green_dragon',40),('young_red_dragon',40),('young_silver_dragon',40),('young_white_dragon',40),('zombie',20)
  ) as v(key, feet)
 where n.key = v.key
   and n.game_id is null;

-- A SELF-CHECK THAT REFUSES RATHER THAN REPORTS. 259 rows were
-- generated from 259 matches; if fewer landed, a key has been renamed
-- since the match was computed and the difference would otherwise sit
-- in the database as a handful of monsters that quietly never got a
-- speed.
do $$
declare
  filled int;
begin
  select count(*) into filled from public.npcs
   where game_id is null and speed is not null;
  if filled <> 259 then
    raise exception
      'expected 259 seeded speeds, found % - a key has moved since the SRD match was computed',
      filled;
  end if;
end $$;

-- 248's comment said 134 creatures. It is 280, and has been since the
-- bestiary grew in 137-144; the migration itself is applied and stays
-- as written - 115 - but the live comment is metadata and can be put
-- right where there is a reason to touch it, which this is.
comment on column public.npcs.speed is
  'Walking speed in FEET, or null for "nobody has said" - deliberately '
  'not defaulted, because 280 creatures would have claimed a number '
  'nobody checked. 0 is a real answer: either the SRD says 0 ft., or '
  'it gives no walking speed at all because the creature flies or '
  'swims. The walking speed ONLY: swim, climb, fly and burrow arrive '
  'with the modes that spend them. 259 of 280 seeded from SRD 5.1 via '
  'dnd5eapi.co on 2026-10-25; the other 21 are listed in 249. Read by '
  '`speed.rs`, the one place a creature''s speed is resolved.';
