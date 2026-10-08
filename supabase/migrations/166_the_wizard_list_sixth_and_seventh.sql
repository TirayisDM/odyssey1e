-- 166. THE WIZARD LIST: SIXTH AND SEVENTH CIRCLE.
--
-- 29 spells, finishing both levels: 19 of 19 at sixth, 15 of 15 at
-- seventh. 21 left after this, all of them 8th and 9th, and 167 ends
-- it. Same contract as 161 to 165 and 102-104: mechanical values read
-- off the published list, every description written here.
--
-- EVERY SAVE SPELL IN THIS FILE CARRIES `on_save`, which is what 164
-- was for. The question to ask of each one is not "does it do damage"
-- but "what does a creature that MAKES the save take", and the three
-- answers are half, none, and it was never cast damage to begin with.
--
-- DISINTEGRATE TAKES `none`, not `half`. It is the only big damage
-- spell at these levels where a successful save means nothing at all,
-- and it is also the one that looks most like the ones that halve.
--
-- 10d6+40 IN THE DICE COLUMN, which the catalogue has done once before
-- - Magic Missile is 3d4+3 - and Finger of Death does the same with
-- 7d8+30. The flat part is not a modifier anyone can earn or lose, it
-- is the spell.
--
-- SUNBEAM AND WALL OF ICE both damage again after the turn they are
-- cast, and both keep their dice. Sunbeam fires again on each of your
-- turns for the same 6d8; Wall of Ice does 10d6 where it appears and a
-- separate 5d6 to anything pushing through a broken panel, and only the
-- first of those is in the column. The second is in the text, because
-- 158 allows one number per row and this spell has two.
--
-- ARCANE SWORD IS `Attack` AND ARCANE HAND IS NOT, though both are a
-- conjured weapon you steer with a bonus action afterwards. The sword
-- swings the moment it appears - casting it is an attack roll - and the
-- hand does nothing at all until the turn after. 165 drew the same line
-- with Faithful Hound.
--
-- FORCECAGE AND PROJECT IMAGE ARE NOT CONCENTRATION, which is worth
-- saying only because every neighbour at this level is. An hour of cage
-- and a day of double both run on their own.
--
-- REUNITED. The SQL below is the database's own record of what ran,
-- MD5-verified when 8f726b4 recovered these four files; the header
-- above it is the original, off the laptop that wrote them. They were
-- separated because 165-168 were APPLIED WITHOUT THEIR HEADERS - the
-- statement was pasted on its own - and Supabase stores the statement,
-- so the reasoning never left this machine. 161-164 went in whole and
-- never lost anything.
--
-- THE LESSON IS ABOUT THE APPLY, NOT THE RECOVERY: a migration applied
-- body-only is a migration whose header exists in exactly one place.

insert into spells
  (key, name, roll_name, level, cast_type, category, school, save_ability, dice,
   on_save, concentration, ritual, range, duration, casting_time, components,
   material, classes, special_text, description)
values
('sp_chainlightning','Chain Lightning','Chain Lightning',6,'Save','6th Circle','evocation','dex','10d8',
 'half',false,false,'150 feet','Instant','1 action',array['v','s','m'],'a bit of fur, a rod of amber, glass or crystal, and three silver pins',
 array['sorcerer','wizard'],
 'One target, then up to three more of your choice within 30 feet of it, none of them struck twice. Each makes its own Dexterity save. One extra arc per slot level above 6th.',
 'A bolt strikes what you point at and leaps from it to everything standing near.'),
('sp_circleofdeath','Circle of Death','Circle of Death',6,'Save','6th Circle','necromancy','con','8d6',
 'half',false,false,'150 feet','Instant','1 action',array['v','s','m'],'the powder of a crushed black pearl worth 500 gp',
 array['sorcerer','warlock','wizard'],
 'A 60-foot-radius sphere - the widest area in the catalogue. +2d6 per slot level above 6th.',
 'Negative energy ripples outward and drains the life from everything it washes over.'),
('sp_contingency','Contingency','Contingency',6,'Utility','6th Circle','evocation',null,null,
 null,false,false,'Self','10 days','10 minutes',array['v','s','m'],'a statuette of yourself carved from ivory and set with gems worth 1,500 gp',
 array['wizard'],
 'The stored spell must be 5th level or lower, must have a casting time of one action, and must target you alone. You expend both slots now. It fires the moment the circumstance you described comes about, once, and then the whole thing ends. ONE AT A TIME: casting it again discards what was stored.',
 'You prepare a spell now and name the moment it will cast itself later.'),
