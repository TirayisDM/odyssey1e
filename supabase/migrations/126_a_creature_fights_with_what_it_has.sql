-- 126. A CREATURE FIGHTS WITH WHAT IT HAS.
--
-- The bestiary needs creatures that can swing, and half of them have no
-- weapons - a wolf has a mouth. 027 gave this game sixty-odd weapons and
-- 050 gave them TECHNIQUES: named moves with their own dice, their own
-- crit and fumble ranges, a tier, a level gate and a line of text saying
-- what the move actually does to somebody.
--
-- NATURAL ATTACKS GET THE SAME TREATMENT, which was Dave's call and is
-- the better one. The alternative was a flat "bite 1d6" per creature,
-- or a separate item per damage step - `bite_1d6`, `bite_1d8`,
-- `bite_2d6` - which is a catalogue full of near-duplicates and no
-- scaling rule anywhere.
--
-- THE LEVEL GATE DOES THE SCALING. One `bite` item, three techniques at
-- min_level 1, 3 and 5. A wolf reaches Snap; an owlbear reaches Crush
-- the Throat. That is the same rule a character levelling into Split the
-- Collar already follows, so a creature's growing teeth and a fighter's
-- growing repertoire are one mechanism rather than two.
--
-- ---------------------------------------------------------------------
-- WHAT A NATURAL WEAPON IS, AS A ROW
-- ---------------------------------------------------------------------
--
-- An ITEM, because that is what the attack path reads. `attack::resolve`
-- matches a technique by `roll_name` against the EQUIPPED loadout, and
-- `load_for_loadout` reads techniques for equipped weapons - so a bite
-- that is not an item is a bite nothing can roll.
--
-- WEIGHT AND SLOTS ARE ZERO, and the size is tiny. A mouth is not
-- carried and must not count against what a creature can lift; 036's
-- encumbrance walk reads both of those and would otherwise have a bear
-- labouring under its own teeth.
--
-- `content_tags` MARKS THEM `natural`. Nothing reads that tag yet. It is
-- there because a shop listing every creature's jaws is the obvious next
-- complaint, and the tag is the cheapest place to filter from - stated
-- here so the next person knows it is a hook rather than decoration.
--
-- THEY ARE STILL ORDINARY OBJECTS, which is the honest limitation: a DM
-- can unequip a wolf's mouth or sell it. Making a natural weapon bound
-- to its creature wants a column nothing else needs yet, and a goblin
-- selling its claws is a funnier bug than it is a dangerous one.

insert into items
  (key, name, kind, weapon_class, damage_number, damage_denomination, damage_types,
   properties, content_tags, size, weight, slots, price, denom, description)
values
('bite','Bite','weapon','simpleM',1,6,array['piercing'],
 array[]::text[],array['natural'],'tiny',0,0,0,'cp',
 'Teeth. Whether that is a wolf''s long jaw or a ghoul''s broken one changes the manner rather than the mechanics.'),
('claws','Claws','weapon','simpleM',1,4,array['slashing'],
 array[]::text[],array['natural'],'tiny',0,0,0,'cp',
 'Hooked and raking. Fast, shallow, and wearing — a creature that leads with claws is usually setting up its mouth.'),
('slam','Slam','weapon','simpleM',1,6,array['bludgeoning'],
 array[]::text[],array['natural'],'tiny',0,0,0,'cp',
 'Mass delivered without finesse — a fist, a stump, a cold heavy limb. What an animated thing fights with when it has no edge.'),
('gore','Gore','weapon','simpleM',1,8,array['piercing'],
 array[]::text[],array['natural'],'tiny',0,0,0,'cp',
 'Horn or tusk with the whole body behind it. Needs room to run and loses most of its point without it.'),
('talons','Talons','weapon','simpleM',1,6,array['slashing'],
 array[]::text[],array['natural'],'tiny',0,0,0,'cp',
 'A grip that closes and does not open. Built for carrying something off as much as for opening it.'),
('sting','Sting','weapon','simpleM',1,4,array['piercing'],
 array[]::text[],array['natural'],'tiny',0,0,0,'cp',
 'A small wound and whatever was in it. The puncture is rarely the problem.'),
