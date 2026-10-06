-- 142. SEVENTY-ONE MORE OF THE SRD.
--
-- 132 creatures in the shared bestiary and the holes were specific: no
-- dragon older than a wyrmling, no lich, no vampire, no golem, no
-- lycanthrope, and none of the beasts a DM reaches for most - a mastiff,
-- a riding horse, an elephant. This fills those rather than working
-- alphabetically through the book.
--
-- 203 statblocks after this, out of an SRD that runs past three hundred.
--
-- ---------------------------------------------------------------------
-- THE SAME WARNING 130 CARRIED, AND IT HAS NOT GOT WEAKER
-- ---------------------------------------------------------------------
--
-- AC, HP and the six abilities are SRD-derived FROM MEMORY. They are
-- right in shape and will be wrong in places, and the famous ones are
-- better than the obscure ones: a pit fiend and a werewolf are numbers
-- that get read often, a nalfeshnee is not. SPOT-CHECK ANYTHING BEFORE A
-- SESSION RUNS ON IT.
--
-- What IS verified is that these are the rows intended - every creature
-- and every kit row digested locally and against the live database after
-- applying, both matching - and that every `item_key` below names an
-- item that exists, which is the failure 131 had to chase.
--
-- THE HONEST WAY TO FINISH IS STILL A FILE. `creature_io.rs` reads an
-- `odyssey1e.creature` envelope, and a machine-readable SRD dump through
-- it beats another batch of recollection. This is the last batch worth
-- writing by hand.
--
-- ---------------------------------------------------------------------
-- "FROM NONMAGICAL ATTACKS" IS NOT EXPRESSIBLE, AND 23 OF THESE WANT IT
-- ---------------------------------------------------------------------
--
-- The commonest resistance in 5e is "bludgeoning, piercing and slashing
-- FROM NONMAGICAL ATTACKS" - every lycanthrope, both vampires, all four
-- golems, most fiends. 116's vocabulary has `resist.bludgeoning` and no
-- way to say the qualifier.
--
-- RECORDED AS PLAIN RESISTANCE, following 130 - the Intellect Devourer
-- has carried it that way since 131 and a bestiary where the same rule
-- is written two ways is worse than one where it is written imprecisely.
-- But be clear about what that costs: a party's magic sword is halved
-- against a werewolf, which is the exact opposite of what the rule is
-- FOR. The qualifier is the point of the rule and we are dropping it.
--
-- The fix is a target that carries it - `resist.slashing.nonmagical` -
-- read by `resist.rs` and applied against the weapon's own grants, since
-- 100 already knows whether a weapon is enchanted. Not built here
-- because it would change how 131's creatures resolve too, and that is
-- its own change with its own verification. Logged in STATUS.

-- ---------------------------------------------------------------------
-- A NOTE FOR WHOEVER RECOVERS THIS ONE
-- ---------------------------------------------------------------------
--
-- `schema_migrations.statements` holds this migration with the header
-- above replaced by a single line pointing at this file - the applied
-- SQL is identical, the prose is not. Every other migration in this
-- folder was applied with its header intact and can be recovered from
-- the database byte-for-byte; this one cannot. Git is the record for
-- 142, and `tools/recover_migrations.py` would give you the statements
-- and none of the reasoning.