('sp_disintegrate','Disintegrate','Disintegrate',6,'Save','6th Circle','transmutation','dex','10d6+40',
 'none',false,false,'60 feet','Instant','1 action',array['v','s','m'],'a lodestone and a pinch of dust',
 array['sorcerer','wizard'],
 'A SUCCESSFUL SAVE TAKES NOTHING - this is not one of the spells that halves. A creature reduced to 0 hit points by it is reduced to dust, and only a True Resurrection or a Wish brings it back. It destroys a Large or smaller nonmagical object outright, or a 10-foot cube of one, and it ends a Wall of Force. +3d6 per slot level above 6th.',
 'A thin green ray, and whatever it touches stops existing.'),
('sp_eyebite','Eyebite','Eyebite',6,'Save','6th Circle','necromancy','wis',null,
 null,true,false,'Self','Up to 1 minute','1 action',array['v','s'],null,
 array['bard','sorcerer','warlock','wizard'],
 'One creature within 60 feet makes a Wisdom save as you cast it, and you may target another on each of your turns thereafter. Choose the effect when you cast: ASLEEP, unconscious until damaged or woken; PANICKED, frightened and forced to move away from you; or SICKENED, poisoned, with a save at the end of each of its turns.',
 'Your eyes go black and one of three ruinous things happens to whoever meets them.'),
('sp_fleshtostone','Flesh to Stone','Flesh to Stone',6,'Save','6th Circle','transmutation','con',null,
 null,true,false,'60 feet','Up to 1 minute','1 action',array['v','s','m'],'a pinch of lime, water and earth',
 array['warlock','wizard'],
 'A failed Constitution save leaves the target restrained as the stone takes hold, and it saves again at the end of each of its turns. THREE FAILURES PETRIFY IT for the duration; three successes end the spell. If it is still petrified when the duration runs out, the petrification is PERMANENT.',
 'Flesh begins turning to stone, and whether it finishes is decided over the next few rounds.'),
('sp_freezingsphere','Freezing Sphere','Freezing Sphere',6,'Save','6th Circle','evocation','con','10d6',
 'half',false,false,'300 feet','Instant','1 action',array['v','s','m'],'a small crystal sphere',
 array['wizard'],
 'A 60-foot-radius burst. Water in the area freezes to a depth of 6 inches for a minute, and a creature caught in the ice saves or is restrained. You may instead hold the globe unexploded and throw or set it down later, up to a minute afterwards. +1d6 per slot level above 6th.',
 'A globe of cold flies out and bursts, or waits in your hand until you choose to let it go.'),
('sp_globeofinvulnerability','Globe of Invulnerability','Globe of Invulnerability',6,'Utility','6th Circle','abjuration',null,null,
 null,true,false,'Self (10-foot radius)','Up to 1 minute','1 action',array['v','s','m'],'a glass or crystal bead that shatters when the spell ends',
 array['sorcerer','wizard'],
 'Any spell of 5TH LEVEL OR LOWER cast from outside fails to affect anything inside, and its slot is still spent. One level higher per slot level above 6th. A spell cast from INSIDE the barrier works normally, going out.',
 'A shimmering shell that lesser magic cannot reach into, though it can be cast out of.'),
('sp_guardsandwards','Guards and Wards','Guards and Wards',6,'Utility','6th Circle','abjuration',null,null,
 null,false,false,'Touch','24 hours','10 minutes',array['v','s','m'],'burning incense, brimstone and oil, a knotted string, a drop of blood and a silver rod worth 10 gp',
 array['bard','wizard'],
 'Up to 2,500 square feet of floor, 20 feet high, in any shape you can walk. Corridors fog, doors lock and hide themselves, stairs fill with webs, and you may name a password and the creatures it exempts. You may also seat one of a short list of spells in the area: Dancing Lights, Magic Mouth, Stinking Cloud, Gust of Wind or Suggestion. Cast it every day for a year and it becomes permanent.',
 'A building is turned against intruders, room by room, for a day.'),
('sp_instantsummons','Instant Summons','Instant Summons',6,'Utility','6th Circle','conjuration',null,null,
 null,false,true,'Touch','Until dispelled','1 minute',array['v','s','m'],'a sapphire worth 1,000 gp',
 array['wizard'],
 'The sapphire is marked to one object of 10 pounds or less and no more than 6 feet in any dimension. Crush the sapphire and speak the name and the object arrives in your free hand, FROM ANY DISTANCE AND ANY PLANE. If another creature is holding it, the stone instead tells you who and roughly where, and the object does not come.',
 'A gem remembers one object, and breaking the gem calls that object to your hand.'),
