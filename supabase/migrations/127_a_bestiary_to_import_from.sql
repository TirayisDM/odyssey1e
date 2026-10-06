-- 127. A BESTIARY TO IMPORT FROM.
--
-- The Creatures tab shipped in 123 with three statblocks to choose from
-- and all three were goblins. The mechanism was done and there was
-- nothing to do it with.
--
-- THIRTY-SIX CREATURES, covering all fourteen of 5e's types - which is
-- deliberate rather than tidy: 122 put `creature_type` in because
-- fifteen spells are written against it, and a reference where every
-- entry is a humanoid would never exercise any of them. Hold Person now
-- has humanoids to refuse and non-humanoids to fail against; Cure
-- Wounds has constructs and undead to do nothing for.
--
-- STATS ARE THE SRD'S, reduced to what this schema holds: armour class,
-- hit points, the six scores, size and a level taken from the hit dice
-- count. What the SRD has and this does not - senses, speeds, saving
-- throw bonuses, legendary actions, spellcasting - is simply absent
-- rather than approximated.
--
-- ---------------------------------------------------------------------
-- RESISTANCE, AT LAST WITH SOMETHING TO SIT ON
-- ---------------------------------------------------------------------
--
-- 116 built resistance and 121 gave statblocks somewhere to state it,
-- and until now not one row did. Fourteen of these do:
--
--   Skeleton        vulnerable to bludgeoning, immune to poison
--   Ice Mephit      immune cold and poison, vulnerable fire AND bludgeoning
--   Magma Mephit    immune fire and poison, vulnerable cold
--   Specter         resists five energy types, immune to poison
--   Animated Armor  immune to poison and psychic
--
-- NO "NONMAGICAL" RESISTANCES, and that is a gap rather than a choice.
-- The grant vocabulary cannot say "bludgeoning, piercing and slashing
-- FROM NONMAGICAL ATTACKS", which is 5e's commonest line - so the Wight
-- and the Specter carry only their unconditional resistances. Giving
-- them blanket physical resistance would make both considerably tougher
-- than the book intends, and overstating a monster is worse than
-- understating one.
--
-- ---------------------------------------------------------------------
-- THEY FIGHT WITH TECHNIQUES
-- ---------------------------------------------------------------------
--
-- 126 gave natural attacks the same treatment weapons have had since
-- 050: Bite, Claws, Slam, Gore, Talons, Sting, Tendrils and Hooves, each
-- with three named moves gated by level. A creature's kit is equipment
-- like anybody's, so a wolf reaches Snap and an owlbear reaches Crush
-- the Throat without a single rule written for monsters specifically.
--
-- `proficient_override` IS TRUE ON EVERY KIT ROW. A bear is proficient
-- with its own teeth, and an ogre with the club it has carried all its
-- life; leaving that to weapon-class matching would have half the
-- bestiary swinging at a penalty for no reason anybody could see.
--
-- EXISTING ROWS ARE UPDATED, NOT DUPLICATED, and the kit is replaced
-- rather than added to - so running this twice leaves one goblin with
-- one axe, not two with four.

insert into npcs
  (key, name, creature_type, size, level, prof_bonus, ac, hp_max,
   str, dex, con, intl, wis, cha, grants, notes, weapon_profs, armor_profs)
