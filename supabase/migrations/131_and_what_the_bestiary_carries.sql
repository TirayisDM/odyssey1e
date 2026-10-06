-- 131. AND WHAT THE BESTIARY CARRIES.
--
-- 130's 95 creatures arrived with nothing in their hands. This gives them
-- 158 kit rows, which is what makes them fight: `instantiate_npc` turns
-- each row into an `objects` row, 084 says a non-null slot means equipped,
-- and `attack::resolve` reads techniques only for the EQUIPPED loadout.
-- A creature with no kit is a creature with no attack.
--
-- THE DELETE IS THE IDEMPOTENCY, not a cleanup. `npc_items` has no unique
-- key to conflict on, so re-running an insert would double every kit. The
-- delete is scoped to exactly the 95 keys this migration owns and will not
-- touch 127's rows or anything a game added of its own.
--
-- ---------------------------------------------------------------------
-- ONE WEAPON HELD, THE REST CARRIED
-- ---------------------------------------------------------------------
--
-- 128 fixed `instantiate_npc` so that ranking happens among HELD weapons
-- only, and so a second held weapon takes the off hand when no shield
-- wants it. These rows are written to that shape: the primary attack is
-- `equipped = true`, the secondary is carried, and a creature with a
-- shield holds the shield rather than a second blade.
--
-- A SECOND LIMB IS CARRIED, NOT HELD, and that is deliberate rather than
-- a modelling failure. A bear's claws sit as `equipped = false` next to
-- its bite because two hands is two hands and 128 will not put three
-- things in them. The DM swaps which limb is live; the alternative was a
-- multiattack concept that nothing in the engine has yet.
--
-- `proficient_override = true` ON EVERY ROW. A wolf is proficient with its
-- own mouth, and a giant is proficient with the axe it has carried for a
-- century. Leaving this to the weapon-proficiency walk would have a bear
-- rolling at a penalty to bite someone.
--
-- NATURAL WEAPONS ARE STILL ORDINARY OBJECTS, which 126 already admitted:
-- a DM can unequip a troll's claws or sell them. The tag `natural` on the
-- item is the hook for fixing that and nothing reads it yet.

delete from npc_items where game_id is null and npc_key in ('acolyte','archer','berserker','druid_npc','knight','mage','priest','scout','spy','thug','veteran','tribal_warrior','gnoll','lizardfolk','merfolk','orc_war_chief','goblin_boss','kobold_dragonshield','cult_fanatic','guard_captain','giant_rat','giant_bat','boar','black_bear','panther','lion','tiger','dire_wolf','giant_eagle','giant_snake','giant_scorpion','giant_toad','crocodile','ape','giant_boar','rhinoceros','giant_elk','warhorse','giant_centipede','swarm_of_rats','shadow','ghast','mummy','wraith','skeleton_warhorse','minotaur_skeleton','quasit','hell_hound','succubus','deva','black_dragon_wyrmling','green_dragon_wyrmling','red_dragon_wyrmling','blue_dragon_wyrmling','wyvern','ankheg','basilisk','bulette','cockatrice','hippogriff','griffon','minotaur','rust_monster','ettercap','harpy_matriarch','stone_giant','frost_giant','fire_giant','ettin','troll','dust_mephit','steam_mephit','air_elemental','earth_elemental','fire_elemental','water_elemental','scarecrow','helmed_horror','ochre_jelly','black_pudding','violet_fungus','shambling_mound','awakened_tree','grick','darkmantle','gargoyle','satyr','blink_dog','pixie','gas_spore','carrion_crawler','hook_horror','otyugh','chuul','intellect_devourer');