('sp_irresistibledance','Irresistible Dance','Irresistible Dance',6,'Save','6th Circle','enchantment','wis',null,
 null,true,false,'30 feet','Up to 1 minute','1 action',array['v'],null,
 array['bard','wizard'],
 'NO SAVE WHEN IT LANDS - the target simply dances, with disadvantage on Dexterity saves and attack rolls, and everything attacking it has advantage. It may spend its action on a Wisdom save to stop, and it saves automatically on its turn only if it is incapacitated.',
 'The target begins to caper and shuffle in place, and cannot usefully do anything else.'),
('sp_magicjar','Magic Jar','Magic Jar',6,'Save','6th Circle','necromancy','cha',null,
 null,false,false,'Self','Until dispelled','1 minute',array['v','s','m'],'a gem, crystal or reliquary worth 500 gp',
 array['wizard'],
 'Your body falls catatonic and your soul enters the container. From there you may try to take a humanoid body within 100 feet: a failed Charisma save and you are in it, with its statistics but your own alignment, Intelligence, Wisdom, Charisma and class features. ITS SOUL GOES TO THE CONTAINER. If the host body dies while you hold it, you must save or die with it. BREAKING THE CONTAINER ends the spell and strands you.',
 'You leave your body in a jar and go looking for a better one to wear.'),
('sp_masssuggestion','Mass Suggestion','Mass Suggestion',6,'Save','6th Circle','enchantment','wis',null,
 null,false,false,'60 feet','24 hours','1 action',array['v','m'],'a snake tongue and either a bit of honeycomb or a drop of sweet oil',
 array['bard','sorcerer','warlock','wizard'],
 'Up to twelve creatures that can hear and understand you. The course of action must sound reasonable; an obviously harmful one fails outright. Damaging a target ends it for that target. Duration climbs with the slot: 10 days at 7th, 30 days at 8th, a year and a day at 9th.',
 'A sentence or two of instruction, and a dozen listeners find themselves agreeing.'),
('sp_moveearth','Move Earth','Move Earth',6,'Utility','6th Circle','transmutation',null,null,
 null,true,false,'120 feet','Up to 2 hours','1 action',array['v','s','m'],'an iron blade and a bag of mixed soils',
 array['druid','sorcerer','wizard'],
 'A 40-foot area of dirt, sand or clay, raised or lowered by up to 20 feet, redirected every 10 minutes as an action. IT IS SLOW - the ground shifts gradually enough that nothing standing on it is trapped or hurt. Stone is untouched, and so are structures.',
 'The landscape itself is dug, piled and reshaped while you watch.'),
('sp_programmedillusion','Programmed Illusion','Programmed Illusion',6,'Utility','6th Circle','illusion',null,null,
 null,false,false,'120 feet','Until dispelled','1 action',array['v','s','m'],'a bit of fleece and jade dust worth 25 gp',
 array['bard','wizard'],
 'Up to a 30-foot cube, playing for 5 minutes or less when the condition you describe is met, then going dormant for 10 minutes. The trigger can be as broad or as particular as you like, but it must be something seen or heard within 30 feet of the illusion. Investigation against your save DC sees through it, and physical interaction gives it away at once.',
 'An illusion waits, dormant and invisible, until the thing you described happens.'),
('sp_sunbeam','Sunbeam','Sunbeam',6,'Save','6th Circle','evocation','con','6d8',
 'half',true,false,'Self (60-foot line)','Up to 1 minute','1 action',array['v','s','m'],'a magnifying glass',
 array['druid','sorcerer','wizard'],
 'A failed Constitution save takes 6d8 radiant and is BLINDED until your next turn; a success takes half and is not blinded. You may fire the beam again as an action on each of your turns until the spell ends. Undead and oozes have disadvantage. Bright sunlight fills the line for the whole duration.',
 'A line of searing daylight that you can fire again every turn you keep hold of it.'),
