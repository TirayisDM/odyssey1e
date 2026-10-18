-- 218. Scenery you can open.
--
-- A crate in the corner, a barrel, a bookshelf. Dave's line: scenery
-- that can be EXAMINED, SEARCHED or LOOTED FROM. Three interactions
-- are planned - Normal, Find Traps and Search - and the last two are
-- rolls against a difficulty. **ONLY NORMAL IS BUILT HERE:** opening it
-- shows what is inside, and what is inside can be taken.
--
-- A FIXTURE IS A CONTAINER THAT CANNOT BE CARRIED. That is the whole
-- idea, and it is why this is a `kind` rather than a new table: every
-- question a container already answers - what it accepts, how big a
-- thing it takes, how much room is left - is the same question for a
-- crate, and `accepts`, `holds_size` and `capacity_slots` are already
-- on `items`. What differs is one fact, enforced below: it stays where
-- it is put.
--
--
-- ONE. THE KIND, AND THE ENTITY THAT MAKES IT HOLD THINGS.
--
-- `container_gets_an_entity` was the single gate: an object gets its
-- own entity only when its item is `kind = 'container'`, and without an
-- entity nothing can point at it, so nothing can be inside it. A
-- fixture with no entity would have been scenery that could not be
-- looted - the one thing it is for. So the gate widens to both.
--
-- THE ENTITY IS STILL `kind = 'container'` on purpose. `entities.kind`
-- says what a thing DOES in the holder chain, and a fixture holds
-- things exactly as a sack does. Calling it 'fixture' there would make
-- `holders.rs` learn a second word for one behaviour.
--
--
-- TWO. A FIXTURE IS A PARTICULAR THING, NOT A STACK.
--
-- `objects_stack_idx` is unique on `(holder_id, item_key)` where the
-- name is null, and a floor is a holder - which is the whole reason two
-- dropped rations merge on the ground. **For a crate that is wrong.**
-- Two crates in one room hold different things and have different
-- entities; merging them into "Crate x2" would be one row with one
-- inside, and the second insert would simply be refused by the index.
--
-- So a fixture is NAMED when it arrives, after its item, and is forced
-- to quantity 1. A named row is outside that index, and `merge_into`
-- refuses a named thing in either direction, so two crates sit side by
-- side and never become each other. A DM renaming one to "the broken
-- crate" is the same mechanism, used deliberately.
--
--
-- THREE. AND IT STAYS WHERE IT IS PUT.
--
-- **IN THE DATABASE, NOT IN THE COMMANDS.** Four code paths already
-- move an object between holders and more will follow; a rule written
-- in each of them is a rule that the fifth one forgets. One trigger
-- covers every path there is or will be, including SQL typed by hand,
-- and it refuses with a sentence rather than a constraint name.
--
-- A fixture's holder is a LOCATION, or nothing at all. Nowhere stays
-- legal because 033 made nowhere a real answer - a crate built before
-- anybody decided which room it is in is not misfiled.
--
--
-- FOUR. WHAT IS LOOSE IN A ROOM IS EVERYONE'S - **HOWEVER DEEP.**
--
-- This is a bug that predates fixtures, found while building them and
-- proved before it was touched. 037 gave `objects` a third write
-- clause: holder is null, or the holder is a location, and you are in
-- the game. It asks about the IMMEDIATE holder only. So:
--
--     a chest standing in a room, rations inside it
--       holder_character(chest entity)     = null
--       holder_is_a_location(chest entity) = false
--       a player taking the rations        = 0 rows, REFUSED
--
-- Nobody owns the rations and nobody could take them. The chain is one
-- step longer than the clause can see, and `holder_character` right
-- beside it walks that chain properly - it was only ever the location
-- half that stopped at the first step.
--
-- `loose_in_the_world` is that walk, and it is deliberately NOT called
-- `holder_roots_in_a_location`: it answers true for a null holder too,
-- and a name that said "location" while returning true for nowhere
-- would be the kind of small lie this codebase keeps paying for.
--
-- IT GOES IN `security_doors`' ALLOWLIST, which is a wall with a hole
-- in it and wants its reason stated: like `holder_character` and
-- `holder_is_a_location`, THIS IS WHAT THE POLICIES THEMSELVES CALL to
-- decide who owns an entity, so it cannot check its caller - a caller
-- check would consult the policies that are mid-flight calling it. What
-- bounds it is the same thing that bounds the other two: it takes a
-- bare entity uuid and returns one boolean, and you only ever learn an
-- entity uuid by reading `objects` THROUGH RLS.

