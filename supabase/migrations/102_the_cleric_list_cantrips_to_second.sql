-- 102. THE CLERIC LIST: CANTRIPS, 1st AND 2nd.
--
-- The PHB cleric list, which is 106 spells across ten levels. This is
-- the first third; 103 and 104 carry the rest.
--
-- THE TEXT IS OURS. Every line is written from scratch to say what a
-- spell DOES in one sentence, so a player at the table does not have to
-- look it up for the common case. It is NOT the PHB's wording and does
-- not replace owning the PHB - most of these have ranges of effect,
-- saving-throw details and upcasting rules this does not carry.
--
-- `dice` IS THE BASE, never an upcast and never with a modifier folded
-- in. Cure Wounds is 1d8 and the caster's Wisdom is added at the roll,
-- which is exactly what 101 cleared the baked 2d8+4 for.
--
-- `cast_type` KEEPS 006'S VOCABULARY - Attack, Save, Heal, Utility -
-- because the narrator already reads it and a second set of words for
-- the same four ideas would be a translation nobody asked for.
--
-- EXISTING KEYS ARE UPDATED, NOT DUPLICATED. Nineteen of these were
-- already here from the AppSheet export under keys like `sp_sacredflame`,
-- so the conflict target is 101's own global-key index and the update
-- normalises what the export had baked.

insert into spells
  (key, name, roll_name, level, cast_type, category, school, save_ability,
   dice, concentration, ritual, range, duration, casting_time,
   components, material, classes, description)
values
-- ============================== CANTRIPS ==============================
('sp_guidance','Guidance','Guidance',0,'Utility','Cantrips','divination',null,
 '1d4',true,false,'Touch','1 minute','1 action',array['v','s'],null,array['cleric','druid'],
 'A willing creature you touch adds 1d4 to one ability check of its choice before the spell ends.'),
('sp_light','Light','Light',0,'Utility','Cantrips','evocation','dex',
 null,false,false,'Touch','1 hour','1 action',array['v','m'],'a firefly or phosphorescent moss',array['cleric','bard','sorcerer','wizard'],
 'An object you touch sheds bright light in a 20-foot radius and dim light for 20 beyond. An unwilling bearer may dodge the spell with a Dexterity save.'),
('sp_mending','Mending','Mending',0,'Utility','Cantrips','transmutation',null,
 null,false,false,'Touch','Instantaneous','1 minute',array['v','s','m'],'two lodestones',array['cleric','bard','druid','sorcerer','wizard'],
 'Repairs a single break or tear in an object no larger than a foot across. It restores no hit points and does nothing to a magic item.'),
('sp_resistance','Resistance','Resistance',0,'Utility','Cantrips','abjuration',null,
 '1d4',true,false,'Touch','1 minute','1 action',array['v','s','m'],'a miniature cloak',array['cleric','druid'],
 'A willing creature you touch adds 1d4 to one saving throw of its choice before the spell ends.'),
('sp_sacredflame','Sacred Flame','Sacred Flame',0,'Save','Cantrips','evocation','dex',
 '1d8',false,false,'60 feet','Instantaneous','1 action',array['v','s'],null,array['cleric'],
 'Radiant fire falls on one creature you can see. A failed Dexterity save takes the damage, and cover does not help against it. The dice grow at 5th, 11th and 17th level.'),
('sp_sparethedying','Spare the Dying','Spare the Dying',0,'Heal','Cantrips','necromancy',null,
 null,false,false,'Touch','Instantaneous','1 action',array['v','s'],null,array['cleric'],
 'A creature at 0 hit points becomes stable. It heals nothing and does nothing to the undead or a construct.'),
('sp_thaumaturgy','Thaumaturgy','Thaumaturgy',0,'Utility','Cantrips','transmutation',null,
 null,false,false,'30 feet','Up to 1 minute','1 action',array['v'],null,array['cleric'],
 'A small wonder: your voice booms, flames change colour, a door slams, the ground trembles. Up to three at once, and none of them does any harm.'),

