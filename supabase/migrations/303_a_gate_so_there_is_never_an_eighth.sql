-- =====================================================================
-- 303_a_gate_so_there_is_never_an_eighth.sql
-- odyssey1e — the search_path half of what 196 did for grants
-- =====================================================================
--
-- 296 carried forward: "two trigger functions have a mutable
-- `search_path` - a_position_belongs_to_one_map and
-- a_square_is_inside_its_map." That number came from reading Supabase's
-- advisor. **THE DATABASE SAYS SEVEN**, and it took three readings to
-- get there:
--
--   the advisor, in 296      2   and it is right about those two
--   the migration files      8   two of them wrong, one missed
--   pg_proc, asked directly  7   and this is the answer
--
--   2 with no setting at all   a_position_belongs_to_one_map (230)
--                              a_square_is_inside_its_map    (232)
--   5 pinned to `public`       copy_kit                      (124)
--                              instantiate_character         (193)
--                              instantiate_npc               (193)
--                              no_templates_in_encounters    (123)
--                              unheld_is_unequipped          (089)
--
-- ---------------------------------------------------------------------
-- AND THE FILES AND THE DATABASE DISAGREE ABOUT THREE OF THEM
-- ---------------------------------------------------------------------
--
-- **`unheld_is_unequipped` IS NOT IN ANY FILE SCAN, BECAUSE 089 WROTE
-- IT WITHOUT A SCHEMA.** `create or replace function
-- unheld_is_unequipped()` - no `public.` - so a scan looking for
-- `function public.<name>` never sees the one definition that matters.
-- The database sees it fine. It is SECURITY DEFINER pinned to `public`,
-- which is the most exposed shape on this list.
--
-- **`touch_updated_at` AND `freeze_roll_identity` ARE PINNED IN THE
-- DATABASE AND NO MIGRATION PINS THEM.** 001 creates them bare and
-- nothing in 181 files alters them, yet `pg_proc` has them pinned to
-- the empty string. Somebody did it from outside the repo - the SQL
-- editor, or the advisor's own fix button - which means **a database
-- rebuilt from these migrations would not match this one.** That is
-- 115 the other way round: an applied migration is the record, and a
-- change applied without one is a change with no record at all.
--
-- They are altered below anyway. The statement is a no-op against this
-- database and load-bearing against a fresh one, and the alternative is
-- a file that is quietly untrue.
--
-- ---------------------------------------------------------------------
-- AND THE TWO THE ADVISOR NAMES ARE THE LEAST OF THE SEVEN
-- ---------------------------------------------------------------------
--
-- Both are SECURITY INVOKER. `a_position_belongs_to_one_map` names no
-- table at all - it writes `new.x := null` and returns.
-- `a_square_is_inside_its_map` already writes `public.locations` in
-- full. There is nothing in either body for a shadowed name to resolve
-- to, so the lint is formally right and the exposure is nil.
--
-- The five pinned to `public` are where the risk actually is, and they
-- are SECURITY DEFINER. 077 wrote down why, about three different
-- functions: *"the temporary schema is still searched ahead of it for
-- tables, so a session holding a temp table named `characters` would
-- have sync_character_level update that instead."* Writing to the wrong
-- table and reporting success is this codebase's named defect class.
--
-- **THE ADVISOR DOES NOT FLAG `public`, BECAUSE A SETTING IS PRESENT.**
-- That silence is how five of the seven stayed out of the list the
-- carried note was built from, and it is the reason the check below is
-- deliberately stricter than the advisor.
--
-- ---------------------------------------------------------------------
-- WHY IT IS THE THIRD TIME
-- ---------------------------------------------------------------------
--
-- 002 revoked the default grants. 017 did it again for the functions
-- 009-016 had added. 077 did it again, fifteen times over, plus three
-- wrong search paths. Then 196 stopped the GRANT half from ever
-- drifting again - an event trigger, `close_a_new_function`, strips
-- PUBLIC and `anon` as the DDL completes - and **the grant half has not
-- drifted since.**
--
-- The search_path half got a convention instead. 230, 232 and 001 are
-- what a convention produces. So this migration fixes the seven,
-- and then puts the rule in the one place that has already proved it
-- can hold a rule.
--
-- ---------------------------------------------------------------------
-- WHAT IS ACTUALLY AT RISK, STATED HONESTLY
-- ---------------------------------------------------------------------
--
-- Nothing a player can reach. Postgres refuses to execute a trigger
-- function any other way - "trigger functions can only be called as
-- triggers" - and the four RPCs raise unless `is_game_dm` says the
-- caller is the DM. Reaching any of this needs a direct database
-- connection that nobody outside this machine has.
--
-- It is fixed because the surface is supposed to be auditable at a
-- glance, which is 077's argument, and because the fourth time would
-- have cost another cycle.
--
-- ---------------------------------------------------------------------
-- A GATE AND AN AUDIT, AND WHY BOTH
-- ---------------------------------------------------------------------
--
-- The gate is the event trigger: a function BORN in `public` without
-- `search_path` pinned to the empty string is refused, by name, as the
-- DDL completes. It also catches the quiet one nothing today would
-- notice - **`create or replace` resets a function's settings**, so a
-- careless replacement silently unpins a pinned function.
--
-- The audit is `security_doors()`, which gains a fourth class. It is
-- there because the gate fires on CREATE FUNCTION and nothing else:
-- `alter function ... reset search_path` would walk straight past it.
-- The planted-fault block below uses exactly that hole to prove the
-- audit works, which is the honest way to show why there are two.
--
-- ONE PREDICATE, NOT TWO. Both ask `search_path_is_pinned(oid)`. 288's
-- rule is that a rule written twice disagrees eventually, and this one
-- would be written in a SQL audit and a plpgsql event trigger - the two
-- places most likely to drift apart.
--
-- ---------------------------------------------------------------------
-- AND MY OWN INSTRUMENT WAS WRONG FIRST
-- ---------------------------------------------------------------------
--
-- The scan of the files got it wrong TWICE, in opposite directions.
--
-- FIRST it reported eight with no setting, four of them wrongly: the
-- pattern matched `search_path = ''` and `search_path = public` but not
-- `search_path to 'public'`, so every quoted pin read as absent.
--
-- THEN, corrected, it still missed `unheld_is_unequipped` entirely,
-- because 089 declares it without a schema and the pattern wanted
-- `function public.<name>`.
--
-- Both are 302's lesson arriving one cycle later - a green check and a
-- check measuring the wrong thing look identical - and the thing that
-- told them apart both times was **asking the database**. 115: an
-- applied migration is the record, not a draft. The first version of
-- this migration refused with one fault it could not name, and that
-- fault was `unheld_is_unequipped`.
--
-- Which is also why the gate is inside this file. `execute_sql` and
-- `get_advisors` are both denied this session; `apply_migration` is
-- not, so a migration that asks and raises is the only instrument that
-- can reach the catalog at all.
-- =====================================================================


