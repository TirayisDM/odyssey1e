-- 117. A TRAIT THAT SAID THE ENGINE COULD NOT DO THIS.
--
-- Unt'garoth's Elemental Resilience has read "NOT APPLIED BY THE
-- ENGINE: there is no resistance system yet, so halve it at the table"
-- since 056, and it was true every day until 116. It is not true now:
-- fire and cold reach Falon's Resistances block and a swing of either
-- writes half the hit points.
--
-- `applied` IS WHAT THE TALENTS TAB READS. A trait flagged false is
-- skipped there entirely, on the stated grounds that a DM-adjudicated
-- trait "is not a thing this character can DO without somebody ruling
-- on it" - which is exactly what has stopped being the case.
--
-- THE TEXT MATTERS AS MUCH AS THE FLAG. Prose that tells a DM to halve
-- it at the table, beside an engine that has already halved it, is how
-- a creature takes a quarter damage - and it is a worse fault than the
-- missing feature was, because it reads like an instruction.
--
-- JSONB REWRITTEN IN PLACE by name rather than by position. The traits
-- array is ordered and a positional update would silently rewrite a
-- different trait the next time somebody inserts one above it.
update public.species s
   set traits = (
         select jsonb_agg(
                  case when t->>'name' = 'Elemental Resilience'
                       then jsonb_build_object(
                              'kind',    'feature',
                              'name',    'Elemental Resilience',
                              'applied', true,
                              'text',    'Resistance to fire and cold damage - half damage from either, rounded down. Applied by the engine: it appears under Resistances on the character sheet and a hit of either type writes half the hit points.'
                            )
                       else t
                  end
                  order by ord
                )
           from jsonb_array_elements(s.traits) with ordinality as x(t, ord)
       )
 where s.key = 'untgaroth'
   and exists (
         select 1 from jsonb_array_elements(s.traits) t
          where t->>'name' = 'Elemental Resilience'
       );

-- NOT TOUCHING Ny'ook's "Magic and Toxin Resistance", which says the
-- poison half "stays a DM call: nothing in the catalogue is poisonous
-- for the engine to recognise". That is still true - no item or spell
-- in the catalogue deals poison damage - and whether the Ny'ook get
-- poison DAMAGE resistance on top of their save bonus is a design
-- decision rather than a bug. It would be one grant when Dave wants it.