-- ================================ 1st ================================
('sp_bane','Bane','Bane',1,'Save','1st Circle','enchantment','cha',
 '1d4',true,false,'30 feet','Up to 1 minute','1 action',array['v','s','m'],'a drop of blood',array['cleric','bard'],
 'Up to three creatures that fail a Charisma save subtract 1d4 from each attack roll and saving throw they make while it lasts.'),
('sp_bless','Bless','Bless',1,'Utility','1st Circle','enchantment',null,
 '1d4',true,false,'30 feet','Up to 1 minute','1 action',array['v','s','m'],'a sprinkling of holy water',array['cleric','paladin'],
 'Up to three creatures add 1d4 to each attack roll and saving throw they make while it lasts.'),
('sp_command','Command','Command',1,'Save','1st Circle','enchantment','wis',
 null,false,false,'60 feet','1 round','1 action',array['v'],null,array['cleric','paladin'],
 'One word of command. A creature that fails a Wisdom save obeys on its next turn - approach, drop, flee, grovel, halt - unless the order would plainly harm it.'),
('sp_createordestroywater','Create or Destroy Water','Create or Destroy Water',1,'Utility','1st Circle','transmutation',null,
 null,false,false,'30 feet','Instantaneous','1 action',array['v','s','m'],'a drop of water or a pinch of dust',array['cleric','druid'],
 'Ten gallons of clean water appear in a container or fall as rain, or as much is destroyed or turned to fog.'),
('sp_curewounds','Cure Wounds','Cure Wounds',1,'Heal','1st Circle','evocation',null,
 '1d8',false,false,'Touch','Instantaneous','1 action',array['v','s'],null,array['cleric','bard','druid','paladin','ranger'],
 'A creature you touch regains 1d8 plus your spellcasting modifier in hit points. Nothing for a construct or the undead.'),
('sp_detectevilandgood','Detect Evil and Good','Detect Evil and Good',1,'Utility','1st Circle','divination',null,
 null,true,false,'Self (30-foot radius)','Up to 10 minutes','1 action',array['v','s'],null,array['cleric','paladin'],
 'You sense aberrations, celestials, elementals, fey, fiends and undead within 30 feet, and any ground that has been consecrated or desecrated. Not what they are, only where.'),
('sp_detectmagic','Detect Magic','Detect Magic',1,'Utility','1st Circle','divination',null,
 null,true,true,'Self (30-foot radius)','Up to 10 minutes','1 action',array['v','s'],null,array['cleric','bard','druid','paladin','ranger','sorcerer','wizard'],
 'You sense magic within 30 feet, and may spend an action to learn the school of one aura. A ritual, so it can be cast without a slot given the ten minutes.'),
('sp_detectpoisonanddisease','Detect Poison and Disease','Detect Poison and Disease',1,'Utility','1st Circle','divination',null,
 null,true,true,'Self (30-foot radius)','Up to 10 minutes','1 action',array['v','s','m'],'a yew leaf',array['cleric','druid','paladin','ranger'],
 'You sense poisons, poisonous creatures and diseases within 30 feet, and what kind each is.'),
('sp_guidingbolt','Guiding Bolt','Guiding Bolt',1,'Attack','1st Circle','evocation',null,
 '4d6',false,false,'120 feet','1 round','1 action',array['v','s'],null,array['cleric'],
 'A ranged spell attack for 4d6 radiant damage, and the next attack against that target before your next turn has advantage.'),
('sp_healingword','Healing Word','Healing Word',1,'Heal','1st Circle','evocation',null,
 '1d4',false,false,'60 feet','Instantaneous','1 bonus action',array['v'],null,array['cleric','bard','druid'],
 'A creature you can see regains 1d4 plus your spellcasting modifier. A bonus action and sixty feet, which is what makes it the spell that gets somebody up.'),
('sp_inflictwounds','Inflict Wounds','Inflict Wounds',1,'Attack','1st Circle','necromancy',null,
 '3d10',false,false,'Touch','Instantaneous','1 action',array['v','s'],null,array['cleric'],
 'A melee spell attack for 3d10 necrotic damage.'),
('sp_protectionfromevilandgood','Protection from Evil and Good','Protection from Evil and Good',1,'Utility','1st Circle','abjuration',null,
 null,true,false,'Touch','Up to 10 minutes','1 action',array['v','s','m'],'holy water or powdered silver and iron, consumed',array['cleric','paladin','warlock','wizard'],
 'Aberrations, celestials, elementals, fey, fiends and undead have disadvantage to hit the warded creature, and cannot charm, frighten or possess it.'),
