-- =====================================================================
-- 048_review_the_armoury.sql
-- odyssey1e — reading every weapon back against its own ladder
-- =====================================================================
--
-- 045 put weapons on the size ladder and priced their slots from it:
--
--   tiny  0.25    sm  0.5    med  1    lg  2
--
-- That rule is right and uniform - the twelve weapons sitting at one
-- slot are all `med`, which is the ladder behaving, not a gap. What a
-- review turns up is not broken slots but four weapons filed at the
-- wrong SIZE, which then priced correctly for a size they should never
-- have had.
--
-- ---------------------------------------------------------------------
-- FOUR MISFILED, ALL THE SAME MISTAKE: judged by how heavy they feel
-- rather than by how much room they take.
--
--   javelin      small -> medium. Two metres of shaft. It is not heavy,
--                which is what `sm` was reading, but you cannot put it
--                in a pack. `spear` is already lg for this reason.
--   shortbow     small -> medium. A four-foot two-handed bow. The
--                longbow is lg and the distinction is worth keeping,
--                so medium rather than large.
--   staff_sling  medium -> large. It is a STAFF. `quarterstaff` is lg
--                and this is the same stick with a pocket on the end.
--                My own row from 042, filed by weight.
--   rapier       small -> medium. A blade longer than a longsword's,
--                filed small because it is light. `longsword` is med.
--
-- TWO-HANDED IS THE TELL. Every other weapon with `two` is lg; the
-- shortbow and the staff sling were the exceptions and neither had a
-- reason. That is a rule worth remembering the next time a weapon is
-- added: if it needs both hands it does not go in a pack.
--
-- ---------------------------------------------------------------------
-- THE NET GETS A WEIGHTED RIM, AND THEREFORE THREE ATTACKS
--
-- 043 left the net with no techniques and said why: it deals no damage,
-- its whole function is `spc`, and three attacks rolling damage for a
-- weapon with no damage would be worse than an honest absence.
--
-- The instruction since is that three is required for every weapon, so
-- the absence has to be resolved rather than explained. It is resolved
-- by giving the net what real ones had: lead weights around the rim.
-- A retiarius net was not a bedsheet - the weights are what make it
-- open in the air and what make it hurt. 1d4 bludgeoning is that, and
-- it makes the net a weapon rather than an exception.
--
-- Its three are about entanglement, because that is still what it is
-- for. The damage is incidental and the dice say so.
-- =====================================================================

update public.items set size = 'med', slots = 1 where key = 'javelin'     and game_id is null;
update public.items set size = 'med', slots = 1 where key = 'shortbow'    and game_id is null;
update public.items set size = 'lg',  slots = 2 where key = 'staff_sling' and game_id is null;
update public.items set size = 'med', slots = 1 where key = 'rapier'      and game_id is null;

-- The rim, and the damage that comes with it.
update public.items
   set damage_number = 1,
       damage_denomination = 4,
       damage_types = array['bludgeoning']::text[]
 where key = 'net' and game_id is null;

comment on column public.items.slots is
  'How much room one of these takes, priced from `size` since 045: tiny 0.25, sm 0.5, med 1, lg 2. Fractional on purpose - a coin is 0.2, so a 5-slot purse holds 25. If a weapon needs both hands it is lg; 048 caught the two that were not.';

insert into public.techniques
  (key, game_id, name, roll_name, category, tier, min_level, dice,
   crit_min, fumble_max, special_text, item_key, mode)
values
  ('n_cast_the_net', null, 'Cast the Net', 'cast the net', 'Net Work', 'Class 1', 1, '1d4', 20, 1,
   'Cast the Net: opened by the weights on its rim and thrown to fall wide. The damage is the lead, not the mesh.',
   'net', 'ranged'),
  ('n_tangle', null, 'Tangle', 'tangle', 'Net Work', 'Class 1', 3, '1d4', 20, 1,
   'Tangle: the mesh closes as it lands - the target makes a DEX save (DC 8 + prof + DEX) or is restrained until it spends an action to cut or wriggle free.',
   'net', 'ranged'),
  ('n_draw_the_cord', null, 'Draw the Cord', 'draw the cord', 'Net Work', 'Class 2', 5, '2d4', 19, 1,
   'Draw the Cord: the retiarius trick - the net thrown and the line hauled in one motion. The target makes a STR save (DC 8 + prof + DEX) or is pulled 10 ft toward you and knocked prone.',
   'net', 'ranged');
