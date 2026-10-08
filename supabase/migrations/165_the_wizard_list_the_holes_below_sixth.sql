-- 165. THE WIZARD LIST: THE HOLES BELOW SIXTH.
--
-- RECOVERED FROM THE DATABASE, NOT FROM THE LAPTOP THAT WROTE IT.
-- Applied 2026-10-07 at 21:35 UTC; the file was never pushed. Supabase
-- kept the statement and stripped the comment header, so everything
-- below this block is EXACTLY what ran - MD5-verified against
-- `supabase_migrations.schema_migrations` - and everything inside it
-- was written afterwards by somebody who was not there.
--
-- THE ORIGINAL HEADER IS STILL ON THE LAPTOP and will say why rather
-- than what. Overwrite this file from there; the SQL will match.
--
-- WHAT IT DOES, read off the statement: 161-163 filled the wizard list
-- level by level and left gaps - 24 wizard-only spells below sixth
-- circle that those three did not carry. This is that set, from Find
-- Familiar at first to Wall of Force at fifth.
--
-- WORTH NOTING, because it is the first migration to do it: these rows
-- set `on_save` in the INSERT itself rather than being swept afterwards
-- the way 164 had to sweep 161-163. Black Tentacles and Phantasmal
-- Killer are both 'none', and both for the same reason - neither deals
-- its damage on the save that the spell's own DC is rolled against.

insert into spells
  (key, name, roll_name, level, cast_type, category, school, save_ability, dice,
   on_save, concentration, ritual, range, duration, casting_time, components,
   material, classes, special_text, description)
values
('sp_findfamiliar','Find Familiar','Find Familiar',1,'Utility','1st Circle','conjuration',null,null,
 null,false,true,'10 feet','Instant','1 hour',array['v','s','m'],
 'charcoal, incense and herbs worth 10 gp, burnt in a brass brazier',
 array['wizard'],
 'The familiar is a celestial, fey or fiend in animal shape - bat, cat, crab, frog, hawk, lizard, octopus, owl, poisonous snake, fish, rat, raven, sea horse, spider or weasel. It acts on its own initiative and CANNOT ATTACK. You may use an action to see and hear through it, and you may cast a touch spell through it at a range of 100 feet.',
 'A spirit in animal shape becomes yours: a scout, a second pair of eyes, and a hand for touch spells.'),
('sp_floatingdisk','Floating Disk','Floating Disk',1,'Utility','1st Circle','conjuration',null,null,
 null,false,true,'30 feet','1 hour','1 action',array['v','s','m'],'a drop of mercury',
 array['wizard'],
 'Three feet across, carries 500 pounds, floats 3 feet off the ground and follows you at 20 feet. It cannot cross a change in height greater than 10 feet, and it drops everything and vanishes if you get more than 100 feet away.',
 'A horizontal plate of force that carries your load and trails after you.'),
('sp_grease','Grease','Grease',1,'Save','1st Circle','conjuration','dex',null,
 null,false,false,'60 feet','1 minute','1 action',array['v','s','m'],'a bit of pork rind or butter',
 array['wizard'],
 'A 10-foot square of difficult terrain. Anything standing there when it appears makes a Dexterity save or falls prone, and anything that enters or ends its turn there saves as well.',
 'Slick grease across the ground: a Dexterity save, or down you go.'),
('sp_acidarrow','Acid Arrow','Acid Arrow',2,'Attack','2nd Circle','evocation',null,'4d4',
 null,false,false,'90 feet','Instant','1 action',array['v','s','m'],'powdered rhubarb leaf and an adder stomach',
 array['wizard'],
 'A ranged spell attack. A hit deals 4d4 acid immediately and 2d4 more at the end of the target''s next turn. A MISS STILL DEALS HALF the initial damage and nothing afterwards. Both rolls gain 1d4 per slot level above 2nd.',
 'A green shaft of acid that bursts on impact and keeps eating afterwards.'),
('sp_arcanelock','Arcane Lock','Arcane Lock',2,'Utility','2nd Circle','abjuration',null,null,
 null,false,false,'Touch','Until dispelled','1 action',array['v','s','m'],'gold dust worth 25 gp, consumed',
 array['wizard'],
 'You and anyone you name as you cast it open the thing normally. For everybody else the DC to break, force or pick it rises by 10. Knock suppresses the lock for 10 minutes.',
 'A door, chest or window is sealed shut until somebody dispels the spell.'),
('sp_arcanistsmagicaura','Arcanist''s Magic Aura','Magic Aura',2,'Utility','2nd Circle','illusion',null,null,
 null,false,false,'Touch','24 hours','1 action',array['v','s','m'],'a small square of silk',
 array['wizard'],
 'Choose either effect or both. FALSE AURA makes divination report a different school of magic, or none at all, and can make a magic item read as mundane or a mundane one read as magical. MASK makes a creature read as a different creature type for every spell that cares. Cast on the same target every day for 30 days and it becomes permanent.',
 'Divination magic is told a convincing lie about what it is looking at.'),
