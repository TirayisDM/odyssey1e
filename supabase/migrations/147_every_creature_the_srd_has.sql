-- 147. EVERY CREATURE THE SRD HAS.
--
-- 146 checked our 203 against the published SRD and fixed fifteen. This
-- is the other half of that read: the 77 statblocks the SRD has that we
-- did not. 280 creatures after it, and the bestiary is now the book
-- rather than as much of the book as I could remember.
--
-- THE NUMBERS ARE READ, NOT RECALLED, which is new. Every figure below -
-- armour class, hit points, the six abilities, every resistance - was
-- taken off the published SRD rather than written from memory, and the
-- resistances were parsed into 116's vocabulary mechanically rather than
-- transcribed by hand. 142's warning that these are "SRD-derived from
-- memory, right in shape and wrong in places" does not apply to this
-- batch.
--
-- THE PROSE IS OURS. Every `notes` line is written for this bestiary in
-- the voice 127 and 142 use - what the thing DOES at a table, in one
-- sentence. Nothing is copied.
--
-- ---------------------------------------------------------------------
-- WHAT THIS FILLS
-- ---------------------------------------------------------------------
--
--   THE DRAGONS, finally complete. We had five chromatic colours at
--   three ages. The SRD has ten colours at four, and the metallics -
--   brass, bronze, copper, gold, silver - were missing entirely, as was
--   every ancient dragon of any colour. 37 dragons now, from a wyrmling
--   to the Ancient Gold.
--
--   THE TOP END, which did not exist. The bestiary stopped at the Pit
--   Fiend. It now runs to the Tarrasque, by way of the Kraken, the
--   Solar, the two sphinxes and the Purple Worm.
--
--   THE DINOSAURS, the hags, the nagas, the genies, four more devils,
--   and the ordinary-looking things a dungeon needs - a Rug of
--   Smothering, a Will-o'-Wisp, a Doppelganger, a Cloaker.
--
-- ---------------------------------------------------------------------
-- ONE DELIBERATE DEVIATION, AND IT IS A BALANCE DECISION
-- ---------------------------------------------------------------------
--
-- 127 set `prof_bonus` by its own two-step rule - 2 up to level 9, 3
-- above - "so one catalogue has one scale". That rule was written when
-- the catalogue topped out around CR 10, and following it to CR 30 would
-- give the Tarrasque a +3 proficiency bonus. It would hit less often
-- than a guard captain.
--
-- SO THESE 77 USE 5e's OWN PROFICIENCY BY CR: 2 up to 4, then 3, 4, 5,
-- 6, 7, 8, 9 every four ratings. It is the book's number and the column
-- has always allowed up to 9.
--
-- THE EXISTING 203 ARE NOT RETUNED, and that restraint is 129's lesson
-- paid forward: a balance change arriving as a side effect of a data
-- migration is exactly what 132 had to reverse. So the catalogue now has
-- two proficiency scales and this header is where that is written down.
-- Deriving the old ones from CR is a one-line change whenever Dave wants
-- it, and it is a change to how 203 existing creatures hit.
--
-- LEVEL IS THE CHALLENGE RATING for these, floored at 1, which is also
-- 127's stated intent - "roughly the party level this is a fair fight
-- for" is what CR already means. The floor is because there is no party
-- level 0 and five of these are CR 0 or below 1. 127's own numbers were
-- set by hand and drift from the rule; these do not.
--
-- ---------------------------------------------------------------------
-- WHAT IS STILL NOT HERE
-- ---------------------------------------------------------------------
--
-- Legendary actions, lair actions, breath weapons, regeneration,
-- spellcasting and every other trait. A dragon here bites, claws, lashes
-- and buffets; it does not breathe. That is a whole mechanism rather
-- than a column, and inventing a half of one for 37 dragons would be
-- worse than the honest absence - the statblock screen shows the prose a
-- DM needs and the breath stays a DM call, as it has been for every
-- creature since 127.
--
-- Six of our creatures have no SRD page and were left alone, as Dave
-- asked: the Goblin Boss, Guard Captain, Orc War Chief, Harpy Matriarch,
-- Dire Boar and Kobold Dragonshield are ours. So, it turns out, are the
-- Archer, Banshee, Pixie, Acolyte, Druid, Carrion Crawler, Hook Horror,
-- Helmed Horror, Intellect Devourer, Scarecrow, Twig Blight, Yuan-ti
-- Pureblood and Gas Spore, which have no page on that SRD either.