('tendrils','Tendrils','weapon','simpleM',1,6,array['bludgeoning'],
 array[]::text[],array['natural'],'tiny',0,0,0,'cp',
 'Reaching, gripping and too many to watch at once. What has no face fights with these.'),
('hooves','Hooves','weapon','simpleM',1,6,array['bludgeoning'],
 array[]::text[],array['natural'],'tiny',0,0,0,'cp',
 'Struck downward or kicked back. A great deal of weight through a very small area.')
on conflict (key) where game_id is null do update set
  name = excluded.name, kind = excluded.kind, weapon_class = excluded.weapon_class,
  damage_number = excluded.damage_number,
  damage_denomination = excluded.damage_denomination,
  damage_types = excluded.damage_types, properties = excluded.properties,
  content_tags = excluded.content_tags, size = excluded.size,
  weight = excluded.weight, slots = excluded.slots,
  price = excluded.price, denom = excluded.denom,
  description = excluded.description;

-- ---------------------------------------------------------------------
-- AND THE MOVES
-- ---------------------------------------------------------------------
--
-- THREE PER LIMB, AT LEVELS 1, 3 AND 5, which is the shape the weapon
-- techniques already use: a Class 1 move everything can make, a Class 1
-- move with a rider, and a Class 2 move that costs something to reach.
--
-- `special_text` STATES THE RIDER AND THE DM APPLIES IT, exactly as the
-- weapon techniques do. The engine rolls the dice and honours the crit
-- and fumble ranges; "the target is grappled" is a sentence a person
-- reads. That line is not a shortcut - it is where 050 drew the border
-- and this does not move it.
--
-- `mode` IS melee FOR ALL OF THEM. A creature with a ranged natural
-- attack - a spit, a breath - is a different shape and wants its own
-- item rather than a mode on a mouth.

insert into techniques
  (key, name, roll_name, category, tier, min_level, dice, crit_min, fumble_max,
   special_text, item_key, mode)
values
('nb_snap','Snap','snap','Maw Work','Class 1',1,'1d6',20,1,
 '🦷 Snap: a short fast closing bite, taken and released before anything can be done about it.','bite','melee'),
('nb_worry_the_limb','Worry the Limb','worry the limb','Maw Work','Class 1',3,'1d8',20,1,
 '🩸 Worry the Limb: the jaw closes and the body does the rest - the target is grappled (escape DC 8 + prof + STR) and cannot be bitten again while held.','bite','melee'),
('nb_crush_the_throat','Crush the Throat','crush the throat','Maw Work','Class 2',5,'2d6',19,2,
 '💀 Crush the Throat: everything the jaw has, aimed where it matters. Crits on 19-20 and fumbles on 1-2.','bite','melee'),
('nc_rake','Rake','rake','Rend','Class 1',1,'1d4',20,1,
 '🐾 Rake: a shallow fast swipe. On its own it is an irritation, which is the point - it is setting up the mouth.','claws','melee'),
('nc_both_paws','Both Paws','both paws','Rend','Class 1',3,'2d4',20,1,
 '🫳 Both Paws: struck together from either side - the target makes a STR save (DC 8 + prof + STR) or is knocked prone.','claws','melee'),
('nc_open_the_belly','Open the Belly','open the belly','Rend','Class 2',5,'2d6',19,1,
 '🩸 Open the Belly: a long tearing pull rather than a swipe - the target bleeds 1d4 per turn until treated. Crits on 19-20.','claws','melee'),
('ns_battering_blow','Battering Blow','battering blow','Bulk','Class 1',1,'1d6',20,1,
 '🪨 Battering Blow: weight swung on the end of a limb, with nothing clever about it.','slam','melee'),
('ns_overbear','Overbear','overbear','Bulk','Class 1',3,'1d8',20,1,
 '⬇️ Overbear: stepping into the blow with the whole mass behind it - the target makes a STR save (DC 8 + prof + STR) or is knocked prone and the attacker may step into its space.','slam','melee'),
('ns_pin_and_press','Pin and Press','pin and press','Bulk','Class 2',5,'2d6',19,2,
 '🧱 Pin and Press: the target is driven against whatever is behind it - grappled (escape DC 8 + prof + STR) and at disadvantage on its next attack. Crits on 19-20, fumbles on 1-2.','slam','melee'),