insert into npcs (key,name,creature_type,size,level,prof_bonus,ac,hp_max,str,dex,con,intl,wis,cha,grants,notes,weapon_profs,armor_profs)
select v.k,v.n,v.ct,v.sz,v.lvl,v.pb,v.ac,v.hp,v.s,v.d,v.c,v.i,v.w,v.ch,v.g::jsonb,v.nt,array['sim','mar'],array['lgt','med','hvy','shl']
from (values

-- ---- DRAGONS, young and adult. The wyrmlings had nothing above them --

('young_black_dragon','Young Black Dragon','dragon','lg',11,3,18,127,19,14,17,12,11,15,'[{"source":"Young Black Dragon","target":"immune.acid"}]','Hunts from water it has made unpleasant to be in. Patient in a way the wyrmling is not.'),
('young_blue_dragon','Young Blue Dragon','dragon','lg',12,3,18,152,21,10,19,14,13,17,'[{"source":"Young Blue Dragon","target":"immune.lightning"}]','Arrives out of a clear sky with the storm already behind it.'),
('young_green_dragon','Young Green Dragon','dragon','lg',11,3,18,136,19,12,17,16,13,15,'[{"source":"Young Green Dragon","target":"immune.poison"}]','Would rather talk you into something. The breath is what happens when that fails.'),
('young_red_dragon','Young Red Dragon','dragon','lg',13,3,18,178,23,10,21,14,11,19,'[{"source":"Young Red Dragon","target":"immune.fire"}]','Already believes the mountain is its own, and is nearly large enough to be right.'),
('young_white_dragon','Young White Dragon','dragon','lg',10,3,17,133,18,10,18,6,11,12,'[{"source":"Young White Dragon","target":"immune.cold"}]','The least clever of them and the most direct, which is not an improvement.'),
('adult_black_dragon','Adult Black Dragon','dragon','huge',16,3,19,195,23,14,21,14,13,17,'[{"source":"Adult Black Dragon","target":"immune.acid"}]','The swamp is its, and everything living in the swamp knows the arrangement.'),
('adult_blue_dragon','Adult Blue Dragon','dragon','huge',17,3,19,225,25,10,23,16,15,19,'[{"source":"Adult Blue Dragon","target":"immune.lightning"}]','Digs its lair and lets the desert bury the evidence.'),
('adult_green_dragon','Adult Green Dragon','dragon','huge',16,3,19,207,23,12,21,18,15,17,'[{"source":"Adult Green Dragon","target":"immune.poison"}]','Has been running the forest through other people for a century.'),
('adult_red_dragon','Adult Red Dragon','dragon','huge',18,3,19,256,27,10,25,16,13,21,'[{"source":"Adult Red Dragon","target":"immune.fire"}]','The one everybody means. Vain, ancient, and entirely capable of it.'),
('adult_white_dragon','Adult White Dragon','dragon','huge',15,3,18,200,22,10,22,8,12,12,'[{"source":"Adult White Dragon","target":"immune.cold"}]','Keeps its kills frozen and stacked, which is as close to planning as it gets.'),

-- ---- THE CLASSICS. Named monsters a campaign eventually wants -------

('lich','Lich','undead','med',18,3,17,135,11,16,16,20,14,16,'[{"source":"Lich","target":"resist.cold"},{"source":"Lich","target":"resist.lightning"},{"source":"Lich","target":"resist.necrotic"},{"source":"Lich","target":"immune.poison"},{"source":"Lich","target":"resist.bludgeoning"},{"source":"Lich","target":"resist.piercing"},{"source":"Lich","target":"resist.slashing"}]','Gave up everything that could die and kept everything that could plan.'),
('vampire','Vampire','undead','med',14,3,16,144,18,18,18,17,15,18,'[{"source":"Vampire","target":"resist.necrotic"},{"source":"Vampire","target":"resist.bludgeoning"},{"source":"Vampire","target":"resist.piercing"},{"source":"Vampire","target":"resist.slashing"}]','Charming, unhurried, and counting on you to be polite about it.'),
('vampire_spawn','Vampire Spawn','undead','med',6,2,15,82,16,16,16,11,10,12,'[{"source":"Vampire Spawn","target":"resist.necrotic"},{"source":"Vampire Spawn","target":"resist.bludgeoning"},{"source":"Vampire Spawn","target":"resist.piercing"},{"source":"Vampire Spawn","target":"resist.slashing"}]','What is left of somebody who met a vampire and was not interesting enough to keep.'),
('ghost','Ghost','undead','med',5,2,11,45,7,13,10,10,12,17,'[{"source":"Ghost","target":"resist.acid"},{"source":"Ghost","target":"resist.fire"},{"source":"Ghost","target":"resist.lightning"},{"source":"Ghost","target":"resist.thunder"},{"source":"Ghost","target":"immune.cold"},{"source":"Ghost","target":"immune.necrotic"},{"source":"Ghost","target":"immune.poison"},{"source":"Ghost","target":"resist.bludgeoning"},{"source":"Ghost","target":"resist.piercing"},{"source":"Ghost","target":"resist.slashing"}]','Still working on something, and will go through you to keep working on it.'),
('banshee','Banshee','undead','med',5,2,12,58,1,14,10,12,11,17,'[{"source":"Banshee","target":"resist.acid"},{"source":"Banshee","target":"resist.fire"},{"source":"Banshee","target":"resist.lightning"},{"source":"Banshee","target":"resist.thunder"},{"source":"Banshee","target":"immune.cold"},{"source":"Banshee","target":"immune.necrotic"},{"source":"Banshee","target":"immune.poison"},{"source":"Banshee","target":"resist.bludgeoning"},{"source":"Banshee","target":"resist.piercing"},{"source":"Banshee","target":"resist.slashing"}]','Was beautiful once and has not forgiven anybody for noticing.'),
('medusa','Medusa','monstrosity','med',7,2,15,127,10,15,16,12,13,15,'[]','Lives among statues that used to be people who came to argue.'),
('manticore','Manticore','monstrosity','lg',4,2,14,68,17,16,17,7,12,8,'[]','Talks while it fights, and the tail is working the whole time.'),
('chimera','Chimera','monstrosity','lg',7,2,14,114,19,11,19,3,14,10,'[]','Three bad ideas sharing one body and no agreement about the plan.'),
('hydra','Hydra','monstrosity','huge',9,2,15,172,20,12,20,2,10,7,'[]','Cutting a head off is a decision, and it is usually the wrong one.'),
('treant','Treant','plant','huge',10,3,16,138,23,8,21,12,16,12,'[{"source":"Treant","target":"vulnerable.fire"},{"source":"Treant","target":"resist.bludgeoning"},{"source":"Treant","target":"resist.piercing"}]','Slow to decide and slower to stop, and it has been deciding about you for a while.'),
('unicorn','Unicorn','celestial','lg',6,2,12,67,18,14,15,11,17,16,'[{"source":"Unicorn","target":"immune.poison"}]','The forest belongs to it in a way nobody had to enforce.'),
('roc','Roc','monstrosity','grg',12,3,15,248,28,10,20,3,10,9,'[]','Takes a horse the way a gull takes a chip, and from about as far up.'),
('mimic','Mimic','monstrosity','med',3,2,12,58,17,12,15,5,13,8,'[{"source":"Mimic","target":"immune.acid"}]','It was a chest a moment ago and it is extremely pleased with itself.'),
('oni','Oni','giant','lg',8,2,16,110,19,11,16,14,12,15,'[]','Large, clever and a liar, in that order of surprise.'),

-- ---- FIENDS. Devils bargain, demons do not -------------------------

('balor','Balor','fiend','huge',19,3,19,262,26,15,22,20,16,22,'[{"source":"Balor","target":"immune.fire"},{"source":"Balor","target":"immune.poison"},{"source":"Balor","target":"resist.cold"},{"source":"Balor","target":"resist.lightning"},{"source":"Balor","target":"resist.bludgeoning"},{"source":"Balor","target":"resist.piercing"},{"source":"Balor","target":"resist.slashing"}]','Commands other demons, which tells you what it does to things that are not.'),
('marilith','Marilith','fiend','lg',16,3,18,189,18,20,20,18,16,20,'[{"source":"Marilith","target":"resist.cold"},{"source":"Marilith","target":"resist.fire"},{"source":"Marilith","target":"resist.lightning"},{"source":"Marilith","target":"immune.poison"},{"source":"Marilith","target":"resist.bludgeoning"},{"source":"Marilith","target":"resist.piercing"},{"source":"Marilith","target":"resist.slashing"}]','Six arms, six blades, and the tail is not one of the six.'),
('vrock','Vrock','fiend','lg',6,2,15,104,17,15,18,8,13,8,'[{"source":"Vrock","target":"resist.cold"},{"source":"Vrock","target":"resist.fire"},{"source":"Vrock","target":"resist.lightning"},{"source":"Vrock","target":"immune.poison"}]','A vulture that got into something it should not have, and enjoyed it.'),
('hezrou','Hezrou','fiend','lg',8,2,16,136,19,17,20,5,12,13,'[{"source":"Hezrou","target":"resist.cold"},{"source":"Hezrou","target":"resist.fire"},{"source":"Hezrou","target":"resist.lightning"},{"source":"Hezrou","target":"immune.poison"}]','You smell it before you see it and you keep smelling it afterwards.'),
('glabrezu','Glabrezu','fiend','lg',9,2,17,157,20,15,21,19,17,16,'[{"source":"Glabrezu","target":"resist.cold"},{"source":"Glabrezu","target":"resist.fire"},{"source":"Glabrezu","target":"resist.lightning"},{"source":"Glabrezu","target":"immune.poison"}]','Offers you exactly what you wanted, which is how you can tell.'),
('nalfeshnee','Nalfeshnee','fiend','lg',13,3,18,184,21,10,22,19,12,15,'[{"source":"Nalfeshnee","target":"resist.cold"},{"source":"Nalfeshnee","target":"resist.fire"},{"source":"Nalfeshnee","target":"resist.lightning"},{"source":"Nalfeshnee","target":"immune.poison"}]','Enormously clever and enormously vain, and will tell you about both.'),
('lemure','Lemure','fiend','med',1,2,7,13,10,5,11,1,11,3,'[{"source":"Lemure","target":"immune.fire"},{"source":"Lemure","target":"immune.poison"},{"source":"Lemure","target":"resist.cold"}]','The bottom of the arrangement. There are always more of them.'),
('bone_devil','Bone Devil','fiend','lg',9,2,19,142,18,16,18,13,14,16,'[{"source":"Bone Devil","target":"immune.fire"},{"source":"Bone Devil","target":"immune.poison"},{"source":"Bone Devil","target":"resist.cold"},{"source":"Bone Devil","target":"resist.bludgeoning"},{"source":"Bone Devil","target":"resist.piercing"},{"source":"Bone Devil","target":"resist.slashing"}]','Supervises. The sting is for when supervision has not worked.'),
('erinyes','Erinyes','fiend','med',12,3,18,153,18,16,18,14,14,18,'[{"source":"Erinyes","target":"immune.fire"},{"source":"Erinyes","target":"immune.poison"},{"source":"Erinyes","target":"resist.cold"},{"source":"Erinyes","target":"resist.bludgeoning"},{"source":"Erinyes","target":"resist.piercing"},{"source":"Erinyes","target":"resist.slashing"}]','Sent after somebody specific, and you are standing next to them.'),
('horned_devil','Horned Devil','fiend','lg',11,3,18,148,22,17,21,12,16,17,'[{"source":"Horned Devil","target":"immune.fire"},{"source":"Horned Devil","target":"immune.poison"},{"source":"Horned Devil","target":"resist.cold"},{"source":"Horned Devil","target":"resist.bludgeoning"},{"source":"Horned Devil","target":"resist.piercing"},{"source":"Horned Devil","target":"resist.slashing"}]','Hangs back, throws fire, and comes in when you are already busy.'),
('pit_fiend','Pit Fiend','fiend','lg',20,3,19,300,26,14,24,22,18,24,'[{"source":"Pit Fiend","target":"immune.fire"},{"source":"Pit Fiend","target":"immune.poison"},{"source":"Pit Fiend","target":"resist.cold"},{"source":"Pit Fiend","target":"resist.bludgeoning"},{"source":"Pit Fiend","target":"resist.piercing"},{"source":"Pit Fiend","target":"resist.slashing"}]','Does not need to fight you and will, because the contract was clear.'),

-- ---- CONSTRUCTS AND THE TWO MISSING GIANTS -------------------------

('flesh_golem','Flesh Golem','construct','med',6,2,9,93,19,9,18,6,10,5,'[{"source":"Flesh Golem","target":"immune.lightning"},{"source":"Flesh Golem","target":"immune.poison"},{"source":"Flesh Golem","target":"resist.bludgeoning"},{"source":"Flesh Golem","target":"resist.piercing"},{"source":"Flesh Golem","target":"resist.slashing"}]','Somebody made it out of several people and it has not been told.'),
('clay_golem','Clay Golem','construct','lg',11,3,14,133,20,9,18,3,8,1,'[{"source":"Clay Golem","target":"immune.acid"},{"source":"Clay Golem","target":"immune.poison"},{"source":"Clay Golem","target":"immune.psychic"},{"source":"Clay Golem","target":"resist.bludgeoning"},{"source":"Clay Golem","target":"resist.piercing"},{"source":"Clay Golem","target":"resist.slashing"}]','Moves like wet earth and hits like dry stone.'),
('stone_golem','Stone Golem','construct','lg',12,3,17,178,22,9,20,3,11,1,'[{"source":"Stone Golem","target":"immune.poison"},{"source":"Stone Golem","target":"immune.psychic"},{"source":"Stone Golem","target":"resist.bludgeoning"},{"source":"Stone Golem","target":"resist.piercing"},{"source":"Stone Golem","target":"resist.slashing"}]','Was a statue for two hundred years and is prepared to be one again.'),
('iron_golem','Iron Golem','construct','lg',17,3,20,210,24,9,20,3,11,1,'[{"source":"Iron Golem","target":"immune.fire"},{"source":"Iron Golem","target":"immune.poison"},{"source":"Iron Golem","target":"immune.psychic"},{"source":"Iron Golem","target":"resist.bludgeoning"},{"source":"Iron Golem","target":"resist.piercing"},{"source":"Iron Golem","target":"resist.slashing"}]','There is no clever answer to this one. There is only enough damage.'),
('shield_guardian','Shield Guardian','construct','lg',8,2,17,142,18,8,18,7,10,3,'[]','Takes the blow meant for whoever is wearing the amulet, every time, without comment.'),
('cloud_giant','Cloud Giant','giant','huge',12,3,14,200,27,10,22,12,16,16,'[]','Rich, bored and slightly embarrassed about how it makes its money.'),
('storm_giant','Storm Giant','giant','huge',15,3,16,230,29,14,20,16,18,18,'[{"source":"Storm Giant","target":"immune.lightning"},{"source":"Storm Giant","target":"immune.thunder"},{"source":"Storm Giant","target":"resist.cold"}]','Grave, enormous, and genuinely sorry about what is going to happen.'),

-- ---- PEOPLE. The ones a town has and the ones it does not ----------

('commoner','Commoner','humanoid','med',1,2,10,4,10,10,10,10,10,10,'[]','A farmer, a carter, a cook. Four hit points and a whole life.'),
('noble','Noble','humanoid','med',1,2,15,9,11,12,11,12,14,16,'[]','Will remember this, and has people.'),
('bandit_captain','Bandit Captain','humanoid','med',6,2,15,65,15,16,14,14,11,14,'[]','Keeps the others in line by being the most dangerous thing in the camp.'),
('assassin','Assassin','humanoid','med',8,2,15,78,11,16,14,13,11,10,'[]','Was already in the room. The fight is what happens after the first attack misses.'),
('gladiator','Gladiator','humanoid','med',9,2,16,112,18,15,16,10,12,15,'[]','Fights for a crowd and has learned exactly how long to take.'),
('drow','Drow','humanoid','med',2,2,15,13,10,14,10,11,11,12,'[]','Comes from somewhere with no sky and has opinions about yours.'),
('duergar','Duergar','humanoid','med',3,2,16,26,14,11,14,11,10,9,'[{"source":"Duergar","target":"resist.poison"}]','A dwarf the deep places kept, and kept working.'),
('sahuagin','Sahuagin','humanoid','med',3,2,12,22,13,11,12,12,13,9,'[]','Comes out of the surf in numbers and goes back the same way.'),
('yuan_ti_pureblood','Yuan-ti Pureblood','humanoid','med',4,2,11,40,11,12,11,13,12,14,'[{"source":"Yuan-ti Pureblood","target":"immune.poison"}]','Passes for human until it is no longer useful to.'),

-- ---- LYCANTHROPES. See the header about "nonmagical" ---------------

('wererat','Wererat','humanoid','med',3,2,12,33,10,15,12,11,10,8,'[{"source":"Wererat","target":"resist.bludgeoning"},{"source":"Wererat","target":"resist.piercing"},{"source":"Wererat","target":"resist.slashing"}]','Runs the bad end of a city and can leave through a drain.'),
('werewolf','Werewolf','humanoid','med',4,2,12,58,15,13,14,10,11,10,'[{"source":"Werewolf","target":"resist.bludgeoning"},{"source":"Werewolf","target":"resist.piercing"},{"source":"Werewolf","target":"resist.slashing"}]','Was somebody you knew this morning and will be again in the morning.'),
('wereboar','Wereboar','humanoid','med',5,2,12,78,17,10,15,10,11,8,'[{"source":"Wereboar","target":"resist.bludgeoning"},{"source":"Wereboar","target":"resist.piercing"},{"source":"Wereboar","target":"resist.slashing"}]','Picks a direction and stops being interested in any other one.'),
('weretiger','Weretiger','humanoid','med',6,2,12,120,17,15,16,10,13,11,'[{"source":"Weretiger","target":"resist.bludgeoning"},{"source":"Weretiger","target":"resist.piercing"},{"source":"Weretiger","target":"resist.slashing"}]','Hunts for reasons it would rather not explain to you.'),
('werebear','Werebear','humanoid','lg',7,2,10,135,19,10,17,11,12,12,'[{"source":"Werebear","target":"resist.bludgeoning"},{"source":"Werebear","target":"resist.piercing"},{"source":"Werebear","target":"resist.slashing"}]','Usually the one trying to stop the fight, right up until it is not.'),

-- ---- BEASTS. The ones a table actually asks for --------------------

('mastiff','Mastiff','beast','med',1,2,12,5,13,14,12,3,12,7,'[]','Loyal, loud, and will have a go at something far larger.'),
('riding_horse','Riding Horse','beast','lg',1,2,10,13,16,10,12,2,11,7,'[]','Fast, patient, and entirely done with this once the shouting starts.'),
('giant_wasp','Giant Wasp','beast','med',1,2,12,13,10,14,10,1,10,3,'[]','The sound arrives first and that is all the warning there is.'),
('giant_wolf_spider','Giant Wolf Spider','beast','med',1,2,13,11,12,16,13,3,12,4,'[]','Does not spin anything. Runs you down instead.'),
('giant_hyena','Giant Hyena','beast','lg',2,2,12,45,16,14,14,2,12,7,'[]','Waits for something else to do the work and then takes the work.'),
('giant_octopus','Giant Octopus','beast','lg',2,2,11,52,17,13,13,4,10,4,'[]','Eight arms and a grip that was not negotiable from the start.'),
('swarm_of_insects','Swarm of Insects','beast','med',2,2,12,22,3,13,10,1,7,1,'[{"source":"Swarm of Insects","target":"resist.bludgeoning"},{"source":"Swarm of Insects","target":"resist.piercing"},{"source":"Swarm of Insects","target":"resist.slashing"}]','There is nothing to hit. That is the whole of the problem.'),
('polar_bear','Polar Bear','beast','lg',4,2,12,42,20,10,16,2,13,7,'[]','White, enormous, and the only thing out here that is hunting on purpose.'),
('saber_toothed_tiger','Saber-Toothed Tiger','beast','lg',4,2,12,52,18,14,15,3,12,8,'[]','Built entirely around one bite and it only needs the one.'),
('killer_whale','Killer Whale','beast','huge',4,2,12,90,19,10,13,3,12,7,'[]','Clever, coordinated, and playing a longer game than you are.'),
('elephant','Elephant','beast','huge',5,2,12,76,22,9,17,3,11,6,'[]','Will go around you if you let it and through you if you do not.'),
('giant_shark','Giant Shark','beast','huge',6,2,13,126,23,11,21,1,10,5,'[]','Twenty feet of appetite that has never once hesitated.'),
('giant_crocodile','Giant Crocodile','beast','huge',6,2,14,85,21,9,17,2,10,7,'[]','The log was not a log and the river is now a bad idea.'),
('mammoth','Mammoth','beast','huge',8,2,13,126,24,9,21,3,11,6,'[]','An elephant with worse weather and a worse temper.'),
('giant_ape','Giant Ape','beast','huge',10,3,12,157,23,14,18,7,12,7,'[]','Throws what is available, and almost everything is available.')

) as v(k,n,ct,sz,lvl,pb,ac,hp,s,d,c,i,w,ch,g,nt)
on conflict (key) where game_id is null do update set
  name=excluded.name, creature_type=excluded.creature_type, size=excluded.size,
  level=excluded.level, prof_bonus=excluded.prof_bonus, ac=excluded.ac,
  hp_max=excluded.hp_max, str=excluded.str, dex=excluded.dex, con=excluded.con,
  intl=excluded.intl, wis=excluded.wis, cha=excluded.cha,
  grants=excluded.grants, notes=excluded.notes,
  weapon_profs=excluded.weapon_profs, armor_profs=excluded.armor_profs;

