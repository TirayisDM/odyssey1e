-- 187. WHAT A FAILED SAVE LEAVES ON YOU.
--
-- The catalogue has never been able to say that Hold Person
-- PARALYSES. It says it in prose, in `special_text`, for a DM to read
-- and apply - and the panel showed an effect called "Hold Person" with
-- no hint of what the target was actually suffering.
--
-- `grants` cannot carry it. 100's vocabulary is for things that change
-- A NUMBER, and `grants::parse` drops any row with neither value nor
-- dice. A condition is not a number: "blinded" is a bundle - cannot
-- see, fails sight checks, attacks against have ADVANTAGE - and 5e is
-- careful never to price advantage. Forcing it into a grant would mean
-- inventing a number the book refuses to give.
--
-- So: one column naming the condition a FAILED SAVE imposes, and
-- `conditions.rs` holding the closed list of fifteen.
--
-- ---------------------------------------------------------------------
-- ON THE FAILED SAVE, NOT ON THE CAST
-- ---------------------------------------------------------------------
--
-- This is why it is a column and not another grant even if grants
-- could hold it. `cast_spell` applies a spell's grants to the target
-- the moment it is cast - which is right for Bless, and would
-- PARALYSE SOMEBODY WHO MADE THEIR SAVE. The condition belongs where
-- the outcome is known, which is `resolve_spell_save`, and that is a
-- different half of the spell: 158 split them for exactly this reason.
--
-- ---------------------------------------------------------------------
-- WHAT IS AND IS NOT HERE
-- ---------------------------------------------------------------------
--
-- 22 SPELLS, each checked against the published list. Where a spell
-- imposes a condition only after repeated failures - Flesh to Stone
-- needs three - the column holds the condition and the rider holds the
-- count, because a column that said "after three" would be a second
-- rule nobody else can read.
--
-- ONE CONDITION PER SPELL. Several impose two: Sleep is unconscious
-- (and prone follows from it), Flesh to Stone restrains THEN petrifies,
-- Sunbeam blinds as well as burning. The column names the one the
-- failed save produces, and the rider carries the rest. A list would
-- be the right answer the day a second spell needs it for real rather
-- than for tidiness.
--
-- CONDITIONS FROM AN ATTACK SPELL ARE NOT WIRED. There is no "on a
-- hit" half to hang them on yet, and only a handful of spells need it.
--
-- EXHAUSTION IS NOT IMPOSED BY ANY SPELL HERE. Sickening Radiance is
-- not in this catalogue, and Wish's cost is a DM conversation rather
-- than an automatic effect.
--
-- TWO WERE DRAFTED AND TAKEN OUT, both caught by checking the cast
-- type before applying rather than after:
--
--   SLEEP really does make creatures unconscious, and it has NO SAVE -
--   it is 5d8 hit points spent on the weakest first. It is `Utility`
--   for that reason, `resolve_spell_save` will never run for it, and a
--   column nothing can read is a promise nothing keeps. Left NULL
--   until something can express "roll a pool and spend it".
--
--   SILENCE WAS SIMPLY WRONG. It makes an AREA soundless; it does not
--   deafen a creature, and nobody in it is "Deafened" in the sense the
--   condition means. The rule was misremembered and the data would
--   have carried the mistake into play.

alter table public.spells
  add column if not exists imposes text;

comment on column public.spells.imposes is
  'The condition a FAILED SAVE leaves on the target, as a `conditions.rs` '
  'key without the `cond.` prefix - "paralysed", "frightened". NULL for '
  'the great majority. Applied by `resolve_spell_save` and never at cast '
  'time, because a condition applied on casting would land on somebody '
  'who made their save. Where a spell imposes more than one, or needs '
  'repeated failures, the column names the main one and special_text '
  'carries the rest - see 187.';

update public.spells set imposes = 'blinded'      where game_id is null and key = 'sp_blindnessdeafness';
update public.spells set imposes = 'blinded'      where game_id is null and key = 'sp_sunbeam';
update public.spells set imposes = 'blinded'      where game_id is null and key = 'sp_sunburst';
update public.spells set imposes = 'charmed'      where game_id is null and key = 'sp_charmperson';
update public.spells set imposes = 'charmed'      where game_id is null and key = 'sp_dominateperson';
update public.spells set imposes = 'charmed'      where game_id is null and key = 'sp_dominatemonster';
update public.spells set imposes = 'charmed'      where game_id is null and key = 'sp_modifymemory';
update public.spells set imposes = 'frightened'   where game_id is null and key = 'sp_fear';
update public.spells set imposes = 'frightened'   where game_id is null and key = 'sp_phantasmalkiller';
update public.spells set imposes = 'frightened'   where game_id is null and key = 'sp_weird';
update public.spells set imposes = 'frightened'   where game_id is null and key = 'sp_eyebite';
update public.spells set imposes = 'incapacitated' where game_id is null and key = 'sp_hypnoticpattern';
update public.spells set imposes = 'paralysed'    where game_id is null and key = 'sp_holdperson';
update public.spells set imposes = 'paralysed'    where game_id is null and key = 'sp_holdmonster';
update public.spells set imposes = 'petrified'    where game_id is null and key = 'sp_fleshtostone';
update public.spells set imposes = 'poisoned'     where game_id is null and key = 'sp_contagion';
update public.spells set imposes = 'poisoned'     where game_id is null and key = 'sp_stinkingcloud';
update public.spells set imposes = 'prone'        where game_id is null and key = 'sp_grease';
update public.spells set imposes = 'restrained'   where game_id is null and key = 'sp_blacktentacles';
update public.spells set imposes = 'restrained'   where game_id is null and key = 'sp_web';
update public.spells set imposes = 'restrained'   where game_id is null and key = 'sp_resilientsphere';
update public.spells set imposes = 'stunned'      where game_id is null and key = 'sp_powerwordstun';
