-- 129. EIGHT MORE LIMBS, AND LOWER GATES.
--
-- 126 gave the bestiary eight natural weapons and twenty-four moves. The
-- creatures 130 is about to add need eight more limbs that 126 had no
-- reason to invent: a tail, a beak, tusks, a wing, a pseudopod, a fist,
-- a constriction and a spray of spines.
--
-- SAME SHAPE AS 126, for the same reasons stated there at length: an
-- ITEM, because `attack::resolve` matches techniques against the equipped
-- loadout and a limb that is not an item is a limb nothing can roll;
-- weight and slots at zero so 036's encumbrance walk does not have a bear
-- labouring under its own teeth; `content_tags = array['natural']` as the
-- filter hook; tiny size.
--
-- A FIST AND A CONSTRICTION ARE NOT ANATOMY, which is worth admitting.
-- `fist` is what an ape or a mummy hits with and `constrict` is what a
-- snake does with its whole body - neither is a discrete organ. They are
-- items because the attack path needs them to be, and the alternative was
-- a second mechanism for "attacks that are not held in a hand".
--
-- ---------------------------------------------------------------------
-- AND THE GATES COME DOWN TO 1 / 2 / 4
-- ---------------------------------------------------------------------
--
-- 126 gated each limb's three moves at levels 1, 3 and 5, copying the
-- shape 043 uses for weapons. That shape assumes a character who LEVELS
-- UP, and a creature does not. A wolf is a wolf. Gating its repertoire on
-- a progression it will never walk left every level-2 creature in 127
-- with exactly one move, which is not "three attacks minimum" by any
-- reading.
--
-- AT 1 / 2 / 4 a level-2 creature has two moves per limb and a level-4
-- one has all three, and most of the bestiary clears level 4.
--
-- THE UPDATE BELOW IS SCOPED BY OWNERSHIP AND NOT BY KIND, and that is a
-- mistake 132 corrects. `where game_id is null` catches 043's player
-- weapon techniques as well as the natural ones, so this retuned the
-- whole game rather than the bestiary. The statement is left here as it
-- ran, per 115: an applied migration is a record, not a draft.

insert into items
  (key, name, kind, weapon_class, damage_number, damage_denomination, damage_types,
   properties, content_tags, size, weight, slots, price, denom, description)
values
('tail','Tail','weapon','simpleM',1,8,array['bludgeoning'],array[]::text[],array['natural'],'tiny',0,0,0,'cp','A heavy counterweight swung on purpose. Comes from a direction nobody was watching.'),
('beak','Beak','weapon','simpleM',1,10,array['piercing'],array[]::text[],array['natural'],'tiny',0,0,0,'cp','Hooked and built for tearing rather than holding. One blow does the work of several.'),
('tusks','Tusks','weapon','simpleM',1,6,array['slashing'],array[]::text[],array['natural'],'tiny',0,0,0,'cp','Upward and outward. The damage is done on the way up, which is why it is hard to see coming.'),
('wing','Wing Buffet','weapon','simpleM',1,6,array['bludgeoning'],array[]::text[],array['natural'],'tiny',0,0,0,'cp','More air than contact, and the air is the point - it moves you rather than hurting you.'),
('pseudopod','Pseudopod','weapon','simpleM',1,6,array['bludgeoning'],array[]::text[],array['natural'],'tiny',0,0,0,'cp','A limb that was not there a moment ago and will not be there afterwards.'),
('fist','Fist','weapon','simpleM',1,6,array['bludgeoning'],array[]::text[],array['natural'],'tiny',0,0,0,'cp','A hand closed and swung. What anything with arms and no weapon fights with.'),
('constrict','Coils','weapon','simpleM',1,8,array['bludgeoning'],array[]::text[],array['natural'],'tiny',0,0,0,'cp','Not a blow at all - a grip that tightens every time the thing inside it breathes out.'),
('spines','Spines','weapon','simpleM',1,6,array['piercing'],array[]::text[],array['natural'],'tiny',0,0,0,'cp','Not aimed. Being close enough is the whole of the mechanism.')
on conflict (key) where game_id is null do update set
  name = excluded.name, kind = excluded.kind, weapon_class = excluded.weapon_class,
  damage_number = excluded.damage_number,
  damage_denomination = excluded.damage_denomination,
  damage_types = excluded.damage_types, content_tags = excluded.content_tags,
  size = excluded.size, weight = excluded.weight, slots = excluded.slots,
  price = excluded.price, denom = excluded.denom, description = excluded.description;

insert into techniques
  (key, name, roll_name, category, tier, min_level, dice, crit_min, fumble_max,
   special_text, item_key, mode)
values
('ntl_sweep','Sweep','sweep','Tail Work','Class 1',1,'1d8',20,1,
 '🦎 Sweep: a low heavy arc that nobody standing in front of it was looking at.','tail','melee'),
('ntl_knock_down','Knock Down','knock down','Tail Work','Class 1',2,'1d8',20,1,
 '⬇️ Knock Down: taken across the legs - the target makes a STR save (DC 8 + prof + STR) or is knocked prone.','tail','melee'),
('ntl_lash_around','Lash Around','lash around','Tail Work','Class 2',4,'2d8',19,2,
 '🌀 Lash Around: the whole body turns into it and reaches behind - a second enemy within 10 ft takes 1d8. Crits on 19-20, fumbles on 1-2.','tail','melee'),
('nbk_tear','Tear','tear','Beak Work','Class 1',1,'1d10',20,1,
 '🦅 Tear: one closing pull. There is no second part to it.','beak','melee'),
('nbk_strike_the_eye','Strike the Eye','strike the eye','Beak Work','Class 1',2,'1d10',20,1,
 '👁️ Strike the Eye: aimed high and small - the target makes a DEX save (DC 8 + prof + DEX) or is blinded until the end of its next turn.','beak','melee'),