('sp_wallofice','Wall of Ice','Wall of Ice',6,'Save','6th Circle','evocation','dex','10d6',
 'half',true,false,'120 feet','Up to 10 minutes','1 action',array['v','s','m'],'a small piece of quartz',
 array['wizard'],
 'Ten panels of 10 feet by 10, a foot thick, or a dome or sphere up to 10 feet across. The 10d6 cold is for anything caught in the wall as it appears. A BROKEN PANEL LEAVES FRIGID AIR: pushing through it is a second Dexterity save for 5d6 cold, half on a success. Each panel has AC 12 and 30 hit points, and a destroyed panel is replaced by that frigid air for the duration. +2d6 to the first and +1d6 to the second per slot level above 6th.',
 'A wall of ice a foot thick, which keeps hurting even after somebody breaks it.'),
('sp_arcanesword','Arcane Sword','Arcane Sword',7,'Attack','7th Circle','evocation',null,'3d10',
 null,true,false,'60 feet','Up to 1 minute','1 action',array['v','s','m'],'a miniature sword of platinum, copper and zinc worth 250 gp',
 array['bard','wizard'],
 'The sword attacks the moment it appears - a melee spell attack for 3d10 force against anything within 5 feet of it. On every turn after that, a BONUS ACTION moves it up to 20 feet and attacks again.',
 'A blade of pure force hangs in the air and cuts where you send it.'),
('sp_delayedblastfireball','Delayed Blast Fireball','Delayed Blast Fireball',7,'Save','7th Circle','evocation','dex','12d6',
 'half',true,false,'150 feet','Up to 1 minute','1 action',array['v','s','m'],'a tiny ball of bat guano and sulphur',
 array['sorcerer','wizard'],
 'A 20-foot-radius sphere when it goes off, which is when your concentration ends. IT GROWS WHILE IT WAITS: +1d6 for every round that passes. A creature touching the bead takes the whole charge at once and the bead moves with it. +1d6 to the base per slot level above 7th.',
 'A Fireball that does not go off yet, sitting as a bead of light and swelling.'),
('sp_fingerofdeath','Finger of Death','Finger of Death',7,'Save','7th Circle','necromancy','con','7d8+30',
 'half',false,false,'60 feet','Instant','1 action',array['v','s'],null,
 array['sorcerer','warlock','wizard'],
 'A humanoid KILLED by this rises at the start of your next turn as a zombie under your command, permanently.',
 'Negative energy tears through one creature, and if it kills a person that person gets back up for you.'),
('sp_forcecage','Forcecage','Forcecage',7,'Save','7th Circle','evocation','cha',null,
 null,false,false,'100 feet','1 hour','1 action',array['v','s','m'],'ruby dust worth 1,500 gp',
 array['bard','warlock','wizard'],
 'Either a solid 10-foot CUBE, which nothing crosses, or a 20-foot CAGE of bars an inch apart, which only something small enough to slip between them crosses. Anything already inside when it forms is caught without a save. TELEPORTING OUT is a Charisma save, and a failure wastes the attempt. It cannot be dispelled and it cannot be damaged; only a Disintegrate destroys it.',
 'A box of force appears around something and it has an hour to think about it.'),
('sp_magnificentmansion','Magnificent Mansion','Magnificent Mansion',7,'Utility','7th Circle','conjuration',null,null,
 null,false,false,'300 feet','24 hours','1 minute',array['v','s','m'],'a miniature ivory portal, a piece of polished marble and a tiny silver spoon, each worth 5 gp',
 array['bard','wizard'],
 'Up to fifty interconnected 10-foot cubes, staffed by servants who fetch, clean and serve a banquet for a hundred. Only the creatures you designate can find or use the door, which is invisible and 5 feet wide. NOTHING STAYS: when the spell ends, anything inside is expelled, and the food and the servants simply stop existing.',
 'A door opens onto a furnished house that is not anywhere, and closes again in a day.'),
('sp_miragearcane','Mirage Arcane','Mirage Arcane',7,'Utility','7th Circle','illusion',null,null,
 null,false,false,'Sight','10 days','10 minutes',array['v','s'],null,
 array['bard','druid','wizard'],
 'Up to a square mile. UNLIKE HALLUCINATORY TERRAIN IT IS TACTILE: the ground feels like what it looks like, difficult terrain can be made easy or the reverse, and buildings may be added, removed or disguised. Creatures and equipment are not. True Seeing sees the real terrain but the illusion still obstructs movement.',
 'A mile of countryside becomes other countryside, convincingly enough to walk on.'),