values
('bandit','Bandit','humanoid','med',2,2,12,11,11,12,12,10,10,10,'[]'::jsonb,'Desperate rather than wicked, mostly. Fights in numbers and runs when the numbers turn.',array['sim','mar'],array['lgt','med','hvy','shl']),
('guard','Guard','humanoid','med',2,2,16,11,13,12,12,10,11,10,'[]'::jsonb,'Paid to stand somewhere and discourage people. Will fight, but would rather you moved along.',array['sim','mar'],array['lgt','med','hvy','shl']),
('orc','Orc','humanoid','med',2,2,13,15,16,12,16,7,11,10,'[]'::jsonb,'Built to close the distance and swing something heavy once it arrives.',array['sim','mar'],array['lgt','med','hvy','shl']),
('hobgoblin','Hobgoblin','humanoid','med',2,2,18,11,13,12,12,10,10,9,'[]'::jsonb,'Disciplined where a goblin is not. Fights in a line and holds it.',array['sim','mar'],array['lgt','med','hvy','shl']),
('kobold','Kobold','humanoid','sm',2,2,12,5,7,15,9,8,7,8,'[]'::jsonb,'Small, numerous and fond of traps. Brave only in a crowd, and sensible otherwise.',array['sim','mar'],array['lgt','med','hvy','shl']),
('cultist','Cultist','humanoid','med',2,2,12,9,11,12,10,10,11,10,'[]'::jsonb,'Someone ordinary who has been promised something. The robes are the least of it.',array['sim','mar'],array['lgt','med','hvy','shl']),
('bugbear','Bugbear','humanoid','med',5,2,16,27,15,14,13,8,11,9,'[]'::jsonb,'Large, quiet, and much faster off the mark than its size suggests. Opens from surprise where it can.',array['sim','mar'],array['lgt','med','hvy','shl']),
('skeleton','Skeleton','undead','med',2,2,13,13,10,14,15,6,8,5,'[{"target":"vulnerable.bludgeoning","source":"Skeleton"},{"target":"immune.poison","source":"Skeleton"}]'::jsonb,'Bones held together by whatever raised them. A hammer does what a sword cannot.',array['sim','mar'],array['lgt','med','hvy','shl']),
('zombie','Zombie','undead','med',3,2,8,22,13,6,16,3,6,5,'[{"target":"immune.poison","source":"Zombie"}]'::jsonb,'Slow, stupid and very hard to put down. Keeps coming after it should have stopped.',array['sim','mar'],array['lgt','med','hvy','shl']),
('ghoul','Ghoul','undead','med',5,2,12,22,13,15,10,7,10,6,'[{"target":"immune.poison","source":"Ghoul"}]'::jsonb,'Hungry, and quick with it. The claws are what you remember.',array['sim','mar'],array['lgt','med','hvy','shl']),
('wight','Wight','undead','med',6,2,14,45,15,14,16,10,13,15,'[{"target":"resist.necrotic","source":"Wight"}]'::jsonb,'What is left of someone who was once in charge. Still is, in its way.',array['sim','mar'],array['lgt','med','hvy','shl']),
('specter','Specter','undead','med',5,2,12,22,1,14,11,10,10,11,'[{"target":"resist.acid","source":"Specter"},{"target":"resist.cold","source":"Specter"},{"target":"resist.fire","source":"Specter"},{"target":"resist.lightning","source":"Specter"},{"target":"resist.thunder","source":"Specter"},{"target":"immune.poison","source":"Specter"}]'::jsonb,'No body to speak of, and no interest in yours beyond what it can take from it.',array['sim','mar'],array['lgt','med','hvy','shl']),
('imp','Imp','fiend','tiny',3,2,13,10,6,17,13,11,12,14,'[{"target":"resist.cold","source":"Imp"},{"target":"immune.fire","source":"Imp"},{"target":"immune.poison","source":"Imp"}]'::jsonb,'Small, clever and entirely untrustworthy. Usually working for somebody worse.',array['sim','mar'],array['lgt','med','hvy','shl']),
('dretch','Dretch','fiend','sm',4,2,11,18,11,11,12,5,8,3,'[{"target":"resist.cold","source":"Dretch"},{"target":"resist.fire","source":"Dretch"},{"target":"resist.lightning","source":"Dretch"},{"target":"immune.poison","source":"Dretch"}]'::jsonb,'The lowest thing the Abyss makes. Stupid, foul, and rarely alone.',array['sim','mar'],array['lgt','med','hvy','shl']),
('magma_mephit','Magma Mephit','elemental','sm',5,2,11,22,8,12,12,7,10,10,'[{"target":"immune.fire","source":"Magma Mephit"},{"target":"immune.poison","source":"Magma Mephit"},{"target":"vulnerable.cold","source":"Magma Mephit"}]'::jsonb,'A spiteful handful of molten rock. Bursts when killed, and knows it.',array['sim','mar'],array['lgt','med','hvy','shl']),
('ice_mephit','Ice Mephit','elemental','sm',6,2,11,21,7,13,10,9,11,12,'[{"target":"immune.cold","source":"Ice Mephit"},{"target":"immune.poison","source":"Ice Mephit"},{"target":"vulnerable.fire","source":"Ice Mephit"},{"target":"vulnerable.bludgeoning","source":"Ice Mephit"}]'::jsonb,'Brittle, shrill and malicious. Shatters loudly and takes something with it.',array['sim','mar'],array['lgt','med','hvy','shl']),
('animated_armor','Animated Armor','construct','med',6,2,18,33,14,11,13,1,3,1,'[{"target":"immune.poison","source":"Animated Armor"},{"target":"immune.psychic","source":"Animated Armor"}]'::jsonb,'An empty suit that stands up when someone walks past. Nothing inside it to reason with.',array['sim','mar'],array['lgt','med','hvy','shl']),
('flying_sword','Flying Sword','construct','sm',5,2,17,17,12,15,11,1,5,1,'[{"target":"immune.poison","source":"Flying Sword"},{"target":"immune.psychic","source":"Flying Sword"}]'::jsonb,'A blade that fights on its own. Hard to pin down, and tireless.',array['sim','mar'],array['lgt','med','hvy','shl']),
('gray_ooze','Gray Ooze','ooze','med',3,2,8,22,12,6,16,1,6,2,'[{"target":"resist.acid","source":"Gray Ooze"},{"target":"resist.cold","source":"Gray Ooze"},{"target":"resist.fire","source":"Gray Ooze"}]'::jsonb,'Looks like wet stone until it moves. Eats metal, which the party notices afterwards.',array['sim','mar'],array['lgt','med','hvy','shl']),
('gelatinous_cube','Gelatinous Cube','ooze','lg',8,2,6,84,14,3,20,1,6,1,'[]'::jsonb,'Fills the corridor completely and is almost invisible doing it. Whatever it has eaten is still inside.',array['sim','mar'],array['lgt','med','hvy','shl']),
('twig_blight','Twig Blight','plant','sm',1,2,13,4,6,13,12,4,8,3,'[{"target":"vulnerable.fire","source":"Twig Blight"}]'::jsonb,'A bundle of sticks that stands up. Harmless alone, and never alone.',array['sim','mar'],array['lgt','med','hvy','shl']),
('shrieker','Shrieker','plant','med',3,2,5,13,1,1,10,1,3,1,'[]'::jsonb,'A fungus that screams when disturbed. The scream is the whole of the threat, and it is enough.',array['sim','mar'],array['lgt','med','hvy','shl']),
('wolf','Wolf','beast','med',2,2,13,11,12,15,12,3,12,6,'[]'::jsonb,'Hunts in a pack and fights like one. Goes for the one at the back.',array['sim','mar'],array['lgt','med','hvy','shl']),
('brown_bear','Brown Bear','beast','lg',4,2,11,34,19,10,16,2,13,7,'[]'::jsonb,'Not hunting you, and will still take your arm off if you are between it and what it wants.',array['sim','mar'],array['lgt','med','hvy','shl']),
('giant_spider','Giant Spider','beast','lg',4,2,14,26,14,16,12,2,11,4,'[]'::jsonb,'Waits above rather than in front. The web is the dangerous part.',array['sim','mar'],array['lgt','med','hvy','shl']),
('dire_boar','Dire Boar','beast','lg',4,2,12,30,17,10,16,2,9,5,'[]'::jsonb,'Bad tempered at rest and worse in motion. Does not stop when it should.',array['sim','mar'],array['lgt','med','hvy','shl']),
('owlbear','Owlbear','monstrosity','lg',7,2,13,59,20,12,17,3,12,7,'[]'::jsonb,'A bear with a beak and none of the hesitation. Nobody agrees where they came from.',array['sim','mar'],array['lgt','med','hvy','shl']),
('harpy','Harpy','monstrosity','med',7,2,11,38,12,13,12,7,10,13,'[]'::jsonb,'Sings first. What follows is much less pleasant than the singing.',array['sim','mar'],array['lgt','med','hvy','shl']),
('ogre','Ogre','giant','lg',7,2,11,59,19,8,16,5,7,7,'[]'::jsonb,'Enormous, slow and uncomplicated. Hits once, and that is usually sufficient.',array['sim','mar'],array['lgt','med','hvy','shl']),
('hill_giant','Hill Giant','giant','huge',10,3,13,105,21,8,19,5,9,6,'[]'::jsonb,'Eats constantly and thinks rarely. Throws what it cannot reach.',array['sim','mar'],array['lgt','med','hvy','shl']),
('white_dragon_wyrmling','White Dragon Wyrmling','dragon','med',5,2,16,32,14,10,14,5,10,11,'[{"target":"immune.cold","source":"White Dragon Wyrmling"}]'::jsonb,'Young, vicious and already proud. The breath is the thing to respect.',array['sim','mar'],array['lgt','med','hvy','shl']),
('pseudodragon','Pseudodragon','dragon','tiny',2,2,13,7,6,15,13,10,12,10,'[]'::jsonb,'Cat-sized, and considerably cleverer than a cat. Chooses its company carefully.',array['sim','mar'],array['lgt','med','hvy','shl']),
('sprite','Sprite','fey','tiny',1,2,15,2,3,18,10,14,13,11,'[]'::jsonb,'Tiny, invisible when it wants to be, and a far better shot than it looks.',array['sim','mar'],array['lgt','med','hvy','shl']),
('dryad','Dryad','fey','med',5,2,11,22,10,12,11,14,15,18,'[]'::jsonb,'Bound to one tree and concerned with very little else. Would rather charm you than fight.',array['sim','mar'],array['lgt','med','hvy','shl']),
('pegasus','Pegasus','celestial','lg',7,2,12,59,18,15,16,10,15,13,'[]'::jsonb,'Will not be ridden by just anyone, and knows the difference.',array['sim','mar'],array['lgt','med','hvy','shl']),
('gibbering_mouther','Gibbering Mouther','aberration','med',9,2,9,67,10,8,16,3,10,6,'[]'::jsonb,'Many mouths, and all of them talking. The ground softens underneath it.',array['sim','mar'],array['lgt','med','hvy','shl'])
on conflict (key) where game_id is null do update set
  name = excluded.name, creature_type = excluded.creature_type,
  size = excluded.size, level = excluded.level, prof_bonus = excluded.prof_bonus,
  ac = excluded.ac, hp_max = excluded.hp_max,
  str = excluded.str, dex = excluded.dex, con = excluded.con,
  intl = excluded.intl, wis = excluded.wis, cha = excluded.cha,
  grants = excluded.grants, notes = excluded.notes,
  weapon_profs = excluded.weapon_profs, armor_profs = excluded.armor_profs;