-- ---------------------------------------------------------------------
-- 1. THE ONES WITH NO SETTING AT ALL
--
-- A trigger keeps firing across an ALTER - the trigger points at the
-- function - so this changes a setting rather than replacing anything.
-- None of these bodies holds a bare name, so no behaviour changes.
--
-- The last two are already pinned in THIS database by a hand nobody
-- recorded, so those two statements do nothing here and are the only
-- reason a database rebuilt from these files would come out the same.
-- ---------------------------------------------------------------------

alter function public.a_position_belongs_to_one_map() set search_path = '';
alter function public.a_square_is_inside_its_map()    set search_path = '';
alter function public.touch_updated_at()              set search_path = '';
alter function public.freeze_roll_identity()          set search_path = '';


-- ---------------------------------------------------------------------
-- 2. THE FIVE PINNED TO `public`
--
-- 077's first case: each body already writes every table, type and
-- function it touches in full - `public.objects%rowtype`,
-- `public.characters`, `public.is_game_dm`, `public.copy_kit` - so the
-- setting changes and the body stands. Read line by line before this
-- was written, because 077's third function needed replacing whole and
-- assuming would have been how to find that out the hard way.
--
-- Four of the five are what the app calls to place a creature, copy a
-- kit and enrol an actor; the fifth fires on every write to `objects`.
-- A name that stops resolving fails at CALL time, not at apply time, so
-- all five are exercised in the running app after this lands.
-- ---------------------------------------------------------------------

