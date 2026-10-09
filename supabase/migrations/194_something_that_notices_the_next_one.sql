-- 194. SOMETHING THAT NOTICES THE NEXT ONE.
--
-- The second half of 193, and the half that matters, for the reason 193
-- gave: 002 already did this job once and every migration since undid it.
-- A fix that has to be remembered has already failed. 193's `alter
-- default privileges` stops one way of re-opening the wall; this is what
-- notices the others.
--
-- SHAPED LIKE `check_item_keys` (090), ON PURPOSE. Same contract: a flat
-- list of fault descriptions, and AN EMPTY LIST IS THE PASSING ANSWER.
-- Same reason for existing, too - it asserts something the schema cannot.
-- There is no constraint that can require a SECURITY DEFINER function to
-- look at its caller, so a query over the catalogue is the only thing
-- that will ever notice one that does not.
--
-- IT LIVES IN SQL RATHER THAN RUST because the facts are in `pg_proc`
-- and `pg_class`, which PostgREST cannot reach. That makes it the one
-- check that cannot follow the pattern of living in a tested rules module,
-- so the compensation is that THE MIGRATION RUNS IT - see the bottom of
-- this file. It has passed once, here, against the real catalogue.
--
-- ---------------------------------------------------------------------
-- THE THREE CLASSES IT LOOKS FOR
-- ---------------------------------------------------------------------
--
-- 1. A function `PUBLIC` or `anon` may execute. This is the hole 193
--    found: five functions, three of them dangerous, reachable with the
--    publishable key that ships inside the app.
--
-- 2. A SECURITY DEFINER function a signed-in user may execute that never
--    looks at who is calling. SECURITY DEFINER means "do not consult the
--    policies", so such a function is RLS switched off for anyone who can
--    name it. `instantiate_*` were in this class until 193.
--
-- 3. A table with RLS off, or with RLS on and no policy. Neither is a
--    thing anyone would do deliberately; both are what a migration that
--    creates a table and forgets looks like from the outside.
--
-- All three passed when this file was applied, which is the only claim
-- being made. It is a tripwire, not a proof.

create or replace function public.security_doors()
returns table (fault text, object_name text, detail text)
language sql
stable
security invoker
set search_path to ''
as $chk$

  -- 1. OPEN TO THE UNAUTHENTICATED.
  select 'executable without signing in'::text,
         p.proname::text,
         coalesce(array_to_string(p.proacl, ' | '), 'default grant to PUBLIC')::text
    from pg_catalog.pg_proc p
    join pg_catalog.pg_namespace n on n.oid = p.pronamespace
   where n.nspname = 'public'
     and (p.proacl is null                      -- null acl IS the default: PUBLIC
          or exists (select 1
                       from pg_catalog.aclexplode(p.proacl) a
                      where a.privilege_type = 'EXECUTE'
                        and (a.grantee = 0                       -- 0 is PUBLIC
                             or a.grantee = 'anon'::regrole)))

  union all

  -- 2. BYPASSES RLS AND NEVER ASKS WHO IS CALLING.
  --
  -- Trigger functions are excluded: they reference NEW and fail outside a
  -- trigger, so a grant on one buys nothing.
  --
  -- THE ALLOWLIST IS TWO FUNCTIONS, AND THE REASON IS NOT "they are
  -- fine". `holder_character` and `holder_is_a_location` are what the
  -- POLICIES THEMSELVES CALL to decide who owns an entity. They cannot
  -- check their caller because a caller check would consult the policies
  -- that are mid-flight calling them. They are the asking, so they are
  -- the one thing that cannot ask. What bounds them instead: each takes
  -- a bare entity UUID and returns one fact about it, and you only ever
  -- learn an entity UUID by reading `objects` THROUGH RLS, which already
  -- holds you to your own game.
  --
  -- Anything added to this list is a wall with a hole in it and a comment
  -- explaining why the hole is load-bearing. Add slowly.
  select 'bypasses RLS without checking the caller'::text,
         p.proname::text,
         pg_catalog.pg_get_function_identity_arguments(p.oid)::text
    from pg_catalog.pg_proc p
    join pg_catalog.pg_namespace n on n.oid = p.pronamespace
   where n.nspname = 'public'
     and p.prosecdef
     and p.prorettype <> 'pg_catalog.trigger'::regtype
     and p.proname not in ('holder_character', 'holder_is_a_location')
     and exists (select 1
                   from pg_catalog.aclexplode(p.proacl) a
                  where a.privilege_type = 'EXECUTE'
                    and a.grantee = 'authenticated'::regrole)
     and pg_catalog.pg_get_functiondef(p.oid)
           !~* '(auth\.uid|is_game_dm|is_game_member)'

  union all

  -- 3. A TABLE THAT IS NOT BEHIND THE WALL AT ALL.
  select case when not c.relrowsecurity
                then 'row security is off'
                else 'row security is on with no policy, so nothing is visible'
         end::text,
         c.relname::text,
         ''::text
    from pg_catalog.pg_class c
    join pg_catalog.pg_namespace n on n.oid = c.relnamespace
   where n.nspname = 'public'
     and c.relkind = 'r'
     and (not c.relrowsecurity
          or not exists (select 1 from pg_catalog.pg_policy p
                          where p.polrelid = c.oid))

  order by 1, 2
$chk$;

comment on function public.security_doors() is
  '194. Faults in the authorization wall that no constraint can catch. '
  'An empty result is the passing answer. Run it after any migration '
  'that adds a function or a table.';

-- NOT GRANTED TO `authenticated`, deliberately. The output is a map of
-- which functions bypass RLS, which is reconnaissance and nothing a
-- player needs. It is a development check, run from the SQL editor.
revoke execute on function public.security_doors() from public;
revoke execute on function public.security_doors() from anon;
revoke execute on function public.security_doors() from authenticated;

-- AND RUN IT NOW, so the check is not merely written but has passed
-- against the live catalogue once, in the same transaction that
-- defined it. If 193 missed anything, this migration does not apply.
do $gate$
declare
  faults text;
  n      int;
begin
  select count(*),
         string_agg(format('%s: %s %s', d.fault, d.object_name, d.detail), E'\n  ')
    into n, faults
    from public.security_doors() d;

  if n > 0 then
    raise exception E'194: the wall has % hole(s):\n  %', n, faults;
  end if;
end
$gate$;