insert into npcs (key,name,creature_type,size,level,prof_bonus,ac,hp_max,str,dex,con,intl,wis,cha,grants,notes,weapon_profs,armor_profs)
select v.k,v.n,v.ct,v.sz,v.lvl,v.pb,v.ac,v.hp,v.s,v.d,v.c,v.i,v.w,v.ch,v.g::jsonb,v.nt,array['sim','mar'],array['lgt','med','hvy','shl']
from (values
('homunculus','Homunculus','construct','tiny',1,2,13,5,4,15,11,10,10,7,'[{"source":"Homunculus","target":"immune.poison"}]','Somebody''s spare hands and spare eyes. Dies when its maker''s attention wanders.'),
('stirge','Stirge','beast','tiny',1,2,14,2,4,16,11,2,8,6,'[]','A mosquito the size of a cat, and it does not leave once it has landed.'),
('aarakocra','Aarakocra','humanoid','med',1,2,12,13,10,14,10,11,12,11,'[]','Keeps to the high air and comes down only when there is a reason.'),
('grimlock','Grimlock','humanoid','med',1,2,11,11,16,12,12,9,8,6,'[]','Blind, and it has not needed eyes down here for a very long time.'),
('pteranodon','Pteranodon','beast','med',1,2,13,13,12,15,10,2,9,5,'[]','A beak on wings. Takes what it can carry and is gone before the shouting starts.'),
('magmin','Magmin','elemental','sm',1,2,14,9,7,15,12,8,11,10,'[{"source":"Magmin","target":"immune.fire"},{"source":"Magmin","target":"resist.bludgeoning.nonmagical"},{"source":"Magmin","target":"resist.piercing.nonmagical"},{"source":"Magmin","target":"resist.slashing.nonmagical"}]','A small cheerful thing made of embers that sets fire to whatever it touches, including you.'),
('brass_dragon_wyrmling','Brass Dragon Wyrmling','dragon','med',1,2,16,16,15,10,13,10,11,13,'[{"source":"Brass Dragon Wyrmling","target":"immune.fire"}]','Talkative even at this size, and will keep you talking longer than is safe.'),
('copper_dragon_wyrmling','Copper Dragon Wyrmling','dragon','med',1,2,16,22,15,12,13,14,11,13,'[{"source":"Copper Dragon Wyrmling","target":"immune.acid"}]','Finds itself very funny. The acid is part of the joke.'),
('centaur','Centaur','monstrosity','lg',2,2,12,45,18,14,14,9,13,11,'[]','Half a cavalry charge that needs no horse and takes no orders.'),
('bronze_dragon_wyrmling','Bronze Dragon Wyrmling','dragon','med',2,2,17,32,17,10,15,12,11,15,'[{"source":"Bronze Dragon Wyrmling","target":"immune.lightning"}]','Already patrolling a stretch of coast it has decided is its business.'),
('allosaurus','Allosaurus','beast','lg',2,2,13,51,19,13,17,2,12,5,'[]','Runs faster than anything that size has any right to.'),
('azer','Azer','elemental','med',2,2,17,39,17,12,15,12,13,10,'[{"source":"Azer","target":"immune.fire"},{"source":"Azer","target":"immune.poison"}]','A smith made of the forge rather than standing at it. Everything it holds is already hot.'),
('sea_hag','Sea Hag','fey','med',2,2,14,52,16,13,16,12,12,13,'[]','Hideous on purpose, and the first thing it does is make you look.'),
('plesiosaurus','Plesiosaurus','beast','lg',2,2,13,68,18,15,16,2,12,5,'[]','The neck arrives a long moment before the rest of it does.'),
('ogre_zombie','Ogre Zombie','undead','lg',2,2,8,85,19,6,18,3,6,5,'[{"source":"Ogre Zombie","target":"immune.poison"}]','All of an ogre''s weight and none of its reluctance to keep walking.'),
('merrow','Merrow','monstrosity','lg',2,2,13,45,18,10,15,8,10,9,'[]','What the sea made of a drowned people, and it remembers being one.'),
('rug_of_smothering','Rug of Smothering','construct','lg',2,2,12,33,17,14,10,1,3,1,'[{"source":"Rug of Smothering","target":"immune.poison"},{"source":"Rug of Smothering","target":"immune.psychic"}]','It was a rug. It is still a rug, and now it is on top of you.'),
('silver_dragon_wyrmling','Silver Dragon Wyrmling','dragon','med',2,2,17,45,19,10,17,12,11,15,'[{"source":"Silver Dragon Wyrmling","target":"immune.cold"}]','Polite, curious, and entirely willing to freeze you if the politeness fails.'),
('will_o_wisp','Will-o''-Wisp','undead','tiny',2,2,19,22,1,28,10,13,14,10,'[{"source":"Will-o''-Wisp","target":"immune.lightning"},{"source":"Will-o''-Wisp","target":"immune.poison"},{"source":"Will-o''-Wisp","target":"resist.acid"},{"source":"Will-o''-Wisp","target":"resist.bludgeoning.nonmagical"},{"source":"Will-o''-Wisp","target":"resist.cold"},{"source":"Will-o''-Wisp","target":"resist.fire"},{"source":"Will-o''-Wisp","target":"resist.necrotic"},{"source":"Will-o''-Wisp","target":"resist.piercing.nonmagical"},{"source":"Will-o''-Wisp","target":"resist.slashing.nonmagical"},{"source":"Will-o''-Wisp","target":"resist.thunder"}]','A light that wants you to follow it, and knows exactly where the bog is deepest.'),
('ankylosaurus','Ankylosaurus','beast','huge',3,2,15,68,19,11,15,2,12,5,'[]','Armoured at both ends and willing to use the back one.'),
('doppelganger','Doppelganger','monstrosity','med',3,2,14,52,11,18,14,11,12,14,'[]','Has been in the room for a while, wearing somebody you trust.'),
('green_hag','Green Hag','fey','med',3,2,17,82,18,12,16,13,14,14,'[]','Bargains, and the bargain is always worse than it sounded in the wood.'),
('bearded_devil','Bearded Devil','fiend','med',3,2,13,52,16,15,15,9,11,11,'[{"source":"Bearded Devil","target":"immune.fire"},{"source":"Bearded Devil","target":"immune.poison"},{"source":"Bearded Devil","target":"resist.cold"},{"source":"Bearded Devil","target":"resist.bludgeoning.nonmagical"},{"source":"Bearded Devil","target":"resist.piercing.nonmagical"},{"source":"Bearded Devil","target":"resist.slashing.nonmagical"}]','The beard moves on its own and the wounds it opens do not close.'),
('gold_dragon_wyrmling','Gold Dragon Wyrmling','dragon','med',3,2,17,60,19,14,17,14,11,16,'[{"source":"Gold Dragon Wyrmling","target":"immune.fire"}]','Already certain it knows what is right, and already able to enforce it.'),
('nightmare','Nightmare','fiend','lg',3,2,13,68,18,15,16,10,13,15,'[{"source":"Nightmare","target":"immune.fire"}]','A horse that burns, ridden by whatever can stand to ride it.'),
('couatl','Couatl','celestial','med',4,2,19,97,16,20,17,18,20,18,'[{"source":"Couatl","target":"immune.psychic"},{"source":"Couatl","target":"immune.bludgeoning.nonmagical"},{"source":"Couatl","target":"immune.piercing.nonmagical"},{"source":"Couatl","target":"immune.slashing.nonmagical"},{"source":"Couatl","target":"resist.radiant"}]','A feathered serpent that has been watching this situation far longer than you have.'),
('lamia','Lamia','monstrosity','lg',4,2,13,97,16,13,15,14,15,16,'[]','Rules a ruin, and would rather corrupt you than kill you.'),
('barbed_devil','Barbed Devil','fiend','med',5,3,15,110,16,17,18,12,14,14,'[{"source":"Barbed Devil","target":"immune.fire"},{"source":"Barbed Devil","target":"immune.poison"},{"source":"Barbed Devil","target":"resist.cold"},{"source":"Barbed Devil","target":"resist.bludgeoning.nonmagical"},{"source":"Barbed Devil","target":"resist.piercing.nonmagical"},{"source":"Barbed Devil","target":"resist.slashing.nonmagical"}]','Hurts to hit. That is not a side effect, it is the design.'),
('gorgon','Gorgon','monstrosity','lg',5,3,19,114,20,11,18,2,12,7,'[]','An iron bull with iron breath. What it exhales does not wash off.'),
('salamander','Salamander','elemental','lg',5,3,15,90,18,14,15,11,10,12,'[{"source":"Salamander","target":"immune.fire"},{"source":"Salamander","target":"vulnerable.cold"},{"source":"Salamander","target":"resist.bludgeoning.nonmagical"},{"source":"Salamander","target":"resist.piercing.nonmagical"},{"source":"Salamander","target":"resist.slashing.nonmagical"}]','Coils around you while it stabs, and the coil is the part that kills.'),
('roper','Roper','monstrosity','lg',5,3,20,93,18,8,17,7,16,6,'[]','Was a stalagmite for an hour and will be one again afterwards.'),
('triceratops','Triceratops','beast','huge',5,3,13,95,22,9,17,2,11,5,'[]','Does not want anything from you except that you be somewhere else.'),
('night_hag','Night Hag','fiend','med',5,3,17,112,18,15,16,16,14,16,'[{"source":"Night Hag","target":"resist.cold"},{"source":"Night Hag","target":"resist.fire"},{"source":"Night Hag","target":"resist.bludgeoning.nonmagical"},{"source":"Night Hag","target":"resist.piercing.nonmagical"},{"source":"Night Hag","target":"resist.slashing.nonmagical"}]','Visits while you sleep and takes something you will not notice is gone.'),
('xorn','Xorn','elemental','med',5,3,19,73,17,10,22,11,10,11,'[{"source":"Xorn","target":"resist.piercing.nonmagical"},{"source":"Xorn","target":"resist.slashing.nonmagical"}]','Swims through stone and only wants the metal you are carrying.'),
('drider','Drider','monstrosity','lg',6,3,19,123,16,16,18,13,14,12,'[]','Was somebody once, and has had a long time underground to think about it.'),
('invisible_stalker','Invisible Stalker','elemental','med',6,3,14,104,16,19,14,10,15,11,'[{"source":"Invisible Stalker","target":"immune.poison"},{"source":"Invisible Stalker","target":"resist.bludgeoning.nonmagical"},{"source":"Invisible Stalker","target":"resist.piercing.nonmagical"},{"source":"Invisible Stalker","target":"resist.slashing.nonmagical"}]','Was sent, resents it, and will finish the task before it resents you.'),
('young_brass_dragon','Young Brass Dragon','dragon','lg',6,3,17,110,19,10,17,12,11,15,'[{"source":"Young Brass Dragon","target":"immune.fire"}]','Would genuinely rather have the conversation, and is dangerous either way.'),
('young_copper_dragon','Young Copper Dragon','dragon','lg',7,3,17,119,19,12,17,16,13,15,'[{"source":"Young Copper Dragon","target":"immune.acid"}]','Will set you a riddle. Getting it wrong is not fatal; being boring might be.'),
('cloaker','Cloaker','aberration','lg',8,3,14,78,17,15,12,13,12,14,'[]','Hung on the ceiling with the other cloaks until one of them moved.'),
('tyrannosaurus_rex','Tyrannosaurus Rex','beast','huge',8,3,13,136,25,10,19,2,12,9,'[]','The bite is the whole argument and the tail settles anything left over.'),
('young_bronze_dragon','Young Bronze Dragon','dragon','lg',8,3,18,142,21,10,19,14,13,17,'[{"source":"Young Bronze Dragon","target":"immune.lightning"}]','Guards a coastline and expects to be thanked for it.'),
('spirit_naga','Spirit Naga','monstrosity','lg',8,3,15,75,18,17,14,16,15,16,'[{"source":"Spirit Naga","target":"immune.poison"}]','Dies, comes back, and remembers who was responsible the first time.'),
('chain_devil','Chain Devil','fiend','med',8,3,16,85,18,15,18,11,12,14,'[{"source":"Chain Devil","target":"immune.fire"},{"source":"Chain Devil","target":"immune.poison"},{"source":"Chain Devil","target":"resist.cold"},{"source":"Chain Devil","target":"resist.bludgeoning.nonmagical"},{"source":"Chain Devil","target":"resist.piercing.nonmagical"},{"source":"Chain Devil","target":"resist.slashing.nonmagical"}]','The chains are not holding it. They are what it brought.'),
('young_silver_dragon','Young Silver Dragon','dragon','lg',9,4,18,168,23,10,21,14,11,19,'[{"source":"Young Silver Dragon","target":"immune.cold"}]','Spends more time as a person than as a dragon, which is how it learns things.'),
('guardian_naga','Guardian Naga','monstrosity','lg',10,4,18,127,19,18,16,16,19,18,'[{"source":"Guardian Naga","target":"immune.poison"}]','Guards something and will explain, once, why you should leave it alone.'),
('aboleth','Aboleth','aberration','lg',10,4,17,135,21,9,15,18,15,18,'[]','Older than the gods and has forgotten nothing in all that time.'),
('young_gold_dragon','Young Gold Dragon','dragon','lg',10,4,18,178,23,14,21,16,13,20,'[{"source":"Young Gold Dragon","target":"immune.fire"}]','Will know whether you are lying before you have finished the sentence.'),
('behir','Behir','monstrosity','huge',11,4,17,168,23,16,18,7,14,12,'[{"source":"Behir","target":"immune.lightning"}]','Twelve legs, a lightning breath, and it swallows what it has knocked down.'),
('djinni','Djinni','elemental','lg',11,4,17,161,21,15,22,15,16,20,'[{"source":"Djinni","target":"immune.lightning"},{"source":"Djinni","target":"immune.thunder"}]','Proud, generous, and extremely particular about the wording.'),
('efreeti','Efreeti','elemental','lg',11,4,17,200,22,12,24,16,15,16,'[{"source":"Efreeti","target":"immune.fire"}]','Grants what you asked for rather than what you meant, and enjoys the difference.'),
('gynosphinx','Gynosphinx','monstrosity','lg',11,4,17,136,18,15,16,18,18,18,'[{"source":"Gynosphinx","target":"immune.psychic"},{"source":"Gynosphinx","target":"resist.bludgeoning.nonmagical"},{"source":"Gynosphinx","target":"resist.piercing.nonmagical"},{"source":"Gynosphinx","target":"resist.slashing.nonmagical"}]','Knows the answer and will trade it for a better question.'),
('remorhaz','Remorhaz','monstrosity','huge',11,4,17,195,24,13,21,4,10,5,'[{"source":"Remorhaz","target":"immune.cold"},{"source":"Remorhaz","target":"immune.fire"}]','Comes up through the ice already glowing, and the ice is why you did not hear it.'),
('adult_brass_dragon','Adult Brass Dragon','dragon','huge',13,5,18,172,23,10,21,14,13,17,'[{"source":"Adult Brass Dragon","target":"immune.fire"}]','Lonely, which is more dangerous in a dragon than malice.'),
('rakshasa','Rakshasa','fiend','med',13,5,16,110,14,17,18,13,16,20,'[{"source":"Rakshasa","target":"immune.bludgeoning.nonmagical"},{"source":"Rakshasa","target":"immune.piercing.nonmagical"},{"source":"Rakshasa","target":"immune.slashing.nonmagical"},{"source":"Rakshasa","target":"vulnerable.piercing"}]','Wealthy, charming and backwards in ways you notice too late. Ordinary weapons slide off; a blessed blade does not.'),
('ice_devil','Ice Devil','fiend','lg',14,5,18,180,21,14,18,18,15,18,'[{"source":"Ice Devil","target":"immune.cold"},{"source":"Ice Devil","target":"immune.fire"},{"source":"Ice Devil","target":"immune.poison"},{"source":"Ice Devil","target":"resist.bludgeoning.nonmagical"},{"source":"Ice Devil","target":"resist.piercing.nonmagical"},{"source":"Ice Devil","target":"resist.slashing.nonmagical"}]','Runs a war from the middle of it, and is the worst thing on its own side.'),
('adult_copper_dragon','Adult Copper Dragon','dragon','huge',14,5,18,184,23,12,21,18,15,17,'[{"source":"Adult Copper Dragon","target":"immune.acid"}]','Still telling jokes, now with four centuries of material.'),
('adult_bronze_dragon','Adult Bronze Dragon','dragon','huge',15,5,19,212,25,10,23,16,15,19,'[{"source":"Adult Bronze Dragon","target":"immune.lightning"}]','Picks a side in other people''s wars and then wins it.'),
('purple_worm','Purple Worm','monstrosity','grg',15,5,18,247,28,7,22,1,8,4,'[]','Arrives from underneath. There is no warning worth the name.'),
('mummy_lord','Mummy Lord','undead','med',15,5,17,97,18,10,17,11,18,16,'[{"source":"Mummy Lord","target":"immune.necrotic"},{"source":"Mummy Lord","target":"immune.poison"},{"source":"Mummy Lord","target":"vulnerable.fire"},{"source":"Mummy Lord","target":"immune.bludgeoning.nonmagical"},{"source":"Mummy Lord","target":"immune.piercing.nonmagical"},{"source":"Mummy Lord","target":"immune.slashing.nonmagical"}]','Was a king and a priest, and has not accepted that either is over.'),
('planetar','Planetar','celestial','lg',16,5,19,200,24,20,24,19,22,25,'[{"source":"Planetar","target":"resist.radiant"},{"source":"Planetar","target":"resist.bludgeoning.nonmagical"},{"source":"Planetar","target":"resist.piercing.nonmagical"},{"source":"Planetar","target":"resist.slashing.nonmagical"}]','Sent to settle something, and grieves about it afterwards rather than during.'),
('adult_silver_dragon','Adult Silver Dragon','dragon','huge',16,5,19,243,27,10,25,16,13,21,'[{"source":"Adult Silver Dragon","target":"immune.cold"}]','Keeps a human life somewhere, with people in it who do not know.'),
('adult_gold_dragon','Adult Gold Dragon','dragon','huge',17,6,19,256,27,14,25,16,15,24,'[{"source":"Adult Gold Dragon","target":"immune.fire"}]','The one that judges, and has never once been talked out of a judgement.'),
('androsphinx','Androsphinx','monstrosity','lg',17,6,17,199,22,10,20,16,18,23,'[{"source":"Androsphinx","target":"immune.psychic"},{"source":"Androsphinx","target":"immune.bludgeoning.nonmagical"},{"source":"Androsphinx","target":"immune.piercing.nonmagical"},{"source":"Androsphinx","target":"immune.slashing.nonmagical"}]','Guards a truth, and its roar is the first warning and the second one too.'),
('dragon_turtle','Dragon Turtle','dragon','grg',17,6,20,341,25,10,20,10,12,12,'[{"source":"Dragon Turtle","target":"resist.fire"}]','The island you anchored at. Steam comes off the water before it surfaces.'),
('ancient_white_dragon','Ancient White Dragon','dragon','grg',20,6,20,333,26,10,26,10,13,14,'[{"source":"Ancient White Dragon","target":"immune.cold"}]','The simplest ancient dragon and the one most likely to simply eat you.'),
('ancient_brass_dragon','Ancient Brass Dragon','dragon','grg',20,6,20,297,27,10,25,16,15,19,'[{"source":"Ancient Brass Dragon","target":"immune.fire"}]','Has not had a proper conversation in two hundred years and intends to have one now.'),
('ancient_black_dragon','Ancient Black Dragon','dragon','grg',21,7,22,367,27,14,25,16,15,19,'[{"source":"Ancient Black Dragon","target":"immune.acid"}]','The swamp has been its for so long that the swamp has started to agree.'),
('ancient_copper_dragon','Ancient Copper Dragon','dragon','grg',21,7,21,350,27,12,25,20,17,19,'[{"source":"Ancient Copper Dragon","target":"immune.acid"}]','Enormous, ancient, and still doing the voices.'),
('solar','Solar','celestial','lg',21,7,21,243,26,22,26,25,25,30,'[{"source":"Solar","target":"immune.necrotic"},{"source":"Solar","target":"immune.poison"},{"source":"Solar","target":"resist.radiant"},{"source":"Solar","target":"resist.bludgeoning.nonmagical"},{"source":"Solar","target":"resist.piercing.nonmagical"},{"source":"Solar","target":"resist.slashing.nonmagical"}]','Sent when the matter is already decided. The bow does not miss.'),
('ancient_green_dragon','Ancient Green Dragon','dragon','grg',22,7,21,385,27,12,25,20,17,19,'[{"source":"Ancient Green Dragon","target":"immune.poison"}]','Has run the forest through intermediaries for so long that nobody alive knows it is there.'),
('ancient_bronze_dragon','Ancient Bronze Dragon','dragon','grg',22,7,22,444,29,10,27,18,17,21,'[{"source":"Ancient Bronze Dragon","target":"immune.lightning"}]','Remembers every war it has fought in and has fought in most of them.'),
('ancient_blue_dragon','Ancient Blue Dragon','dragon','grg',23,7,22,481,29,10,27,18,17,21,'[{"source":"Ancient Blue Dragon","target":"immune.lightning"}]','The desert is a roof over its hall and the storm above is its doing.'),
('ancient_silver_dragon','Ancient Silver Dragon','dragon','grg',23,7,22,487,30,10,29,18,15,23,'[{"source":"Ancient Silver Dragon","target":"immune.cold"}]','Has outlived every friend it ever made among the short-lived, repeatedly.'),
('kraken','Kraken','monstrosity','grg',23,7,18,472,30,11,25,22,18,20,'[{"source":"Kraken","target":"immune.lightning"},{"source":"Kraken","target":"immune.bludgeoning.nonmagical"},{"source":"Kraken","target":"immune.piercing.nonmagical"},{"source":"Kraken","target":"immune.slashing.nonmagical"}]','Worshipped in places, which does not make it a god and does not help them.'),
('ancient_red_dragon','Ancient Red Dragon','dragon','grg',24,7,22,546,30,10,29,18,15,23,'[{"source":"Ancient Red Dragon","target":"immune.fire"}]','The one everybody means when they say dragon, and worse than the stories.'),
('ancient_gold_dragon','Ancient Gold Dragon','dragon','grg',24,7,22,546,30,14,29,18,17,28,'[{"source":"Ancient Gold Dragon","target":"immune.fire"}]','Will hear you out in full, and has already decided.'),
('tarrasque','Tarrasque','monstrosity','grg',30,9,25,676,30,11,30,3,11,11,'[{"source":"Tarrasque","target":"immune.fire"},{"source":"Tarrasque","target":"immune.poison"},{"source":"Tarrasque","target":"immune.bludgeoning.nonmagical"},{"source":"Tarrasque","target":"immune.piercing.nonmagical"},{"source":"Tarrasque","target":"immune.slashing.nonmagical"}]','There is one. It is asleep. The campaign is about keeping it that way.')
) as v(k,n,ct,sz,lvl,pb,ac,hp,s,d,c,i,w,ch,g,nt)
on conflict (key) where game_id is null do update set
  name=excluded.name, creature_type=excluded.creature_type, size=excluded.size,
  level=excluded.level, prof_bonus=excluded.prof_bonus, ac=excluded.ac,
  hp_max=excluded.hp_max, str=excluded.str, dex=excluded.dex, con=excluded.con,
  intl=excluded.intl, wis=excluded.wis, cha=excluded.cha,
  grants=excluded.grants, notes=excluded.notes,
  weapon_profs=excluded.weapon_profs, armor_profs=excluded.armor_profs;

