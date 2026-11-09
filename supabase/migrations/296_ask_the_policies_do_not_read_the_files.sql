-- 296. ASK THE POLICIES. DO NOT READ THE FILES.
--
-- 292 recorded that `profiles` still asks `auth.uid()` in its own
-- policies. 294 carried it forward. 295 struck it, because live all
-- three of those policies read `(select current_profile())` - 197
-- rewrote 42 policies through a loop over `pg_catalog.pg_policy` that
-- names no table, so grepping the migration files for the rewrite
-- finds nothing and reads exactly like a table that was missed.
--
-- TWO CYCLES CARRIED A SECURITY FAULT THAT DID NOT EXIST, because the
-- only instrument anybody had was reading prose and guessing. That is
-- what this migration is about. Not the policy - the policy was fine.
--
-- AND THE NOTE WAS RIGHT ABOUT THE DISEASE AND WRONG ABOUT THE
-- PATIENT. Asking the catalogue found three policies that really do
-- compare `owner_uid` to `auth.uid()`, and they are not on `profiles`.
-- They are on `character_images`, which 283 created AFTER 197 ran, so
-- 197's loop never saw them and nothing since has looked.
--
--   characters:       owner_uid = (select current_profile())
--   character_images: c.owner_uid = auth.uid()
--
-- THE SAME COLUMN, COMPARED TO TWO DIFFERENT THINGS, IN TWO TABLES
-- THAT JOIN ON EACH OTHER. That is 288 - a rule written twice
-- disagrees eventually - and here it had already disagreed. A person
-- signed in through a second identity cannot add, replace or delete a
-- picture of their own character.
--
-- So this migration does two things, in this order:
--
--   ONE. Fix those three policies.
--   TWO. Install the check that found them, so the next one fails at
--        `apply` instead of being carried in a log for three cycles.
--
-- WHY IT LIVES IN THE DATABASE AND NOT IN `cargo test`. The Rust suite
-- is 1443 tests in 0.08 seconds and touches no network; `npm test`
-- reads files off disk. Neither can ask this database anything without
-- becoming slow and needing a credential, and a credential is the one
-- thing that must never enter this repo - it is public. So the
-- assertions go where the thing being asserted lives, phrased the way
-- 193/194/195 phrased `security_doors()`: a function, an empty result
-- is the pass, and a `do` block that refuses to apply the migration if
-- it is not clean.
--
-- WHAT THIS CANNOT DO, said here rather than discovered later. It
-- checks that a policy asks the right question of the right identity.
-- It cannot check that the answer is RIGHT. Proving that Alice cannot
-- read Bob's character needs four accounts signing in and comparing
-- what they see, which is what the 001 rig was built for and is a much
-- bigger piece of work. A check that claimed otherwise would be worse
-- than no check.
--
-- AND WHY A CURL PROBE IS NOT ENOUGH, which is worth writing down
-- because it was tried first. Asking the live REST API with the
-- publishable key and no bearer token is a real test and it found the
-- real answer - 42501 permission denied, on every table that matters.
-- But `scribed_spells` answered 200 with an empty array, and its
-- policy is correct: the EXISTS over `objects` finds nothing, so
-- `is_game_member` is never reached to refuse. **A LIVE PROBE CANNOT
-- TELL AN EMPTY TABLE FROM A WALLED ONE.** The catalogue can, because
-- it reads the rule rather than the consequence.

-- =====================================================================
-- ONE. THE THREE POLICIES THAT ASK A SIGN-IN WHO IT IS
-- =====================================================================
--
-- 197's rule, which has not changed: a PERSON is `profiles.id`, and
-- `public.current_profile()` resolves it. `auth.uid()` is a SIGN-IN. A
-- person may hold more than one, and `characters.owner_uid` holds the
-- person.
--
-- Dropped and recreated rather than edited, because a policy has no
-- `alter ... using`. 115 applies to the FILES, not to the objects:
-- 283 stays exactly as it was written and this corrects forward.
--
-- `(select public.current_profile())` keeps the parenthesised SELECT
-- 197 used throughout, which lets the planner hoist it to one
-- evaluation per statement instead of one per row.

drop policy "character_images: owner or dm writes" on public.character_images;
drop policy "character_images: owner or dm updates" on public.character_images;
drop policy "character_images: owner or dm deletes" on public.character_images;

