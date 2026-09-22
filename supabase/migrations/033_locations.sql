-- =====================================================================
-- 033_locations.sql
-- odyssey1e — somewhere to be
-- =====================================================================
--
-- 026 and 031 both ended on the same admission: a dropped object has no
-- location, because there is no floor, no room and no container to put
-- it on. 032 answered half of that - a chest is somewhere to be. This
-- is the other half, and it is the one 031's header named outright:
-- "a character gets one. A chest gets one. A room WILL get one."
--
-- A LOCATION IS AN ENTITY. That is the whole design, and everything
-- below follows from it. `objects.holder_id` already points at an
-- entity, so a room having one means a sword can rest on its floor with
-- NO CHANGE TO THE OBJECTS SCHEMA AT ALL. 031 was built for this.
--
-- DEPTH, NOT TIERS.
--
-- `parent_id` is a nullable self-reference and that is the entire
-- hierarchy. A universe, a continent, a tavern and a broom cupboard are
-- the same kind of row at different depths. There is no level column
-- and no path column: both are a recursive query away, and a stored
-- depth is one reparent from lying. Same reasoning as the dying
-- condition in 015 and the challenge watermark in 011 - derive what can
-- be derived, store only what cannot.
--
-- The prior art is Dave's own. OdysseyAIRPG's scene schema carries
-- `hierarchy.level` and `hierarchy.path`, both materialised, with the
-- convention "0=region, 1=settlement, 2=district, 3=building". Its live
-- data has 56 scenes and already disagrees with that comment. Computing
-- it is not a refinement of that design; it is the lesson from it.
--
-- `kind` IS THE MEDIUM, NOT THE RANK. structure, outdoors, underground,
-- aquatic, aerial, vehicle - the same axis OdysseyAIRPG's Config.js
-- documents for a scene. Depth says how far in you are; kind says what
-- sort of place it is. A ship is a `vehicle` that contains rooms and
-- will one day move.
--
-- AND IT IS CHECKED, because that field drifted in the original. The
-- scene `type` list is six values in Config.js, and the live data holds
-- `kingdom` and `village` - a FACTION type from campaign.js that leaked
-- into a place type, with nothing to stop it. Same species as the armour
-- spelling in 008: two vocabularies that look alike and no boundary
-- between them.
--
-- ZONES DO NOT EXIST HERE, deliberately. OdysseyAIRPG needed them
-- because a scene was heavyweight, so a room inside a building had to
-- be a lesser thing with its own shape and its own `parentZone` special
-- case. A location is cheap, so the Watch Room is simply a location
-- whose parent is The Guardhouse. One concept instead of two.
--
-- WHAT IS DELIBERATELY NOT HERE: travel, access, ownership, property,
-- coordinates. The original defined all of them up front and
-- `location.position` is still null years later. Access control is next
-- and it is what makes theft mean something - acquire.rs has been built
-- and waiting since 2026-09-22 - but it is a subsystem, not a column on
-- this table.
-- =====================================================================

-- ---------------------------------------------------------------------
-- The third kind of holder.
-- ---------------------------------------------------------------------

alter table public.entities drop constraint entities_kind_check;
alter table public.entities
  add constraint entities_kind_check
  check (kind in ('character', 'container', 'location'));

comment on column public.entities.kind is
  'character, container or location. Not a discriminator queries branch on - it is here so a reader can tell what they are looking at and a policy can ask. 033 added the third, which 031 said was coming.';

-- ---------------------------------------------------------------------

create table public.locations (
  id           uuid primary key default gen_random_uuid(),
  game_id      uuid not null references public.games(id) on delete cascade,
  entity_id    uuid unique references public.entities(id) on delete cascade,
  -- RESTRICT, not cascade. Deleting a continent should not silently
  -- delete its settlements and spill every object in them onto nowhere.
  -- Empty a place before you remove it; the refusal is the point.
  parent_id    uuid references public.locations(id) on delete restrict,
  name         text not null,
  kind         text not null default 'structure'
                 check (kind in ('structure', 'outdoors', 'underground',
                                 'aquatic', 'aerial', 'vehicle')),
  description  text,
  created_at   timestamptz not null default now()
);

