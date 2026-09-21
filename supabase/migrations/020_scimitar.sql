-- =====================================================================
-- 020_scimitar.sql
-- odyssey1e — the weapon that makes the goblin match its own statblock
-- =====================================================================
--
-- 019 gave the goblin a handaxe and it came out at +1 to hit for 1d6-1,
-- where the Monster Manual prints +4 for 1d6+2. Nothing was wrong. A
-- handaxe is lgt and thr but NOT fin, so ability_for gives it STR, and
-- a goblin has STR 8. The book's goblin carries a SCIMITAR, which is
-- finesse, so it swings on DEX 14 instead.
--
-- This adds the scimitar and hands the goblin one. It keeps the axe.
--
-- WHY BOTH, AND WHY THIS IS WORTH A MIGRATION RATHER THAN A FIX. The
-- two weapons sit on one creature with one set of scores, and the only
-- difference between them is a three-letter property:
--
--   Handaxe   lgt thr        STR -1, prof +2  ->  +1   1d6-1
--   Scimitar  fin lgt        DEX +2, prof +2  ->  +4   1d6+2
--
-- Same goblin, same roll path, two answers, and the reason is visible
-- rather than asserted. That is the entire case for 019 having taken
-- scores and real gear over a table of stored to-hit numbers, standing
-- on one roster row where anyone can see it.
--
-- The scimitar is martialM and the goblin is still proficient with it,
-- because npc_items reads NULL proficiency as TRUE - a monster is
-- proficient with what its statblock hands it, and the book agrees.
-- =====================================================================

insert into public.items
  (key, game_id, name, kind, base_item,
   weapon_class, damage_number, damage_denomination, damage_types, properties,
   range_reach, range_value, range_long,
   rarity, price, denom, weight)
values
  ('scimitar', null, 'Scimitar', 'weapon', 'scimitar',
   'martialM', 1, 6, array['slashing']::text[], array['fin','lgt']::text[],
   null, null, null,
   null, 25, 'gp', 3);

insert into public.npc_items (npc_key, item_key, game_id, quantity, equipped)
values ('goblin', 'scimitar', null, 1, true);