('sp_purifyfoodanddrink','Purify Food and Drink','Purify Food and Drink',1,'Utility','1st Circle','transmutation',null,
 null,false,true,'10 feet','Instantaneous','1 action',array['v','s'],null,array['cleric','druid','paladin'],
 'All food and drink in a 5-foot sphere is rid of poison and disease.'),
('sp_sanctuary','Sanctuary','Sanctuary',1,'Utility','1st Circle','abjuration','wis',
 null,false,false,'30 feet','1 minute','1 bonus action',array['v','s','m'],'a silver mirror',array['cleric'],
 'Anything targeting the warded creature must pass a Wisdom save or choose a new target. It ends the moment the warded creature attacks or casts at somebody.'),
('sp_shieldoffaith','Shield of Faith','Shield of Faith',1,'Utility','1st Circle','abjuration',null,
 null,true,false,'60 feet','Up to 10 minutes','1 bonus action',array['v','s','m'],'a written scrap of scripture',array['cleric','paladin'],
 '+2 AC to one creature you can see, for as long as you hold concentration.'),

-- ================================ 2nd ================================
('sp_aid','Aid','Aid',2,'Heal','2nd Circle','abjuration',null,
 null,false,false,'30 feet','8 hours','1 action',array['v','s','m'],'a strip of white cloth',array['cleric','paladin'],
 'Three creatures gain 5 maximum and 5 current hit points for eight hours. Five more per slot level above 2nd.'),
('sp_augury','Augury','Augury',2,'Utility','2nd Circle','divination',null,
 null,false,true,'Self','Instantaneous','1 minute',array['v','s','m'],'specially marked sticks or bones worth 25 gp',array['cleric'],
 'You learn whether a course of action in the next half hour brings weal, woe, both or neither. Casting it more than once in a day risks a random answer.'),
('sp_blindnessdeafness','Blindness/Deafness','Blindness or Deafness',2,'Save','2nd Circle','necromancy','con',
 null,false,false,'30 feet','1 minute','1 action',array['v'],null,array['cleric','bard','sorcerer','wizard'],
 'One creature that fails a Constitution save is blinded or deafened, your choice. It saves again at the end of each of its turns.'),
('sp_calmemotions','Calm Emotions','Calm Emotions',2,'Save','2nd Circle','enchantment','cha',
 null,true,false,'60 feet','Up to 1 minute','1 action',array['v','s'],null,array['cleric','bard'],
 'Humanoids in a 20-foot sphere who fail a Charisma save either lose any charm or fright on them, or become indifferent to creatures they were hostile to.'),
('sp_continualflame','Continual Flame','Continual Flame',2,'Utility','2nd Circle','evocation',null,
 null,false,false,'Touch','Until dispelled','1 action',array['v','s','m'],'ruby dust worth 50 gp, consumed',array['cleric','wizard'],
 'A flame that gives light like a torch, needs no air or fuel, gives off no heat, and lasts until somebody dispels it.'),
('sp_enhanceability','Enhance Ability','Enhance Ability',2,'Utility','2nd Circle','transmutation',null,
 null,true,false,'Touch','Up to 1 hour','1 action',array['v','s','m'],'fur or a feather from a beast',array['cleric','bard','druid','sorcerer'],
 'A creature gains advantage on checks with one ability of your choice, and some choices add more - temporary hit points with Constitution, a doubled carry with Strength.'),
('sp_findtraps','Find Traps','Find Traps',2,'Utility','2nd Circle','divination',null,
 null,false,false,'120 feet','Instantaneous','1 action',array['v','s'],null,array['cleric','druid','ranger'],
 'You learn whether a trap is present within range and what kind of danger it is, but not where it is.'),
('sp_gentlerepose','Gentle Repose','Gentle Repose',2,'Utility','2nd Circle','necromancy',null,
 null,false,true,'Touch','10 days','1 action',array['v','s','m'],'a pinch of salt and a copper piece on each eye',array['cleric','wizard'],
 'A corpse does not decay and cannot become undead. The ten days do not count against the time limit on raising it.'),
