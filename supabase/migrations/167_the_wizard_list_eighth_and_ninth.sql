-- 167. THE WIZARD LIST: EIGHTH AND NINTH CIRCLE.
--
-- 21 spells, and the list is finished: 13 of 13 at eighth, 12 of 12 at
-- ninth, 206 wizard spells in the catalogue. Same contract as 161 to
-- 166 and 102-104: mechanical values read off the published list, every
-- description written here.
--
-- TELEPATHY IS A JUDGEMENT CALL. The reference we have been checking
-- against puts it outside the SRD, as copyright material from the
-- Player's Handbook, and leaves it off both the full spell index and
-- the wizard list. It does the same to Feign Death - WHICH THIS
-- CATALOGUE ALREADY HOLDS, seeded in 102 as a cleric spell that
-- happens to be a wizard spell too. So the choice was between a list
-- that holds one of a pair and not the other for no reason anybody
-- could reconstruct later, or one that holds both. Both. If the
-- reference is right and we are wrong, the correction is one row.
--
-- FEEBLEMIND IS A CASE 158 DID NOT NAME. Its 4d6 psychic lands whether
-- the target makes the save or not - the save only decides whether its
-- Intelligence and Charisma drop to 1. 158 gave `on_save` three
-- answers: half, none, and NULL for dice that are not save damage. This
-- is a fourth: damage that is not conditional on the save at all.
--
-- It goes in as NULL, which is the least wrong of the three, and that
-- means `save_damage` returns `None` and the engine writes no hit point
-- event - the DM reads the 4d6 off the card and applies it. The
-- alternative was `half` or `none`, and both of those are a wrong
-- number applied with total confidence, which is worse than a right
-- number applied by hand. A fourth value - say `always` - is the actual
-- fix, and it is a Rust change with tests, not a data change, so it is
-- not in this file. Magic Missile is the same shape and 102 reached the
-- same place by a different route: Utility, dice in the column, nothing
-- automatic.
--
-- TIME STOP KEEPS ITS 1d4+1 OUT OF THE DICE COLUMN, which is 158 at its
-- plainest. Those are turns, not damage, and a column that holds both
-- is a column that means nothing.
--
-- METEOR SWARM AND PRISMATIC WALL both have more damage than one column
-- holds. The swarm does 20d6 fire AND 20d6 bludgeoning and only the
-- fire is stored; the wall does 10d6 per intact layer and the column
-- holds one layer. 163 did the same with Ice Storm. The text carries
-- the rest.
--
-- POWER WORD KILL AND MAZE ARE `Utility`, not `Save`, because neither
-- has a saving throw anywhere in it. The word kills outright under 100
-- hit points and the maze is escaped on an Intelligence CHECK against a
-- fixed DC 20. POWER WORD STUN IS `Save` on the strength of the
-- recurring save only, the way 166 handled Irresistible Dance, and the
-- text says the landing is automatic.
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
('sp_antipathysympathy','Antipathy/Sympathy','Antipathy/Sympathy',8,'Save','8th Circle','enchantment','wis',null,
 null,false,false,'60 feet','10 days','1 hour',array['v','s','m'],'a lump of alum soaked in vinegar for antipathy, or a drop of honey for sympathy',
 array['druid','wizard'],
 'Choose a creature or a 200-foot cube, and the kind of creature that reacts to it. ANTIPATHY frightens them and drives them away while they can see or sense it; SYMPATHY compels them to approach and stay. A failed Wisdom save holds for a minute, with a fresh save every minute for antipathy and every hour for sympathy. A creature that saves is immune for a minute.',
 'Something is made repellent, or irresistible, to a kind of creature you name.'),
('sp_clone','Clone','Clone',8,'Utility','8th Circle','necromancy',null,null,
 null,false,false,'Touch','Instant','1 hour',array['v','s','m'],'a diamond worth 1,000 gp and a cubic inch of the creature flesh, both consumed, and a sealable vessel worth 2,000 gp',
 array['wizard'],
 'The clone grows inert for 120 DAYS and then waits indefinitely. When the original dies, its soul transfers to the clone if the soul is free and willing, and the clone wakes with all the memories and abilities the original had. The original body becomes inert and cannot be revived, since the soul has gone elsewhere. Grown from a piece of a young creature, the clone is that age.',
 'A spare body is grown in a vat, and a soul with somewhere to go when it dies.'),
