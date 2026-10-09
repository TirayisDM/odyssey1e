-- 195. THE CHECK HAD A BUG OF ITS OWN, AND IT WAS A GOOD ONE.
--
-- 194's second class calls `pg_get_functiondef(p.oid)` in a WHERE clause
-- and relies on `n.nspname = 'public'` - which comes from a JOIN - to
-- have already excluded everything else. POSTGRES DOES NOT PROMISE THAT.
-- A join-derived predicate is applied when the planner gets to it, and a
-- cheap column filter is applied first, so on a different plan
-- `pg_get_functiondef` is called on rows from `pg_catalog` - where it
-- reaches an AGGREGATE, which it refuses to describe:
--
--   ERROR: 42809: "array_agg" is an aggregate function
--
-- Then the check does not report a clean wall. It throws, and a check
-- that throws is a check nobody can tell apart from a check that failed.
--
-- HOW IT WAS FOUND, because the way matters more than the bug: an
-- ad-hoc query written minutes later, looking for every function that
-- mentions `auth.uid()`, made THE SAME MISTAKE and threw immediately.
-- 194 had been applied and passed; the identical shape failed three
-- times in a row at the console. The difference was never the code, it
-- was that 194's extra filters - `prosecdef`, a grant to `authenticated` -
-- happened to be cheap and selective enough that the planner put them
-- first. **194 PASSED BY LUCK OF THE QUERY PLAN.**
--
-- THE FIX IS TO STOP DEPENDING ON ORDER AT ALL:
--
--   `pronamespace = 'public'::regnamespace` is a scalar comparison on the
--   row itself, so there is no join for the planner to defer, and
--   `prokind = 'f'` excludes aggregates and window functions by
--   construction rather than by hoping they are filtered first.
--
-- Fixed forward per 115 - 194 is applied and stays as it is.

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
   where p.pronamespace = 'public'::regnamespace
     and (p.proacl is null                      -- null acl IS the default: PUBLIC
          or exists (select 1
                       from pg_catalog.aclexplode(p.proacl) a
                      where a.privilege_type = 'EXECUTE'
                        and (a.grantee = 0                       -- 0 is PUBLIC
                             or a.grantee = 'anon'::regrole)))

  union all

  -- 2. BYPASSES RLS AND NEVER ASKS WHO IS CALLING.
  --
  -- `prokind = 'f'` is not decoration. `pg_get_functiondef` throws on an
  -- aggregate, and this predicate is the only thing that guarantees it is
  -- never handed one - see the header. The namespace test is on the row
  -- rather than through a join for exactly the same reason.
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
   where p.pronamespace = 'public'::regnamespace
     and p.prokind = 'f'
     and p.prosecdef
     and p.prorettype <> 'pg_catalog.trigger'::regtype
     and p.proname not in ('holder_character', 'holder_is_a_location')
     and exists (select 1
                   from pg_catalog.aclexplode(p.proacl) a
                  where a.privilege_type = 'EXECUTE'
                    and a.grantee = 'authenticated'::regrole)
     and pg_catalog.pg_get_functiondef(p.oid)
           !~* '(auth\.uid|current_profile|is_game_dm|is_game_member)'

  union all

  -- 3. A TABLE THAT IS NOT BEHIND THE WALL AT ALL.
  select case when not c.relrowsecurity
                then 'row security is off'
                else 'row security is on with no policy, so nothing is visible'
         end::text,
         c.relname::text,
         ''::text
    from pg_catalog.pg_class c
   where c.relnamespace = 'public'::regnamespace
     and c.relkind = 'r'
     and (not c.relrowsecurity
          or not exists (select 1 from pg_catalog.pg_policy p
                          where p.polrelid = c.oid))

  order by 1, 2
$chk$;

comment on function public.security_doors() is
  '194, fixed in 195. Faults in the authorization wall that no constraint '
  'can catch. An empty result is the passing answer. Run it after any '
  'migration that adds a function or a table.';

revoke execute on function public.security_doors() from public;
revoke execute on function public.security_doors() from anon;
revoke execute on function public.security_doors() from authenticated;

-- PROVE THE FIX ON THE CASE THAT BREAKS THE OLD ONE: ask for every
-- public function's definition with no namespace join to lean on. If
-- `prokind` were missing this would be the query that throws.
do $proof$
declare n int;
begin
  select count(*) into n
    from pg_catalog.pg_proc p
   where p.pronamespace = 'public'::regnamespace
     and p.prokind = 'f'
     and pg_catalog.pg_get_functiondef(p.oid) is not null;
  raise notice '195: described % public functions without throwing', n;

  select count(*) into n from public.security_doors();
  if n > 0 then
    raise exception '195: the wall has % hole(s)', n;
  end if;
end
$proof$;
