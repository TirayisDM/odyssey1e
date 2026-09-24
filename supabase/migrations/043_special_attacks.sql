-- =====================================================================
-- 043_special_attacks.sql
-- odyssey1e - three ways to use every weapon in the armoury
-- =====================================================================
--
-- 006 seeded techniques for three weapons. The other fifty-four had a
-- damage die and nothing else, which means the item panel offered them
-- one button - the weapon's name - while the Mace of the Deep Song
-- offered ten. A catalogue where one weapon is interesting and the rest
-- are numbers is not a catalogue, it is one weapon and some filler.
--
-- 159 techniques here, covering every weapon that had none. Every one
-- has AT LEAST THREE, which was the floor asked for and is enforced by
-- the query at the bottom of this comment rather than by hope.
--
-- ---------------------------------------------------------------------
-- THE SHAPE: THREE TIERS, AND THEY ARE NOT THREE SIZES OF THE SAME HIT
--
--   level 1   the signature move - what this weapon is FOR
--   level 3   a control effect - a save, a condition, a disarm
--   level 5   the decisive one - bigger dice, better crits, or a cost
--
-- A weapon that got three variations on "swing hard" would be three
-- buttons doing one thing. So the techniques are built from what each
-- weapon actually solved: a guisarme pulls riders off horses, a ranseur
-- traps blades in its side prongs, an estoc goes through maille that a
-- falchion would only dent, and a bec de corbin has a hammer on one
-- side and a spike on the other because plate respects one and joints
-- respect the other.
--
-- ---------------------------------------------------------------------
-- MODES ARE VALIDATED, NOT ASSUMED
--
-- `equipment::modes` derives what a weapon can do from its class and
-- properties, and `check_item_keys` reports any technique naming a mode
-- the engine never generates. A technique written in an unreachable
-- mode is not an error anywhere - it is simply never offered, which is
-- the worst kind of bug to find. Every row here was checked against the
-- weapon's own class before it was written, and the count came back 0.
--
-- THE NET HAS NONE, and that is the one deliberate gap. It deals no
-- damage at all - its entire function is `spc`, which means restrain,
-- and there is no condition system to restrain anybody with. Three
-- "special attacks" that rolled damage for a weapon with no damage
-- would be worse than the honest absence. Same reasoning that kept the
-- blowgun out of 027.
--
-- SPECIAL TEXT IS PROSE THE ENGINE DOES NOT READ. Bleed, stun, armour
-- reduction and forced movement are all written for the table to
-- adjudicate, exactly as 006's rows are. The dice, the crit range and
-- the fumble range ARE mechanical and do fire. Nothing here pretends a
-- condition system exists.
-- =====================================================================

insert into public.techniques
  (key, game_id, name, roll_name, category, tier, min_level, dice,
   crit_min, fumble_max, special_text, item_key, mode)
