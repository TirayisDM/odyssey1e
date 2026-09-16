-- =====================================================================
-- 002_tighten_function_grants.sql
-- odyssey1e — lock down the SECURITY DEFINER surface
-- =====================================================================
--
-- WHY THIS EXISTS: Postgres grants EXECUTE to PUBLIC by default, and
-- Supabase exposes every public-schema function over PostgREST at
-- /rest/v1/rpc/<name>. So after 001, all nine helpers were reachable as
-- API endpoints by anon and authenticated alike — including the trigger
-- functions, which have no business being callable at all.
--
-- Supabase's own security advisor flags this as
-- anon_security_definer_function_executable / _authenticated_. Running
-- the advisor after every DDL change is worth the ten seconds.
--
-- AFTER THIS MIGRATION the advisor reports three remaining warnings, all
-- intentional and all unavoidable:
--   is_game_member, is_game_dm — a policy is evaluated as the QUERYING
--     role, so authenticated must hold EXECUTE or every policy using
--     them fails closed and the app sees nothing.
--   join_game — authenticated has to be able to call it; that is its
--     entire purpose. It already raises on a null auth.uid().
-- Do not "fix" those three. Fixing them breaks the access model.
-- =====================================================================

-- Trigger functions and internal helpers: callable by nobody.
revoke all on function public.handle_new_user()      from public, anon, authenticated;
revoke all on function public.seat_game_dm()         from public, anon, authenticated;
revoke all on function public.snapshot_roll_names()  from public, anon, authenticated;
revoke all on function public.touch_updated_at()     from public, anon, authenticated;
revoke all on function public.freeze_roll_identity() from public, anon, authenticated;
revoke all on function public.gen_join_code()        from public, anon, authenticated;

-- The three real functions: signed-in users only.
revoke all on function public.is_game_member(uuid) from public, anon;
revoke all on function public.is_game_dm(uuid)     from public, anon;
revoke all on function public.join_game(text)      from public, anon;

grant execute on function public.is_game_member(uuid) to authenticated;
grant execute on function public.is_game_dm(uuid)     to authenticated;
grant execute on function public.join_game(text)      to authenticated;