('sp_ropetrick','Rope Trick','Rope Trick',2,'Utility','2nd Circle','transmutation',null,null,
 null,false,false,'Touch','1 hour','1 action',array['v','s','m'],'powdered corn extract and a twisted loop of parchment',
 array['wizard'],
 'Up to eight Medium or smaller creatures fit inside, and the rope can be pulled up after them so there is no visible entrance. Everything inside falls out when the spell ends.',
 'A rope stands on end and the air at the top of it opens into a room that is not there.'),
('sp_phantomsteed','Phantom Steed','Phantom Steed',3,'Utility','3rd Circle','illusion',null,null,
 null,false,true,'30 feet','1 hour','1 minute',array['v','s'],null,
 array['wizard'],
 'A Large horselike creature, saddled and bridled, with AC 14, 1 HIT POINT, no attacks and a speed of 100 feet. It covers 13 miles an hour at a trot and 10 at a walk, and it vanishes the moment it takes any damage. When the spell ends you have one minute to dismount.',
 'A quasi-real horse appears already saddled and runs tirelessly for an hour.'),
('sp_arcaneeye','Arcane Eye','Arcane Eye',4,'Utility','4th Circle','divination',null,null,
 null,true,false,'30 feet','Up to 1 hour','1 action',array['v','s','m'],'a bit of bat fur',
 array['wizard'],
 'An invisible eye with normal vision and darkvision out to 30 feet, which you see through while you concentrate. Move it 30 feet in any direction as an action. It passes through an opening as small as 1 inch and is stopped flat by solid matter.',
 'A floating eye you see through, sent wherever you can get it to go.'),
('sp_blacktentacles','Black Tentacles','Black Tentacles',4,'Save','4th Circle','conjuration','dex','3d6',
 'none',true,false,'90 feet','Up to 1 minute','1 action',array['v','s','m'],'a piece of tentacle from a giant octopus or giant squid',
 array['wizard'],
 'A 20-foot square of difficult terrain. A failed Dexterity save takes 3d6 bludgeoning and is RESTRAINED; a success takes nothing and stays free. A restrained creature takes 3d6 again at the start of each of its turns, and may spend its action on a Strength or Dexterity check against your save DC to pull loose.',
 'Tentacles tear up through the ground and take hold of whatever is standing on it.'),
('sp_fabricate','Fabricate','Fabricate',4,'Utility','4th Circle','transmutation',null,null,
 null,false,false,'120 feet','Instant','10 minutes',array['v','s'],null,
 array['wizard'],
 'A Large or smaller mass of raw material, or up to ten connected Tiny objects. MINERAL MATTER IS CAPPED AT MEDIUM - stone, crystal, metal. Anything that would normally take a craft, such as a weapon or a suit of armour, requires proficiency with the tools for that craft.',
 'Raw material reshapes itself into a finished object of the same material.'),
('sp_faithfulhound','Faithful Hound','Faithful Hound',4,'Utility','4th Circle','conjuration',null,'4d8',
 null,false,false,'30 feet','8 hours','1 action',array['v','s','m'],'a tiny silver whistle, a piece of bone and a thread',
 array['wizard'],
 'Invisible, and audible only to you. It barks at anything hostile that comes within 30 feet and BITES anything hostile that comes within 5 feet, attacking with your spell attack bonus for 4d8 piercing. It sees through illusion and invisibility, and it vanishes if you move more than 100 feet from it.',
 'A watchdog nobody can see stands guard where you left it and bites what comes close.'),
('sp_fireshield','Fire Shield','Fire Shield',4,'Utility','4th Circle','evocation',null,'2d8',
 null,false,false,'Self','10 minutes','1 action',array['v','s','m'],'a bit of phosphorus or a firefly',
 array['wizard'],
 'Choose WARM for resistance to cold damage or CHILL for resistance to fire. Either way, anything within 5 feet that hits you with a melee attack takes 2d8 - fire from the warm shield, cold from the chill one. The shield sheds bright light in a 10-foot radius.',
 'Flames wreathe you without burning you: they turn one element aside and scorch whatever strikes you.'),
('sp_phantasmalkiller','Phantasmal Killer','Phantasmal Killer',4,'Save','4th Circle','illusion','wis','4d10',
 'none',true,false,'120 feet','Up to 1 minute','1 action',array['v','s'],null,
 array['wizard'],
 'THE FIRST SAVE DOES NOT DAMAGE: a failure leaves the target frightened for the duration. The 4d10 psychic comes at the END OF EACH OF ITS TURNS, on another Wisdom save, all of it or none of it. +1d10 per slot level above 4th.',
 'The target alone sees the thing it fears most, and standing near it is fatal.'),
