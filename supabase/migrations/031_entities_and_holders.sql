-- =====================================================================
-- 031_entities_and_holders.sql
-- odyssey1e — the truss, finally earned
-- =====================================================================
--
-- 026 WROTE THE CONDITION AND THIS IS IT. Its header said: "A holder
-- here is a character and only a character, with a real foreign key.
-- When a location or another object can hold something, that is the
-- moment `entities` is earned - and doing it now with a nullable
-- character_id beside a nullable location_id under an XOR is precisely
-- the shape 022 and 023 removed. One beam is not a truss."
--
-- A container can hold something. So: entities.
--
-- THE PRIOR ART IS DAVE'S OWN. odyssey-engine's container.rs has had
-- `HolderType { Actor, Tile, Container }` and a `Placement { item_uid,
-- holder_type, holder_id }` since long before this port. That is this
-- table, written down: a shared id space over the kinds of thing that
-- can hold something, so a placement is ONE column with ONE foreign key
-- instead of a column per kind under a check constraint nobody can
-- extend.
--
-- ---------------------------------------------------------------------
-- WHAT AN ENTITY IS
--
-- An identity for something that can HOLD. Not for everything - an
-- arrow gets no entity, because nothing goes inside an arrow. A
-- character gets one. A chest gets one. A coin purse gets one. A room
-- will get one when Locations stops being a stub, and that is the whole
-- reason this is a table and not a second nullable column.
--
-- `kind` is not a discriminator the queries branch on; it is there so a
-- reader can tell what they are looking at, and so a policy can ask.
--
-- ---------------------------------------------------------------------
-- WHAT AN OBJECT NOW CARRIES
--
--   holder_id   WHERE IT IS. One column, one FK, nullable. NULL means
--               nowhere - still the loot-in-limbo 026 accepted, and now
--               the only thing that means it.
--
--   entity_id   WHAT IT IS, when it can hold. A chest is both: it is
--               somewhere, and things are inside it. Unique, because an
--               entity belongs to exactly one object.
--
-- `character_id` GOES. Two columns that both answer "where is this" is
-- two sources of truth and the second one would rot - see 028, where a
-- fact stated in two places disagreed with itself and nobody noticed
-- until a goblin swung a sickle at -1.
--
-- A SWORD IN A CHEST IS NOT EQUIPPED. 030's trigger said "unheld means
-- unequipped" and could only check for NULL. It asks a better question
-- now: equipped means held DIRECTLY by a character, so putting a
-- breastplate into a backpack takes it off.
-- =====================================================================

create table public.entities (
  id      uuid primary key default gen_random_uuid(),
  game_id uuid not null references public.games(id) on delete cascade,
  kind    text not null check (kind in ('character', 'container'))
);

comment on table public.entities is
  'An identity for something that can HOLD something else. A shared id space so a placement is ONE column with ONE foreign key - see 031, and see odyssey-engine''s HolderType, which is the same idea from an earlier game. Not everything gets one: nothing goes inside an arrow.';
comment on column public.entities.kind is
  'character or container, and a location when Locations stops being a stub. Not a discriminator queries branch on - it is here so a reader can tell what they are looking at and a policy can ask.';

create index entities_game_idx on public.entities(game_id);

alter table public.entities enable row level security;

create policy "entities: members read"
  on public.entities for select using (public.is_game_member(game_id));
create policy "entities: members write"
  on public.entities for insert with check (public.is_game_member(game_id));
create policy "entities: dm deletes"
  on public.entities for delete using (public.is_game_dm(game_id));

-- ---------------------------------------------------------------------
-- Every character is a holder.
-- ---------------------------------------------------------------------

alter table public.characters
  add column entity_id uuid unique references public.entities(id) on delete cascade;

comment on column public.characters.entity_id is
  'This character AS A HOLDER. Filled by trigger on insert and never by hand. Objects point at this rather than at the character, which is what lets one column mean "where is this" for a chest and a goblin alike.';

insert into public.entities (id, game_id, kind)
select gen_random_uuid(), c.game_id, 'character' from public.characters c;

-- Pair them up. Deterministic rather than clever: one fresh entity per
-- character, matched by row number within each game.
with paired as (
  select c.id as cid, e.id as eid
    from (select id, game_id, row_number() over (partition by game_id order by id) rn
            from public.characters) c
    join (select id, game_id, row_number() over (partition by game_id order by id) rn
            from public.entities where kind = 'character') e
      on e.game_id = c.game_id and e.rn = c.rn
)
update public.characters c set entity_id = p.eid from paired p where p.cid = c.id;