create policy "character_images: owner or dm writes"
  on public.character_images for insert
  with check (exists (select 1 from public.characters c
                      where c.id = character_id
                        and (c.owner_uid = (select public.current_profile())
                             or public.is_game_dm(c.game_id))));

create policy "character_images: owner or dm updates"
  on public.character_images for update
  using (exists (select 1 from public.characters c
                 where c.id = character_id
                   and (c.owner_uid = (select public.current_profile())
                        or public.is_game_dm(c.game_id))))
  with check (exists (select 1 from public.characters c
                      where c.id = character_id
                        and (c.owner_uid = (select public.current_profile())
                             or public.is_game_dm(c.game_id))));

create policy "character_images: owner or dm deletes"
  on public.character_images for delete
  using (exists (select 1 from public.characters c
                 where c.id = character_id
                   and (c.owner_uid = (select public.current_profile())
                        or public.is_game_dm(c.game_id))));

-- The read policy is untouched: it never asked about a sign-in. It
-- defers to `is_game_member` on the parent character, which is 005's
-- pattern and is correct.

-- =====================================================================
-- TWO. THE CHECK THAT FOUND IT
-- =====================================================================
--
-- `security_doors()` audits the FUNCTIONS - who may execute what, and
-- which of them bypass RLS without asking who is calling. It has
-- returned zero faults since 195 and it is not duplicated here.
--
-- This audits the POLICIES, which nothing has ever audited.
--
-- THE SHAPE IS 195'S AND THE REASONS 195 GIVES STILL HOLD: namespace
-- compared as a scalar on the row rather than through a join, because
-- a join-derived predicate is applied when the planner gets to it and
-- 194 passed by luck of the query plan. Nothing here calls
-- `pg_get_functiondef`, so `prokind` is not needed - but the habit of
-- not depending on predicate order is.

create or replace function public.policy_faults()
returns table (fault text, object_name text, detail text)
language sql
stable
security invoker
set search_path to ''
as $chk$

  with pol as (
    select c.relname as tbl,
           p.polname as pol,
           case p.polcmd when 'r' then 'select' when 'a' then 'insert'
                         when 'w' then 'update' when 'd' then 'delete'
                         else 'all' end as act,
           coalesce(pg_catalog.pg_get_expr(p.polqual, p.polrelid), '')
             || ' ' ||
           coalesce(pg_catalog.pg_get_expr(p.polwithcheck, p.polrelid), '')
             as expr
      from pg_catalog.pg_policy p
      join pg_catalog.pg_class c on c.oid = p.polrelid
     where c.relnamespace = 'public'::regnamespace
  )

  -- 1. ASKS A SIGN-IN WHO THE PERSON IS.
  --
  -- 197: `auth.uid()` is the sign-in, `current_profile()` is the
  -- person, and every ownership test wants the person. There is no
  -- allowlist, deliberately. If a policy ever genuinely needs the
  -- sign-in - and none does today - it belongs here as a named
  -- exception with the reason written beside it, the way 195's two
  -- functions are named. An empty list is a stronger statement than a
  -- list with one unexplained entry in it.
  select 'policy asks auth.uid() instead of current_profile()'::text,
         (tbl || ' / ' || pol)::text,
         act::text
    from pol
   where expr ~* 'auth\.uid'

  union all

  -- 2. DOES NOT ASK ANYBODY ANYTHING.
  --
  -- THIS IS THE WALL ITSELF, and it is worth being exact about where
  -- the wall actually is. About 160 of the 168 policies are `to
  -- public`, and `anon` holds select, insert, update and delete on 42
  -- of 44 tables. That looks like a hole and is not one: what stops an
  -- unauthenticated caller is that 193/194/196 revoked execute on the
  -- functions the policies call, so the policy is reached, calls
  -- `is_game_member`, and the statement dies with 42501.
  --
  -- WHICH MEANS A POLICY THAT NAMES NO SUCH FUNCTION IS NOT PROTECTED
  -- BY ANY OF THAT. It is simply open. Confirmed by hand against the
  -- live REST API: `scribe_schools`, whose read policy is literally
  -- `true`, answers 200 to a request carrying nothing but the
  -- publishable key.
  --
  -- SOMEBODY WILL EVENTUALLY "FIX" THE `to public` ROLES AND BELIEVE
  -- THEY HAVE DONE SOMETHING. They will not have. This check is the
  -- note that explains where the wall is, in a form that fails rather
  -- than being read.
  --
  -- THE ALLOWLIST IS ONE TABLE AND THE REASON IS NOT "it is fine".
  -- `scribe_schools` is the cost of scribing per school of magic, with
  -- its reagent - five columns, no `game_id`, no owner, no row that
  -- belongs to anybody. It is a rules reference, the same kind of
  -- thing a printed book holds, and `true` is the honest policy for
  -- it. Anything added here is a table anybody on the internet can
  -- read, so it wants a sentence saying why that is correct. Add
  -- slowly.
  select 'policy names no identity function, so it is open'::text,
         (tbl || ' / ' || pol)::text,
         act::text
    from pol
   -- The pattern is parenthesised because `||` binds LOOSER than `!~*`:
   -- without the brackets this parses as `(expr !~* '...') || '...'` and
   -- Postgres says `argument of AND must be type boolean, not type
   -- text`. Which is a friendly failure, and was still worth a minute.
   where expr !~* ('(current_profile|is_game_dm|is_game_member|shares_a_game'
                   || '|holder_character|holder_is_a_location|auth\.uid)')
     and tbl not in ('scribe_schools')

  union all

  -- 3. A POLICY WITH NOTHING IN IT AT ALL.
  --
  -- A `select` policy with no USING, or a write policy with no WITH
  -- CHECK, is not a wall - the catalogue stores a null expression and
  -- the clause is simply absent. Check 2 would not see it, because an
  -- empty string matches no function name but also fails the regex
  -- test in the other direction. This is cheap and closes that door.
  select 'policy has no expression'::text,
         (c.relname || ' / ' || p.polname)::text,
         case p.polcmd when 'r' then 'select' when 'a' then 'insert'
                       when 'w' then 'update' when 'd' then 'delete'
                       else 'all' end::text
    from pg_catalog.pg_policy p
    join pg_catalog.pg_class c on c.oid = p.polrelid
   where c.relnamespace = 'public'::regnamespace
     and p.polqual is null
     and p.polwithcheck is null

  order by 1, 2
