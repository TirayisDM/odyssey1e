-- 159. TWO SPELLS WITH NO CASTING TIME.
--
-- Dave cast Hold Person with a Lich and found Fireball greyed out:
-- "takes too long".
--
-- `spellcast::cost` reads the column and maps anything that is not an
-- action, a bonus action or a reaction to `TooLong` - and NULL is not
-- any of those. Exactly two spells in the catalogue have no casting
-- time, and both come from the same place:
--
--   sp_fireball     105
--   sp_auraoflife   105
--
-- 105 is "two spells that were never cleric spells" - the pair that came
-- off ONE CHARACTER'S SHEET rather than out of a list, which is what
-- 006's header warned the whole table was. Every spell 102 to 104
-- seeded has a casting time because they were written from the list.
-- These two were rescued from the old data and nobody filled the column
-- in.
--
-- SO FIREBALL HAS NEVER BEEN CASTABLE IN A FIGHT, by anybody, since 105.
-- It took a Lich to notice because until 150 no creature could cast at
-- all, and the only other caster in the game is a cleric who does not
-- have it.
--
-- A DEFAULT WOULD HAVE HIDDEN THIS, which is worth saying because the
-- tempting fix is to make `cost` treat NULL as an action. It is the
-- ordinary case, so the guess would be right almost always - and a
-- spell seeded with no casting time would then look correct forever
-- instead of being visibly wrong the first time somebody reached for
-- it. `TooLong` is a bad answer that announces itself, which is the
-- better failure.

update public.spells
   set casting_time = '1 action'
 where game_id is null
   and casting_time is null
   and key in ('sp_fireball', 'sp_auraoflife');
