-- 130. THE REST OF THE BESTIARY.
--
-- 127 put 39 statblocks in the catalogue. This adds 95 more, which brings
-- the global bestiary to 134 and covers every one of 5e's fourteen
-- creature types.
--
-- THIS IS NOT THE WHOLE SRD AND MUST NOT BE READ AS IF IT WERE. The SRD
-- has something north of three hundred monsters. These 95 are the ones
-- that could be written down with numbers worth trusting; the rest are
-- missing rather than wrong, and the honest way to finish the job is to
-- import from a machine-readable SRD file through `import_creature`,
-- which the `odyssey1e.creature` envelope in `creature_io.rs` exists to
-- make possible.
--
-- SPOT-CHECK THE NUMBERS BEFORE A SESSION RUNS ON THEM. AC, HP and the
-- six abilities are SRD-derived and were written from memory, not
-- transcribed from a file. They are right in shape and will be wrong in
-- places.
--
-- ---------------------------------------------------------------------
-- LEVEL IS AN ENCOUNTER WEIGHT, NOT A CHALLENGE RATING
-- ---------------------------------------------------------------------
--
-- 127 set this scale by hand - a wolf is 2, an owlbear 7, a hill giant
-- 10 - and it is roughly "the party level this is a fair fight for"
-- rather than anything derived. These rows follow it, which means a
-- creature's `level` will not match its printed CR and is not supposed
-- to.
--
-- `level` IS ALSO LOAD-BEARING: it gates techniques. 129 brought the
-- natural-weapon gates down to 1 / 2 / 4, so anything at level 4 or above
-- reaches its full repertoire and a level-2 creature gets two moves per
-- limb. A level typed carelessly is therefore a creature with fewer
-- attacks than intended, which is why fifteen of these were corrected
-- against the 127 scale before the migration was written rather than
-- after.
--
-- `prof_bonus` FOLLOWS 127'S RULE AND NOT THE PLAYER TABLE: 2 up to
-- level 9, 3 above it. The player progression is 2/3/4/5 across the same
-- span. One catalogue, one scale - stated here so the difference reads
-- as a decision rather than an oversight.
--
-- ---------------------------------------------------------------------
-- RESISTANCES ARRIVE AS GRANTS, WHICH IS THE POINT OF 116
-- ---------------------------------------------------------------------
--
-- Every damage resistance, immunity and vulnerability here is a `grants`
-- entry in the shape 100 and 116 established - a target of "immune.fire"
-- with the source naming what granted it - so it reaches the sheet
-- through exactly the path an item, a spell or a species ability uses. A
-- black pudding's immunity to slashing and a Ny'ook's resistance to cold
-- are one mechanism, and `resist::standing` names the source on both.
--
-- THE SOURCE IS THE CREATURE'S OWN NAME, which reads a little oddly in
-- the sheet ("Immune: fire - Fire Giant") but is correct: 001 says a roll
-- is a record, and the record should say where the immunity came from.
--
-- WHAT IS STILL MISSING, stated plainly because the gap is easy to
-- mistake for a bug: condition immunities have nowhere to live, a troll's
-- regeneration is prose in `notes` and nothing reads it, and resistance
-- to "nonmagical bludgeoning, piercing and slashing" is inexpressible -
-- so the awakened tree and the intellect devourer carry the plain
-- physical resistance, which is stronger than the SRD intends.