('ng_lower_the_head','Lower the Head','lower the head','Horn Work','Class 1',1,'1d8',20,1,
 '🐂 Lower the Head: a short shove with the points forward. It wants room and does not have it.','gore','melee'),
('ng_run_through','Run Through','run through','Horn Work','Class 1',3,'2d6',20,2,
 '🏃 Run Through: twenty feet of run and all of it arriving at once - only after moving at least 20 ft straight at the target. Fumbles on 1-2.','gore','melee'),
('ng_toss','Toss','toss','Horn Work','Class 2',5,'2d8',19,2,
 '🎪 Toss: the head comes up at the end of the blow - a target one size larger or smaller is thrown 10 ft and lands prone. Crits on 19-20, fumbles on 1-2.','gore','melee'),
('nt_seize','Seize','seize','Talon Work','Class 1',1,'1d6',20,1,
 '🦅 Seize: a grip that closes on landing and is not designed to open.','talons','melee'),
('nt_carry_off','Carry Off','carry off','Talon Work','Class 1',3,'1d8',20,1,
 '🪽 Carry Off: a target one size smaller is grappled (escape DC 8 + prof + STR) and moved with the attacker until it breaks free.','talons','melee'),
('nt_stoop','Stoop','stoop','Talon Work','Class 2',5,'2d6',19,2,
 '⬇️ Stoop: a dive with the whole weight behind the feet - only from at least 20 ft above. Crits on 19-20 and fumbles on 1-2.','talons','melee'),
('nst_jab','Jab','jab','Venom','Class 1',1,'1d4',20,1,
 '🪡 Jab: a small puncture. The wound is not the problem and both parties know it.','sting','melee'),
('nst_envenom','Envenom','envenom','Venom','Class 1',3,'1d4',20,1,
 '🧪 Envenom: the sting is delivered and held - the target makes a CON save (DC 8 + prof + CON) or takes 2d4 poison and is poisoned until the end of its next turn.','sting','melee'),
('nst_full_measure','Full Measure','full measure','Venom','Class 2',5,'1d6',19,2,
 '☠️ Full Measure: everything the gland holds - the target makes a CON save (DC 8 + prof + CON) or takes 4d4 poison and is poisoned for a minute, saving at the end of each of its turns. Crits on 19-20, fumbles on 1-2.','sting','melee'),
('ntd_lash','Lash','lash','Coil','Class 1',1,'1d6',20,1,
 '🌾 Lash: one of several, and the others are already moving.','tendrils','melee'),
('ntd_entangle','Entangle','entangle','Coil','Class 1',3,'1d6',20,1,
 '🕸️ Entangle: two or three close at once - the target is restrained (escape DC 8 + prof + STR) until it breaks free.','tendrils','melee'),
('ntd_draw_in','Draw In','draw in','Coil','Class 2',5,'2d6',19,2,
 '🌀 Draw In: whatever is held is pulled toward the middle - a grappled target is moved 10 ft closer and has disadvantage on its next save. Crits on 19-20, fumbles on 1-2.','tendrils','melee'),
('nh_kick','Kick','kick','Trample','Class 1',1,'1d6',20,1,
 '🐴 Kick: delivered backward without looking, which is the honest way to do it.','hooves','melee'),
('nh_rear_and_strike','Rear and Strike','rear and strike','Trample','Class 1',3,'2d4',20,1,
 '🐎 Rear and Strike: both forefeet brought down from standing - the target makes a STR save (DC 8 + prof + STR) or is knocked prone.','hooves','melee'),
('nh_trample','Trample','trample','Trample','Class 2',5,'2d6',19,2,
 '🏇 Trample: straight over a prone target rather than around it - only against something already prone, and it provokes nothing on the way through. Crits on 19-20, fumbles on 1-2.','hooves','melee')
on conflict (key) where game_id is null do update set
  name = excluded.name, roll_name = excluded.roll_name,
  category = excluded.category, tier = excluded.tier,
  min_level = excluded.min_level, dice = excluded.dice,
  crit_min = excluded.crit_min, fumble_max = excluded.fumble_max,
  special_text = excluded.special_text,
  item_key = excluded.item_key, mode = excluded.mode;