('sp_demiplane','Demiplane','Demiplane',8,'Utility','8th Circle','conjuration',null,null,
 null,false,false,'60 feet','1 hour','1 action',array['s'],null,
 array['warlock','wizard'],
 'A 30-foot cube of empty room behind a shadowy door, or a return to a demiplane you have made before. ANYTHING LEFT INSIDE WHEN THE DOOR CLOSES STAYS THERE until somebody opens it again, which is what makes this a prison as readily as a vault.',
 'A door onto an empty room that exists nowhere, and which you can come back to.'),
('sp_dominatemonster','Dominate Monster','Dominate Monster',8,'Save','8th Circle','enchantment','wis',null,
 null,true,false,'60 feet','Up to 1 hour','1 action',array['v','s'],null,
 array['bard','sorcerer','warlock','wizard'],
 'Any creature, not just a humanoid - that is the difference from Dominate Person. A failed Wisdom save is charmed and takes your orders, and an action lets you steer it directly. It saves again each time it takes damage. 8 hours at 9th level.',
 'Any creature at all does as you tell it, and precisely so if you spend the action to steer.'),
('sp_feeblemind','Feeblemind','Feeblemind',8,'Save','8th Circle','enchantment','int','4d6',
 null,false,false,'150 feet','Instant','1 action',array['v','s','m'],'a handful of clay, crystal, glass or mineral spheres',
 array['bard','druid','warlock','wizard'],
 'THE 4d6 PSYCHIC LANDS WHETHER THE SAVE SUCCEEDS OR NOT - it is not conditional, which is why on_save is NULL here and the engine will not apply it for you. The save decides the rest: a failure drops Intelligence and Charisma to 1, taking speech, spellcasting and the ability to understand language with them. The target saves again every 30 DAYS, and only Greater Restoration, Heal or Wish ends it sooner.',
 'A mind is broken open: everyone takes the psychic damage, and the unlucky lose themselves with it.'),
('sp_incendiarycloud','Incendiary Cloud','Incendiary Cloud',8,'Save','8th Circle','conjuration','dex','10d8',
 'half',true,false,'150 feet','Up to 1 minute','1 action',array['v','s'],null,
 array['sorcerer','wizard'],
 'A 20-foot-radius sphere that heavily obscures and MOVES 10 FEET AWAY FROM YOU at the start of each of your turns. Anything in it when it appears, and anything starting its turn in it afterwards, makes a Dexterity save. Strong wind disperses it and ends the spell.',
 'A cloud of smoke and white-hot embers that drifts steadily away from its caster.'),
('sp_maze','Maze','Maze',8,'Utility','8th Circle','conjuration',null,null,
 null,true,false,'60 feet','Up to 10 minutes','1 action',array['v','s'],null,
 array['wizard'],
 'NO SAVING THROW: the target simply goes. Escaping is an INTELLIGENCE CHECK against a fixed DC 20 as an action - not against your spell save DC - and a minotaur or goristro succeeds automatically. It returns to the space it left, or the nearest one, whenever the spell ends.',
 'One creature is taken out of the world and put into a labyrinth to find its own way back.'),
('sp_mindblank','Mind Blank','Mind Blank',8,'Utility','8th Circle','abjuration',null,null,
 null,false,false,'Touch','24 hours','1 action',array['v','s'],null,
 array['bard','wizard'],
 'Immunity to psychic damage, to the charmed condition, to anything that reads thoughts or senses emotions, and to every divination spell. IT STOPS A WISH from altering the target for any reason - the only protection in the catalogue that is written to beat a Wish.',
 'A mind is sealed shut: nothing reads it, nothing charms it, nothing rewrites it for a day.'),
('sp_powerwordstun','Power Word Stun','Power Word Stun',8,'Save','8th Circle','enchantment','con',null,
 null,false,false,'60 feet','Instant','1 action',array['v'],null,
 array['bard','sorcerer','warlock','wizard'],
 'NO SAVE WHEN IT LANDS, and no attack roll: a creature with 150 HIT POINTS OR FEWER is stunned, and one with more is unaffected entirely. The stunned creature makes a Constitution save at the end of each of its turns to end it.',
 'A single word of power, and anything not strong enough to shrug it off cannot act.'),