-- ---------------------------------------------------------------------
-- AND WHAT THEY FIGHT WITH
-- ---------------------------------------------------------------------
--
-- 135 is why this is not optional: a statblock with no kit is a creature
-- a DM can enrol and then do nothing with, which is what Goblin Fighter
-- was. Every one of the 71 above gets at least one weapon.
--
-- ON CONFLICT rather than 131's scoped delete - `npc_items_global_idx`
-- is unique on (npc_key, item_key) for global rows, which 131's header
-- said did not exist. 136 found that and 137 used it.
--
-- `proficient_override` IS OMITTED and the column default supplies true,
-- which is 137: a creature is proficient with its own kit. The one row
-- in this schema that was ever null was 022's goblin handaxe, and it
-- took three years to notice.

insert into npc_items (npc_key,item_key,quantity,equipped)
values
-- dragons: four limbs each
('young_black_dragon','bite',1,true),('young_black_dragon','claws',1,true),('young_black_dragon','tail',1,true),('young_black_dragon','wing',1,true),
('young_blue_dragon','bite',1,true),('young_blue_dragon','claws',1,true),('young_blue_dragon','tail',1,true),('young_blue_dragon','wing',1,true),
('young_green_dragon','bite',1,true),('young_green_dragon','claws',1,true),('young_green_dragon','tail',1,true),('young_green_dragon','wing',1,true),
('young_red_dragon','bite',1,true),('young_red_dragon','claws',1,true),('young_red_dragon','tail',1,true),('young_red_dragon','wing',1,true),
('young_white_dragon','bite',1,true),('young_white_dragon','claws',1,true),('young_white_dragon','tail',1,true),('young_white_dragon','wing',1,true),
('adult_black_dragon','bite',1,true),('adult_black_dragon','claws',1,true),('adult_black_dragon','tail',1,true),('adult_black_dragon','wing',1,true),
('adult_blue_dragon','bite',1,true),('adult_blue_dragon','claws',1,true),('adult_blue_dragon','tail',1,true),('adult_blue_dragon','wing',1,true),
('adult_green_dragon','bite',1,true),('adult_green_dragon','claws',1,true),('adult_green_dragon','tail',1,true),('adult_green_dragon','wing',1,true),
('adult_red_dragon','bite',1,true),('adult_red_dragon','claws',1,true),('adult_red_dragon','tail',1,true),('adult_red_dragon','wing',1,true),
('adult_white_dragon','bite',1,true),('adult_white_dragon','claws',1,true),('adult_white_dragon','tail',1,true),('adult_white_dragon','wing',1,true),

