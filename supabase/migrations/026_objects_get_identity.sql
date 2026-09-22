-- =====================================================================
-- 026_objects_get_identity.sql
-- odyssey1e — a sword becomes a particular sword
-- =====================================================================
--
-- THE GAP, named by the framework. Every library wants the same spine:
-- a type, an individual made from it, and wherever it turns up.
-- Characters got theirs in 022 - npcs is the type, a characters row is
-- the individual, encounter_actors is the appearance. Objects had a
-- type and nothing else.
--
-- `character_items` was doing half the job already. It carried charges,
-- attunement, whether a thing was equipped and whether its holder was
-- proficient with it - all facts about ONE sword rather than about
-- swords. But it was keyed (character_id, item_key), and that key is
-- the whole limitation:
--
--   Two shortswords could not differ. One row, one state.
--   Nothing could be named. "Runt's axe" had nowhere to live.
--   Nothing could exist unheld. No sword on the floor, no chest in a
--     room, no loot before someone picks it up.
--
-- 008 wrote the consequence down without flinching: "a +1 sword is not
-- a sword", meaning a second CATALOGUE entry. That was honest and it is
-- the wrong shape - it makes the rulebook grow every time one weapon in
-- one campaign gets enchanted.
--
-- WHAT THIS DOES. The table becomes `objects` and gains a surrogate id.
-- It is no longer a junction between a character and a catalogue row; it
-- is the individual, and who holds it is one of its facts.
--
--   items          the type. Facts about swords.
--   objects        THE SWORD. Its charges, its name, its holder.
--   npc_items      a statblock's kit - still a type-side list, because
--                  a pattern is not an individual.
--
-- STACKS SURVIVE, and deliberately. Seven rations stay one row with
-- quantity 7, exactly as 008 intended: a stack is an object that
-- happens to be several. A named or damaged thing has quantity 1
-- because it cannot be interchangeable with anything.
--
-- UNHELD IS NOW EXPRESSIBLE. character_id becomes nullable, and NULL
-- means the object exists in the campaign and nobody is carrying it.
-- There is nowhere to PUT it yet - Locations is the thinnest shelf in
-- the framework and there is no room, no floor, no container - so today
-- this is loot in limbo. It is still worth having: it is what a drop, a
-- chest and a shop all need, and the alternative was deleting objects
-- when their holder let go of them.
--
-- WHAT THIS IS NOT. It is not the `entities` component work. A holder
-- here is a character and only a character, with a real foreign key.
-- When a location or another object can hold something, that is the
-- moment `entities` is earned - and doing it now with a nullable
-- character_id beside a nullable location_id under an XOR is precisely
-- the shape 022 and 023 removed. One beam is not a truss.
--
-- game_id ARRIVES so an unheld object still belongs somewhere. Until
-- now the campaign was reachable only through the holder, which is no
-- use for a thing nobody holds, and the policies below need it.
-- =====================================================================

alter table public.character_items rename to objects;

-- The identity. A surrogate id, exactly as every reference table here
-- uses, and for the same reason: the natural key cannot be unique.
alter table public.objects drop constraint character_items_pkey;
alter table public.objects add column id uuid not null default gen_random_uuid();
alter table public.objects add constraint objects_pkey primary key (id);

alter table public.objects add column game_id uuid references public.games(id) on delete cascade;
alter table public.objects add column name text;

-- Backfill the campaign from whoever is holding the thing, then make it
-- required: every object belongs to a game even when nobody holds it.
update public.objects o
   set game_id = c.game_id
  from public.characters c
 where c.id = o.character_id;

alter table public.objects alter column game_id set not null;
alter table public.objects alter column character_id drop not null;

comment on table public.objects is
  'THE INDIVIDUAL OBJECT - this sword, not swords. Its charges, its name, whether it is equipped, and who is holding it. `items` is the type it was made from; see 026 for why the junction it used to be could not express two shortswords that differ.';
comment on column public.objects.id is
  'Surrogate, because the natural key cannot be unique: a character may hold two of the same type in different states, which is the whole point of 026.';
comment on column public.objects.item_key is
  'THE TYPE. References items.key by value - see items.key for why not a FK. Every fact about swords lives there; everything about THIS sword lives here.';
comment on column public.objects.character_id is
  'Who is holding it. NULL means nobody - the object exists in the campaign and is not carried. There is nowhere to put it yet, because Locations is still a stub; this is what a drop, a chest and a shop will all need.';
comment on column public.objects.game_id is
  'The campaign. Reached through the holder until 026, which is no use for a thing nobody holds.';
comment on column public.objects.name is
  'What this one is called, when it has earned a name. NULL means call it by its type - most swords are just swords. Same reasoning as encounter_actors.label against a statblock name.';
comment on column public.objects.quantity is
  'Stacks survive 026. Seven rations are one object with quantity 7, not seven objects - a stack is an object that happens to be several. Anything named or damaged is quantity 1, because it cannot be interchangeable.';

-- One character cannot hold two rows of the same INTERCHANGEABLE type -
-- that is what quantity is for. A named one is exempt, because that is
-- exactly the thing the old key made impossible.
create unique index objects_stack_idx
  on public.objects(character_id, item_key)
  where character_id is not null and name is null;

create index objects_game_idx   on public.objects(game_id);
create index objects_holder_idx on public.objects(character_id) where character_id is not null;

-- ---------------------------------------------------------------------
-- Policies. The old ones reached the campaign through the holder, which
-- an unheld object does not have.
-- ---------------------------------------------------------------------

drop policy if exists "character_items: read with character"   on public.objects;
drop policy if exists "character_items: owner or dm writes"    on public.objects;
drop policy if exists "character_items: owner or dm updates"   on public.objects;
drop policy if exists "character_items: owner or dm deletes"   on public.objects;

create policy "objects: members read"
  on public.objects for select
  using (public.is_game_member(game_id));

create policy "objects: holder or dm writes"
  on public.objects for insert
  with check (
    public.is_game_dm(game_id)
    or exists (select 1 from public.characters c
                where c.id = character_id and c.owner_uid = auth.uid())
  );

create policy "objects: holder or dm updates"
  on public.objects for update
  using (
    public.is_game_dm(game_id)
    or exists (select 1 from public.characters c
                where c.id = character_id and c.owner_uid = auth.uid())
  )
  with check (
    public.is_game_dm(game_id)
    or exists (select 1 from public.characters c
                where c.id = character_id and c.owner_uid = auth.uid())
  );

create policy "objects: holder or dm deletes"
  on public.objects for delete
  using (
    public.is_game_dm(game_id)
    or exists (select 1 from public.characters c
                where c.id = character_id and c.owner_uid = auth.uid())
  );

-- ---------------------------------------------------------------------
-- instantiate_npc copies a kit into the new table.
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
    hp_max, ac_mode, ac_override, size)
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
    n.size)
  returning id into cid;

  update public.character_abilities a
     set score = case a.ability::text
                   when 'str' then n.str when 'dex' then n.dex
                   when 'con' then n.con when 'int' then n.intl
                   when 'wis' then n.wis when 'cha' then n.cha
                 end
   where a.character_id = cid;

  -- Each kit entry becomes a real OBJECT now, not a junction row.
  insert into public.objects
    (game_id, character_id, item_key, quantity, equipped, proficient_override)
  select p_game_id, cid, x.item_key, x.quantity, x.equipped,
         coalesce(x.proficient_override, true)
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