delete from npc_items where game_id is null and npc_key in ('bandit','guard','orc','hobgoblin','kobold','cultist','bugbear','skeleton','zombie','ghoul','wight','specter','imp','dretch','magma_mephit','ice_mephit','animated_armor','flying_sword','gray_ooze','gelatinous_cube','twig_blight','shrieker','wolf','brown_bear','giant_spider','dire_boar','owlbear','harpy','ogre','hill_giant','white_dragon_wyrmling','pseudodragon','sprite','dryad','pegasus','gibbering_mouther');

insert into npc_items (npc_key, item_key, quantity, equipped, proficient_override)
values
('bandit','scimitar',1,true,true),
('bandit','crossbow_light',1,true,true),
('bandit','leather',1,true,true),
('guard','spear',1,true,true),
('guard','chain_shirt',1,true,true),
('guard','shield',1,true,true),
('orc','greataxe',1,true,true),
('orc','javelin',3,false,true),
('orc','hide',1,true,true),
('hobgoblin','longsword',1,true,true),
('hobgoblin','longbow',1,false,true),
('hobgoblin','chain_mail',1,true,true),
('hobgoblin','shield',1,true,true),
('kobold','dagger',1,true,true),
('kobold','sling',1,false,true),
('cultist','scimitar',1,true,true),
('cultist','leather',1,true,true),
('bugbear','morningstar',1,true,true),
('bugbear','javelin',3,false,true),
('bugbear','hide',1,true,true),
('bugbear','shield',1,true,true),
('skeleton','shortsword',1,true,true),
('skeleton','shortbow',1,false,true),
('skeleton','leather',1,true,true),
('zombie','slam',1,true,true),
('ghoul','claws',1,true,true),
('ghoul','bite',1,false,true),
('wight','longsword',1,true,true),
('wight','longbow',1,false,true),
('wight','studded_leather',1,true,true),
('specter','slam',1,true,true),
('imp','sting',1,true,true),
('dretch','claws',1,true,true),
('dretch','bite',1,false,true),
('magma_mephit','claws',1,true,true),
('ice_mephit','claws',1,true,true),
('animated_armor','slam',1,true,true),
('flying_sword','longsword',1,true,true),
('gray_ooze','slam',1,true,true),
('gelatinous_cube','slam',1,true,true),
('twig_blight','claws',1,true,true),
('wolf','bite',1,true,true),
('brown_bear','bite',1,true,true),
('brown_bear','claws',1,false,true),
('giant_spider','bite',1,true,true),
('dire_boar','gore',1,true,true),
('owlbear','bite',1,true,true),
('owlbear','claws',1,false,true),
('harpy','claws',1,true,true),
('harpy','club',1,false,true),
('ogre','greatclub',1,true,true),
('ogre','javelin',3,false,true),
('ogre','hide',1,true,true),
('hill_giant','greatclub',1,true,true),
('white_dragon_wyrmling','bite',1,true,true),
('white_dragon_wyrmling','claws',1,false,true),
('pseudodragon','bite',1,true,true),
('pseudodragon','sting',1,false,true),
('sprite','shortbow',1,true,true),
('sprite','shortsword',1,false,true),
('dryad','club',1,true,true),
('pegasus','hooves',1,true,true),
('gibbering_mouther','bite',1,true,true);