-- AND WHAT EACH ONE FIGHTS WITH. 135's rule: a statblock with no kit is a
-- creature a DM can enrol and then do nothing with.

insert into npc_items (npc_key,item_key,quantity,equipped)
values
('homunculus','bite',1,true),
('stirge','sting',1,true),
('aarakocra','talons',1,true),
('aarakocra','javelin',1,true),
('grimlock','club',1,true),
('pteranodon','beak',1,true),
('magmin','fist',1,true),
('brass_dragon_wyrmling','bite',1,true),
('brass_dragon_wyrmling','claws',1,true),
('copper_dragon_wyrmling','bite',1,true),
('copper_dragon_wyrmling','claws',1,true),
('centaur','pike',1,true),
('centaur','longbow',1,true),
('centaur','hooves',1,true),
('bronze_dragon_wyrmling','bite',1,true),
('bronze_dragon_wyrmling','claws',1,true),
('allosaurus','bite',1,true),
('allosaurus','claws',1,true),
('azer','warhammer',1,true),
('sea_hag','claws',1,true),
('plesiosaurus','bite',1,true),
('ogre_zombie','morningstar',1,true),
('merrow','bite',1,true),
('merrow','claws',1,true),
('merrow','trident',1,true),
('rug_of_smothering','constrict',1,true),
('silver_dragon_wyrmling','bite',1,true),
('silver_dragon_wyrmling','claws',1,true),
('will_o_wisp','slam',1,true),
('ankylosaurus','tail',1,true),
('doppelganger','slam',1,true),
('green_hag','claws',1,true),
('bearded_devil','glaive',1,true),
('bearded_devil','bite',1,true),
('gold_dragon_wyrmling','bite',1,true),
('gold_dragon_wyrmling','claws',1,true),
('nightmare','hooves',1,true),
('couatl','bite',1,true),
('couatl','constrict',1,true),
('lamia','claws',1,true),
('lamia','dagger',1,true),
('barbed_devil','claws',1,true),
('barbed_devil','tail',1,true),
('gorgon','gore',1,true),
('gorgon','hooves',1,true),
('salamander','spear',1,true),
('salamander','tail',1,true),
('roper','tendrils',1,true),
('roper','bite',1,true),
('triceratops','gore',1,true),
('triceratops','hooves',1,true),
('night_hag','claws',1,true),
('xorn','claws',1,true),
('xorn','bite',1,true),
('drider','longsword',1,true),
('drider','longbow',1,true),
('drider','bite',1,true),
('invisible_stalker','slam',1,true),
('young_brass_dragon','bite',1,true),
('young_brass_dragon','claws',1,true),
('young_brass_dragon','tail',1,true),
('young_brass_dragon','wing',1,true),
('young_copper_dragon','bite',1,true),
('young_copper_dragon','claws',1,true),
('young_copper_dragon','tail',1,true),
('young_copper_dragon','wing',1,true),
('cloaker','bite',1,true),
('cloaker','tail',1,true),
('tyrannosaurus_rex','bite',1,true),
('tyrannosaurus_rex','tail',1,true),
('young_bronze_dragon','bite',1,true),
('young_bronze_dragon','claws',1,true),
('young_bronze_dragon','tail',1,true),
('young_bronze_dragon','wing',1,true),
('spirit_naga','bite',1,true),
('chain_devil','whip',1,true),
('young_silver_dragon','bite',1,true),
('young_silver_dragon','claws',1,true),
('young_silver_dragon','tail',1,true),
('young_silver_dragon','wing',1,true),
('guardian_naga','bite',1,true),
('aboleth','tendrils',1,true),
('young_gold_dragon','bite',1,true),
('young_gold_dragon','claws',1,true),
('young_gold_dragon','tail',1,true),
('young_gold_dragon','wing',1,true),
('behir','bite',1,true),
('behir','constrict',1,true),
('djinni','scimitar',1,true),
('efreeti','scimitar',1,true),
('gynosphinx','claws',1,true),
('remorhaz','bite',1,true),
('adult_brass_dragon','bite',1,true),
('adult_brass_dragon','claws',1,true),
('adult_brass_dragon','tail',1,true),
('adult_brass_dragon','wing',1,true),
('rakshasa','claws',1,true),
('ice_devil','bite',1,true),
('ice_devil','claws',1,true),
('ice_devil','tail',1,true),
('adult_copper_dragon','bite',1,true),
('adult_copper_dragon','claws',1,true),
('adult_copper_dragon','tail',1,true),
('adult_copper_dragon','wing',1,true),
('adult_bronze_dragon','bite',1,true),
('adult_bronze_dragon','claws',1,true),
('adult_bronze_dragon','tail',1,true),
('adult_bronze_dragon','wing',1,true),
('purple_worm','bite',1,true),
('purple_worm','sting',1,true),
('mummy_lord','slam',1,true),
('planetar','greatsword',1,true),
('adult_silver_dragon','bite',1,true),
('adult_silver_dragon','claws',1,true),
('adult_silver_dragon','tail',1,true),
('adult_silver_dragon','wing',1,true),
('adult_gold_dragon','bite',1,true),
('adult_gold_dragon','claws',1,true),
('adult_gold_dragon','tail',1,true),
('adult_gold_dragon','wing',1,true),
('androsphinx','claws',1,true),
('dragon_turtle','bite',1,true),
('dragon_turtle','claws',1,true),
('dragon_turtle','tail',1,true),
('ancient_white_dragon','bite',1,true),
('ancient_white_dragon','claws',1,true),
('ancient_white_dragon','tail',1,true),
('ancient_white_dragon','wing',1,true),
('ancient_brass_dragon','bite',1,true),
('ancient_brass_dragon','claws',1,true),
('ancient_brass_dragon','tail',1,true),
('ancient_brass_dragon','wing',1,true),
('ancient_black_dragon','bite',1,true),
('ancient_black_dragon','claws',1,true),
('ancient_black_dragon','tail',1,true),
('ancient_black_dragon','wing',1,true),
('ancient_copper_dragon','bite',1,true),
('ancient_copper_dragon','claws',1,true),
('ancient_copper_dragon','tail',1,true),
('ancient_copper_dragon','wing',1,true),
('solar','greatsword',1,true),
('solar','longbow',1,true),
('ancient_green_dragon','bite',1,true),
('ancient_green_dragon','claws',1,true),
('ancient_green_dragon','tail',1,true),
('ancient_green_dragon','wing',1,true),
('ancient_bronze_dragon','bite',1,true),
('ancient_bronze_dragon','claws',1,true),
('ancient_bronze_dragon','tail',1,true),
('ancient_bronze_dragon','wing',1,true),
('ancient_blue_dragon','bite',1,true),
('ancient_blue_dragon','claws',1,true),
('ancient_blue_dragon','tail',1,true),
('ancient_blue_dragon','wing',1,true),
('ancient_silver_dragon','bite',1,true),
('ancient_silver_dragon','claws',1,true),
('ancient_silver_dragon','tail',1,true),
('ancient_silver_dragon','wing',1,true),
('kraken','bite',1,true),
('kraken','tendrils',1,true),
('kraken','constrict',1,true),
('ancient_red_dragon','bite',1,true),
('ancient_red_dragon','claws',1,true),
('ancient_red_dragon','tail',1,true),
('ancient_red_dragon','wing',1,true),
('ancient_gold_dragon','bite',1,true),
('ancient_gold_dragon','claws',1,true),
('ancient_gold_dragon','tail',1,true),
('ancient_gold_dragon','wing',1,true),
('tarrasque','bite',1,true),
('tarrasque','claws',1,true),
('tarrasque','tail',1,true),
('tarrasque','gore',1,true)
on conflict (npc_key, item_key) where game_id is null do update set
  quantity = excluded.quantity, equipped = excluded.equipped;