alter function public.copy_kit(uuid, uuid, uuid, integer)        set search_path = '';
alter function public.instantiate_character(uuid, uuid, text)    set search_path = '';
alter function public.instantiate_npc(uuid, text, text, boolean) set search_path = '';
alter function public.no_templates_in_encounters()               set search_path = '';
alter function public.unheld_is_unequipped()                     set search_path = '';


-- ---------------------------------------------------------------------
-- 3. THE ONE PREDICATE
--
-- `proconfig` is a text[] of `name=value`. An empty search_path comes
-- back as `search_path=` on some versions and `search_path=""` on
-- others, so the value is unquoted before it is compared rather than
-- matched against either spelling. Anything else - `public`,
-- `public, extensions` - is a setting and not a pin, and answers false.
-- ---------------------------------------------------------------------

create or replace function public.search_path_is_pinned(p_oid oid)
returns boolean
language sql
stable
security invoker
set search_path = ''
as $pin$
  select exists (
    select 1
      from pg_catalog.pg_proc p,
           pg_catalog.unnest(coalesce(p.proconfig, '{}'::text[])) as cfg(entry)
     where p.oid = p_oid
       and cfg.entry like 'search_path=%'
       and btrim(substr(cfg.entry, length('search_path=') + 1), '"') = ''
  )
$pin$;

comment on function public.search_path_is_pinned(oid) is
  '303. True when this function pins search_path to the empty string, '
  'which is the house rule for every function in public. Asked by '
  'security_doors() class 4 and by the close_a_new_function event '
  'trigger, so the audit and the gate cannot drift apart.';

revoke execute on function public.search_path_is_pinned(oid) from public;
revoke execute on function public.search_path_is_pinned(oid) from anon;
revoke execute on function public.search_path_is_pinned(oid) from authenticated;


-- ---------------------------------------------------------------------
-- 4. THE AUDIT LEARNS THE FOURTH CLASS
--
-- Classes 1 to 3 are unchanged from 218. Class 4 is new, and it does
-- NOT exclude trigger functions: class 2 excludes them because a GRANT
-- on one buys nothing, which is true about grants and says nothing
-- about search_path. 296 found that gap by reading the advisor rather
-- than by either check, and this is it closed.
--
-- `prokind = 'f'` keeps `pg_get_functiondef` away from aggregates in
-- class 2; class 4 keeps it for the plainer reason that a procedure or
-- an aggregate is not a thing this schema has.
-- ---------------------------------------------------------------------

create or replace function public.security_doors()
returns table(fault text, object_name text, detail text)
language sql
stable
set search_path = ''
as $$

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
  -- THE ALLOWLIST IS THREE FUNCTIONS, AND THE REASON IS NOT "they are
  -- fine". `holder_character`, `holder_is_a_location` and, since 218,
  -- `loose_in_the_world` are what the POLICIES THEMSELVES CALL to
  -- decide who owns an entity. They cannot check their caller because a
  -- caller check would consult the policies that are mid-flight calling
  -- them. They are the asking, so they are the one thing that cannot
  -- ask. What bounds them instead: each takes a bare entity UUID and
  -- returns one fact about it, and you only ever learn an entity UUID by
  -- reading `objects` THROUGH RLS, which already holds you to your own
  -- game.
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
     and p.proname not in ('holder_character', 'holder_is_a_location',
                           'loose_in_the_world')
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

  union all

  -- 4. A search_path THAT IS NOT PINNED SHUT. 303.
  --
  -- Two faults, not one, because they read differently and the second
  -- is the one Supabase's own linter never mentions: a function pinned
  -- to `public` has a setting, so `function_search_path_mutable` is
  -- satisfied, while pg_temp is still searched ahead of `public` for
  -- tables. 077's reason, and five of this schema's functions were
  -- sitting in it.
  select case when not exists (select 1
                                 from pg_catalog.unnest(
                                        coalesce(p.proconfig, '{}'::text[]))
                                        as cfg(entry)
                                where cfg.entry like 'search_path=%')
                then 'search_path is whatever the caller says'
                else 'search_path is pinned, but not to the empty string'
         end::text,
         p.proname::text,
         coalesce(array_to_string(p.proconfig, ' | '), 'no settings at all')::text
    from pg_catalog.pg_proc p
   where p.pronamespace = 'public'::regnamespace
     and p.prokind = 'f'
     and not public.search_path_is_pinned(p.oid)

  order by 1, 2