alter table public.characters alter column entity_id set not null;

create or replace function public.character_gets_an_entity()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
begin
  insert into public.entities (game_id, kind)
  values (new.game_id, 'character')
  returning id into new.entity_id;
  return new;
end;
$$;

comment on function public.character_gets_an_entity() is
  'A character is a holder from the moment it exists. SECURITY DEFINER because a DM enrolling a monster inserts a character they will own but whose entity the insert policy has not seen yet - the same reason instantiate_npc is what it is.';

create trigger characters_get_an_entity
  before insert on public.characters
  for each row execute function public.character_gets_an_entity();

-- AND LOSES IT AGAIN, which is the half that is easy to miss. Nothing
-- deletes an entity when its owner goes, because the foreign key points
-- the other way - so without this a deleted character would leave its
-- entity standing and its gear would go on being held by a ghost. 030
-- says deleting a creature drops what it was carrying, and this is what
-- keeps that true: the entity goes, and objects.holder_id SET NULLs.
create or replace function public.entity_goes_with_its_owner()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
begin
  if old.entity_id is not null then
    delete from public.entities where id = old.entity_id;
  end if;
  return old;
end;
$$;

comment on function public.entity_goes_with_its_owner() is
  'Deletes the entity a character or a container object owned. The FK points from owner to entity, so nothing else would - and a surviving entity would leave gear held by a ghost. Destroying a chest spills it for the same reason: its entity goes, and everything inside SET NULLs to nowhere.';

create trigger characters_lose_their_entity
  after delete on public.characters
  for each row execute function public.entity_goes_with_its_owner();

-- A destroyed chest spills. Same function, same reasoning: the entity
-- is what its contents point at, so it has to go with the object.
create trigger objects_lose_their_entity
  after delete on public.objects
  for each row execute function public.entity_goes_with_its_owner();

-- ---------------------------------------------------------------------
-- Objects point at entities now.
-- ---------------------------------------------------------------------

alter table public.objects
  add column holder_id uuid references public.entities(id) on delete set null;
alter table public.objects
  add column entity_id uuid unique references public.entities(id) on delete cascade;

comment on column public.objects.holder_id is
  'WHERE IT IS. A character''s entity, a container''s entity, or NULL for nowhere. Replaced character_id in 031: two columns answering one question is two sources of truth, and the second rots.';
comment on column public.objects.entity_id is
  'WHAT IT IS, when it can hold - a chest, a purse, a spell book. NULL for an arrow, because nothing goes inside an arrow. A chest carries both: it is somewhere, and things are inside it.';

update public.objects o
   set holder_id = c.entity_id
  from public.characters c
 where c.id = o.character_id;

-- ---------------------------------------------------------------------
-- The old column, and everything that read it.
-- ---------------------------------------------------------------------

drop index if exists public.objects_stack_idx;
drop index if exists public.objects_holder_idx;

drop policy if exists "objects: holder or dm writes"   on public.objects;
drop policy if exists "objects: holder or dm updates"  on public.objects;
drop policy if exists "objects: holder or dm deletes"  on public.objects;

alter table public.objects drop column character_id;

create unique index objects_stack_idx
  on public.objects(holder_id, item_key)
  where holder_id is not null and name is null;

create index objects_holder_idx on public.objects(holder_id) where holder_id is not null;

-- Who, if anyone, ultimately has this - walking up through however many
-- containers. A purse inside a backpack carried by Rodnar is Rodnar's,
-- and a policy that only looked one level up would say it is nobody's.
--
-- Depth-capped at eight. odyssey-engine defaults max nesting to six;
-- this is that plus room, and it is a guard against a cycle rather than
-- a rule about containers - a cycle should be impossible, and a
-- recursive policy that can hang is not worth the elegance.
create or replace function public.holder_character(p_holder uuid)
returns uuid
language plpgsql
stable
security definer
set search_path = ''
as $$
declare
  cur   uuid := p_holder;
  cid   uuid;
  guard int  := 0;
begin
  while cur is not null and guard < 8 loop
    select id into cid from public.characters where entity_id = cur;
    if cid is not null then
      return cid;
    end if;
    -- Not a character, so it is a container: step up to whatever holds
    -- the object that container IS.
    select o.holder_id into cur from public.objects o where o.entity_id = cur;
    guard := guard + 1;
  end loop;
  return null;
