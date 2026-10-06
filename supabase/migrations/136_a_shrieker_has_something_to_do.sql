-- 136. A SHRIEKER HAS SOMETHING TO DO.
--
-- The Shrieker was the one creature in the bestiary with no attack at
-- all, and that is faithful to the SRD: a shrieker is a fungus that
-- screams when light or movement reaches it, and the scream has no
-- mechanics of its own. It brings something else to you. At a table
-- that works; in this engine it is a creature you can enrol, roll
-- initiative for, and then do nothing with.
--
-- SO THE SCREAM IS THE WEAPON. Dave asked for a few kinds of shriek and
-- gave licence to invent them, which is what this is: three moves off
-- one organ, authored the way 126 authored a bite.
--
-- ---------------------------------------------------------------------
-- IT IS AIMED AT ONE CREATURE, AND THAT IS A CHOICE
-- ---------------------------------------------------------------------
--
-- A scream wants to be an area with a save, and this engine has no
-- save-DC attack path - `attack::resolve` rolls to hit against an AC
-- and that is the only shape it knows. Two honest ways to go:
--
--   1. Model the area anyway, and have the DM ignore the to-hit roll
--      the screen insists on showing.
--   2. Aim it, because the fungus screams AT whatever disturbed it.
--
-- This takes the second. A rolled number that the table is supposed to
-- disregard is exactly this codebase's named defect - a calm, plausible,
-- wrong answer - and `spines` already set the precedent that a natural
-- weapon which is barely aimed is still an attack roll here.
--
-- THE CONDITION IS PROSE, like every other rider in the game. A grapple
-- from `worry the limb` is prose, a knockdown is prose, and so is the
-- ringing in somebody's ears: the engine cannot yet make a technique
-- land an effect, and the save DC is written out in the text so the DM
-- can call for it. Perception is the one with teeth - a day is long
-- enough to matter - and `effects` with a `skill.prc` grant is the shape
-- it should eventually take. Nothing applies it today, and the prose
-- says so rather than implying the engine has it.
--
-- ---------------------------------------------------------------------
-- IT SHRIEKS WITH ITS CONSTITUTION
-- ---------------------------------------------------------------------
--
-- 134 taught `ability_for` that a weapon may name its own ability, and
-- this is why. A Shrieker's STR is 1 and its DEX is 1, so every attack
-- this engine could build for it was at -5 before proficiency: a fungus
-- that could not be heard. Its CON is 10. `properties` carries `con`
-- and the scream rolls at +2, which is a thing that happens.
--
-- GATED 1 / 2 / 4, which is the natural-weapon scale 129 set and 132
-- confined to `content_tags @> natural`. For the Shrieker itself the
-- gates no longer decide anything - 133 stopped reading a progression
-- against a creature, so it reaches all three - and they still mean
-- something for any player character who ever grows an organ like this.

insert into items
  (key, name, kind, weapon_class, damage_number, damage_denomination, damage_types,
   properties, content_tags, size, weight, slots, price, denom, description)
values
('shriek','Shriek','weapon','simpleM',1,6,array['thunder'],
 array['con'],array['natural'],'tiny',0,0,0,'cp',
 'Not a mouth and not a voice - the whole body of the fungus resonating at once. It is heard in the teeth before it is heard in the ears.')
on conflict (key) where game_id is null do update set
  name = excluded.name, kind = excluded.kind, weapon_class = excluded.weapon_class,
  damage_number = excluded.damage_number,
  damage_denomination = excluded.damage_denomination,
  damage_types = excluded.damage_types, properties = excluded.properties,
  content_tags = excluded.content_tags, size = excluded.size,
  weight = excluded.weight, slots = excluded.slots, price = excluded.price,
  denom = excluded.denom, description = excluded.description;

-- THE THREE SHRIEKS. Same column list as 126 and the same voice: what
-- it does, in one sentence, with the rider stated where there is one.
-- The DCs are written as formulas with the Shrieker's own number beside
-- them, because a baked 10 would be wrong on anything else that ever
-- carries this organ.

insert into techniques
  (key, name, roll_name, category, tier, min_level, dice, crit_min, fumble_max,
   special_text, item_key, mode)
values
('ns_piercing_shriek','Piercing Shriek','piercing shriek','Shriek','Class 1',1,'1d6',20,1,
 '📢 Piercing Shriek: one flat note held far too long, aimed at whoever disturbed it. On a hit the target makes a CON save (DC 8 + prof + CON, which is 10 for a Shrieker) or their ears ring for a day: -5 to Perception until they have slept it off. NOT APPLIED BY THE ENGINE - nothing yet lets a technique land an effect, so the DM applies the penalty.','shriek','melee'),
('ns_confusion_shriek','Confusion Shriek','confusion shriek','Shriek','Class 1',2,'1d4',20,1,
 '🌀 Confusion Shriek: two notes a semitone apart, which is a sound nothing living is built to parse. Less damage than the others and the damage is not the point. On a hit the target makes a WIS save (DC 8 + prof + CON, 10 for a Shrieker) or loses its next action working out which way is which.','shriek','melee'),
('ns_stunning_shriek','Stunning Shriek','stunning shriek','Shriek','Class 2',4,'1d8',20,1,
 '💥 Stunning Shriek: everything the fungus has, spent at once - it will be quiet for a while afterwards. On a hit the target makes a CON save (DC 8 + prof + CON, 10 for a Shrieker) or is stunned until the end of its next turn.','shriek','melee')
on conflict (key) where game_id is null do update set
  name = excluded.name, roll_name = excluded.roll_name, category = excluded.category,
  tier = excluded.tier, min_level = excluded.min_level, dice = excluded.dice,
  crit_min = excluded.crit_min, fumble_max = excluded.fumble_max,
  special_text = excluded.special_text, item_key = excluded.item_key,
  mode = excluded.mode, removed = false;

-- AND PUT IT IN THE SHRIEKER. 131 used a scoped delete for its kit
-- rows "because `npc_items` has no unique key to conflict on", and that
-- is not true: `npc_items_global_idx` is unique on (npc_key, item_key)
-- where the row is global. So this conflicts on the index rather than
-- deleting first, which leaves anything else the Shrieker is ever given
-- untouched instead of trusting the scope of a delete.

insert into npc_items (npc_key, item_key, quantity, equipped, proficient_override)
values ('shrieker','shriek',1,true,true)
on conflict (npc_key, item_key) where game_id is null do update set
  quantity = excluded.quantity, equipped = excluded.equipped,
  proficient_override = excluded.proficient_override;