-- the classics
('lich','slam',1,true),
('vampire','bite',1,true),('vampire','claws',1,true),
('vampire_spawn','bite',1,true),('vampire_spawn','claws',1,true),
('ghost','slam',1,true),
-- 136's organ, reused: a banshee's wail is the same shape of thing.
('banshee','shriek',1,true),('banshee','slam',1,true),
('medusa','bite',1,true),('medusa','shortsword',1,true),('medusa','shortbow',1,false),
('manticore','bite',1,true),('manticore','claws',1,true),('manticore','spines',1,true),
('chimera','bite',1,true),('chimera','gore',1,true),('chimera','claws',1,true),
('hydra','bite',1,true),
('treant','slam',1,true),
('unicorn','gore',1,true),('unicorn','hooves',1,true),
('roc','beak',1,true),('roc','talons',1,true),
('mimic','pseudopod',1,true),('mimic','bite',1,true),
('oni','glaive',1,true),('oni','claws',1,false),

-- fiends
('balor','longsword',1,true),('balor','whip',1,true),
('marilith','longsword',1,true),('marilith','constrict',1,true),
('vrock','beak',1,true),('vrock','talons',1,true),
('hezrou','bite',1,true),('hezrou','claws',1,true),
('glabrezu','claws',1,true),('glabrezu','fist',1,true),
('nalfeshnee','bite',1,true),('nalfeshnee','claws',1,true),
('lemure','fist',1,true),
('bone_devil','claws',1,true),('bone_devil','sting',1,true),
('erinyes','longsword',1,true),('erinyes','longbow',1,false),
('horned_devil','military_fork',1,true),('horned_devil','tail',1,true),('horned_devil','gore',1,true),
('pit_fiend','bite',1,true),('pit_fiend','claws',1,true),('pit_fiend','mace',1,true),('pit_fiend','tail',1,true),