('nbk_rip_free','Rip Free','rip free','Beak Work','Class 2',4,'2d10',19,2,
 '🩸 Rip Free: taken hold of and pulled away - the target bleeds 1d6 per turn until treated. Crits on 19-20, fumbles on 1-2.','beak','melee'),
('ntk_hook_upward','Hook Upward','hook upward','Tusk Work','Class 1',1,'1d6',20,1,
 '🐗 Hook Upward: driven from below, where armour is thinnest and attention is highest.','tusks','melee'),
('ntk_rip_and_turn','Rip and Turn','rip and turn','Tusk Work','Class 1',2,'1d8',20,1,
 '↩️ Rip and Turn: the head twists at the end of the blow - the target bleeds 1d4 per turn until treated.','tusks','melee'),
('ntk_lift_and_throw','Lift and Throw','lift and throw','Tusk Work','Class 2',4,'2d6',19,2,
 '🎪 Lift and Throw: a target one size smaller is thrown 10 ft and lands prone. Crits on 19-20, fumbles on 1-2.','tusks','melee'),
('nw_buffet','Buffet','buffet','Wing Work','Class 1',1,'1d6',20,1,
 '🪽 Buffet: more air than contact, and the air is doing most of the work.','wing','melee'),
('nw_drive_back','Drive Back','drive back','Wing Work','Class 1',2,'1d6',20,1,
 '💨 Drive Back: the target makes a STR save (DC 8 + prof + STR) or is pushed 10 ft away.','wing','melee'),
('nw_downdraft','Downdraft','downdraft','Wing Work','Class 2',4,'2d6',19,2,
 '🌪️ Downdraft: both wings brought down together - every creature within 10 ft makes a STR save or is knocked prone. Crits on 19-20, fumbles on 1-2.','wing','melee'),
('np_engulf_limb','Engulf a Limb','engulf a limb','Flow','Class 1',1,'1d6',20,1,
 '🫧 Engulf a Limb: it closes around rather than striking, and what it touches starts to hurt.','pseudopod','melee'),
('np_corrode','Corrode','corrode','Flow','Class 1',2,'1d6',20,1,
 '🧪 Corrode: nonmagical metal the target wears or holds takes a permanent -1 penalty, to a maximum of -5, and is destroyed at -5.','pseudopod','melee'),
('np_swallow','Draw Under','draw under','Flow','Class 2',4,'2d6',19,2,
 '🌊 Draw Under: the target is pulled into the mass - restrained (escape DC 8 + prof + STR) and takes 1d6 acid at the start of each of its turns. Crits on 19-20, fumbles on 1-2.','pseudopod','melee'),
('nf_punch','Punch','punch','Brawl','Class 1',1,'1d6',20,1,
 '👊 Punch: a closed hand and the weight behind it. Older than every other entry on this list.','fist','melee'),
('nf_grapple','Take Hold','take hold','Brawl','Class 1',2,'1d4',20,1,
 '🤝 Take Hold: less a blow than a decision - the target is grappled (escape DC 8 + prof + STR).','fist','melee'),
('nf_hammer_blow','Hammer Blow','hammer blow','Brawl','Class 2',4,'2d6',19,2,
 '🔨 Hammer Blow: both hands brought down together on something already held or prone. Crits on 19-20, fumbles on 1-2.','fist','melee'),
('ncs_wrap','Wrap','wrap','Coil Work','Class 1',1,'1d8',20,1,
 '🐍 Wrap: one loop taken, and the rest follows if nobody stops it.','constrict','melee'),
('ncs_tighten','Tighten','tighten','Coil Work','Class 1',2,'1d8',20,1,
 '🫁 Tighten: a grappled target cannot breathe - it is restrained and takes 1d8 at the start of each of its turns until it escapes.','constrict','melee'),
('ncs_crush','Crush','crush','Coil Work','Class 2',4,'2d8',19,2,
 '💀 Crush: everything the body has, applied slowly. Only against a target already grappled. Crits on 19-20, fumbles on 1-2.','constrict','melee'),
('nsp_bristle','Bristle','bristle','Quill Work','Class 1',1,'1d6',20,1,
 '🦔 Bristle: not aimed at anybody. Being close is the whole mechanism.','spines','melee'),
('nsp_loose','Loose a Volley','loose a volley','Quill Work','Class 1',2,'1d6',20,1,
 '🏹 Loose a Volley: every creature within 5 ft makes a DEX save (DC 8 + prof + DEX) or takes 1d6 piercing.','spines','melee'),
('nsp_impale','Impale','impale','Quill Work','Class 2',4,'2d6',19,2,
 '🩸 Impale: a spine left in the wound - the target bleeds 1d4 per turn until it or somebody else spends an action pulling it out. Crits on 19-20, fumbles on 1-2.','spines','melee')
on conflict (key) where game_id is null do update set
  name = excluded.name, roll_name = excluded.roll_name,
  category = excluded.category, tier = excluded.tier,
  min_level = excluded.min_level, dice = excluded.dice,
  crit_min = excluded.crit_min, fumble_max = excluded.fumble_max,
  special_text = excluded.special_text,
  item_key = excluded.item_key, mode = excluded.mode;

-- THE GATES COME DOWN TO 1 / 2 / 4.
-- A creature does not level up. A wolf is a wolf, and gating its
-- repertoire at 1/3/5 left every level-2 creature with exactly one move
-- - which is not "three attacks minimum" by any reading. At 1/2/4 a
-- level-2 creature has two moves per limb and a level-4 one has all
-- three, and almost everything in the bestiary clears level 4.
update techniques set min_level = 2 where game_id is null and min_level = 3;
update techniques set min_level = 4 where game_id is null and min_level = 5;
