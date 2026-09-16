-- =====================================================================
-- 004_dm_reads_own_game.sql
-- odyssey1e — let a DM read the game they own
-- =====================================================================
--
-- SYMPTOM: create_game failed with
--   "new row violates row-level security policy for table games (403)"
-- even though the INSERT's WITH CHECK (dm_uid = auth.uid()) was
-- satisfied. Proven by probe: the identical INSERT without a RETURNING
-- clause succeeded.
--
-- CAUSE: PostgREST sends Prefer: return=representation, so every insert
-- carries a RETURNING clause — and PostgreSQL applies the SELECT policy
-- to a row returned that way. The only SELECT policy on games was
-- is_game_member(id). The DM is seated as a member by seat_game_dm(),
-- an AFTER INSERT trigger, which has NOT fired at the moment RETURNING
-- is evaluated. So the creator could insert their game but not see the
-- row coming back, and PostgREST surfaced that as a policy violation on
-- the insert — which sent us looking at entirely the wrong policy.
--
-- FIX: an owner may read their own game outright. Permissive policies
-- are OR'd, so this widens SELECT rather than replacing the membership
-- rule.
--
-- Right model regardless of the bug: a DM should never lose sight of a
-- campaign they own, even if their membership row were somehow deleted.
--
-- THE GENERAL LESSON: an INSERT ... RETURNING must satisfy BOTH the
-- INSERT WITH CHECK and the SELECT USING policy, and both are evaluated
-- BEFORE any AFTER trigger runs. Any table whose visibility depends on a
-- row that an AFTER trigger creates will hit this. Here only games did —
-- characters and rolls are visible through membership the inserter
-- already holds.
-- =====================================================================

create policy "games: dm reads own"
  on public.games for select
  using (dm_uid = auth.uid());