-- constructs and giants
('flesh_golem','slam',1,true),
('clay_golem','slam',1,true),
('stone_golem','slam',1,true),
('iron_golem','slam',1,true),('iron_golem','greatsword',1,false),
('shield_guardian','fist',1,true),
('cloud_giant','morningstar',1,true),
('storm_giant','greatsword',1,true),

-- people
('commoner','club',1,true),
('noble','rapier',1,true),('noble','breastplate',1,true),
('bandit_captain','scimitar',1,true),('bandit_captain','dagger',1,false),('bandit_captain','studded_leather',1,true),
('assassin','shortsword',1,true),('assassin','crossbow_light',1,false),('assassin','studded_leather',1,true),
('gladiator','spear',1,true),('gladiator','shield',1,true),('gladiator','studded_leather',1,true),
('drow','shortsword',1,true),('drow','crossbow_hand',1,false),('drow','chain_shirt',1,true),
('duergar','war_pick',1,true),('duergar','javelin',1,false),('duergar','scale_mail',1,true),('duergar','shield',1,true),
('sahuagin','bite',1,true),('sahuagin','claws',1,true),('sahuagin','spear',1,false),
('yuan_ti_pureblood','scimitar',1,true),('yuan_ti_pureblood','shortbow',1,false),

-- lycanthropes: the beast half is what is equipped
('wererat','bite',1,true),('wererat','shortsword',1,false),('wererat','crossbow_hand',1,false),
('werewolf','bite',1,true),('werewolf','claws',1,true),('werewolf','spear',1,false),
('wereboar','tusks',1,true),('wereboar','maul',1,false),
('weretiger','bite',1,true),('weretiger','claws',1,true),('weretiger','longbow',1,false),
('werebear','bite',1,true),('werebear','claws',1,true),('werebear','greataxe',1,false),

-- beasts
('mastiff','bite',1,true),
('riding_horse','hooves',1,true),
('giant_wasp','sting',1,true),
('giant_wolf_spider','bite',1,true),
('giant_hyena','bite',1,true),
('giant_octopus','tendrils',1,true),
('swarm_of_insects','bite',1,true),
('polar_bear','bite',1,true),('polar_bear','claws',1,true),
('saber_toothed_tiger','bite',1,true),('saber_toothed_tiger','claws',1,true),
('killer_whale','bite',1,true),
('elephant','gore',1,true),('elephant','hooves',1,true),
('giant_shark','bite',1,true),
('giant_crocodile','bite',1,true),('giant_crocodile','tail',1,true),
('mammoth','gore',1,true),('mammoth','hooves',1,true),
('giant_ape','fist',1,true)

on conflict (npc_key, item_key) where game_id is null do update set
  quantity = excluded.quantity, equipped = excluded.equipped;
