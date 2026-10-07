-- 158. WHAT A SUCCESSFUL SAVE IS WORTH.
--
-- Eleven Save spells carry dice and the engine is about to start rolling
-- them at people, so it has to know two things the catalogue has never
-- said: whether those dice are DAMAGE at all, and what a creature who
-- makes the save takes.
--
-- ---------------------------------------------------------------------
-- THREE OF THE ELEVEN ARE NOT DAMAGE, which is the trap
-- ---------------------------------------------------------------------
--
--   Bane          1d4   the penalty subtracted from the target's attacks
--                       and saves - it is a GRANT, and 114 already runs it
--   Bestow Curse  1d8   extra necrotic on one of four curse options,
--                       conditional and later
--   Geas          5d10  psychic each DAY the target acts against the
--                       order, not on the save
--
-- A rule that damaged on every failed save would have Bane dealing 1d4
-- to somebody as well as cursing them, and Geas killing a creature for
-- failing a save it was always going to fail. The dice column cannot
-- tell these from Fireball, and the prose can - which is no use to an
-- engine.
--
-- ---------------------------------------------------------------------
-- `on_save` SAYS WHAT A SUCCESS IS WORTH, AND NULL SAYS "NOT DAMAGE"
-- ---------------------------------------------------------------------
--
--   'half'  full on a failure, half on a success - 5e's usual
--   'none'  full on a failure, nothing on a success - Sacred Flame
--   NULL    these dice are not save damage; the save does something else
--
-- NULL IS THE DEFAULT AND THAT IS DELIBERATE. An unmarked spell deals
-- nothing rather than guessing, so a spell added next month is inert
-- until somebody says what it does. Inventing damage is the worse
-- failure: a DM can see a spell that did nothing, and cannot see one
-- that quietly did the wrong thing to a player.
--
-- READ OFF EACH SPELL'S OWN PROSE, which states it in every case -
-- "HALF on a success", "no damage on a success", "halved on a Dexterity
-- save". This is that sentence made mechanical, not a judgement call.
--
-- FLAME STRIKE IS UNDERSTATED AND STAYS THAT WAY. The book is 4d6 fire
-- AND 4d6 radiant; `dice` holds 4d6 and there is nowhere to put the
-- second half - the same missing `damage_types` column that stops any
-- of this being resisted. Halving what is there is right; the total is
-- a separate fix.

alter table public.spells
  add column if not exists on_save text;

alter table public.spells
  drop constraint if exists spells_on_save_check;
alter table public.spells
  add constraint spells_on_save_check
  check (on_save is null or on_save in ('half', 'none'));

comment on column public.spells.on_save is
  'What a creature that MAKES the save takes: half the dice, none of '
  'them, or NULL when the dice are not save damage at all (Bane''s '
  'penalty, Geas''s daily toll). See 158.';

-- Full on a failure, half on a success - every one of these says so in
-- its own description.
update public.spells set on_save = 'half'
 where game_id is null
   and key in ('sp_fireball', 'sp_spiritguardians', 'sp_flamestrike',
               'sp_insectplague', 'sp_bladebarrier', 'sp_harm',
               'sp_firestorm');

-- "no damage on a success", which is what a cantrip gets instead of
-- the usual mercy.
update public.spells set on_save = 'none'
 where game_id is null and key = 'sp_sacredflame';