('sp_prismaticspray','Prismatic Spray','Prismatic Spray',7,'Save','7th Circle','evocation','dex','10d6',
 'half',false,false,'Self (60-foot cone)','Instant','1 action',array['v','s'],null,
 array['sorcerer','wizard'],
 'ROLL A D8 FOR EACH TARGET to see which ray strikes it. 1 to 7 deal 10d6 of fire, acid, lightning, poison, cold, or else restrain and begin petrifying, or banish the target to another plane - the last two on Dexterity and Wisdom saves rather than damage. An 8 means TWO RAYS, rolled again, rerolling any further 8.',
 'Eight colours of light leap from your hand and each target finds out which one it got.'),
('sp_projectimage','Project Image','Project Image',7,'Utility','7th Circle','illusion',null,null,
 null,true,false,'500 miles','Up to 1 day','1 action',array['v','s','m'],'a small replica of you worth 5 gp',
 array['bard','wizard'],
 'The double looks and sounds exactly like you and can be moved 60 feet and made to speak and gesture as an action. You may see and hear through it instead of your own senses. IT IS ONLY AN IMAGE: physical interaction gives it away, and so does Investigation against your save DC.',
 'A copy of you stands somewhere up to five hundred miles off and does as you direct.'),
('sp_reversegravity','Reverse Gravity','Reverse Gravity',7,'Save','7th Circle','transmutation','dex',null,
 null,true,false,'100 feet','Up to 1 minute','1 action',array['v','s','m'],'a lodestone and iron filings',
 array['druid','sorcerer','wizard'],
 'A cylinder 50 feet across and 100 feet high. Everything unanchored falls UPWARD to the top of it and stays there. A Dexterity save is only for grabbing something fixed as you go. Anything that reaches the top and has nothing to hold stays floating. WHEN THE SPELL ENDS everything up there falls the whole way back down.',
 'Gravity runs the wrong way inside a tall cylinder, and then stops doing so.'),
('sp_sequester','Sequester','Sequester',7,'Utility','7th Circle','transmutation',null,null,
 null,false,false,'Touch','Until dispelled','1 action',array['v','s','m'],'dust of diamond, emerald, ruby and sapphire worth 5,000 gp, consumed',
 array['wizard'],
 'The target becomes invisible and beyond the reach of divination and scrying, and a willing creature put under it falls into suspended animation, neither ageing nor needing food or air. You may name a condition that ends the spell early - a length of time, or something happening nearby.',
 'A creature or object is hidden away, asleep and unfindable, until the condition you set.'),
('sp_simulacrum','Simulacrum','Simulacrum',7,'Utility','7th Circle','illusion',null,null,
 null,false,false,'Touch','Until dispelled','12 hours',array['v','s','m'],'snow or ice enough for a life-size copy, a piece of the creature being copied, and powdered ruby worth 1,500 gp, consumed',
 array['wizard'],
 'The duplicate has HALF the original maximum hit points, obeys you, and cannot gain levels or recover spent slots. Damage to it is repaired only in an arcane laboratory, at 100 gp per hit point and 10 days per 4d10 restored. ONE AT A TIME: casting it again ends the previous simulacrum, and so does dropping one to 0 hit points, which leaves nothing but snow.',
 'Twelve hours of work over snow produces a lesser copy of somebody, loyal to you.'),
('sp_teleport','Teleport','Teleport',7,'Utility','7th Circle','conjuration',null,null,
 null,false,false,'10 feet','Instant','1 action',array['v'],null,
 array['bard','sorcerer','wizard'],
 'You and up to eight willing creatures, or one object. HOW WELL YOU KNOW THE DESTINATION DECIDES WHETHER IT WORKS: a permanent circle or an associated object always arrives; somewhere you know very well is reliable; a place seen casually, or described to you, or a direction given on a map is progressively worse. A bad roll lands you somewhere similar, somewhere off target, or in the wrong place entirely, and mishaps deal 3d10 force to everybody before you try again.',
 'You and your companions step from here to somewhere far off, with the accuracy you have earned.')
on conflict (key) where game_id is null do update set
  name=excluded.name, roll_name=excluded.roll_name, level=excluded.level,
  cast_type=excluded.cast_type, category=excluded.category, school=excluded.school,
  save_ability=excluded.save_ability, dice=excluded.dice, on_save=excluded.on_save,
  concentration=excluded.concentration, ritual=excluded.ritual,
  range=excluded.range, duration=excluded.duration, casting_time=excluded.casting_time,
  components=excluded.components, material=excluded.material, classes=excluded.classes,
  special_text=excluded.special_text, description=excluded.description;
