-- 118. A BONUS THAT WAS REAL, TESTED, AND UNREACHABLE.
--
-- 098 gave the Ny'ook +2 on saving throws against spells.
-- `resolve_request` has honoured it ever since - but only when the
-- request string ENDS in " vs spell", and the only place that suffix
-- was ever written down was this trait's own prose.
--
-- THE ROLL BOX OFFERED THE WRONG SPELLING. Its placeholder reads
-- "insight, wis save, ath…", and "wis save" is precisely the form that
-- does NOT get the bonus. A player would have had to read the species
-- pane, notice a sentence of instructions, and retype their roll.
--
-- A CIRCUMSTANCE IS A CONTROL, NOT AN INCANTATION. There is a
-- checkbox beside Adv/Dis now - "against a spell" - shown whenever the
-- request is a save, and it says what the tick is worth to THIS
-- character ("+2 from Ny'ook", or that their people give none). It
-- writes the same suffix the engine has always read, so the
-- vocabulary is unchanged; only the way in is.
--
-- PROSE THAT TELLS A PLAYER TO DO THE ENGINE'S JOB outlives the gap it
-- was describing. 117 retired Unt'garoth's "there is no resistance
-- system yet, so halve it at the table" for the same reason: an
-- instruction left standing after the feature lands is worse than the
-- missing feature was, because somebody follows it.
--
-- THE POISON HALF IS LEFT ALONE and still says it is a DM call,
-- because it still is. Nothing in the catalogue deals poison damage,
-- and whether the Ny'ook get poison DAMAGE resistance on top of the
-- save bonus is a design decision rather than a bug - one grant
-- whenever Dave wants it, in 116's vocabulary.
update public.species s
   set traits = (
         select jsonb_agg(
                  case when t->>'name' = 'Magic and Toxin Resistance'
                       then t || jsonb_build_object(
                              'text',
                              '+2 on saving throws against spells, added by the engine. Tick "against a spell" beside the roll - it appears whenever the request is a saving throw, and names the bonus it is worth. The same resistance covers poisons at the table, and that half stays a DM call: nothing in the catalogue is poisonous for the engine to recognise.'
                            )
                       else t
                  end
                  order by ord
                )
           from jsonb_array_elements(s.traits) with ordinality as x(t, ord)
       )
 where s.key = 'nyook'
   and exists (
         select 1 from jsonb_array_elements(s.traits) t
          where t->>'name' = 'Magic and Toxin Resistance'
       );
