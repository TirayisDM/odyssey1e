-- 185. WHAT ANOTHER SLOT LEVEL BUYS.
--
-- Tarren had four first-level slots, spent all four, and could not cast
-- Magic Missile - correctly. He also had two second-level and three
-- third-level slots sitting unused, and Magic Missile can legally be
-- cast with any of them. There was no way to.
--
-- `at_level` has reached `cast_spell` since 107 and picks which slot is
-- SPENT. Nothing scaled what the spell DOES, and every caller passed
-- null, so the parameter had one value in practice.
--
-- ---------------------------------------------------------------------
-- A COLUMN, BECAUSE THE RIDERS DISAGREE
-- ---------------------------------------------------------------------
--
-- 183 refused to guess this and said so. Here is why: upcasting is not
-- one rule, it is five, and only one of them is arithmetic.
--
--   MORE DICE       Fireball +1d6, Blight +1d8, Magic Missile a whole
--                   DART at 1d4+1. This is the one a machine can do.
--   MORE TARGETS    Bless, Hold Person - "one more target per slot
--                   level above 1st"
--   LONGER          Mass Suggestion - 10 days at 7th, 30 at 8th, a year
--                   and a day at 9th
--   WIDER           Confusion - "5 feet wider per slot level"
--   MORE OF THEM    Conjure Elemental - a higher CR
--
-- `at_higher_dice` is the first shape and nothing else. NULL means this
-- spell gains no dice from a bigger slot - which is the answer for most
-- of the catalogue, including every spell whose rider promises targets
-- or duration instead. The rider still says what the DM adds by hand.
--
-- IT IS A FORMULA AND NOT A NUMBER, which is the whole reason it could
-- not be inferred from `dice`. Fireball's extra is a bare die; Magic
-- Missile's is a dart - a die AND a plus one - and a column holding
-- "1d6" could not say the second. `dice::add_formula` merges terms by
-- face so 3d4+3 and one dart is 4d4+4 rather than a string nobody can
-- read.
--
-- ---------------------------------------------------------------------
-- TWO SPELLS TODAY, AND THE REST DELIBERATELY NOT
-- ---------------------------------------------------------------------
--
-- Dave: "lets dial in one spell at a time." Magic Missile and Fireball
-- are the two in play this session and the two whose behaviour has been
-- watched in a real fight. Roughly sixty more carry a "+NdN per slot
-- level" rider and every one of them is a row somebody has to read off
-- the page and check.
--
-- FILLING THEM FROM MY OWN RIDER TEXT IS EXACTLY THE GUESS 183 REFUSED.
-- The riders are prose this project wrote; parsing them to populate a
-- rules column would make the prose load-bearing, and a typo in a
-- sentence would become a damage bug. They go in a level at a time,
-- checked against the published list, the way 161-167 filled the
-- catalogue in the first place.
--
-- UNTIL THEN THE BEHAVIOUR IS HONEST: a NULL means "a bigger slot buys
-- no extra dice", the spell is cast at the chosen level, the right slot
-- is spent, and the card shows the base dice. That is the truth for
-- most of the catalogue and a visible understatement for the rest.

alter table public.spells
  add column if not exists at_higher_dice text;

comment on column public.spells.at_higher_dice is
  'Dice ADDED per slot level above the spell''s own level, as a formula: '
  'Fireball ''1d6'', Magic Missile ''1d4+1''. NULL means a bigger slot '
  'buys no extra dice - see 185. Targets, duration and area are the '
  'DM''s and live in special_text.';

update public.spells set at_higher_dice = '1d4+1'
 where game_id is null and key = 'sp_magicmissile';

update public.spells set at_higher_dice = '1d6'
 where game_id is null and key = 'sp_fireball';
