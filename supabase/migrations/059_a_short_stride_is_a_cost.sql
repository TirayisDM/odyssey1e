-- =====================================================================
-- 059_a_short_stride_is_a_cost.sql
-- odyssey1e — the Unt'gar pay for those legs
-- =====================================================================
--
-- 058 gave a trait a `kind` and marked exactly one drawback, because
-- the Unt'garoth's document states one and the Unt'gar's states none.
-- That left the Unt'gar reading as a people with no cost at all, which
-- is not what their sheet says: 25 feet against a 30-foot norm and the
-- Unt'garoth's 40 is a real disadvantage, on every chase, every
-- retreat, and every round somebody else closes the distance first.
--
-- NOT NEW CONTENT - A RESTATEMENT. `speed` has said 25 since 057. What
-- this adds is the speed appearing where a player will actually look
-- for the shape of their character, marked as the cost it is. The
-- number is unchanged and nothing computes differently.
--
-- APPLIED IS FALSE, like every other movement trait in the schema,
-- because there is still no movement system. When one arrives, `speed`
-- is the column it reads; this trait stays prose either way, the same
-- contract 056 set.
--
-- WHY IT IS WORTH A ROW AT ALL. A drawback nobody can see is a drawback
-- nobody plays around. The Description panel splits features from costs
-- for precisely this reason, and a people whose only disadvantage is
-- buried in a stat line gets none of that.
-- =====================================================================

update public.species
   set traits = traits || jsonb_build_array(jsonb_build_object(
     'name', 'Short Stride',
     'text', 'A walking speed of 25 feet - five short of most peoples '
             || 'and fifteen behind an Unt''garoth. Built for the tunnel '
             || 'rather than the open ground.',
     'applied', false,
     'kind', 'drawback'
   ))
 where key = 'untgar'
   and game_id is null
   -- Idempotent: running this twice must not give them two bad legs.
   and not exists (
     select 1 from jsonb_array_elements(traits) t
      where t->>'name' = 'Short Stride'
   );
