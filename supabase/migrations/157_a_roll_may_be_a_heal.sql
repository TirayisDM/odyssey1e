-- 157. A ROLL MAY BE A HEAL.
--
-- 156 TAUGHT `write_action` TO HANG A HIT POINT EVENT OFF A ROLL WITH
-- role 'heal', AND SAID THIS IN ITS OWN HEADER:
--
--   "'heal' IS A NEW ROLE and nothing else in the schema constrains the
--    column, so no check needs widening."
--
-- THAT WAS WRONG. `rolls_role_check` has constrained it all along:
--
--   CHECK (role IS NULL OR role = ANY (ARRAY['to_hit','damage','check']))
--
-- so the first heal would have failed the insert and taken the whole
-- cast down with it - the action, the roll and the event together,
-- because 012's writer is all-or-nothing and that is the point of it.
--
-- 115 SAYS DO NOT EDIT AN APPLIED MIGRATION, so 156's claim stays where
-- it is and this corrects it. Anybody reading that sentence should read
-- this file next.
--
-- FOUND BY A ROLLED-BACK PROBE rather than by Dave, which is the only
-- reason it is a paragraph instead of a red banner. The probe called
-- `write_action` with a heal row inside a transaction that raises at the
-- end, against the live schema and the live data, and the constraint
-- refused it immediately. Checking a claim about the schema BY ASKING
-- THE SCHEMA costs one query; believing it costs a session.
--
-- ---------------------------------------------------------------------
-- THE VOCABULARY, NOW FOUR WORDS
-- ---------------------------------------------------------------------
--
-- `to_hit` and `check` are d20s that are judged; `damage` and `heal` are
-- the dice that follow from one. 013 made hit points a log of SIGNED
-- deltas so that healing needed no second mechanism, and this is that
-- decision reaching the one column that had not heard about it.
--
-- NULL IS STILL ALLOWED, and still means a roll with no action around it
-- - `rolls_action_role_pair_check` ties those two together and is
-- untouched.

alter table public.rolls drop constraint if exists rolls_role_check;

alter table public.rolls add constraint rolls_role_check
  check (role is null or role = any (array['to_hit', 'damage', 'check', 'heal']));