('sp_privatesanctum','Private Sanctum','Private Sanctum',4,'Utility','4th Circle','abjuration',null,null,
 null,false,false,'120 feet','24 hours','10 minutes',array['v','s','m'],'a thin sheet of lead, a piece of opaque glass, a wad of cloth and powdered chrysolite',
 array['wizard'],
 'From a 5-foot square up to 100 feet on a side, 100 feet more per slot level above 4th. Choose any of: no sound crosses the boundary, the inside is dark to anyone outside, divination cannot reach in, scrying sensors cannot form, and nothing teleports or planeshifts across it. Cast in the same place every day for a year and it becomes permanent.',
 'An area is sealed against sound, sight, scrying and travel, in whatever combination you pick.'),
('sp_resilientsphere','Resilient Sphere','Resilient Sphere',4,'Save','4th Circle','evocation','dex',null,
 null,true,false,'30 feet','Up to 1 minute','1 action',array['v','s','m'],'a hemispherical piece of clear crystal and a matching piece of gum arabic',
 array['wizard'],
 'An unwilling target saves. NOTHING crosses the shell in either direction and nothing damages it, except that a Disintegrate destroys the sphere without harming what is inside. A creature within can push the sphere along at half its speed, and the whole thing weighs nothing however heavy its contents.',
 'A sphere of force closes around one creature or object and nothing gets through it.'),
('sp_secretchest','Secret Chest','Secret Chest',4,'Utility','4th Circle','conjuration',null,null,
 null,false,false,'Touch','Instant','1 action',array['v','s','m'],'an exquisite chest worth 5,000 gp and a tiny replica of it worth 50 gp',
 array['wizard'],
 'Hold the replica and spend an action to recall the chest to an unoccupied space within 5 feet, or to send it away again. AFTER 60 DAYS there is a cumulative 5 percent chance per day that the spell ends and the chest is lost for good; recalling it resets the count. Losing the replica means losing the chest.',
 'A chest and everything in it waits on the Ethereal Plane until you call it back.'),
('sp_arcanehand','Arcane Hand','Arcane Hand',5,'Utility','5th Circle','evocation',null,'4d8',
 null,true,false,'120 feet','Up to 1 minute','1 action',array['v','s','m'],'an eggshell and a snakeskin glove',
 array['wizard'],
 'AC 20, hit points equal to yours, Strength 26 and Dexterity 10. It moves up to 60 feet when you move it, and a bonus action picks one of four things for it to do. CLENCHED FIST: a melee spell attack for 4d8 force. FORCEFUL HAND: a Strength check against the target, shoving it 5 feet plus 5 more for every 10 points you win by. GRASPING HAND: grappling a Huge or smaller creature, and crushing it for 2d6 plus your spellcasting modifier each turn you repeat it. INTERPOSING HAND: standing between you and one creature, giving you half cover against it and doubling what its movement toward you costs. +2d8 to the fist and +2d6 to the crush per slot level above 5th.',
 'A huge hand of force answers you, and every turn you decide what it does with itself.'),
('sp_passwall','Passwall','Passwall',5,'Utility','5th Circle','transmutation',null,null,
 null,false,false,'30 feet','1 hour','1 action',array['v','s','m'],'a pinch of sesame seeds',
 array['wizard'],
 'Up to 5 feet wide, 8 feet tall and 20 feet deep, through wood, plaster or stone. When the spell ends anything still inside the passage is pushed harmlessly out to the nearest unoccupied space.',
 'A tunnel opens through a wall, floor or ceiling, and closes again in an hour.'),
('sp_telepathicbond','Telepathic Bond','Telepathic Bond',5,'Utility','5th Circle','divination',null,null,
 null,false,true,'30 feet','1 hour','1 action',array['v','s','m'],'pieces of eggshell from two different kinds of creature',
 array['wizard'],
 'Up to eight willing creatures, each needing an Intelligence of 3 or higher. They understand one another whatever languages they speak, and distance does not matter so long as they stay on the same plane.',
 'A circle of minds can speak silently to one another for the hour.'),
('sp_walloforce','Wall of Force','Wall of Force',5,'Utility','5th Circle','evocation',null,null,
 null,true,false,'120 feet','Up to 10 minutes','1 action',array['v','s','m'],'a pinch of powder made by crushing a clear gemstone',
 array['wizard'],
 'Ten panels of 10 feet by 10, flat or standing, or a 10-foot-radius sphere, all of it a quarter of an inch thick and invisible. NOTHING physical crosses it and no spell reaches through it. It cannot be damaged, but a Disintegrate destroys it outright and an Antimagic Field ends it.',
 'An invisible wall of force that nothing walks through and nothing casts through.')
on conflict (key) where game_id is null do update set
  name=excluded.name, roll_name=excluded.roll_name, level=excluded.level,
  cast_type=excluded.cast_type, category=excluded.category, school=excluded.school,
  save_ability=excluded.save_ability, dice=excluded.dice, on_save=excluded.on_save,
  concentration=excluded.concentration, ritual=excluded.ritual,
  range=excluded.range, duration=excluded.duration, casting_time=excluded.casting_time,
  components=excluded.components, material=excluded.material, classes=excluded.classes,
  special_text=excluded.special_text, description=excluded.description;