insert into npc_items (npc_key,item_key,quantity,equipped,proficient_override)
values
('acolyte','club',1,true,true),
('archer','longbow',1,true,true),
('archer','shortsword',1,false,true),
('archer','studded_leather',1,true,true),
('berserker','greataxe',1,true,true),
('berserker','hide',1,true,true),
('druid_npc','quarterstaff',1,true,true),
('knight','greatsword',1,true,true),
('knight','plate',1,true,true),
('knight','shield',1,true,true),
('mage','dagger',1,true,true),
('priest','mace',1,true,true),
('priest','chain_shirt',1,true,true),
('scout','shortsword',1,true,true),
('scout','longbow',1,false,true),
('scout','leather',1,true,true),
('spy','shortsword',1,true,true),
('spy','crossbow_hand',1,false,true),
('thug','mace',1,true,true),
('thug','crossbow_light',1,false,true),
('thug','leather',1,true,true),
('veteran','longsword',1,true,true),
('veteran','shortsword',1,false,true),
('veteran','splint',1,true,true),
('tribal_warrior','spear',1,true,true),
('tribal_warrior','hide',1,true,true),
('gnoll','spear',1,true,true),
('gnoll','longbow',1,false,true),
('gnoll','hide',1,true,true),
('gnoll','shield',1,true,true),
('lizardfolk','spear',1,true,true),
('lizardfolk','shield',1,true,true),
('merfolk','spear',1,true,true),
('orc_war_chief','greataxe',1,true,true),
('orc_war_chief','javelin',3,false,true),
('orc_war_chief','chain_mail',1,true,true),
('goblin_boss','scimitar',1,true,true),
('goblin_boss','javelin',3,false,true),
('goblin_boss','chain_shirt',1,true,true),
('goblin_boss','shield',1,true,true),
('kobold_dragonshield','shortsword',1,true,true),
('kobold_dragonshield','shield',1,true,true),
('kobold_dragonshield','leather',1,true,true),
('cult_fanatic','dagger',1,true,true),
('cult_fanatic','leather',1,true,true),
('guard_captain','longsword',1,true,true),
('guard_captain','half_plate',1,true,true),
('guard_captain','shield',1,true,true),
('giant_rat','bite',1,true,true),
('giant_bat','bite',1,true,true),
('giant_bat','wing',1,false,true),
('boar','tusks',1,true,true),
('black_bear','bite',1,true,true),
('black_bear','claws',1,false,true),
('panther','bite',1,true,true),
('panther','claws',1,false,true),
('lion','bite',1,true,true),
('lion','claws',1,false,true),
('tiger','bite',1,true,true),
('tiger','claws',1,false,true),
('dire_wolf','bite',1,true,true),
('giant_eagle','beak',1,true,true),
('giant_eagle','talons',1,false,true),
('giant_snake','bite',1,true,true),
('giant_snake','constrict',1,false,true),
('giant_scorpion','claws',1,true,true),
('giant_scorpion','sting',1,false,true),
('giant_toad','bite',1,true,true),
('crocodile','bite',1,true,true),
('crocodile','tail',1,false,true),
('ape','fist',1,true,true),
('giant_boar','tusks',1,true,true),
('rhinoceros','gore',1,true,true),
('giant_elk','gore',1,true,true),
('giant_elk','hooves',1,false,true),
('warhorse','hooves',1,true,true),
('giant_centipede','bite',1,true,true),
('swarm_of_rats','bite',1,true,true),
('shadow','claws',1,true,true),
('ghast','claws',1,true,true),
('ghast','bite',1,false,true),
('mummy','fist',1,true,true),
('wraith','claws',1,true,true),
('skeleton_warhorse','hooves',1,true,true),
('minotaur_skeleton','greataxe',1,true,true),
('minotaur_skeleton','gore',1,false,true),
('quasit','claws',1,true,true),
('hell_hound','bite',1,true,true),
('succubus','claws',1,true,true),
('deva','mace',1,true,true),
('black_dragon_wyrmling','bite',1,true,true),
('black_dragon_wyrmling','claws',1,false,true),
('green_dragon_wyrmling','bite',1,true,true),
('green_dragon_wyrmling','claws',1,false,true),
('red_dragon_wyrmling','bite',1,true,true),
('red_dragon_wyrmling','claws',1,false,true),
('blue_dragon_wyrmling','bite',1,true,true),
('blue_dragon_wyrmling','claws',1,false,true),
('wyvern','bite',1,true,true),
('wyvern','sting',1,false,true),
('wyvern','claws',1,false,true),
('ankheg','bite',1,true,true),
('basilisk','bite',1,true,true),
('bulette','bite',1,true,true),
('cockatrice','bite',1,true,true),
('hippogriff','beak',1,true,true),
('hippogriff','claws',1,false,true),
('griffon','beak',1,true,true),
('griffon','claws',1,false,true),
('minotaur','greataxe',1,true,true),
('minotaur','gore',1,false,true),
('rust_monster','bite',1,true,true),
('ettercap','bite',1,true,true),
('ettercap','claws',1,false,true),
('harpy_matriarch','claws',1,true,true),
('harpy_matriarch','wing',1,false,true),
('stone_giant','greatclub',1,true,true),
('frost_giant','greataxe',1,true,true),
('fire_giant','greatsword',1,true,true),
('fire_giant','plate',1,true,true),
('ettin','morningstar',1,true,true),
('ettin','battleaxe',1,false,true),
('troll','bite',1,true,true),
('troll','claws',1,false,true),
('dust_mephit','claws',1,true,true),
('steam_mephit','claws',1,true,true),
('air_elemental','slam',1,true,true),
('air_elemental','wing',1,false,true),
('earth_elemental','slam',1,true,true),
('fire_elemental','slam',1,true,true),
('water_elemental','slam',1,true,true),
('scarecrow','claws',1,true,true),
('helmed_horror','longsword',1,true,true),
('helmed_horror','shield',1,true,true),
('ochre_jelly','pseudopod',1,true,true),
('black_pudding','pseudopod',1,true,true),
('violet_fungus','tendrils',1,true,true),
('shambling_mound','slam',1,true,true),
('shambling_mound','constrict',1,false,true),
('awakened_tree','slam',1,true,true),
('grick','tendrils',1,true,true),
('grick','beak',1,false,true),
('darkmantle','constrict',1,true,true),
('gargoyle','bite',1,true,true),
('gargoyle','claws',1,false,true),
('satyr','shortbow',1,true,true),
('satyr','gore',1,false,true),
('blink_dog','bite',1,true,true),
('pixie','shortbow',1,true,true),
('gas_spore','spines',1,true,true),
('carrion_crawler','tendrils',1,true,true),
('carrion_crawler','bite',1,false,true),
('hook_horror','claws',1,true,true),
('otyugh','bite',1,true,true),
('otyugh','tendrils',1,false,true),
('chuul','claws',1,true,true),
('chuul','tendrils',1,false,true),
('intellect_devourer','claws',1,true,true);