end;
$$;

comment on function public.holder_character(uuid) is
  'The character who ultimately has whatever is in this holder, walking up through containers. A purse inside a backpack carried by Rodnar is Rodnar''s. Depth-capped at eight: a guard against a cycle, not a rule about containers.';

revoke all on function public.holder_character(uuid) from public, anon;
grant execute on function public.holder_character(uuid) to authenticated;

create policy "objects: holder or dm writes"
  on public.objects for insert
  with check (
    public.is_game_dm(game_id)
    or exists (select 1 from public.characters c
                where c.id = public.holder_character(holder_id)
                  and c.owner_uid = auth.uid())
  );

create policy "objects: holder or dm updates"
  on public.objects for update
  using (
    public.is_game_dm(game_id)
    or exists (select 1 from public.characters c
                where c.id = public.holder_character(holder_id)
                  and c.owner_uid = auth.uid())
  )
  with check (
    public.is_game_dm(game_id)
    or exists (select 1 from public.characters c
                where c.id = public.holder_character(holder_id)
                  and c.owner_uid = auth.uid())
  );

create policy "objects: holder or dm deletes"
  on public.objects for delete
  using (
    public.is_game_dm(game_id)
    or exists (select 1 from public.characters c
                where c.id = public.holder_character(holder_id)
                  and c.owner_uid = auth.uid())
  );

-- ---------------------------------------------------------------------
-- 030's trigger, asking a better question.
-- ---------------------------------------------------------------------

create or replace function public.unheld_is_unequipped()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
begin
  -- EQUIPPED MEANS IN SOMEBODY'S HANDS, not merely somewhere. Until 031
  -- the only alternative to being held was being nowhere, so a NULL
  -- check was the whole rule. Now a breastplate can be in a backpack,
  -- and a breastplate in a backpack is not being worn.
  if new.holder_id is null
     or not exists (select 1 from public.characters c where c.entity_id = new.holder_id)
  then
    new.equipped := false;
    new.attuned  := false;
  end if;
  return new;
end;
$$;

update public.objects
   set equipped = false, attuned = false
 where (equipped or attuned)
   and (holder_id is null
        or not exists (select 1 from public.characters c where c.entity_id = holder_id));

-- ---------------------------------------------------------------------
-- instantiate_npc, which inserted character_id.
-- ---------------------------------------------------------------------

create or replace function public.instantiate_npc(
  p_npc_key  text,
  p_game_id  uuid,
  p_label    text
)
returns uuid
language plpgsql
security invoker
set search_path = ''
as $$
declare
  n   public.npcs%rowtype;
  cid uuid;
  eid uuid;
begin
  select * into n
    from public.npcs
   where key = p_npc_key
     and (game_id is null or game_id = p_game_id)
   order by game_id nulls last
   limit 1;

  if n.key is null then
    raise exception 'no statblock with key %', p_npc_key;
  end if;

  insert into public.characters (
    game_id, owner_uid, name, is_npc, level, prof_bonus,
    hp_max, ac_mode, ac_override, size,
    weapon_profs, armor_profs)
  values (
    p_game_id,
    (select g.dm_uid from public.games g where g.id = p_game_id),
    coalesce(nullif(btrim(p_label), ''), n.name),
    true,
    n.level,
    n.prof_bonus,
    n.hp_max,
    'flat',
    n.ac,
    n.size,
    n.weapon_profs,
    n.armor_profs)
  returning id, entity_id into cid, eid;

  update public.character_abilities a
     set score = case a.ability::text
                   when 'str' then n.str when 'dex' then n.dex
                   when 'con' then n.con when 'int' then n.intl
                   when 'wis' then n.wis when 'cha' then n.cha
                 end
   where a.character_id = cid;

  insert into public.objects
    (game_id, holder_id, item_key, quantity, equipped, proficient_override)
  select p_game_id, eid, x.item_key, x.quantity, x.equipped,
         x.proficient_override
    from (
      select distinct on (i.item_key) i.*
        from public.npc_items i
       where i.npc_key = p_npc_key
         and (i.game_id is null or i.game_id = p_game_id)
       order by i.item_key, i.game_id nulls last
    ) x;

  return cid;
end;
$$;

revoke all on function public.instantiate_npc(text, uuid, text) from public, anon;
grant execute on function public.instantiate_npc(text, uuid, text) to authenticated;