$$;

comment on function public.security_doors() is
  '218, and class 4 since 303. Four ways the wall can be open: a '
  'function anyone can execute, a SECURITY DEFINER function that never '
  'asks who is calling, a table with no row security, and a function '
  'whose search_path is not pinned to the empty string. An empty result '
  'is the passing answer. Asked by wall_faults().';

revoke execute on function public.security_doors() from public;
revoke execute on function public.security_doors() from anon;
revoke execute on function public.security_doors() from authenticated;


-- ---------------------------------------------------------------------
-- 5. AND THE GATE LEARNS THE SECOND HALF OF ITS JOB
--
-- 196 created this to strip the PUBLIC and `anon` grants a new function
-- is born with, and that half is untouched. What is new is the refusal:
-- a function created in `public` without a pinned search_path does not
-- get to exist, and the error names it.
--
-- Checked first, revoked second, so the message is about the thing that
-- is wrong rather than about a grant on a function that is about to be
-- rolled back anyway.
--
-- WORTH KNOWING FOR NEXT TIME: a throwaway probe created inside a
-- `do` block needs the pin too. 196's own `born_closed_probe` would be
-- refused by this, and that is the right answer - it is the shape a
-- real mistake arrives in.
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
    -- to tell the two apart. 303: a REPLACE that drops the SET clause
    -- also arrives here, which is the only place it can be caught.
    if r.schema_name = 'public' and r.command_tag = 'CREATE FUNCTION' then

      if not public.search_path_is_pinned(r.objid) then
        raise exception
          '303: % was created without a pinned search_path - add: %',
          r.object_identity, $q$set search_path = ''$q$
          using hint = 'Every function in public pins it shut, so a name that is not schema-qualified fails at creation instead of resolving to whatever the caller had in scope. See 303.';
      end if;

      execute format('revoke execute on function %s from public', r.object_identity);
      execute format('revoke execute on function %s from anon',   r.object_identity);
    end if;
  end loop;
end;
$evt$;

comment on function public.close_a_new_function() is
  '196, and the search_path half since 303. Refuses a new function in '
  'public whose search_path is not pinned to the empty string, then '
  'strips the PUBLIC and anon EXECUTE grants it was born with. Fires '
  'from the functions_are_born_closed event trigger; never called '
  'directly.';

revoke execute on function public.close_a_new_function() from public;
revoke execute on function public.close_a_new_function() from anon;
revoke execute on function public.close_a_new_function() from authenticated;


-- =====================================================================
-- 6. AND IT REFUSES TO APPLY IF THE WALL IS NOT CLEAN
-- =====================================================================
--
-- 195's pattern, and 296's words: a fault reported here stops the
-- migration, so the fix and the check land together or neither does.
-- It names what it found, because a check that throws without saying
-- what it saw is indistinguishable from a check that is itself broken.
--
-- **AND IT NAMES THEM IN THE EXCEPTION, NOT IN A NOTICE.** 296 wrote
-- the faults out with `raise notice` and then raised a bare count, and
-- that works in the SQL editor. Applied through the management API it
-- does not: the first version of this migration refused with
-- *"wall_faults() reports 1 fault(s) - see the notices above"* and
-- **there were no notices above** - the API returns the error and
-- discards the rest. A diagnostic that only prints where nobody is
-- looking is 294's lesson wearing a different hat, so the names are
-- aggregated into the message that does come back.

do $gate$
declare
  n      int;
  names  text;