begin;

/* ---------- ONE: the kind ---------- */

alter table public.items drop constraint items_kind_check;
alter table public.items add constraint items_kind_check
  check (kind = any (array['weapon', 'armor', 'equipment', 'consumable',
                           'loot', 'container', 'instrument', 'fixture']));

create or replace function public.container_gets_an_entity()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
declare
  holds boolean;
begin
  if new.entity_id is not null then
    return new;
  end if;

  -- 218. A FIXTURE HOLDS THINGS TOO. Without an entity nothing can
  -- point at it, so scenery could not be looted from - which is the
  -- one thing it exists for.
  select i.kind in ('container', 'fixture') into holds
    from public.items i
   where i.key = new.item_key
     and (i.game_id is null or i.game_id = new.game_id)
   order by i.game_id nulls last
   limit 1;

  if coalesce(holds, false) then
    -- STILL 'container'. `entities.kind` says what a thing DOES in the
    -- holder chain, and this one holds things.
    insert into public.entities (game_id, kind)
    values (new.game_id, 'container')
    returning id into new.entity_id;
  end if;
  return new;
end;
$$;

/* ---------- TWO and THREE: a particular thing, and it stays put ---------- */

create or replace function public.a_fixture_stays_put()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
declare
  it record;
begin
  select i.kind, i.name into it
    from public.items i
   where i.key = new.item_key
     and (i.game_id is null or i.game_id = new.game_id)
   order by i.game_id nulls last
   limit 1;

  if it.kind is distinct from 'fixture' then
    return new;
  end if;

  if new.holder_id is not null
     and not public.holder_is_a_location(new.holder_id)
  then
    raise exception
      '% is scenery - it stands in a place and cannot be carried or packed',
      coalesce(new.name, it.name, new.item_key);
  end if;

  -- A PARTICULAR THING. Named so it never joins a stack, and one so
  -- the name is a sentence about a single crate.
  new.name := coalesce(new.name, it.name);
  new.quantity := 1;
  return new;
end;
$$;

drop trigger if exists objects_a_fixture_stays_put on public.objects;
create trigger objects_a_fixture_stays_put
  before insert or update on public.objects
  for each row execute function public.a_fixture_stays_put();

/* ---------- FOUR: loose in a room, however deep ---------- */

create or replace function public.loose_in_the_world(p_holder uuid)
returns boolean
language plpgsql
stable security definer
set search_path = ''
as $$
declare
  cur   uuid := p_holder;
  guard int  := 0;
begin
  -- Nowhere is everyone's, which is what 033 settled and 037 wrote
  -- into the policy as `holder_id is null`.
  if p_holder is null then
    return true;
  end if;

  while cur is not null and guard < 32 loop
    if exists (select 1 from public.locations l where l.entity_id = cur) then
      return true;
    end if;
    -- SOMEBODY'S, and the walk stops. A thing in a bag on a person is
    -- theirs however deep the bag is, which is `holder_character`'s
    -- half of this and must not be contradicted here.
    if exists (select 1 from public.characters c where c.entity_id = cur) then
      return false;
    end if;
    select o.holder_id into cur from public.objects o where o.entity_id = cur;
    guard := guard + 1;
  end loop;

  -- The chain ran out without meeting anybody: a chest that is itself
  -- nowhere. Loose, by the same reading as a null holder.
  -- Thirty-two deep is not a chain, it is a fault, and an unreadable
  -- chain is refused rather than guessed at.
  return guard < 32;
end;
$$;

grant execute on function public.loose_in_the_world(uuid) to authenticated;