('sp_sunburst','Sunburst','Sunburst',8,'Save','8th Circle','evocation','con','12d6',
 'half',false,false,'150 feet','Instant','1 action',array['v','s','m'],'fire and a piece of sunstone',
 array['druid','sorcerer','wizard'],
 'A 60-foot radius. A failed Constitution save takes 12d6 radiant and is BLINDED for a minute, with a save at the end of each of its turns to recover sight. Undead and oozes have disadvantage. The burst dispels any magical darkness in the area.',
 'Daylight detonates: everything nearby is burned and most of it cannot see afterwards.'),
('sp_telepathy','Telepathy','Telepathy',8,'Utility','8th Circle','evocation',null,null,
 null,false,false,'Unlimited','24 hours','1 action',array['v','s','m'],'a pair of platinum rings worth 1,000 gp each, worn by you and the target',
 array['wizard'],
 'One willing creature you are familiar with, ON THE SAME PLANE AND AT ANY DISTANCE. You understand each other regardless of language, and you may send images, sounds and other sensations along with the words. 165 Telepathic Bond links eight minds for an hour at short range; this links one for a day at none.',
 'You and one other mind can speak and share senses across any distance for a day.'),
('sp_foresight','Foresight','Foresight',9,'Utility','9th Circle','divination',null,null,
 null,false,false,'Touch','8 hours','1 minute',array['v','s','m'],'a hummingbird feather',
 array['bard','druid','warlock','wizard'],
 'For eight hours the target CANNOT BE SURPRISED and has advantage on every attack roll, ability check and saving throw, while everything attacking it has disadvantage. Casting it again on anybody ends the earlier one.',
 'A creature sees just far enough ahead that nothing catches it out all day.'),
('sp_imprisonment','Imprisonment','Imprisonment',9,'Save','9th Circle','abjuration','wis',null,
 null,false,false,'30 feet','Until dispelled','1 minute',array['v','s','m'],'a likeness of the target, and a component for the chosen binding worth 500 gp per Hit Die of the target',
 array['warlock','wizard'],
 'A failed Wisdom save and the target is held indefinitely, needing neither air, food nor sleep, and not ageing. Choose the form: BURIAL deep in the earth, CHAINING to the spot, HEDGED PRISON in a tiny demiplane, MINIMUS CONTAINMENT in a gem, or SLUMBER, simply asleep forever. YOU NAME THE CONDITION THAT RELEASES IT when you cast it, and a Dispel Magic only works if cast with a 9th-level slot. One target per casting, and casting it again on the same target ends the first.',
 'One creature is put away, alive and unageing, until the condition you chose comes about.'),
('sp_meteorswarm','Meteor Swarm','Meteor Swarm',9,'Save','9th Circle','evocation','dex','20d6',
 'half',false,false,'1 mile','Instant','1 action',array['v','s'],null,
 array['sorcerer','wizard'],
 'FOUR 40-FOOT-RADIUS SPHERES, which may be placed to overlap, though nothing in an overlap is hit twice. TWO DAMAGE TYPES: 20d6 fire AND 20d6 bludgeoning on a failed Dexterity save, half of both on a success - the column holds the fire only. Unattended flammable objects in the area catch fire.',
 'Four burning rocks fall out of the sky onto places you choose up to a mile away.'),
('sp_powerwordkill','Power Word Kill','Power Word Kill',9,'Utility','9th Circle','enchantment',null,null,
 null,false,false,'60 feet','Instant','1 action',array['v'],null,
 array['bard','sorcerer','warlock','wizard'],
 'NO SAVE AND NO ATTACK ROLL ANYWHERE IN IT. A creature with 100 HIT POINTS OR FEWER dies outright; one with more is unaffected. That is the whole spell.',
 'You say one word and anything already hurt enough to hear it stops living.'),
('sp_prismaticwall','Prismatic Wall','Prismatic Wall',9,'Save','9th Circle','abjuration','dex','10d6',
 'half',false,false,'60 feet','10 minutes','1 action',array['v','s'],null,
 array['wizard'],
 'Up to 90 feet long, 30 high and an inch thick, or a 30-foot-radius sphere. SEVEN LAYERS, each crossed in order, and the column holds one layer of 10d6. RED fire, ORANGE acid, YELLOW lightning, GREEN poison and BLUE cold each deal 10d6 on a Dexterity save for half. INDIGO is a Constitution save or restrained, then petrified on three failures. VIOLET is a Wisdom save or blinded, then banished to another plane. Each layer is destroyed by its own answer - Cone of Cold the red, Disintegrate the orange, and so on - and a layer must be gone before the next can be touched. A Dispel Magic takes one layer, or the whole wall on a 9th-level slot.',
 'Seven coloured layers of light, each with its own way of stopping whoever walks into it.'),