$chk$;

comment on function public.policy_faults() is
  '296. Faults in the POLICIES, the way security_doors() audits the '
  'FUNCTIONS. An empty result is the passing answer. It checks that a '
  'policy asks the right question of the right identity; it cannot '
  'check that the answer is right - that needs four accounts and the '
  '001 rig. Run it after any migration that adds a table or a policy.';

revoke execute on function public.policy_faults() from public;
revoke execute on function public.policy_faults() from anon;
revoke execute on function public.policy_faults() from authenticated;

-- =====================================================================
-- THREE. ONE THING TO RUN
-- =====================================================================
--
-- `security_doors()` has been correct since 195 and has one problem:
-- it only runs when a person remembers to paste it into the SQL
-- editor. A second function to remember is a second thing to forget,
-- so there is now one name that asks both and says which answered.
--
-- `fault` is prefixed rather than merged, so a result still tells you
-- WHICH audit objected without having to know what either of them
-- checks.

create or replace function public.wall_faults()
returns table (audit text, fault text, object_name text, detail text)
language sql
stable
security invoker
set search_path to ''
as $all$
  select 'doors'::text, d.fault, d.object_name, d.detail
    from public.security_doors() d
  union all
  select 'policies'::text, p.fault, p.object_name, p.detail
    from public.policy_faults() p
  order by 1, 2, 3
$all$;

comment on function public.wall_faults() is
  '296. security_doors() and policy_faults() together, so there is one '
  'name to remember. An empty result is the passing answer. This is the '
  'query the first-day checklist in NEW-MACHINE.md runs.';

revoke execute on function public.wall_faults() from public;
revoke execute on function public.wall_faults() from anon;
revoke execute on function public.wall_faults() from authenticated;

-- =====================================================================
-- AND IT REFUSES TO APPLY IF THE WALL IS NOT CLEAN
-- =====================================================================
--
-- 195's `do $proof$` pattern, which is the half that makes this a
-- check rather than a query somebody could run. A fault reported here
-- stops the migration, so the fix and the check land together or
-- neither does.
--
-- It also names what it found, because 195 learned that a check which
-- throws without saying what it saw is indistinguishable from a check
-- that is itself broken.