drop policy if exists "objects: holder or dm updates" on public.objects;
create policy "objects: holder or dm updates"
  on public.objects for update
  using (
    public.is_game_dm(game_id)
    or exists (select 1 from public.characters c
                where c.id = public.holder_character(objects.holder_id)
                  and c.owner_uid = (select public.current_profile()))
    or (public.loose_in_the_world(holder_id) and public.is_game_member(game_id)))
  with check (
    public.is_game_dm(game_id)
    or exists (select 1 from public.characters c
                where c.id = public.holder_character(objects.holder_id)
                  and c.owner_uid = (select public.current_profile()))
    or (public.loose_in_the_world(holder_id) and public.is_game_member(game_id)));

drop policy if exists "objects: holder or dm deletes" on public.objects;
create policy "objects: holder or dm deletes"
  on public.objects for delete
  using (
    public.is_game_dm(game_id)
    or exists (select 1 from public.characters c
                where c.id = public.holder_character(objects.holder_id)
                  and c.owner_uid = (select public.current_profile()))
    or (public.loose_in_the_world(holder_id) and public.is_game_member(game_id)));

drop policy if exists "objects: holder or dm writes" on public.objects;
create policy "objects: holder or dm writes"
  on public.objects for insert
  with check (
    public.is_game_dm(game_id)
    or exists (select 1 from public.characters c
                where c.id = public.holder_character(objects.holder_id)
                  and c.owner_uid = (select public.current_profile()))
    or (public.loose_in_the_world(holder_id) and public.is_game_member(game_id)));

/* ---------- and the gate learns about the new hole ---------- */

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

  order by 1, 2
$$;

/* ---------- a starting set of scenery ---------- */
--
-- GLOBAL ROWS, `game_id` null, so every campaign has them. A DM wanting
-- "the Shrine of Vale" adds a row of their own with their `game_id` and
-- 174's rule gives it precedence over any global sharing its key.
--
-- NO WEIGHT AND NO PRICE, both nullable and both meaningless here: a
-- thing that cannot be carried has no carry weight, and a thing that is
-- not for sale has no price. `slots` is left at its default of 1 for
-- the same reason - it is how much room this takes INSIDE something,
-- and a fixture is never inside anything.

insert into public.items (key, name, kind, size, holds_size, capacity_slots, description)
values
  ('crate',      'Crate',          'fixture', 'lg', 'lg',  30,
   'A wooden crate, lid prised loose.'),
  ('barrel',     'Barrel',         'fixture', 'lg', 'med', 25,
   'A banded barrel standing on end.'),
  ('strongbox',  'Strongbox',      'fixture', 'med','med', 10,
   'An iron-bound box bolted to the floor.'),
  ('cupboard',   'Cupboard',       'fixture', 'lg', 'med', 30,
   'A tall cupboard with warped doors.'),
  ('bookshelf',  'Bookshelf',      'fixture', 'lg', 'sm',  20,
   'Shelves of ledgers, spines cracked.'),
  ('desk',       'Desk',           'fixture', 'lg', 'sm',  15,
   'A writing desk with shallow drawers.'),
  ('wardrobe',   'Wardrobe',       'fixture', 'lg', 'lg',  40,
   'A heavy wardrobe, one door ajar.'),
  ('rubble',     'Pile of Rubble', 'fixture', 'lg', 'med', 20,
   'Fallen stone and splintered timber.')
on conflict do nothing;

/* ---------- and prove every part of it, before this commits ---------- */

do $check$
declare
  g uuid; p1 uuid; bryn_e uuid; loc_e uuid;
  crate uuid; crate_e uuid; crate2 uuid; loot uuid;
  chest uuid; chest_e uuid; rats uuid;
  n int; msg text; blew boolean;