comment on table public.locations is
  'Somewhere to be. A universe, a continent, a tavern and a broom cupboard are the same kind of row at different depths - parent_id is the entire hierarchy. Every location is an entity, so objects rest in one through holder_id with no change to the objects schema.';
comment on column public.locations.entity_id is
  'Its identity as a HOLDER. Set by trigger on insert, like a character''s. This is what lets objects.holder_id point at a room.';
comment on column public.locations.parent_id is
  'What contains this. NULL is the top of its own tree, so a universe and a loose cave are the same shape. NO LEVEL COLUMN AND NO PATH COLUMN: both are a recursive query away and a stored depth is one reparent from lying. OdysseyAIRPG materialised both and its data already disagrees with its own convention.';
comment on column public.locations.kind is
  'The MEDIUM, not the rank: structure, outdoors, underground, aquatic, aerial, vehicle. Depth says how far in you are, kind says what sort of place it is - a ship is a vehicle that contains rooms. Checked because the original''s equivalent field drifted, taking a faction type from campaign.js into a place type with nothing to stop it.';

create index locations_game_idx   on public.locations(game_id, name);
create index locations_parent_idx on public.locations(parent_id) where parent_id is not null;

-- ---------------------------------------------------------------------
-- A location is a holder from the moment it exists.
--
-- The same shape as characters_get_an_entity, and SECURITY DEFINER for
-- the same reason: the row and its entity are inserted together, and the
-- entity insert policy has not seen the location yet.
-- ---------------------------------------------------------------------

create or replace function public.location_gets_an_entity()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
begin
  insert into public.entities (game_id, kind)
  values (new.game_id, 'location')
  returning id into new.entity_id;
  return new;
end;
$$;

comment on function public.location_gets_an_entity() is
  'A location is a holder from the moment it exists. Mirrors character_gets_an_entity, and SECURITY DEFINER for the same reason.';

create trigger locations_get_an_entity
  before insert on public.locations
  for each row execute function public.location_gets_an_entity();

alter table public.locations alter column entity_id set not null;

-- AND LOSES IT AGAIN. The same function 031 wrote for characters and
-- containers: it reads old.entity_id and deletes it, which is table
-- agnostic. Deleting a room takes its entity, and everything resting on
-- its floor SET NULLs back to nowhere - the same way destroying a chest
-- spills it.
create trigger locations_lose_their_entity
  after delete on public.locations
  for each row execute function public.entity_goes_with_its_owner();

-- ---------------------------------------------------------------------
-- A place cannot be inside itself.
--
-- The mirror of no_container_cycles, and integrity rather than rules
-- for the same reason: a cycle does not make an answer wrong, it makes
-- a walk run until its guard trips.
--
-- Deeper guard than containers get. Eight was room enough for a purse
-- in a backpack; a universe, galaxy, system, world, continent, region,
-- settlement, district, building, floor, room, cupboard is twelve
-- before anyone has been unreasonable.
-- ---------------------------------------------------------------------

create or replace function public.no_location_cycles()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
declare
  cur   uuid := new.parent_id;
  guard int  := 0;
begin
  if new.parent_id is null then
    return new;
  end if;
  while cur is not null and guard < 32 loop
    if cur = new.id then
      raise exception 'a location cannot be inside itself';
    end if;
    select l.parent_id into cur from public.locations l where l.id = cur;
    guard := guard + 1;
  end loop;
  if guard >= 32 then
    raise exception 'locations nested too deep, or already looping';
  end if;
  return new;
end;
$$;

comment on function public.no_location_cycles() is
  'Refuses to put a place inside itself, directly or through any chain. Integrity rather than rules, the same as no_container_cycles. Capped at 32: twelve levels is a universe down to a cupboard, and the rest is headroom.';