do $proof$
declare
  n int;
  r record;
begin
  -- The three that started this, by name, so the fix is proven and not
  -- assumed.
  select count(*) into n
    from pg_catalog.pg_policy p
    join pg_catalog.pg_class c on c.oid = p.polrelid
   where c.relname = 'character_images'
     and coalesce(pg_catalog.pg_get_expr(p.polqual, p.polrelid), '')
         || coalesce(pg_catalog.pg_get_expr(p.polwithcheck, p.polrelid), '')
         ~* 'auth\.uid';
  if n > 0 then
    raise exception '296: character_images still asks auth.uid() in % policy(ies)', n;
  end if;

  select count(*) into n from public.policy_faults();
  if n > 0 then
    for r in select * from public.policy_faults() loop
      raise notice '296: policy fault - % | % | %', r.fault, r.object_name, r.detail;
    end loop;
    raise exception '296: % policy fault(s) - see the notices above', n;
  end if;

  select count(*) into n from public.wall_faults();
  if n > 0 then
    raise exception '296: wall_faults() reports % fault(s)', n;
  end if;

  select count(*) into n
    from pg_catalog.pg_policy p
    join pg_catalog.pg_class c on c.oid = p.polrelid
   where c.relnamespace = 'public'::regnamespace;
  raise notice '296: % policies audited, no faults', n;
end
$proof$;

-- =====================================================================
-- AND IT PROVES IT CAN FAIL
-- =====================================================================
--
-- A CHECK THAT HAS NEVER REPORTED ANYTHING IS INDISTINGUISHABLE FROM A
-- CHECK THAT CANNOT. `security_doors()` has returned zero rows for a
-- hundred cycles, which is the right answer and also no evidence at
-- all, and 195 only discovered its bug because somebody happened to
-- write the same query by hand and watched it throw.
--
-- So each of the three checks is fired at a planted fault here, on a
-- scratch table that is created and dropped inside this block. If a
-- check stops working - a regex that no longer matches, a join that
-- drops rows - this is what notices, and it notices at `apply` rather
-- than the next time somebody has a reason to doubt the answer.
--
-- IT ASSERTS THE BASELINE IS ZERO FIRST, because "the check saw one
-- fault" means nothing if it was already seeing one.

do $planted$
declare
  n_base int; n_signin int; n_open int; n_empty int;
begin
  select count(*) into n_base from public.policy_faults();

  create table public.zz_296_proof (
    id uuid primary key default gen_random_uuid(),
    owner_uid uuid
  );
  alter table public.zz_296_proof enable row level security;

  -- CHECK 1: a policy that asks the sign-in who the person is.
  create policy "zz: asks a sign in" on public.zz_296_proof
    for select using (owner_uid = auth.uid());
  select count(*) into n_signin
    from public.policy_faults() where fault ~ 'auth.uid';
  drop policy "zz: asks a sign in" on public.zz_296_proof;

  -- CHECK 2: a policy that asks nobody anything.
  create policy "zz: asks nobody" on public.zz_296_proof
    for select using (true);
  select count(*) into n_open
    from public.policy_faults() where fault ~ 'no identity function';
  drop policy "zz: asks nobody" on public.zz_296_proof;

  -- CHECK 3: a policy with no expression in it at all. Postgres
  -- accepts this - `for all` with neither USING nor WITH CHECK - and
  -- it is the one shape check 2 is blind to.
  create policy "zz: says nothing" on public.zz_296_proof for all;
  select count(*) into n_empty
    from public.policy_faults() where fault ~ 'no expression';
  drop policy "zz: says nothing" on public.zz_296_proof;

  drop table public.zz_296_proof;

  raise notice
    '296: baseline % | sign-in check saw % | open check saw % | empty check saw %',
    n_base, n_signin, n_open, n_empty;

  if n_base <> 0 then
    raise exception '296: the baseline was not clean, so the planted faults prove nothing';
  end if;
  if n_signin <> 1 or n_open <> 1 or n_empty <> 1 then
    raise exception '296: a check did not fire on its planted fault';
  end if;
  raise notice '296: all three checks fire on a planted fault, and the scratch table is gone';
end
$planted$;