begin
  select c.entity_id, c.game_id into bryn_e, g
    from public.characters c where c.name = 'Bryn' and not c.is_npc;
  select id into p1 from public.profiles where display_name = 'Player One';
  select l.entity_id into loc_e from public.locations l where l.game_id = g limit 1;

  if g is null or loc_e is null or p1 is null then
    raise notice 'fixtures absent - skipping the live check';
    return;
  end if;

  -- (a) a fixture gets an entity, a name and a quantity of one
  insert into public.objects (game_id, holder_id, item_key, quantity)
    values (g, loc_e, 'crate', 7) returning id, entity_id into crate, crate_e;
  if crate_e is null then
    raise exception '218 FAILED: a crate got no entity, so nothing can be in it';
  end if;
  if (select name from public.objects where id = crate) is null then
    raise exception '218 FAILED: a crate was left unnamed and would stack';
  end if;
  if (select quantity from public.objects where id = crate) <> 1 then
    raise exception '218 FAILED: a crate kept a quantity of seven';
  end if;

  -- (b) a second plain crate stands beside the first
  insert into public.objects (game_id, holder_id, item_key)
    values (g, loc_e, 'crate') returning id into crate2;
  if crate2 = crate then
    raise exception '218 FAILED: the second crate merged into the first';
  end if;

  -- (c) it refuses to be carried
  blew := false;
  begin
    update public.objects set holder_id = bryn_e where id = crate;
  exception when others then blew := true; msg := sqlerrm;
  end;
  if not blew then
    raise exception '218 FAILED: a crate was picked up and carried away';
  end if;

  -- (d) a player takes loot OUT of the crate  (the deep-chain fix)
  insert into public.objects (game_id, holder_id, item_key, quantity)
    values (g, crate_e, 'rations', 3) returning id into loot;
  perform set_config('role', 'authenticated', true);
  perform set_config('request.jwt.claims',
                     json_build_object('sub', p1, 'role', 'authenticated')::text, true);
  update public.objects set holder_id = bryn_e where id = loot;
  get diagnostics n = row_count;
  perform set_config('role', 'postgres', true);
  if n <> 1 then
    raise exception '218 FAILED: a player could not take loot out of a crate';
  end if;
  -- Out of the way before (e) sends a second lot of the same thing to
  -- the same hands. A raw UPDATE does not merge the way `move_into`
  -- does, so the stack index would refuse it - which the first run of
  -- this check discovered, and is the index doing its job.
  delete from public.objects where id = loot;

  -- (e) and the same for a portable chest standing in the room, which
  --     is the bug that predates all of this
  insert into public.objects (game_id, holder_id, item_key, quantity)
    values (g, loc_e, 'chest', 1) returning id, entity_id into chest, chest_e;
  insert into public.objects (game_id, holder_id, item_key, quantity)
    values (g, chest_e, 'rations', 2) returning id into rats;
  perform set_config('role', 'authenticated', true);
  perform set_config('request.jwt.claims',
                     json_build_object('sub', p1, 'role', 'authenticated')::text, true);
  update public.objects set holder_id = bryn_e where id = rats;
  get diagnostics n = row_count;
  perform set_config('role', 'postgres', true);
  if n <> 1 then
    raise exception '218 FAILED: the deep-chain fix did not reach a chest in a room';
  end if;

  -- (f) and somebody else's pack is still nobody else's business
  perform set_config('role', 'authenticated', true);
  perform set_config('request.jwt.claims',
                     json_build_object('sub', p1, 'role', 'authenticated')::text, true);
  update public.objects set holder_id = loc_e
   where id = (select o.id from public.objects o
                 join public.characters c on c.entity_id = o.holder_id
                where c.name = 'Falon' and c.game_id = g limit 1);
  get diagnostics n = row_count;
  perform set_config('role', 'postgres', true);
  if n <> 0 then
    raise exception '218 FAILED: the door opened onto somebody else''s kit';
  end if;

  raise notice '218 holds. A crate refuses to be carried: %', msg;
  raise exception 'ROLLBACK_THE_FIXTURES_BUT_KEEP_THE_SCHEMA';
exception
  when others then
    if sqlerrm <> 'ROLLBACK_THE_FIXTURES_BUT_KEEP_THE_SCHEMA' then
      raise;
    end if;
    -- The probe's rows are gone with the subtransaction; the DDL above
    -- is not, because it is not in this block.
    null;
end
$check$;

commit;