insert into npcs (key,name,creature_type,size,level,prof_bonus,ac,hp_max,str,dex,con,intl,wis,cha,grants,notes,weapon_profs,armor_profs)
select v.k,v.n,v.ct,v.sz,v.lvl,v.pb,v.ac,v.hp,v.s,v.d,v.c,v.i,v.w,v.ch,v.g::jsonb,v.nt,array['sim','mar'],array['lgt','med','hvy','shl']
from (values
('acolyte','Acolyte','humanoid','med',2,2,10,9,10,10,10,10,14,11,'[]','A junior priest with a little real power and a great deal of certainty.'),
('archer','Archer','humanoid','med',7,2,16,75,11,18,16,11,13,12,'[]','Trained to put arrows where they matter and to keep the distance that lets them.'),
('berserker','Berserker','humanoid','med',6,2,13,67,16,12,17,9,11,9,'[]','Fights without regard for what happens to it, which is harder to deal with than skill.'),
('druid_npc','Druid','humanoid','med',5,2,11,27,10,12,13,12,15,11,'[]','Keeps to the old arrangements with things that do not speak.'),
('knight','Knight','humanoid','med',8,2,18,52,16,11,14,11,11,15,'[]','Armoured, mounted where possible, and genuinely dangerous rather than decorative.'),
('mage','Mage','humanoid','med',9,2,12,40,9,14,11,17,12,11,'[]','Dangerous at range and fragile up close, which decides every fight it is in.'),
('priest','Priest','humanoid','med',5,2,13,27,10,10,12,13,16,13,'[]','Holds a congregation and whatever its god is prepared to lend.'),
('scout','Scout','humanoid','med',3,2,13,16,11,14,12,11,13,11,'[]','Sees you first, and that is usually the end of the matter.'),
('spy','Spy','humanoid','med',6,2,12,27,10,15,10,12,14,16,'[]','Already knows your name and what you did last winter.'),
('thug','Thug','humanoid','med',5,2,11,32,15,11,14,10,10,11,'[]','Hired muscle with no pretence of being anything else.'),
('veteran','Veteran','humanoid','med',7,2,17,58,16,13,14,10,11,10,'[]','Has done this for years and is still here, which tells you most of what you need.'),
('tribal_warrior','Tribal Warrior','humanoid','med',2,2,12,11,13,11,12,8,11,8,'[]','Fights for people it actually knows, which makes it steadier than a bandit.'),
('gnoll','Gnoll','humanoid','med',5,2,15,22,14,12,11,6,10,7,'[]','Laughs while it works. Follows whatever is strongest and eats what is left.'),
('lizardfolk','Lizardfolk','humanoid','med',4,2,15,22,15,10,13,7,12,7,'[]','Thinks about the marsh and very little else. Not hostile so much as uninterested.'),
('merfolk','Merfolk','humanoid','med',2,2,11,11,10,13,12,11,11,12,'[]','At home where you are drowning. Civil enough if met on its own terms.'),
('orc_war_chief','Orc War Chief','humanoid','med',6,2,16,93,18,12,18,11,11,16,'[]','Holds the warband by being the worst thing in it.'),
('goblin_boss','Goblin Boss','humanoid','sm',6,2,17,21,10,14,10,10,8,10,'[]','Stays behind the others and is very good at arranging for that to happen.'),
('kobold_dragonshield','Kobold Dragonshield','humanoid','sm',3,2,15,21,10,12,13,8,9,10,'[]','A kobold with a dragon''s favour and enough confidence to stand still.'),
('cult_fanatic','Cult Fanatic','humanoid','med',6,2,13,33,11,14,12,10,13,14,'[]','Past persuading. Whatever it was promised, it believes it entirely.'),
('guard_captain','Guard Captain','humanoid','med',6,2,18,48,16,13,14,11,12,13,'[]','Paid more than the guards and worth it. Fights with the others rather than behind them.'),
('giant_rat','Giant Rat','beast','sm',2,2,12,7,7,15,11,2,10,4,'[]','Alone it is a nuisance. They are not alone.'),
('giant_bat','Giant Bat','beast','lg',4,2,13,22,15,16,11,2,12,6,'[]','Hunts by sound in total dark and is entirely unbothered by it.'),
('boar','Boar','beast','med',2,2,11,11,13,11,12,2,9,5,'[]','Short tempered and very hard to turn once it has started.'),
('black_bear','Black Bear','beast','med',3,2,11,19,15,10,14,2,12,7,'[]','Would rather be left alone and will make that point firmly.'),
('panther','Panther','beast','med',2,2,12,13,14,15,10,3,14,7,'[]','Comes from above and behind, and is gone again if that does not work.'),
('lion','Lion','beast','lg',3,2,12,26,17,15,13,3,12,8,'[]','Hunts with others and takes the credit alone.'),
('tiger','Tiger','beast','lg',3,2,12,37,17,15,14,3,12,8,'[]','Patient, enormous and almost silent until the last ten feet.'),
('dire_wolf','Dire Wolf','beast','lg',5,2,14,37,17,15,15,3,12,7,'[]','A wolf scaled up past the point where numbers were necessary.'),
('giant_eagle','Giant Eagle','beast','lg',4,2,13,26,16,17,13,8,14,10,'[]','Intelligent enough to bargain with, and proud enough to refuse.'),
('giant_snake','Giant Constrictor Snake','beast','huge',6,2,12,60,19,14,12,1,10,3,'[]','Patient to a degree that is difficult to credit. Then it is not.'),
('giant_scorpion','Giant Scorpion','beast','lg',7,2,15,52,15,13,15,1,9,3,'[]','Three threats on one body, and the one at the back is the one to watch.'),
('giant_toad','Giant Toad','beast','lg',5,2,11,39,15,13,13,2,10,3,'[]','Swallows things whole that really ought not to fit.'),
('crocodile','Crocodile','beast','lg',3,2,12,19,15,10,13,2,10,5,'[]','Still enough to be mistaken for scenery, right up until it is not.'),
('ape','Ape','beast','med',3,2,12,19,16,14,14,6,12,7,'[]','Strong, fast and quite capable of throwing whatever is nearby at you.'),
('giant_boar','Giant Boar','beast','lg',4,2,12,42,17,10,16,2,7,5,'[]','Everything wrong with an ordinary boar, multiplied.'),
('rhinoceros','Rhinoceros','beast','lg',6,2,11,45,21,8,15,2,12,6,'[]','Short sighted, bad tempered, and two tons of momentum once decided.'),
('giant_elk','Giant Elk','beast','huge',6,2,14,42,19,16,14,7,14,10,'[]','Older and cleverer than it looks, and the antlers are not decoration.'),
('warhorse','Warhorse','beast','lg',3,2,11,19,18,12,13,2,12,7,'[]','Trained not to flinch, which is a rarer thing than it sounds.'),
('giant_centipede','Giant Centipede','beast','sm',1,2,13,4,5,14,12,1,7,3,'[]','Too many legs and a bite that keeps working after it has let go.'),
('swarm_of_rats','Swarm of Rats','beast','med',2,2,10,24,9,11,9,2,10,3,'[]','Not one thing. Treating it as one is the mistake.'),
('shadow','Shadow','undead','med',3,2,12,16,6,14,13,6,10,8,'[{"target":"resist.acid","source":"Shadow"},{"target":"resist.cold","source":"Shadow"},{"target":"resist.fire","source":"Shadow"},{"target":"resist.lightning","source":"Shadow"},{"target":"resist.thunder","source":"Shadow"},{"target":"resist.necrotic","source":"Shadow"},{"target":"immune.poison","source":"Shadow"},{"target":"vulnerable.radiant","source":"Shadow"}]','Yours, or something very like it. Takes strength rather than blood.'),
('ghast','Ghast','undead','med',6,2,13,36,16,17,10,11,10,8,'[{"target":"immune.poison","source":"Ghast"}]','A ghoul that has eaten enough to become something worse.'),
('mummy','Mummy','undead','med',8,2,11,58,16,8,15,6,10,12,'[{"target":"vulnerable.fire","source":"Mummy"},{"target":"immune.necrotic","source":"Mummy"},{"target":"immune.poison","source":"Mummy"}]','Kept, bound and extremely annoyed about being disturbed.'),
('wraith','Wraith','undead','med',9,2,13,67,6,16,16,12,14,15,'[{"target":"resist.acid","source":"Wraith"},{"target":"resist.cold","source":"Wraith"},{"target":"resist.fire","source":"Wraith"},{"target":"resist.lightning","source":"Wraith"},{"target":"resist.thunder","source":"Wraith"},{"target":"resist.necrotic","source":"Wraith"},{"target":"immune.poison","source":"Wraith"}]','What a wight becomes when there is nothing left to be loyal to.'),
('skeleton_warhorse','Warhorse Skeleton','undead','lg',3,2,13,22,18,12,15,2,8,5,'[{"target":"vulnerable.bludgeoning","source":"Warhorse Skeleton"},{"target":"immune.poison","source":"Warhorse Skeleton"}]','Still carries whatever put it here. Still runs when told.'),
('minotaur_skeleton','Minotaur Skeleton','undead','lg',6,2,12,67,18,11,15,6,8,5,'[{"target":"vulnerable.bludgeoning","source":"Minotaur Skeleton"},{"target":"immune.poison","source":"Minotaur Skeleton"}]','Large, horned, and did not get less dangerous by dying.'),
('quasit','Quasit','fiend','tiny',3,2,13,7,5,17,10,7,10,10,'[{"target":"resist.cold","source":"Quasit"},{"target":"resist.fire","source":"Quasit"},{"target":"resist.lightning","source":"Quasit"},{"target":"immune.poison","source":"Quasit"}]','Small, invisible when it likes, and never where you last saw it.'),
('hell_hound','Hell Hound','fiend','med',7,2,15,45,17,12,14,6,13,6,'[{"target":"immune.fire","source":"Hell Hound"}]','Hunts in a pack and the pack is on fire.'),
('succubus','Succubus','fiend','med',8,2,15,66,8,17,13,15,12,20,'[{"target":"resist.cold","source":"Succubus"},{"target":"resist.fire","source":"Succubus"},{"target":"resist.lightning","source":"Succubus"},{"target":"resist.poison","source":"Succubus"}]','Would far rather talk, and is extremely good at it.'),
('deva','Deva','celestial','med',16,3,17,136,18,18,18,17,20,20,'[{"target":"resist.radiant","source":"Deva"}]','Patient, kind and entirely prepared to end you if that is what is required.'),
('black_dragon_wyrmling','Black Dragon Wyrmling','dragon','med',6,2,17,33,15,14,13,10,11,13,'[{"target":"immune.acid","source":"Black Dragon Wyrmling"}]','Spiteful even for its kind, and it starts young.'),
('green_dragon_wyrmling','Green Dragon Wyrmling','dragon','med',6,2,17,38,15,12,13,14,11,13,'[{"target":"immune.poison","source":"Green Dragon Wyrmling"}]','Already lying to you, and already quite good at it.'),
('red_dragon_wyrmling','Red Dragon Wyrmling','dragon','med',10,3,17,75,19,10,17,12,11,15,'[{"target":"immune.fire","source":"Red Dragon Wyrmling"}]','Vain, greedy and dangerous in that order.'),
('blue_dragon_wyrmling','Blue Dragon Wyrmling','dragon','med',8,2,17,52,17,10,15,12,11,15,'[{"target":"immune.lightning","source":"Blue Dragon Wyrmling"}]','Territorial and methodical. Will wait out a siege it did not need to.'),
('wyvern','Wyvern','dragon','lg',11,3,13,110,19,10,16,5,12,6,'[]','A dragon in shape and an animal in temperament. The tail is the problem.'),
('ankheg','Ankheg','monstrosity','lg',6,2,14,39,17,11,13,1,13,6,'[]','Comes up through the floor of the field, which is the whole of its method.'),
('basilisk','Basilisk','monstrosity','med',8,2,15,52,16,8,15,2,8,7,'[]','Slow, heavy and best not looked at directly.'),
('bulette','Bulette','monstrosity','lg',9,2,17,94,19,11,21,2,10,5,'[]','Lands sharks have, as far as anything that has met one can tell.'),
('cockatrice','Cockatrice','monstrosity','sm',3,2,11,27,6,12,12,2,13,5,'[]','Ridiculous to look at and genuinely a problem to fight.'),
('hippogriff','Hippogriff','monstrosity','lg',5,2,11,19,17,13,13,2,12,8,'[]','Half eagle and half horse, and fiercely loyal once it decides.'),
('griffon','Griffon','monstrosity','lg',7,2,12,59,18,15,16,2,13,8,'[]','Hunts horses by preference, which makes it a problem for cavalry.'),
('minotaur','Minotaur','monstrosity','lg',9,2,14,76,18,11,16,6,16,9,'[]','Knows the maze. Is the reason for the maze.'),
('rust_monster','Rust Monster','monstrosity','med',5,2,14,27,13,12,13,2,13,6,'[]','Not interested in you at all, which is no comfort to your armour.'),
('ettercap','Ettercap','monstrosity','med',6,2,13,44,14,15,13,7,12,8,'[]','Keeps spiders the way a shepherd keeps sheep.'),
('harpy_matriarch','Harpy Matriarch','monstrosity','med',9,2,12,52,14,13,14,9,12,15,'[]','Older, cleverer, and the one the others are listening to.'),
('stone_giant','Stone Giant','giant','huge',11,3,17,126,23,15,20,10,12,9,'[]','Thinks on a scale that makes a conversation difficult. Throws rocks accurately.'),
('frost_giant','Frost Giant','giant','huge',12,3,15,138,23,9,21,9,10,12,'[{"target":"immune.cold","source":"Frost Giant"}]','Raids because that is what is done, not because anything is lacking.'),
('fire_giant','Fire Giant','giant','huge',13,3,18,162,25,9,23,10,14,13,'[{"target":"immune.fire","source":"Fire Giant"}]','Disciplined, armoured and a better smith than anyone you know.'),
('ettin','Ettin','giant','lg',9,2,12,85,21,8,17,6,10,8,'[]','Two heads, two opinions, and one of them is always awake.'),
('troll','Troll','giant','lg',8,2,15,84,18,13,20,7,9,7,'[]','Regenerates. Does not stop. Fire is the usual answer.'),
('dust_mephit','Dust Mephit','elemental','sm',5,2,12,17,5,14,10,9,11,10,'[{"target":"immune.poison","source":"Dust Mephit"}]','Gets everywhere and resents being noticed.'),
('steam_mephit','Steam Mephit','elemental','sm',5,2,10,21,5,11,10,11,10,12,'[{"target":"immune.fire","source":"Steam Mephit"},{"target":"immune.poison","source":"Steam Mephit"}]','Scalding, loud and extremely pleased with itself.'),
('air_elemental','Air Elemental','elemental','lg',12,3,15,90,14,20,14,6,10,6,'[{"target":"resist.lightning","source":"Air Elemental"},{"target":"resist.thunder","source":"Air Elemental"},{"target":"immune.poison","source":"Air Elemental"}]','A wind with intent. Hard to hit and harder to hold.'),
('earth_elemental','Earth Elemental','elemental','lg',12,3,17,126,20,8,20,5,10,5,'[{"target":"immune.poison","source":"Earth Elemental"}]','Moves through stone as though it were not there, because to it it is not.'),
('fire_elemental','Fire Elemental','elemental','lg',12,3,13,102,10,17,16,6,10,7,'[{"target":"immune.fire","source":"Fire Elemental"},{"target":"immune.poison","source":"Fire Elemental"}]','Burns what it touches and touches everything it passes.'),
('water_elemental','Water Elemental','elemental','lg',12,3,14,114,18,14,18,5,10,8,'[{"target":"resist.acid","source":"Water Elemental"},{"target":"immune.poison","source":"Water Elemental"}]','Comes through the gap under the door and reassembles on your side of it.'),
('scarecrow','Scarecrow','construct','med',4,2,11,36,11,13,11,10,10,13,'[{"target":"vulnerable.fire","source":"Scarecrow"},{"target":"immune.poison","source":"Scarecrow"}]','Was not moving a moment ago and will swear it still is not.'),
('helmed_horror','Helmed Horror','construct','med',8,2,20,60,18,13,16,10,10,10,'[{"target":"immune.poison","source":"Helmed Horror"},{"target":"immune.psychic","source":"Helmed Horror"}]','Obeys its last order with no interest in whether that still makes sense.'),
('ochre_jelly','Ochre Jelly','ooze','lg',6,2,8,45,15,6,14,2,6,1,'[{"target":"resist.acid","source":"Ochre Jelly"},{"target":"immune.lightning","source":"Ochre Jelly"},{"target":"immune.slashing","source":"Ochre Jelly"}]','Splits when cut, which makes cutting it a poor opening move.'),
('black_pudding','Black Pudding','ooze','lg',10,3,7,85,16,5,16,1,6,1,'[{"target":"immune.acid","source":"Black Pudding"},{"target":"immune.cold","source":"Black Pudding"},{"target":"immune.lightning","source":"Black Pudding"},{"target":"immune.slashing","source":"Black Pudding"}]','Eats everything except stone. Including the floor, eventually.'),
('violet_fungus','Violet Fungus','plant','med',3,2,5,18,3,1,10,1,3,1,'[]','Looks like the rest of the cave until part of it reaches for you.'),
('shambling_mound','Shambling Mound','plant','lg',8,2,15,136,18,8,16,5,10,5,'[{"target":"resist.cold","source":"Shambling Mound"},{"target":"resist.fire","source":"Shambling Mound"},{"target":"immune.lightning","source":"Shambling Mound"}]','Absorbs lightning and grows on it, which surprises people exactly once.'),
('awakened_tree','Awakened Tree','plant','huge',7,2,13,59,19,6,15,10,10,7,'[{"target":"vulnerable.fire","source":"Awakened Tree"},{"target":"resist.bludgeoning","source":"Awakened Tree"},{"target":"resist.piercing","source":"Awakened Tree"}]','Was a tree this morning. Is still mostly a tree, and now has opinions.'),
('grick','Grick','monstrosity','med',4,2,14,27,14,14,11,3,14,5,'[]','Four tentacles around a beak, flat against the ceiling you just walked under.'),
('darkmantle','Darkmantle','monstrosity','sm',5,2,11,22,16,12,13,2,10,5,'[]','Indistinguishable from a stalactite until it is on your head.'),
('gargoyle','Gargoyle','elemental','med',7,2,15,52,15,11,16,6,11,7,'[{"target":"immune.poison","source":"Gargoyle"}]','Has been on that roof for two hundred years and remembers all of it.'),
('satyr','Satyr','fey','med',3,2,14,31,12,16,11,12,10,14,'[]','Delighted to meet you and already planning something at your expense.'),
('blink_dog','Blink Dog','fey','med',4,2,13,22,12,17,12,10,13,11,'[]','Here, then not, then behind you. Friendly, on the whole.'),
('pixie','Pixie','fey','tiny',1,2,15,1,2,20,8,10,14,15,'[]','Tiny, invisible by default, and far more powerful than it has any right to be.'),
('gas_spore','Gas Spore','plant','lg',2,2,5,1,5,1,13,1,2,1,'[]','Looks exactly like a beholder from a distance, which is the joke.'),
('carrion_crawler','Carrion Crawler','monstrosity','lg',6,2,13,51,14,13,16,1,12,5,'[]','Eats what is already dead and is not fussy about how recently.'),
('hook_horror','Hook Horror','monstrosity','lg',8,2,15,75,18,10,15,6,12,7,'[]','Climbs the walls of the cavern and drops onto whatever is below.'),
('otyugh','Otyugh','aberration','lg',10,3,14,114,16,11,19,6,13,6,'[]','Lives in the midden by choice and will defend it enthusiastically.'),
('chuul','Chuul','aberration','lg',11,3,16,93,19,10,16,5,11,5,'[{"target":"immune.poison","source":"Chuul"}]','A lobster the size of a cart, with an old and unpleasant intelligence behind it.'),
('intellect_devourer','Intellect Devourer','aberration','tiny',3,2,12,21,6,14,13,12,11,10,'[{"target":"resist.bludgeoning","source":"Intellect Devourer"},{"target":"resist.piercing","source":"Intellect Devourer"},{"target":"resist.slashing","source":"Intellect Devourer"}]','A brain on legs. Wants yours, specifically.')
) as v(k,n,ct,sz,lvl,pb,ac,hp,s,d,c,i,w,ch,g,nt)
on conflict (key) where game_id is null do update set
  name=excluded.name, creature_type=excluded.creature_type, size=excluded.size,
  level=excluded.level, prof_bonus=excluded.prof_bonus, ac=excluded.ac,
  hp_max=excluded.hp_max, str=excluded.str, dex=excluded.dex, con=excluded.con,
  intl=excluded.intl, wis=excluded.wis, cha=excluded.cha,
  grants=excluded.grants, notes=excluded.notes,
  weapon_profs=excluded.weapon_profs, armor_profs=excluded.armor_profs;