('sp_holdperson','Hold Person','Hold Person',2,'Save','2nd Circle','enchantment','wis',
 null,true,false,'60 feet','Up to 1 minute','1 action',array['v','s','m'],'a straight piece of iron',array['cleric','bard','druid','sorcerer','warlock','wizard'],
 'A humanoid that fails a Wisdom save is paralysed, saving again at the end of each of its turns. One more target per slot level above 2nd.'),
('sp_lesserrestoration','Lesser Restoration','Lesser Restoration',2,'Utility','2nd Circle','abjuration',null,
 null,false,false,'Touch','Instantaneous','1 action',array['v','s'],null,array['cleric','bard','druid','paladin','ranger'],
 'Ends one disease or one condition on a creature you touch: blinded, deafened, paralysed or poisoned.'),
('sp_locateobject','Locate Object','Locate Object',2,'Utility','2nd Circle','divination',null,
 null,true,false,'Self (1,000 feet)','Up to 10 minutes','1 action',array['v','s','m'],'a forked twig',array['cleric','bard','druid','paladin','ranger','wizard'],
 'You sense the direction of a named object within 1,000 feet, if you have seen it. Lead of any thickness blocks it.'),
('sp_prayerofhealing','Prayer of Healing','Prayer of Healing',2,'Heal','2nd Circle','evocation',null,
 '2d8',false,false,'30 feet','Instantaneous','10 minutes',array['v'],null,array['cleric'],
 'Up to six creatures each regain 2d8 plus your spellcasting modifier. Ten minutes to cast, which is why it is a spell for after the fight.'),
('sp_protectionfrompoison','Protection from Poison','Protection from Poison',2,'Utility','2nd Circle','abjuration',null,
 null,false,false,'Touch','1 hour','1 action',array['v','s'],null,array['cleric','druid','paladin','ranger'],
 'Neutralises one poison on the creature, and for an hour it has advantage on saves against poison and resistance to poison damage.'),
('sp_silence','Silence','Silence',2,'Utility','2nd Circle','illusion',null,
 null,true,true,'120 feet','Up to 10 minutes','1 action',array['v','s'],null,array['cleric','bard','ranger'],
 'No sound can be created within or pass into a 20-foot sphere. Creatures inside are deafened and immune to thunder damage, and nothing verbal can be cast there.'),
('sp_spiritualweapon','Spiritual Weapon','Spiritual Weapon',2,'Attack','2nd Circle','evocation',null,
 '1d8',false,false,'60 feet','1 minute','1 bonus action',array['v','s'],null,array['cleric'],
 'A floating spectral weapon appears and strikes for 1d8 force plus your spellcasting modifier. A bonus action to cast and a bonus action to move and attack again.'),
('sp_wardingbond','Warding Bond','Warding Bond',2,'Utility','2nd Circle','abjuration',null,
 null,false,false,'Touch','1 hour','1 action',array['v','s','m'],'a pair of platinum rings worth 50 gp each, worn by both',array['cleric'],
 'The warded creature gains +1 AC and saves and resistance to all damage - and you take the same damage it does, which is the whole of the bargain.'),
('sp_zoneoftruth','Zone of Truth','Zone of Truth',2,'Save','2nd Circle','enchantment','cha',
 null,false,false,'60 feet','10 minutes','1 action',array['v','s'],null,array['cleric','bard','paladin'],
 'Creatures in a 15-foot sphere that fail a Charisma save cannot speak a deliberate lie. They know they are affected, and can still evade a question.')

on conflict (key) where game_id is null do update set
  name = excluded.name, roll_name = excluded.roll_name, level = excluded.level,
  cast_type = excluded.cast_type, category = excluded.category,
  school = excluded.school, save_ability = excluded.save_ability,
  dice = excluded.dice, concentration = excluded.concentration,
  ritual = excluded.ritual, range = excluded.range, duration = excluded.duration,
  casting_time = excluded.casting_time, components = excluded.components,
  material = excluded.material, classes = excluded.classes,
  description = excluded.description,
  spell_atk = null, dc = null;