values
  ('b_long_edge', null, 'Long Edge', 'long edge', 'Polearm Drill', 'Class 1', 1, '2d4', 20, 1, '🪓 Long Edge: two feet of blade on four feet of haft, and no subtlety anywhere.', 'bardiche', 'melee'),
  ('b_rest_and_cut', null, 'Rest and Cut', 'rest and cut', 'Polearm Drill', 'Class 1', 3, '2d4', 19, 1, '🧿 Rest and Cut: the lower haft braced against the ground for a controlled heavy blow — crits on 19-20 while you have not moved this turn.', 'bardiche', 'melee'),
  ('b_cleave_the_column', null, 'Cleave the Column', 'cleave the column', 'Polearm Drill', 'Class 2', 5, '3d4', 19, 2, '🌀 Cleave the Column: one cut through a rank — a second enemy within 10 ft takes 2d4 slashing. Fumbles on 1-2.', 'bardiche', 'melee'),
  ('b_bite_deep', null, 'Bite Deep', 'bite deep', 'Axe Work', 'Class 1', 1, '1d8', 20, 1, '🪓 Bite Deep: the whole edge, driven with the hips.', 'battleaxe', 'melee'),
  ('b_beard_the_shield', null, 'Beard the Shield', 'beard the shield', 'Axe Work', 'Class 1', 3, '1d8', 20, 1, '🪝 Beard the Shield: the hooked lower edge drags a shield down — if the target carries a shield it loses that shield''s AC until the start of its next turn.', 'battleaxe', 'melee'),
  ('b_split_the_collar', null, 'Split the Collar', 'split the collar', 'Axe Work', 'Class 2', 5, '2d8', 19, 1, '🩸 Split the Collar: a downward blow where neck meets shoulder — the target bleeds 2d4 per turn until treated.', 'battleaxe', 'melee'),
  ('ba_long_beard', null, 'Long Beard', 'long beard', 'Axe Work', 'Class 1', 1, '1d8', 20, 1, '🪝 Long Beard: the hook does as much work as the edge.', 'bearded_axe', 'melee'),
  ('ba_strip_the_guard', null, 'Strip the Guard', 'strip the guard', 'Axe Work', 'Class 1', 3, '1d8', 20, 1, '🧲 Strip the Guard: the beard catches a weapon and turns it out of line — the target makes a STR save (DC 8 + prof + STR) or drops what it holds.', 'bearded_axe', 'melee'),
  ('ba_two_hand_fell', null, 'Two-Hand Fell', 'two-hand fell', 'Axe Work', 'Class 2', 5, '2d8', 19, 2, '⬇️ Two-Hand Fell: taken with both hands on the haft. Fumbles on 1-2.', 'bearded_axe', 'melee'),
  ('bdc_hammer_face', null, 'Hammer Face', 'hammer face', 'Polearm Drill', 'Class 1', 1, '1d10', 20, 1, '🔨 Hammer Face: the blunt side, which is what plate respects — treat the target''s armour as 3 AC lower.', 'bec_de_corbin', 'melee'),
  ('bdc_crow_s_beak', null, 'Crow''s Beak', 'crow''s beak', 'Polearm Drill', 'Class 1', 3, '1d10', 19, 1, '🐦 Crow''s Beak: the back spike punched into a joint — damage becomes piercing, treat armour as 5 AC lower, and crits on 19-20.', 'bec_de_corbin', 'melee'),
  ('bdc_open_the_visor', null, 'Open the Visor', 'open the visor', 'Polearm Drill', 'Class 2', 5, '2d10', 19, 1, '⛑️ Open the Visor: the beak set under the helm and levered — the target makes a CON save (DC 8 + prof + STR) or is stunned until the end of its next turn.', 'bec_de_corbin', 'melee'),
  ('bs_long_thrust', null, 'Long Thrust', 'long thrust', 'Spear Work', 'Class 1', 1, '1d8', 20, 1, '📏 Long Thrust: reach, and a crossbar that stops the shaft going too deep to pull back.', 'boar_spear', 'melee'),
  ('bs_crossbar_stop', null, 'Crossbar Stop', 'crossbar stop', 'Spear Work', 'Class 1', 3, '1d8', 20, 1, '🛑 Crossbar Stop: the bar catches on the wound — a target that hits you this turn makes a STR save (DC 8 + prof + STR) or cannot move closer until the end of its next turn.', 'boar_spear', 'melee'),
  ('bs_hold_the_beast', null, 'Hold the Beast', 'hold the beast', 'Spear Work', 'Class 2', 5, '2d8', 19, 1, '🐻 Hold the Beast: the spear set and the target''s own weight driving it home — against a target that moved toward you this turn, crits on 19-20 and it is restrained until it makes a STR save (DC 8 + prof + STR) at the end of its turn.', 'boar_spear', 'melee'),
  ('b_spinning_throw', null, 'Spinning Throw', 'spinning throw', 'Thrown Work', 'Class 1', 1, '1d4', 20, 1, '🌀 Spinning Throw: two weights and a cord, and the cord is the weapon.', 'bolas', 'ranged'),
  ('b_tangle_the_legs', null, 'Tangle the Legs', 'tangle the legs', 'Thrown Work', 'Class 1', 3, '1d4', 20, 1, '🧵 Tangle the Legs: the target makes a DEX save (DC 8 + prof + DEX) or its speed is 0 until it spends an action to cut free.', 'bolas', 'ranged'),
  ('b_bind_the_arms', null, 'Bind the Arms', 'bind the arms', 'Thrown Work', 'Class 2', 5, '2d4', 19, 1, '🔒 Bind the Arms: thrown high — the target makes a DEX save (DC 8 + prof + DEX) or is restrained until it spends an action to cut free.', 'bolas', 'ranged'),
  ('c_rap_the_knuckles', null, 'Rap the Knuckles', 'rap the knuckles', 'Club Work', 'Class 1', 1, '1d4', 20, 1, '🖐️ Rap the Knuckles: a short jab at the hands — the target makes a DEX save (DC 8 + prof + STR) or drops one held item.', 'club', 'melee'),
  ('c_skull_knock', null, 'Skull Knock', 'skull knock', 'Club Work', 'Class 1', 3, '1d6', 19, 1, '💫 Skull Knock: a downward blow behind the ear — on a hit the target makes a CON save (DC 8 + prof + STR) or has disadvantage on its next attack.', 'club', 'melee'),
  ('c_two_hand_swing', null, 'Two-Hand Swing', 'two-hand swing', 'Club Work', 'Class 2', 5, '2d4', 20, 2, '⬇️ Two-Hand Swing: both fists on the haft and everything behind it — a Medium or smaller target makes a STR save (DC 8 + prof + STR) or is knocked prone. Leaves you open: fumbles on 1-2.', 'club', 'melee'),
  ('ch_snap_shot', null, 'Snap Shot', 'snap shot', 'Bolt Work', 'Class 1', 1, '1d6', 20, 1, '⚡ Snap Shot: one hand, close range, no warning.', 'crossbow_hand', 'ranged'),
  ('ch_hidden_draw', null, 'Hidden Draw', 'hidden draw', 'Bolt Work', 'Class 1', 3, '1d6', 19, 1, '🕶️ Hidden Draw: loosed from beneath a cloak — against a target that has not acted this combat, crits on 19-20 and treat its armour as 3 AC lower.', 'crossbow_hand', 'ranged'),
  ('ch_poisoned_quarrel', null, 'Poisoned Quarrel', 'poisoned quarrel', 'Bolt Work', 'Class 2', 5, '2d6', 20, 1, '☠️ Poisoned Quarrel: the target makes a CON save (DC 8 + prof + DEX) or is poisoned until the end of its next turn.', 'crossbow_hand', 'ranged'),
  ('cl_aimed_bolt', null, 'Aimed Bolt', 'aimed bolt', 'Bolt Work', 'Class 1', 1, '1d8', 20, 1, '🎯 Aimed Bolt: the bow holds the draw so the shooter can hold the aim.', 'crossbow_light', 'ranged'),
  ('cl_punch_bolt', null, 'Punch Bolt', 'punch bolt', 'Bolt Work', 'Class 1', 3, '1d8', 20, 1, '⚔️ Punch Bolt: a square-headed quarrel — treat the target''s armour as 3 AC lower.', 'crossbow_light', 'ranged'),
  ('cl_braced_shot', null, 'Braced Shot', 'braced shot', 'Bolt Work', 'Class 2', 5, '2d8', 19, 1, '🧿 Braced Shot: the stock set against something solid — crits on 19-20 while you have not moved this turn.', 'crossbow_light', 'ranged'),
  ('d_slip_the_guard', null, 'Slip the Guard', 'slip the guard', 'Knife Work', 'Class 1', 1, '1d4', 19, 1, '🗡️ Slip the Guard: the point goes where the armour is not — treat the target''s armour as 2 AC lower.', 'dagger', 'melee'),
  ('d_opened_vein', null, 'Opened Vein', 'opened vein', 'Knife Work', 'Class 1', 3, '1d4', 19, 1, '🩸 Opened Vein: a draw cut across an exposed limb — the target bleeds 1d4 per turn (2d4 on a crit) until treated.', 'dagger', 'melee'),
  ('d_throat_reach', null, 'Throat Reach', 'throat reach', 'Knife Work', 'Class 2', 5, '3d4', 19, 1, '🎯 Throat Reach: taken only against a target that cannot see you — crits on 19-20 and on a crit the target makes a CON save (DC 8 + prof + STR) or cannot speak or cast with a verbal component until healed.', 'dagger', 'melee'),
  ('d_flick', null, 'Flick', 'flick', 'Thrown Work', 'Class 1', 1, '1d4', 19, 1, '⚡ Flick: barely a windup, and already in the air.', 'dart', 'ranged'),
  ('d_weighted_dart', null, 'Weighted Dart', 'weighted dart', 'Thrown Work', 'Class 1', 3, '1d4', 19, 1, '🩸 Weighted Dart: the target bleeds 1d4 per turn until treated.', 'dart', 'ranged'),
  ('d_handful', null, 'Handful', 'handful', 'Thrown Work', 'Class 2', 5, '2d4', 19, 1, '✨ Handful: three thrown as one — if a second enemy is within 5 ft of the target it takes 1d4 piercing.', 'dart', 'ranged'),
  ('e_armour_seam', null, 'Armour Seam', 'armour seam', 'Sword Work', 'Class 1', 1, '1d10', 19, 1, '⚔️ Armour Seam: a stiff spike made for the gaps rather than the plate — treat the target''s armour as 4 AC lower.', 'estoc', 'melee'),
  ('e_two_hand_drive', null, 'Two-Hand Drive', 'two-hand drive', 'Sword Work', 'Class 1', 3, '1d10', 19, 1, '➡️ Two-Hand Drive: the point set and the whole body behind it — the target makes a STR save (DC 8 + prof + STR) or is pushed 5 ft and its speed is 0 until the end of its next turn.', 'estoc', 'melee'),
  ('e_through_the_maille', null, 'Through the Maille', 'through the maille', 'Sword Work', 'Class 2', 5, '3d8', 18, 1, '🎯 Through the Maille: what the weapon exists for — against a target wearing medium or heavy armour, crits on 18-20 and treat its armour as 6 AC lower.', 'estoc', 'melee'),
  ('f_cleaving_cut', null, 'Cleaving Cut', 'cleaving cut', 'Sword Work', 'Class 1', 1, '1d8', 20, 1, '🪓 Cleaving Cut: a chopper with a sword''s balance.', 'falchion', 'melee'),
  ('f_shear_the_haft', null, 'Shear the Haft', 'shear the haft', 'Sword Work', 'Class 1', 3, '1d8', 19, 1, '🪓 Shear the Haft: a heavy cut at a wooden shaft — the target makes a STR save (DC 8 + prof + STR) or drops a two-handed or hafted weapon.', 'falchion', 'melee'),
  ('f_butcher_s_blow', null, 'Butcher''s Blow', 'butcher''s blow', 'Sword Work', 'Class 2', 5, '2d8', 19, 2, '🩸 Butcher''s Blow: weight, edge and no subtlety — the target bleeds 2d4 per turn until treated. Fumbles on 1-2.', 'falchion', 'melee'),
  ('f_round_the_shield', null, 'Round the Shield', 'round the shield', 'Flail Work', 'Class 1', 1, '1d8', 20, 1, '🛡️ Round the Shield: the head travels where the shield is not — if the target carries a shield it gains no AC from it against this attack.', 'flail', 'melee'),
  ('f_wrap_the_weapon', null, 'Wrap the Weapon', 'wrap the weapon', 'Flail Work', 'Class 1', 3, '1d8', 20, 1, '🧲 Wrap the Weapon: the chain binds a blade — the target makes a STR save (DC 8 + prof + STR) or drops what it holds.', 'flail', 'melee'),
  ('f_skull_ring', null, 'Skull Ring', 'skull ring', 'Flail Work', 'Class 2', 5, '2d8', 19, 1, '🔔 Skull Ring: the head comes over the top — the target makes a CON save (DC 8 + prof + STR) or is stunned until the end of its next turn.', 'flail', 'melee'),
  ('fm_flange_bite', null, 'Flange Bite', 'flange bite', 'Mace Work', 'Class 1', 1, '1d8', 20, 1, '⚔️ Flange Bite: ridges concentrate the blow instead of spreading it — treat the target''s armour as 3 AC lower.', 'flanged_mace', 'melee'),
  ('fm_numb_the_arm', null, 'Numb the Arm', 'numb the arm', 'Mace Work', 'Class 1', 3, '1d8', 20, 1, '🦾 Numb the Arm: a blow to the shoulder — the target makes a CON save (DC 8 + prof + STR) or has disadvantage on its next attack.', 'flanged_mace', 'melee'),
  ('fm_crush_the_helm', null, 'Crush the Helm', 'crush the helm', 'Mace Work', 'Class 2', 5, '2d8', 19, 1, '⛑️ Crush the Helm: the target makes a CON save (DC 8 + prof + STR) or is stunned until the end of its next turn.', 'flanged_mace', 'melee'),
  ('f_short_chop', null, 'Short Chop', 'short chop', 'Axe Work', 'Class 1', 1, '1d6', 20, 1, '🪓 Short Chop: a close-quarters blow from a throwing axe kept in hand.', 'francisca', 'melee'),
  ('f_shield_splitter', null, 'Shield Splitter', 'shield splitter', 'Axe Work', 'Class 1', 3, '1d8', 20, 1, '🛡️ Shield Splitter: the wedge head bites into a shield rim and stays — the target''s shield gives no AC until it spends an action to work the axe free.', 'francisca', 'melee'),
  ('f_volley_step', null, 'Volley Step', 'volley step', 'Axe Work', 'Class 2', 5, '2d6', 19, 1, '🎯 Volley Step: the Frankish habit — close, strike, and be inside the guard before the shield comes back. Crits on 19-20.', 'francisca', 'melee'),
  ('g_shield_wall_thrust', null, 'Shield Wall Thrust', 'shield wall thrust', 'Sword Work', 'Class 1', 1, '1d6', 19, 1, '🛡️ Shield Wall Thrust: the legion''s answer — you gain +2 AC until the start of your next turn if an ally is adjacent to you.', 'gladius', 'melee'),
  ('g_gut_thrust', null, 'Gut Thrust', 'gut thrust', 'Sword Work', 'Class 1', 3, '1d6', 19, 1, '🩸 Gut Thrust: short and low, under the ribs — the target bleeds 1d6 per turn until treated.', 'gladius', 'melee'),
  ('g_step_and_kill', null, 'Step and Kill', 'step and kill', 'Sword Work', 'Class 2', 5, '2d6', 19, 1, '🎯 Step and Kill: one pace inside the reach of a longer weapon — against a target wielding a two-handed or reach weapon, crits on 18-20.', 'gladius', 'melee'),
  ('g_sweeping_cut', null, 'Sweeping Cut', 'sweeping cut', 'Polearm Drill', 'Class 1', 1, '1d10', 20, 1, '🌀 Sweeping Cut: a blade on a pole, used as a blade.', 'glaive', 'melee'),
  ('g_keep_the_line', null, 'Keep the Line', 'keep the line', 'Polearm Drill', 'Class 1', 3, '1d10', 20, 1, '🛑 Keep the Line: a cut that says no further — the target makes a STR save (DC 8 + prof + STR) or cannot move closer to you until the end of its next turn.', 'glaive', 'melee'),
  ('g_reaping_arc', null, 'Reaping Arc', 'reaping arc', 'Polearm Drill', 'Class 2', 5, '2d10', 19, 2, '🌾 Reaping Arc: a full turn at the end of the haft — every enemy within 10 ft takes 1d10 slashing. Fumbles on 1-2.', 'glaive', 'melee'),
  ('g_hewing_blow', null, 'Hewing Blow', 'hewing blow', 'Axe Work', 'Class 1', 1, '1d12', 20, 1, '🪓 Hewing Blow: the simplest thing in the armoury and among the worst to be hit by.', 'greataxe', 'melee'),
  ('g_wide_arc', null, 'Wide Arc', 'wide arc', 'Axe Work', 'Class 1', 3, '1d12', 20, 2, '🌀 Wide Arc: every other enemy within 5 ft takes 1d8 slashing. Fumbles on 1-2.', 'greataxe', 'melee'),
  ('g_executioner_s_drop', null, 'Executioner''s Drop', 'executioner''s drop', 'Axe Work', 'Class 2', 5, '3d10', 19, 2, '⬇️ Executioner''s Drop: raised overhead and given everything — against a prone or restrained target, crits on 18-20. Fumbles on 1-2.', 'greataxe', 'melee'),
  ('g_overhead_fell', null, 'Overhead Fell', 'overhead fell', 'Club Work', 'Class 1', 1, '1d8', 20, 1, '⬇️ Overhead Fell: raised and dropped like an axe on a log.', 'greatclub', 'melee'),
  ('g_hip_toss', null, 'Hip Toss', 'hip toss', 'Club Work', 'Class 1', 3, '1d8', 20, 1, '⬇️ Hip Toss: a low sweeping blow — a Large or smaller target makes a STR save (DC 8 + prof + STR) or is knocked prone.', 'greatclub', 'melee'),
  ('g_bone_breaker', null, 'Bone Breaker', 'bone breaker', 'Club Work', 'Class 2', 5, '2d8', 20, 2, '🦴 Bone Breaker: a committed two-handed swing at a limb — the target makes a CON save (DC 8 + prof + STR) or its speed is halved until it is healed. Fumbles on 1-2.', 'greatclub', 'melee'),
  ('g_descending_cut', null, 'Descending Cut', 'descending cut', 'Sword Work', 'Class 1', 1, '2d6', 20, 1, '⬇️ Descending Cut: from the roof guard, straight down.', 'greatsword', 'melee'),
  ('g_zwerchhau', null, 'Zwerchhau', 'zwerchhau', 'Sword Work', 'Class 1', 3, '2d6', 19, 1, '↔️ Zwerchhau: a horizontal strike that answers a descending one — taken against a target that hit you since your last turn, and crits on 19-20.', 'greatsword', 'melee'),
  ('g_blossom_strike', null, 'Blossom Strike', 'blossom strike', 'Sword Work', 'Class 2', 5, '3d6', 19, 2, '🌸 Blossom Strike: every enemy within 5 ft takes 2d6 slashing. Committed and open: fumbles on 1-2.', 'greatsword', 'melee'),
  ('g_hooking_cut', null, 'Hooking Cut', 'hooking cut', 'Polearm Drill', 'Class 1', 1, '2d4', 20, 1, '🪝 Hooking Cut: the point of a guisarme is the hook, not the blade.', 'guisarme', 'melee'),
  ('g_unhorse', null, 'Unhorse', 'unhorse', 'Polearm Drill', 'Class 1', 3, '2d4', 20, 1, '🐴 Unhorse: the hook takes a leg or a stirrup — a mounted or Large or smaller target makes a STR save (DC 8 + prof + STR) or is knocked prone.', 'guisarme', 'melee'),
  ('g_drag_to_the_ground', null, 'Drag to the Ground', 'drag to the ground', 'Polearm Drill', 'Class 2', 5, '3d4', 19, 1, '⬇️ Drag to the Ground: hooked and hauled — the target makes a STR save (DC 8 + prof + STR) or is pulled 10 ft toward you and knocked prone.', 'guisarme', 'melee'),
  ('h_axe_head', null, 'Axe Head', 'axe head', 'Polearm Drill', 'Class 1', 1, '1d10', 20, 1, '🪓 Axe Head: the cutting edge of a weapon that has three answers.', 'halberd', 'melee'),
  ('h_back_spike', null, 'Back Spike', 'back spike', 'Polearm Drill', 'Class 1', 3, '1d10', 20, 1, '⚔️ Back Spike: turned to the rear spike — damage becomes piercing and treat the target''s armour as 4 AC lower.', 'halberd', 'melee'),
  ('h_hook_and_drop', null, 'Hook and Drop', 'hook and drop', 'Polearm Drill', 'Class 2', 5, '2d8', 20, 1, '⬇️ Hook and Drop: the fluke catches a rider or a shield rim and pulls — a Large or smaller target makes a STR save (DC 8 + prof + STR) or is knocked prone.', 'halberd', 'melee'),
  ('h_hook_the_rim', null, 'Hook the Rim', 'hook the rim', 'Axe Work', 'Class 1', 1, '1d6', 20, 1, '🪝 Hook the Rim: the beard catches a shield and drags it aside — if the target carries a shield it loses that shield''s AC until the start of its next turn.', 'handaxe', 'melee'),
  ('h_split_grip', null, 'Split Grip', 'split grip', 'Axe Work', 'Class 1', 3, '1d6', 19, 1, '🖐️ Split Grip: a chop at the weapon hand — the target makes a DEX save (DC 8 + prof + STR) or drops what it holds.', 'handaxe', 'melee'),
  ('h_cleave_down', null, 'Cleave Down', 'cleave down', 'Axe Work', 'Class 2', 5, '2d6', 19, 2, '⬇️ Cleave Down: everything committed to one falling blow. Fumbles on 1-2.', 'handaxe', 'melee'),
  ('hp_saddle_strike', null, 'Saddle Strike', 'saddle strike', 'Pick Work', 'Class 1', 1, '1d6', 20, 1, '🐴 Saddle Strike: short, one-handed, and meant to be swung from horseback — treat the target''s armour as 3 AC lower.', 'horsemans_pick', 'melee'),
  ('hp_pull_from_the_saddle', null, 'Pull from the Saddle', 'pull from the saddle', 'Pick Work', 'Class 1', 3, '1d6', 20, 1, '⬇️ Pull from the Saddle: the spike hooks and hauls — a mounted or Large or smaller target makes a STR save (DC 8 + prof + STR) or is knocked prone.', 'horsemans_pick', 'melee'),
  ('hp_rapid_pecks', null, 'Rapid Pecks', 'rapid pecks', 'Pick Work', 'Class 2', 5, '2d6', 19, 1, '⚡ Rapid Pecks: three short blows where one would do — crits on 19-20 and on a crit the target bleeds 1d6 per turn until treated.', 'horsemans_pick', 'melee'),
  ('j_short_thrust', null, 'Short Thrust', 'short thrust', 'Spear Work', 'Class 1', 1, '1d6', 20, 1, '➡️ Short Thrust: held near the head and used as a stabbing spear.', 'javelin', 'melee'),
  ('j_keep_away', null, 'Keep Away', 'keep away', 'Spear Work', 'Class 1', 3, '1d6', 20, 1, '↩️ Keep Away: a thrust that buys distance — the target makes a STR save (DC 8 + prof + STR) or cannot move closer to you until the end of its next turn.', 'javelin', 'melee'),
  ('j_braced_point', null, 'Braced Point', 'braced point', 'Spear Work', 'Class 2', 5, '2d6', 19, 1, '🧿 Braced Point: the butt set against the ground — against a target that moved toward you this turn, crits on 19-20.', 'javelin', 'melee'),
  ('k_hook_the_shield', null, 'Hook the Shield', 'hook the shield', 'Hooked Blades', 'Class 1', 1, '1d8', 20, 1, '🪝 Hook the Shield: the inner curve catches the rim and pulls — if the target carries a shield it loses that shield''s AC until the start of its next turn.', 'khopesh', 'melee'),
  ('k_catch_the_arm', null, 'Catch the Arm', 'catch the arm', 'Hooked Blades', 'Class 1', 3, '1d8', 20, 1, '🦾 Catch the Arm: the hook takes a limb rather than a weapon — the target makes a STR save (DC 8 + prof + STR) or is grappled until it breaks free.', 'khopesh', 'melee'),
  ('k_pull_and_open', null, 'Pull and Open', 'pull and open', 'Hooked Blades', 'Class 2', 5, '2d8', 19, 1, '⬅️ Pull and Open: the hook drags the guard aside and the edge goes through what is behind it — treat the target''s armour as 4 AC lower.', 'khopesh', 'melee'),
  ('l_couched_point', null, 'Couched Point', 'couched point', 'Mounted Drill', 'Class 1', 1, '1d12', 20, 1, '🐴 Couched Point: tucked under the arm and aimed by the horse.', 'lance', 'melee'),
  ('l_unseat', null, 'Unseat', 'unseat', 'Mounted Drill', 'Class 1', 3, '1d12', 20, 1, '⬇️ Unseat: struck at the rider rather than the mount — a mounted target makes a STR save (DC 8 + prof + STR) or is knocked from its saddle and lands prone.', 'lance', 'melee'),
  ('l_full_charge', null, 'Full Charge', 'full charge', 'Mounted Drill', 'Class 2', 5, '3d12', 19, 2, '💥 Full Charge: horse, rider and lance as one weight — usable only if you moved at least 20 ft toward the target this turn. Fumbles on 1-2.', 'lance', 'melee'),
  ('l_full_draw', null, 'Full Draw', 'full draw', 'Archery', 'Class 1', 1, '1d8', 20, 1, '💪 Full Draw: a hundred pounds held at the ear for as long as the aim takes.', 'longbow', 'ranged'),
  ('l_bodkin_point', null, 'Bodkin Point', 'bodkin point', 'Archery', 'Class 1', 3, '1d8', 20, 1, '⚔️ Bodkin Point: a narrow armour-piercing head — treat the target''s armour as 4 AC lower.', 'longbow', 'ranged'),
  ('l_arcing_volley', null, 'Arcing Volley', 'arcing volley', 'Archery', 'Class 2', 5, '2d8', 19, 1, '📏 Arcing Volley: loosed high to fall behind cover — ignores half and three-quarters cover, and crits on 19-20.', 'longbow', 'ranged'),
  ('l_bind_and_strike', null, 'Bind and Strike', 'bind and strike', 'Sword Work', 'Class 1', 1, '1d8', 20, 1, '🔁 Bind and Strike: the blades meet, yours wins the leverage — if the target attacked you since your last turn, treat its armour as 2 AC lower.', 'longsword', 'melee'),
  ('l_half_sword', null, 'Half-Sword', 'half-sword', 'Sword Work', 'Class 1', 3, '1d8', 19, 1, '⚔️ Half-Sword: the off hand grips the blade to drive the point into a gap — treat the target''s armour as 4 AC lower. Crits on 19-20.', 'longsword', 'melee'),
  ('l_mordhau', null, 'Mordhau', 'mordhau', 'Sword Work', 'Class 2', 5, '2d6', 20, 1, '🔨 Mordhau: the sword reversed and the crossguard swung as a hammer — damage becomes bludgeoning and the target makes a CON save (DC 8 + prof + STR) or is stunned until the end of its next turn.', 'longsword', 'melee'),
  ('m_flanged_blow', null, 'Flanged Blow', 'flanged blow', 'Mace Work', 'Class 1', 1, '1d6', 20, 1, '⚔️ Flanged Blow: force through plate rather than edge against it — treat the target''s armour as 2 AC lower.', 'mace', 'melee'),
  ('m_ring_the_helm', null, 'Ring the Helm', 'ring the helm', 'Mace Work', 'Class 1', 3, '1d6', 19, 1, '🔔 Ring the Helm: a blow to the head that does its damage through the helmet — the target makes a CON save (DC 8 + prof + STR) or is stunned until the end of its next turn.', 'mace', 'melee'),
  ('m_shatter_the_arm', null, 'Shatter the Arm', 'shatter the arm', 'Mace Work', 'Class 2', 5, '2d6', 20, 1, '🦴 Shatter the Arm: a heavy blow to the shield arm — the target makes a CON save (DC 8 + prof + STR) or drops its shield and has disadvantage on attacks until healed.', 'mace', 'melee'),
  ('m_sledge_blow', null, 'Sledge Blow', 'sledge blow', 'Hammer Work', 'Class 1', 1, '2d6', 20, 1, '🔨 Sledge Blow: a smith''s hammer scaled up to the size of a man.', 'maul', 'melee'),
  ('m_ground_shatter', null, 'Ground Shatter', 'ground shatter', 'Hammer Work', 'Class 1', 3, '2d6', 20, 1, '⬇️ Ground Shatter: struck at the earth beside a target — it makes a DEX save (DC 8 + prof + STR) or is knocked prone.', 'maul', 'melee'),
  ('m_anvil_fall', null, 'Anvil Fall', 'anvil fall', 'Hammer Work', 'Class 2', 5, '3d6', 19, 2, '💥 Anvil Fall: raised over the head and dropped — the target makes a CON save (DC 8 + prof + STR) or is stunned until the end of its next turn. Fumbles on 1-2.', 'maul', 'melee'),
  ('mf_two_points', null, 'Two Points', 'two points', 'Polearm Drill', 'Class 1', 1, '1d8', 20, 1, '➡️ Two Points: a hay fork that stayed in the muster roll for three hundred years.', 'military_fork', 'melee'),
  ('mf_fork_the_weapon', null, 'Fork the Weapon', 'fork the weapon', 'Polearm Drill', 'Class 1', 3, '1d8', 20, 1, '🧲 Fork the Weapon: the tines close on a haft — the target makes a STR save (DC 8 + prof + STR) or drops a two-handed or hafted weapon.', 'military_fork', 'melee'),
  ('mf_hold_at_length', null, 'Hold at Length', 'hold at length', 'Polearm Drill', 'Class 2', 5, '2d8', 19, 1, '🛑 Hold at Length: braced against the body — the target cannot move closer to you until it makes a STR save (DC 8 + prof + STR) at the end of its turn.', 'military_fork', 'melee'),
  ('m_spiked_blow', null, 'Spiked Blow', 'spiked blow', 'Mace Work', 'Class 1', 1, '1d8', 20, 1, '⚔️ Spiked Blow: a mace that also punctures — treat the target''s armour as 2 AC lower.', 'morningstar', 'melee'),
  ('m_tear_free', null, 'Tear Free', 'tear free', 'Mace Work', 'Class 1', 3, '1d8', 20, 1, '🩸 Tear Free: the spikes come out slower than they went in — the target bleeds 1d6 per turn until treated.', 'morningstar', 'melee'),
  ('m_full_swing', null, 'Full Swing', 'full swing', 'Mace Work', 'Class 2', 5, '2d8', 19, 1, '💥 Full Swing: all of it, into one place — the target makes a CON save (DC 8 + prof + STR) or has disadvantage on attacks until the end of its next turn.', 'morningstar', 'melee'),
  ('p_point_in_line', null, 'Point in Line', 'point in line', 'Polearm Drill', 'Class 1', 1, '1d10', 20, 1, '📏 Point in Line: eighteen feet of ash and a foot of steel.', 'pike', 'melee'),
  ('p_hedgehog', null, 'Hedgehog', 'hedgehog', 'Polearm Drill', 'Class 1', 3, '1d10', 20, 1, '🦔 Hedgehog: set with an ally beside you — you gain +2 AC until the start of your next turn if an ally is adjacent.', 'pike', 'melee'),
  ('p_receive_the_charge', null, 'Receive the Charge', 'receive the charge', 'Polearm Drill', 'Class 2', 5, '3d8', 18, 1, '🧿 Receive the Charge: the butt in the earth — against a target that moved toward you this turn, crits on 18-20 and it makes a STR save (DC 8 + prof + STR) or is knocked prone.', 'pike', 'melee'),
  ('q_snap_strike', null, 'Snap Strike', 'snap strike', 'Staff Work', 'Class 1', 1, '1d6', 20, 1, '⚡ Snap Strike: one end and then the other, faster than either looks.', 'quarterstaff', 'melee'),
  ('q_sweep_the_ankles', null, 'Sweep the Ankles', 'sweep the ankles', 'Staff Work', 'Class 1', 3, '1d6', 20, 1, '⬇️ Sweep the Ankles: a low strike below the shield — a Large or smaller target makes a DEX save (DC 8 + prof + STR) or is knocked prone.', 'quarterstaff', 'melee'),
  ('q_both_ends', null, 'Both Ends', 'both ends', 'Staff Work', 'Class 2', 5, '2d6', 19, 1, '🔄 Both Ends: a full rotation striking twice in one motion — if a second enemy is within 5 ft it takes 1d6 bludgeoning.', 'quarterstaff', 'melee'),
  ('r_long_thrust', null, 'Long Thrust', 'long thrust', 'Polearm Drill', 'Class 1', 1, '2d4', 20, 1, '➡️ Long Thrust: a spear with opinions.', 'ranseur', 'melee'),
  ('r_catch_the_blade', null, 'Catch the Blade', 'catch the blade', 'Polearm Drill', 'Class 1', 3, '2d4', 20, 1, '🧲 Catch the Blade: the side prongs exist to trap a weapon — the target makes a STR save (DC 8 + prof + STR) or drops what it holds.', 'ranseur', 'melee'),
  ('r_pin_the_line', null, 'Pin the Line', 'pin the line', 'Polearm Drill', 'Class 2', 5, '3d4', 19, 1, '📌 Pin the Line: the prongs set against the body — the target''s speed is 0 until it makes a STR save (DC 8 + prof + STR) at the end of its turn.', 'ranseur', 'melee'),
  ('r_lunge', null, 'Lunge', 'lunge', 'Sword Work', 'Class 1', 1, '1d8', 19, 1, '➡️ Lunge: the whole body behind a point that was already there.', 'rapier', 'melee'),
  ('r_disengage_and_riposte', null, 'Disengage and Riposte', 'disengage and riposte', 'Sword Work', 'Class 1', 3, '1d8', 19, 1, '🔄 Disengage and Riposte: taken against a target that missed you since your last turn — treat its armour as 3 AC lower.', 'rapier', 'melee'),
  ('r_thrust_to_the_eye', null, 'Thrust to the Eye', 'thrust to the eye', 'Sword Work', 'Class 2', 5, '2d8', 18, 1, '👁️ Thrust to the Eye: nothing but the point and a very small target — crits on 18-20, and on a crit the target is blinded until it is healed.', 'rapier', 'melee'),
  ('s_draw_cut', null, 'Draw Cut', 'draw cut', 'Sword Work', 'Class 1', 1, '1d6', 19, 1, '🩸 Draw Cut: the curve does the work on the way past — the target bleeds 1d4 per turn until treated.', 'scimitar', 'melee'),
  ('s_rising_slash', null, 'Rising Slash', 'rising slash', 'Sword Work', 'Class 1', 3, '1d6', 19, 1, '↗️ Rising Slash: up from the hip and inside the guard — the target makes a DEX save (DC 8 + prof + DEX) or drops one held item.', 'scimitar', 'melee'),
  ('s_whirl', null, 'Whirl', 'whirl', 'Sword Work', 'Class 2', 5, '2d6', 19, 1, '🌀 Whirl: a turning cut through everything in reach — every other enemy within 5 ft takes 1d6 slashing.', 'scimitar', 'melee'),
  ('s_backhand_draw', null, 'Backhand Draw', 'backhand draw', 'Knife Work', 'Class 1', 1, '1d6', 20, 1, '↩️ Backhand Draw: the broad back of the blade turns a parry aside on the way in.', 'seax', 'melee'),
  ('s_rip', null, 'Rip', 'rip', 'Knife Work', 'Class 1', 3, '1d6', 19, 1, '🩸 Rip: the heavy single edge tears rather than cuts — the target bleeds 1d4 per turn until treated.', 'seax', 'melee'),
  ('s_seax_and_shield', null, 'Seax and Shield', 'seax and shield', 'Knife Work', 'Class 2', 5, '2d6', 20, 1, '🛡️ Seax and Shield: fought close behind a shield — you gain +2 AC until the start of your next turn. Requires a shield equipped.', 'seax', 'melee'),
  ('s_quick_loose', null, 'Quick Loose', 'quick loose', 'Archery', 'Class 1', 1, '1d6', 20, 1, '⚡ Quick Loose: drawn and released in one motion.', 'shortbow', 'ranged'),
  ('s_leg_shot', null, 'Leg Shot', 'leg shot', 'Archery', 'Class 1', 3, '1d6', 20, 1, '🦵 Leg Shot: the target makes a CON save (DC 8 + prof + DEX) or its speed is halved until healed.', 'shortbow', 'ranged'),
  ('s_double_draw', null, 'Double Draw', 'double draw', 'Archery', 'Class 2', 5, '2d6', 19, 1, '🎯 Double Draw: two arrows nocked and loosed together — crits on 19-20 and on a crit the target bleeds 1d6 per turn until treated.', 'shortbow', 'ranged'),
  ('s_quick_thrust', null, 'Quick Thrust', 'quick thrust', 'Sword Work', 'Class 1', 1, '1d6', 19, 1, '⚡ Quick Thrust: short, straight and back on guard before the reply.', 'shortsword', 'melee'),
  ('s_underarm', null, 'Underarm', 'underarm', 'Sword Work', 'Class 1', 3, '1d6', 19, 1, '🗡️ Underarm: the point finds the gap under a raised arm — treat the target''s armour as 3 AC lower.', 'shortsword', 'melee'),
  ('s_two_blades', null, 'Two Blades', 'two blades', 'Sword Work', 'Class 2', 5, '2d6', 19, 1, '⚔️ Two Blades: a second short blade in the off hand — if a second enemy is within 5 ft it takes 1d6 piercing. Requires a light weapon in the other hand.', 'shortsword', 'melee'),
  ('s_inside_cut', null, 'Inside Cut', 'inside cut', 'Hooked Blades', 'Class 1', 1, '1d4', 19, 1, '🪝 Inside Cut: the curve reaches past a guard that a straight blade cannot.', 'sickle', 'melee'),
  ('s_hamstring', null, 'Hamstring', 'hamstring', 'Hooked Blades', 'Class 1', 3, '1d4', 19, 1, '🦵 Hamstring: a hooking cut behind the knee — the target makes a CON save (DC 8 + prof + STR) or its speed is halved until healed.', 'sickle', 'melee'),
  ('s_reap', null, 'Reap', 'reap', 'Hooked Blades', 'Class 2', 5, '2d4', 19, 1, '🌾 Reap: a low sweeping pull — a Medium or smaller target makes a DEX save (DC 8 + prof + STR) or is knocked prone, and bleeds 1d4 per turn until treated.', 'sickle', 'melee'),
  ('s_whirl_and_loose', null, 'Whirl and Loose', 'whirl and loose', 'Sling Work', 'Class 1', 1, '1d4', 20, 1, '🌀 Whirl and Loose: a cord, a stone, and more range than either deserves.', 'sling', 'ranged'),
  ('s_temple_shot', null, 'Temple Shot', 'temple shot', 'Sling Work', 'Class 1', 3, '1d4', 20, 1, '🔔 Temple Shot: the target makes a CON save (DC 8 + prof + DEX) or is stunned until the end of its next turn.', 'sling', 'ranged'),
  ('s_lead_bullet', null, 'Lead Bullet', 'lead bullet', 'Sling Work', 'Class 2', 5, '2d4', 19, 1, '⚔️ Lead Bullet: cast lead rather than a river stone — treat the target''s armour as 3 AC lower and crits on 19-20.', 'sling', 'ranged'),
  ('s_measured_thrust', null, 'Measured Thrust', 'measured thrust', 'Spear Work', 'Class 1', 1, '1d6', 20, 1, '📏 Measured Thrust: the spear''s advantage is that it arrives first.', 'spear', 'melee'),
  ('s_shield_wall_jab', null, 'Shield Wall Jab', 'shield wall jab', 'Spear Work', 'Class 1', 3, '1d6', 20, 1, '🛡️ Shield Wall Jab: struck from behind cover — you gain +2 AC until the start of your next turn if an ally is adjacent to you.', 'spear', 'melee'),
  ('s_set_against_the_charge', null, 'Set Against the Charge', 'set against the charge', 'Spear Work', 'Class 2', 5, '2d6', 19, 1, '🧿 Set Against the Charge: the butt in the earth and the point at chest height — against a target that moved toward you this turn, crits on 19-20 and it makes a STR save (DC 8 + prof + STR) or is knocked prone.', 'spear', 'melee'),
  ('ss_long_cast', null, 'Long Cast', 'long cast', 'Sling Work', 'Class 1', 1, '1d6', 20, 1, '📏 Long Cast: the staff doubles the arm and quadruples the range.', 'staff_sling', 'ranged'),
  ('ss_lobbed_shot', null, 'Lobbed Shot', 'lobbed shot', 'Sling Work', 'Class 1', 3, '1d6', 20, 1, '🎯 Lobbed Shot: thrown in a high arc — ignores half and three-quarters cover.', 'staff_sling', 'ranged'),
  ('ss_siege_stone', null, 'Siege Stone', 'siege stone', 'Sling Work', 'Class 2', 5, '2d6', 19, 1, '🪨 Siege Stone: a stone that needs two hands to load — a Medium or smaller target makes a STR save (DC 8 + prof + DEX) or is knocked prone.', 'staff_sling', 'ranged'),
  ('t_three_points', null, 'Three Points', 'three points', 'Spear Work', 'Class 1', 1, '1d6', 20, 1, '➡️ Three Points: one of them finds a gap.', 'trident', 'melee'),
  ('t_trap_the_blade', null, 'Trap the Blade', 'trap the blade', 'Spear Work', 'Class 1', 3, '1d6', 20, 1, '🧲 Trap the Blade: the outer tines close on a weapon — the target makes a STR save (DC 8 + prof + STR) or drops what it holds.', 'trident', 'melee'),
  ('t_pin_to_the_ground', null, 'Pin to the Ground', 'pin to the ground', 'Spear Work', 'Class 2', 5, '2d6', 19, 1, '📌 Pin to the Ground: driven through and held — a prone target''s speed is 0 until it makes a STR save (DC 8 + prof + STR) at the end of its turn.', 'trident', 'melee'),
  ('v_chopping_blow', null, 'Chopping Blow', 'chopping blow', 'Polearm Drill', 'Class 1', 1, '1d10', 20, 1, '🪓 Chopping Blow: a cleaver lashed to a pole, which is exactly what it was.', 'voulge', 'melee'),
  ('v_split_the_shield', null, 'Split the Shield', 'split the shield', 'Polearm Drill', 'Class 1', 3, '1d10', 20, 1, '🛡️ Split the Shield: the target makes a STR save (DC 8 + prof + STR) or its shield gives no AC until it is repaired.', 'voulge', 'melee'),
  ('v_overhand_fell', null, 'Overhand Fell', 'overhand fell', 'Polearm Drill', 'Class 2', 5, '2d10', 19, 2, '⬇️ Overhand Fell: from full height, with the whole haft behind it. Fumbles on 1-2.', 'voulge', 'melee'),
  ('wf_threshing_blow', null, 'Threshing Blow', 'threshing blow', 'Flail Work', 'Class 1', 1, '1d10', 20, 1, '🌾 Threshing Blow: a grain flail grown up — the head arrives after the swing has stopped.', 'war_flail', 'melee'),
  ('wf_over_the_wall', null, 'Over the Wall', 'over the wall', 'Flail Work', 'Class 1', 3, '1d10', 20, 1, '🛡️ Over the Wall: struck down past a raised shield — the target gains no AC from a shield against this attack and makes a DEX save (DC 8 + prof + STR) or is knocked prone.', 'war_flail', 'melee'),
  ('wf_whirlwind', null, 'Whirlwind', 'whirlwind', 'Flail Work', 'Class 2', 5, '2d10', 19, 2, '🌀 Whirlwind: momentum kept and spent — every other enemy within 5 ft takes 1d10 bludgeoning. Fumbles on 1-2.', 'war_flail', 'melee'),
  ('wp_punch_through', null, 'Punch Through', 'punch through', 'Pick Work', 'Class 1', 1, '1d8', 20, 1, '⚔️ Punch Through: a spike concentrates a blow into a square inch — treat the target''s armour as 4 AC lower.', 'war_pick', 'melee'),
  ('wp_set_and_wrench', null, 'Set and Wrench', 'set and wrench', 'Pick Work', 'Class 1', 3, '1d8', 20, 1, '🔧 Set and Wrench: the spike buried and turned — the target bleeds 1d6 per turn and removing the pick costs it an action.', 'war_pick', 'melee'),
  ('wp_helm_splitter', null, 'Helm Splitter', 'helm splitter', 'Pick Work', 'Class 2', 5, '2d8', 19, 1, '⛑️ Helm Splitter: driven down through the crown — the target makes a CON save (DC 8 + prof + STR) or is stunned until the end of its next turn.', 'war_pick', 'melee'),
  ('ws_upturned_blade', null, 'Upturned Blade', 'upturned blade', 'Polearm Drill', 'Class 1', 1, '2d4', 19, 1, '🌾 Upturned Blade: a harvest tool reforged point-up, which is the only change it needed.', 'war_scythe', 'melee'),
  ('ws_cut_the_legs', null, 'Cut the Legs', 'cut the legs', 'Polearm Drill', 'Class 1', 3, '2d4', 19, 1, '🦵 Cut the Legs: low and level — the target makes a DEX save (DC 8 + prof + STR) or its speed is halved until healed, and bleeds 1d4 per turn.', 'war_scythe', 'melee'),
  ('ws_harvest', null, 'Harvest', 'harvest', 'Polearm Drill', 'Class 2', 5, '3d4', 19, 2, '🌀 Harvest: the long sweep it was shaped for — every enemy within 10 ft takes 2d4 slashing. Fumbles on 1-2.', 'war_scythe', 'melee'),
  ('w_dent_the_plate', null, 'Dent the Plate', 'dent the plate', 'Hammer Work', 'Class 1', 1, '1d8', 20, 1, '⚔️ Dent the Plate: force carried through steel — treat the target''s armour as 3 AC lower.', 'warhammer', 'melee'),
  ('w_beak_side', null, 'Beak Side', 'beak side', 'Hammer Work', 'Class 1', 3, '1d8', 19, 1, '🪝 Beak Side: turned to use the back spike — damage becomes piercing and treat the target''s armour as 5 AC lower.', 'warhammer', 'melee'),
  ('w_two_hand_drive', null, 'Two-Hand Drive', 'two-hand drive', 'Hammer Work', 'Class 2', 5, '2d8', 20, 1, '⬇️ Two-Hand Drive: both hands and a full step — a Large or smaller target makes a STR save (DC 8 + prof + STR) or is pushed 10 ft and knocked prone.', 'warhammer', 'melee'),
  ('w_crack', null, 'Crack', 'crack', 'Whip Work', 'Class 1', 1, '1d4', 19, 1, '⚡ Crack: the tip breaks the sound barrier before it breaks skin.', 'whip', 'melee'),
  ('w_snare_the_ankle', null, 'Snare the Ankle', 'snare the ankle', 'Whip Work', 'Class 1', 3, '1d4', 20, 1, '🧵 Snare the Ankle: the lash wraps a leg — the target makes a DEX save (DC 8 + prof + DEX) or is knocked prone.', 'whip', 'melee'),
  ('w_disarm_at_range', null, 'Disarm at Range', 'disarm at range', 'Whip Work', 'Class 2', 5, '2d4', 19, 1, '🧲 Disarm at Range: the lash takes the weapon and not the hand — the target makes a STR save (DC 8 + prof + DEX) or its weapon lands 10 ft away.', 'whip', 'melee');