('sp_shapechange','Shapechange','Shapechange',9,'Utility','9th Circle','transmutation',null,null,
 null,true,false,'Self','Up to 1 hour','1 action',array['v','s','m'],'a jade circlet worth 1,500 gp, worn on your head',
 array['druid','wizard'],
 'Any creature whose CHALLENGE RATING IS NO HIGHER THAN YOUR LEVEL, and you must have seen that sort of creature. You gain its hit points as a separate pool, its statistics and its traits, and you keep your own alignment, personality, Intelligence, Wisdom and Charisma, and your class features if the new body can use them. YOU MAY CHANGE AGAIN as an action, any number of times, for as long as you concentrate.',
 'You become a creature as powerful as you are, and may become another whenever you like.'),
('sp_timestop','Time Stop','Time Stop',9,'Utility','9th Circle','transmutation',null,null,
 null,false,false,'Self','Instant','1 action',array['v'],null,
 array['sorcerer','wizard'],
 '1d4 + 1 TURNS, which is not in the dice column because those are turns and not damage. Everything else is frozen while you act. THE SPELL ENDS EARLY if anything you do affects another creature or an object somebody else is carrying, and it ends if you move more than 1,000 feet from where you cast it.',
 'Time stops for everyone but you, for a few turns, and ends the moment you touch anybody.'),
('sp_truepolymorph','True Polymorph','True Polymorph',9,'Save','9th Circle','transmutation','wis',null,
 null,true,false,'30 feet','Up to 1 hour','1 action',array['v','s','m'],'a drop of mercury, a dollop of gum arabic and a wisp of smoke',
 array['bard','warlock','wizard'],
 'Creature to creature, creature to object, or object to creature. An unwilling creature saves. A new creature form must be of a CR or level no higher than the target had, and it keeps only its alignment and personality. CONCENTRATE FOR THE FULL HOUR AND IT BECOMES PERMANENT. A creature changed into a creature reverts when dropped to 0 hit points of the new form.',
 'Anything becomes anything else, and if you hold it long enough it stays that way.'),
('sp_weird','Weird','Weird',9,'Save','9th Circle','illusion','wis','4d10',
 'none',true,false,'120 feet','Up to 1 minute','1 action',array['v','s'],null,
 array['wizard'],
 'A 30-foot-radius sphere, each creature in it seeing its own private horror. THE FIRST SAVE DOES NOT DAMAGE: a failure leaves the target frightened. The 4d10 psychic comes at the END OF EACH OF ITS TURNS on another Wisdom save, all of it or none of it. A frightened creature that makes the save ends the spell for itself. This is Phantasmal Killer across a whole area, which is what 165 said that spell would look like at ninth.',
 'Everything in the area meets the thing it fears most, and each one meets a different thing.'),
('sp_wish','Wish','Wish',9,'Utility','9th Circle','conjuration',null,null,
 null,false,false,'Self','Instant','1 action',array['v'],null,
 array['sorcerer','wizard'],
 'The safe use is DUPLICATING ANY SPELL OF 8TH LEVEL OR LOWER, with no components and no slot. Anything beyond that is the DM deciding what actually happens, and the spell itself warns that a greater wish may be twisted. BEYOND DUPLICATION IT COSTS: 2d4 days too weak to act, with every roll at disadvantage, no recovery from rest, and a cumulative 33 PERCENT CHANCE of never being able to cast Wish again.',
 'You say what you want to be true and reality is obliged, within limits that bite.')
on conflict (key) where game_id is null do update set
  name=excluded.name, roll_name=excluded.roll_name, level=excluded.level,
  cast_type=excluded.cast_type, category=excluded.category, school=excluded.school,
  save_ability=excluded.save_ability, dice=excluded.dice, on_save=excluded.on_save,
  concentration=excluded.concentration, ritual=excluded.ritual,
  range=excluded.range, duration=excluded.duration, casting_time=excluded.casting_time,
  components=excluded.components, material=excluded.material, classes=excluded.classes,
  special_text=excluded.special_text, description=excluded.description;