begin
  select count(*),
         coalesce(string_agg(format('%s: %s (%s)', w.audit, w.fault, w.object_name),
                             ' ;; ' order by w.audit, w.object_name), '')
    into n, names
    from public.wall_faults() w;
  if n > 0 then
    raise exception '303: wall_faults() reports % fault(s): %', n, names;
  end if;

  select count(*) into n
    from pg_catalog.pg_proc p
   where p.pronamespace = 'public'::regnamespace
     and p.prokind = 'f';
  raise notice
    '303: % functions in public, every one pinned to the empty search_path', n;
end
$gate$;


-- =====================================================================
-- 7. AND IT PROVES BOTH HALVES CAN FAIL
-- =====================================================================
--
-- A CHECK THAT HAS NEVER REPORTED ANYTHING IS INDISTINGUISHABLE FROM A
-- CHECK THAT CANNOT. Three faults are planted on a scratch function
-- created and dropped inside this block, and the baseline is asserted
-- clean FIRST, because "the check saw one fault" means nothing if it
-- was already seeing one.
--
-- Fault 1 and 2 go through `alter function`, which the gate never sees.
-- That is not a workaround for the sake of the test - it is the reason
-- the audit exists as well as the gate, demonstrated.

do $planted$
declare
  n          int;
  n_base     int;
  n_none     int;
  n_public   int;
  refused    text;
  pinned_ok  boolean;
begin
  select count(*) into n_base from public.security_doors();
  if n_base <> 0 then
    raise exception
      '303: the baseline showed % fault(s), so the planted faults prove nothing', n_base;
  end if;

  -- Born pinned, because the gate now insists on it. SECURITY INVOKER
  -- on purpose: a DEFINER probe would trip class 2 while it existed.
  create function public.zz_303_probe() returns int
    language sql
    set search_path = ''
    as $q$ select 1 $q$;

  -- FAULT 1: unpinned after the fact.
  alter function public.zz_303_probe() reset search_path;
  select count(*) into n_none
    from public.security_doors()
   where object_name = 'zz_303_probe'
     and fault like '%whatever the caller says%';

  -- FAULT 2: pinned to `public`, which is the shape the advisor is
  -- silent about and five of this schema's functions were in.
  alter function public.zz_303_probe() set search_path = 'public';
  select count(*) into n_public
    from public.security_doors()
   where object_name = 'zz_303_probe'
     and fault like '%not to the empty string%';

  drop function public.zz_303_probe();

  -- FAULT 3: born unpinned. The gate must refuse the CREATE itself.
  -- The sub-block rolls the DDL back with it, so there is nothing to
  -- drop afterwards.
  begin
    create function public.zz_303_unpinned() returns int
      language sql
      as $q$ select 1 $q$;
    refused := null;
  exception when others then
    refused := sqlerrm;
  end;

  -- AND THE GATE MUST STILL LET A CORRECT ONE THROUGH. A gate that
  -- refuses everything passes the test above and is useless.
  begin
    create function public.zz_303_pinned() returns int
      language sql
      set search_path = ''
      as $q$ select 1 $q$;
    pinned_ok := true;
    drop function public.zz_303_pinned();
  exception when others then
    pinned_ok := false;
    raise notice '303: a correctly pinned function was refused: %', sqlerrm;
  end;

  raise notice
    '303: baseline % | no-setting seen % | public-pinned seen % | gate said: %',
    n_base, n_none, n_public, coalesce(refused, 'nothing, it let it through');

  if n_none <> 1 then
    raise exception
      '303: the audit saw % functions with no search_path, expected exactly the probe', n_none;
  end if;
  if n_public <> 1 then
    raise exception
      '303: the audit saw % functions pinned to public, expected exactly the probe', n_public;
  end if;
  if refused is null then
    raise exception '303: the gate let an unpinned function be created';
  end if;
  if refused not like '%303%' then
    raise exception '303: that CREATE failed for some other reason: %', refused;
  end if;
  if not pinned_ok then
    raise exception '303: the gate refused a correctly pinned function';
  end if;

  -- Nothing left behind, asked rather than assumed.
  select count(*) into n from public.wall_faults();
  if n <> 0 then
    raise exception '303: % fault(s) left behind by the planted-fault block', n;
  end if;

  raise notice
    '303: the audit sees both shapes, the gate refuses one and admits the other, and the wall is clean';
end
$planted$;
