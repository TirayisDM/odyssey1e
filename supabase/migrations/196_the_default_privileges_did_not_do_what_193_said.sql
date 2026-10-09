-- 196. 193'S LAST STATEMENT DID NOT WORK, AND IT WAS THE IMPORTANT ONE.
--
-- 193 ended with `alter default privileges ... revoke execute on
-- functions from public` and a header calling it "THE ONLY PART OF THIS
-- THAT SURVIVES THE NEXT MIGRATION". **IT DID NOTHING.** Two migrations
-- later 195's check caught it, which is the only reason this is known.
--
-- ---------------------------------------------------------------------
-- HOW IT WAS CAUGHT
-- ---------------------------------------------------------------------
--
-- 197 - the identity bridge - creates two functions, and its final gate
-- runs `security_doors()`. The migration refused to apply:
--
--   the wall has 2 hole(s)
--
-- The two holes were its own brand-new functions, born executable by
-- `anon` exactly as if 193 had never run. The whole thing rolled back.
--
-- **THIS IS THE CHECK PAYING FOR ITSELF WITHIN TWO MIGRATIONS OF BEING
-- WRITTEN**, and what it caught was a false claim in the header of the
-- migration that introduced it. The rolled-back probe that followed is
-- the proof: a function created after 193 came out
--
--   =X/postgres | postgres=X/postgres | anon=X/postgres | ...
--
-- where `=X/postgres` is PUBLIC. Both doors, still open.
--
-- ---------------------------------------------------------------------
-- WHY, AND IT IS TWO SEPARATE REASONS
-- ---------------------------------------------------------------------
--
-- 1. **`anon` WAS NEVER A DEFAULT - IT IS AN EXPLICIT GRANT.** Supabase
--    ships `alter default privileges` of its own for this database:
--
--      postgres in schema public, functions:
--        {postgres=X, anon=X, authenticated=X, service_role=X}
--
--    193 revoked PUBLIC and never mentioned `anon`, so every new
--    function kept being granted to `anon` by the platform's own
--    default. Revoking `anon` there DOES work and is done below.
--
-- 2. **THE BUILT-IN PUBLIC GRANT CANNOT BE REVOKED THIS WAY.** Verified
--    by probe: after `revoke execute on functions from public`, the
--    stored default correctly loses `anon`, and a function created in
--    the same transaction STILL comes out with `=X/postgres`. The
--    language's own default for functions is EXECUTE to PUBLIC and this
--    statement does not suppress it here.
--
-- So 193's claim was wrong on both halves, and no amount of
-- `alter default privileges` fixes the second one.
--
-- AND A THIRD THING LEARNED WHILE WRITING THIS FILE: `postgres` cannot
-- `alter default privileges for role supabase_admin` - permission
-- denied. The platform's own default for its own role is not ours to
-- change, which is another reason the fix cannot live in that statement.
--
-- ---------------------------------------------------------------------
-- WHAT ACTUALLY MAKES A DOOR BORN CLOSED
-- ---------------------------------------------------------------------
--
-- An EVENT TRIGGER, which is the mechanism that was wanted all along:
-- it runs as the DDL completes, so there is no window and nothing to
-- remember, and it does not care which role created the function.
-- Verified by probe before being written - a function created under it
-- comes out
--
--   postgres=X | authenticated=X | service_role=X
--
-- with no PUBLIC and no `anon`. The migration below then proves it again
-- on itself, by creating a throwaway function and asking what it was
-- born with.
--
-- `authenticated` IS LEFT ALONE deliberately. A signed-in user reaching
-- a SECURITY INVOKER function still meets RLS; the dangerous shape is
-- SECURITY DEFINER, and that is what `security_doors()` class 2 is for.
-- Stripping `authenticated` here would break every legitimate RPC.
--
-- NOTHING IN `public` IS FOR `anon`. The app signs in before it reads
-- anything. If some future function genuinely needs to be callable
-- before sign-in it must grant `anon` back explicitly, which is a
-- visible line in a migration rather than a default nobody chose.