create trigger locations_no_cycles
  before insert or update of parent_id on public.locations
  for each row execute function public.no_location_cycles();

-- A child must be in the same game as its parent. An FK cannot say so,
-- because it points at the id and not at the pair.
create or replace function public.location_parent_same_game()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
declare
  parent_game uuid;
begin
  if new.parent_id is null then
    return new;
  end if;
  select game_id into parent_game from public.locations where id = new.parent_id;
  if parent_game is distinct from new.game_id then
    raise exception 'a location and its parent must be in the same game';
  end if;
  return new;
end;
$$;

create trigger locations_parent_same_game
  before insert or update of parent_id on public.locations
  for each row execute function public.location_parent_same_game();

-- ---------------------------------------------------------------------
-- RLS. Members see the world; the DM builds it.
-- ---------------------------------------------------------------------

alter table public.locations enable row level security;

create policy "locations: members read"
  on public.locations for select using (public.is_game_member(game_id));
create policy "locations: dm writes"
  on public.locations for insert with check (public.is_game_dm(game_id));
create policy "locations: dm updates"
  on public.locations for update
  using (public.is_game_dm(game_id)) with check (public.is_game_dm(game_id));
create policy "locations: dm deletes"
  on public.locations for delete using (public.is_game_dm(game_id));

-- ---------------------------------------------------------------------
-- PICKING SOMETHING UP.
--
-- A gap that was invisible until now. Every write policy on `objects`
-- asks holder_character(holder_id) for a character the caller owns, and
-- an unheld object has no holder - so holder_character returns NULL and
-- only the DM could ever pick anything up. Nothing rested anywhere
-- before this migration, so nobody met it. A room full of loot that no
-- player can touch is a different matter.
--
-- The default is that loose things are free to take: an object resting
-- in a location, or lying nowhere at all, may be taken by any member of
-- the game. That is the sane starting rule and it is deliberately
-- generous - ACCESS CONTROL IS WHAT NARROWS IT. A locked room, an
-- owner, a witness and a crime are the next subsystem, and acquire.rs
-- is already written and waiting for them. This policy is the thing
-- that subsystem will tighten, not a decision that theft is free.
--
-- holder_character needs no change for any of it. A location's entity
-- matches neither branch of its walk - no character owns it, and no
-- object IS it - so the loop ends and returns NULL. An axe on the floor
-- is located and unowned, which is exactly right.
-- ---------------------------------------------------------------------

create or replace function public.holder_is_a_location(p_holder uuid)
returns boolean
language sql
stable
security definer
set search_path = ''
as $$
  select exists (select 1 from public.locations l where l.entity_id = p_holder);
$$;

comment on function public.holder_is_a_location(uuid) is
  'Whether this holder is a place rather than a creature or a container. Used by the objects policies to let a member pick up what is lying about. A separate function rather than an inline EXISTS because four policies ask it and one of them will otherwise drift.';

revoke all on function public.holder_is_a_location(uuid) from public, anon;
grant execute on function public.holder_is_a_location(uuid) to authenticated;

drop policy if exists "objects: holder or dm updates" on public.objects;

create policy "objects: holder or dm updates"
  on public.objects for update
  using (
    public.is_game_dm(game_id)
    or exists (select 1 from public.characters c
                where c.id = public.holder_character(holder_id)
                  and c.owner_uid = auth.uid())
    -- Loose: resting in a place, or nowhere at all.
    or ((holder_id is null or public.holder_is_a_location(holder_id))
        and public.is_game_member(game_id))
  )
  with check (
    public.is_game_dm(game_id)
    or exists (select 1 from public.characters c
                where c.id = public.holder_character(holder_id)
                  and c.owner_uid = auth.uid())
    or ((holder_id is null or public.holder_is_a_location(holder_id))
        and public.is_game_member(game_id))
  );
