-- =====================================================================
-- 017_tighten_new_function_grants.sql
-- odyssey1e — the two functions 009-016 left outside 002's pattern
-- =====================================================================
--
-- WHY THIS EXISTS: 002 established two pieces of hygiene for every
-- function in the public schema — pin search_path, and revoke the
-- EXECUTE that Postgres grants to PUBLIC by default. Every function
-- through 008 carries both. The two added by 012 and 016 carry neither:
--
--   guard_actor_update   trigger fn, no search_path, anon holds EXECUTE
--   write_action         rpc,        no search_path, anon holds EXECUTE
--
-- The advisor caught this as function_search_path_mutable, which is the
-- first NEW warning since 002 and the reason its "run the advisor after
-- every DDL change" line is worth the ten seconds.
--
-- HOW BAD IS IT, HONESTLY: not very, and the reason matters more than
-- the verdict. The search_path attack this lint exists to stop is
-- privilege escalation — you shadow a table name, a SECURITY DEFINER
-- function resolves to your object, and your code runs as the owner.
-- BOTH OF THESE ARE SECURITY INVOKER. They run as the caller, so RLS
-- governs everything they touch and there is no privilege to escalate
-- to. anon holding EXECUTE on write_action is likewise not a hole: anon
-- calling it gets its inserts refused by the same policies that refuse
-- anon everywhere else.
--
-- So this migration is about CONSISTENCY, not a breach. It exists
-- because "the advisor reports exactly three expected warnings" is a
-- sentence worth being able to say. Three known warnings you can skim
-- past is a working alarm; five where two are shrugged off is how the
-- sixth gets missed. And the day one of these is made SECURITY DEFINER
-- for some reason that looks good at the time, the hygiene should
-- already be there rather than being remembered.
--
-- VERIFIED BEFORE WRITING, because pinning search_path on a function
-- whose body is not schema-qualified breaks it at runtime rather than
-- at migration time. Both bodies qualify everything: public.encounters
-- and public.is_game_dm in the guard; public.actions, public.rolls
-- (including the null::public.rolls composite in
-- jsonb_populate_recordset, the one most likely to bite), public
-- .hp_events, public.encounter_actors and public.characters in
-- write_action. Everything else they touch — jsonb operators, nullif,
-- coalesce, the uuid and smallint casts — lives in pg_catalog, which is
-- searched implicitly whatever search_path says.
--
-- That was confirmed by running write_action end to end with
-- search_path emptied and rolling the whole thing back, not by reading
-- alone. All five references resolved.
--
-- ON REVOKING FROM A TRIGGER FUNCTION: safe, and 003 is why the
-- question has to be asked at all. Revoking EXECUTE on a function used
-- in a column DEFAULT silently breaks every insert, because a default
-- is evaluated as the caller. TRIGGER BODIES ARE EXEMPT from that
-- check. guard_actor_update is only ever reached as a trigger, so
-- taking EXECUTE away from everyone is correct — it is the same thing
-- 002 did to the six trigger functions that existed then.
--
-- AFTER THIS MIGRATION the advisor should report exactly the three
-- intentional SECURITY DEFINER warnings from 002's header
-- (is_game_member, is_game_dm, join_game) plus leaked password
-- protection, which is an auth setting and not DDL. Anything else is
-- new and worth reading.
-- =====================================================================

-- Pin the resolution path. Both bodies are fully schema-qualified.
alter function public.guard_actor_update()
  set search_path = '';
alter function public.write_action(jsonb, jsonb, jsonb, jsonb)
  set search_path = '';

-- A trigger function is callable by nobody. Matches the six 002 did.
revoke all on function public.guard_actor_update() from public, anon, authenticated;

-- write_action is the attack path: signed-in users only.
revoke all on function public.write_action(jsonb, jsonb, jsonb, jsonb) from public, anon;
grant execute on function public.write_action(jsonb, jsonb, jsonb, jsonb) to authenticated;

comment on function public.write_action(jsonb, jsonb, jsonb, jsonb) is
  'One swing, written atomically: the action, its rolls, the hit point event and any death-save tally land together or not at all. SECURITY INVOKER on purpose - every table it touches is governed by the caller''s own policies, so it can create nothing the caller could not have inserted by hand. search_path pinned by 017; the body is fully schema-qualified.';