-- ---------------------------------------------------------------------
-- 1. THE HALF OF 193 THAT DOES WORK
-- ---------------------------------------------------------------------
alter default privileges for role postgres in schema public
  revoke execute on functions from anon;

-- ---------------------------------------------------------------------
-- 2. AND THE MECHANISM FOR THE HALF THAT DOES NOT
-- ---------------------------------------------------------------------

create or replace function public.close_a_new_function()
returns event_trigger
language plpgsql
security definer
set search_path to ''
as $evt$
declare
  r record;
begin
  for r in select * from pg_catalog.pg_event_trigger_ddl_commands()
  loop
    -- `CREATE OR REPLACE` arrives under the same tag, and re-revoking a
    -- function that never had the grant is a no-op, so there is no need
    -- to tell the two apart.
    if r.schema_name = 'public' and r.command_tag = 'CREATE FUNCTION' then
      execute format('revoke execute on function %s from public', r.object_identity);
      execute format('revoke execute on function %s from anon',   r.object_identity);
    end if;
  end loop;
end;
$evt$;

comment on function public.close_a_new_function() is
  '196. Strips the PUBLIC and anon EXECUTE grants a new function in '
  'public is born with. Fires from the functions_are_born_closed event '
  'trigger; never called directly.';

-- NOT CALLABLE BY ANYONE. The event trigger machinery invokes this the
-- way it invokes a row trigger - no grant required. Left granted to
-- `authenticated` it would be a SECURITY DEFINER function with no caller
-- check, which is precisely what 195 class 2 reports, and it would be
-- right to report it.
--
-- WORTH KNOWING FOR NEXT TIME: any future `returns event_trigger`
-- function needs this same revoke, because 195 excludes `returns
-- trigger` and not `returns event_trigger`.
revoke execute on function public.close_a_new_function() from public;
revoke execute on function public.close_a_new_function() from anon;
revoke execute on function public.close_a_new_function() from authenticated;

create event trigger functions_are_born_closed
  on ddl_command_end
  when tag in ('CREATE FUNCTION')
  execute function public.close_a_new_function();

-- ---------------------------------------------------------------------
-- 3. SWEEP AGAIN, since 194 and 195 each created a function after 193
-- ---------------------------------------------------------------------
revoke execute on all functions in schema public from public;
revoke execute on all functions in schema public from anon;

-- ---------------------------------------------------------------------
-- 4. PROVE THE MECHANISM ON ITSELF
-- ---------------------------------------------------------------------
--
-- 193 shipped a claim about new functions without ever creating one to
-- look at. This creates one, asks what it was born with using the same
-- `aclexplode` the check uses, and drops it.
do $proof$
declare
  bad int;
  acl text;
  n   int;
begin
  create function public.born_closed_probe() returns int
    language sql security definer as $q$ select 1 $q$;

  select coalesce(array_to_string(p.proacl, ' | '), 'DEFAULT PUBLIC') into acl
    from pg_catalog.pg_proc p
   where p.pronamespace = 'public'::regnamespace and p.proname = 'born_closed_probe';

  select count(*) into bad
    from pg_catalog.pg_proc p,
         pg_catalog.aclexplode(p.proacl) a
   where p.pronamespace = 'public'::regnamespace
     and p.proname = 'born_closed_probe'
     and a.privilege_type = 'EXECUTE'
     and (a.grantee = 0 or a.grantee = 'anon'::regrole);

  if bad > 0 or acl = 'DEFAULT PUBLIC' then
    raise exception '196: a new function is still born open: %', acl;
  end if;

  raise notice '196: a new function is born with: %', acl;

  drop function public.born_closed_probe();

  select count(*) into n from public.security_doors();
  if n > 0 then
    raise exception '196: the wall has % hole(s)', n;
  end if;
end
$proof$;
