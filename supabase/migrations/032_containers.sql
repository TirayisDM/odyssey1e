-- =====================================================================
-- 032_containers.sql
-- odyssey1e — a spell book, a treasure chest and a coin purse
-- =====================================================================
--
-- 031 built the truss. This hangs containers on it.
--
-- A CONTAINER IS AN OBJECT, and that is the decision everything else
-- follows from. A coin purse is a thing you carry, lose, are robbed of
-- and can put inside a backpack - so it is not a table of its own, it
-- is an object that happens to have an entity. `objects.entity_id` is
-- what makes it one, and the trigger below fills it in.
--
-- The facts about a KIND of container live on `items`, beside the
-- weapon and armour facts that are already there. That is the existing
-- shape - `weapon_class` is null on a breastplate and `base_ac` is null
-- on a sword - and a separate profile table would be a second place to
-- look for the same sort of thing.
--
-- ---------------------------------------------------------------------
-- THE VOCABULARY IS DAVE'S, from odyssey-engine's container.rs
--
--   content_tags   what a thing IS: coin, ammunition, lore, liquid.
--   accepts        what a container TAKES. Empty means anything.
--   slots          how much room a thing occupies.
--   capacity_slots how much room a container has.
--
-- That file has had `allowed_tags` and a coin purse restricted to
-- `["coin"]` for a long time; this is the same rule in a schema. Its
-- slot arithmetic comes across intact too: a coin purse is 5 slots and
-- holds 25 coins, so a coin is 0.2 of one.
--
-- WHAT IS NOT CARRIED OVER YET, all of it from the same prior art and
-- all of it deliberate: ItemSize and size_limit (a sword should not go
-- in a purse for a second reason), open/closed and locked state, and
-- max_nesting_depth. The cycle guard below is the one piece of that
-- family that had to come now, because a cycle does not make a rule
-- wrong - it makes `holder_character` walk forever.
--
-- ---------------------------------------------------------------------
-- WHERE THE ACCEPTANCE RULE LIVES: Rust, in containers.rs, tested.
--
-- Deciding whether a purse takes a sword needs `items.accepts` and the
-- sword's `content_tags` - two rows in another table - which is the
-- same reason 008 gave for putting the one-armor rule in equipment.rs
-- rather than in a constraint. The cycle guard is different and stays
-- here: that is an integrity invariant, not a game rule, and the
-- database is the only thing that sees every write.
--
-- COINS ARE A DUMMY, as asked. Three `loot` rows tagged `coin` so the
-- purse has something to refuse everything else in favour of. Real
-- currency is its own subsystem and is next; `items.kind` has no
-- 'currency' value and this migration deliberately does not add one,
-- because that choice belongs with the subsystem that needs it.
-- =====================================================================

alter table public.items add column content_tags   text[] not null default '{}';
alter table public.items add column accepts        text[] not null default '{}';
alter table public.items add column slots          numeric not null default 1;
alter table public.items add column capacity_slots numeric;

comment on column public.items.content_tags is
  'What this thing IS, for a container to decide whether it will take it: coin, ammunition, lore, liquid. From odyssey-engine''s Item.content_tags. Empty means it answers to no restriction and only fits in unrestricted containers.';
comment on column public.items.accepts is
  'For kind=container: which content_tags it will take. EMPTY MEANS ANYTHING - a backpack, not a locked box. A coin purse is {coin}, which is the rule odyssey-engine''s coin_purse() has had all along.';
comment on column public.items.slots is
  'How much room one of these takes. Fractional on purpose: a coin purse is 5 slots and holds 25 coins, so a coin is 0.2 - the same arithmetic the earlier game used.';
comment on column public.items.capacity_slots is
  'For kind=container: how much room it has. NULL on everything else, and NULL on a container means no limit is known - which is a fault, not "infinite".';

-- ---------------------------------------------------------------------
-- An object that is a container gets an entity, the same way a
-- character does. Nothing can be inside it until it has one.
-- ---------------------------------------------------------------------

create or replace function public.container_gets_an_entity()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
declare
  is_container boolean;
begin
  if new.entity_id is not null then
    return new;
  end if;

  select i.kind = 'container' into is_container
    from public.items i
   where i.key = new.item_key
     and (i.game_id is null or i.game_id = new.game_id)
   order by i.game_id nulls last
   limit 1;

  if coalesce(is_container, false) then
    insert into public.entities (game_id, kind)
    values (new.game_id, 'container')
    returning id into new.entity_id;
  end if;
  return new;
end;
$$;

comment on function public.container_gets_an_entity() is
  'Gives a chest, a purse or a spell book its holder identity on the way in, so something can be put inside it. Mirrors character_gets_an_entity, and SECURITY DEFINER for the same reason.';

create trigger objects_container_gets_an_entity
  before insert on public.objects
  for each row execute function public.container_gets_an_entity();

-- ---------------------------------------------------------------------
-- Nothing goes inside itself.
--
-- An INTEGRITY invariant, not a rule of the game, which is why it is
-- here and the acceptance rule is not. A cycle does not make an answer
-- wrong; it makes holder_character walk until its guard trips, and
-- every policy on `objects` calls that function.
-- ---------------------------------------------------------------------

create or replace function public.no_container_cycles()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
declare
  cur   uuid := new.holder_id;
  guard int  := 0;
begin
  if new.entity_id is null or new.holder_id is null then
    return new;
  end if;
  while cur is not null and guard < 16 loop
    if cur = new.entity_id then
      raise exception 'a container cannot be inside itself';
    end if;
    select o.holder_id into cur from public.objects o where o.entity_id = cur;
    guard := guard + 1;
  end loop;
  if guard >= 16 then
    raise exception 'containers nested too deep, or already looping';
  end if;
  return new;
end;
$$;

comment on function public.no_container_cycles() is
  'Refuses to put a container inside itself, directly or through any chain. Integrity rather than rules: a cycle makes holder_character walk until its guard trips, and every objects policy calls it.';

create trigger objects_no_container_cycles
  before insert or update of holder_id on public.objects
  for each row execute function public.no_container_cycles();

-- ---------------------------------------------------------------------
-- The catalogue: containers, and coins to put in one.
-- ---------------------------------------------------------------------

insert into public.items
  (key, game_id, name, kind, base_item, accepts, capacity_slots, slots,
   content_tags, rarity, price, denom, weight)
values
  ('backpack',   null, 'Backpack',      'container', 'backpack',  '{}',                        20, 1,   '{}', null,  2, 'gp', 5),
  ('coin_purse', null, 'Coin Purse',    'container', 'pouch',     array['coin']::text[],        5, 0.5, '{}', null,  5, 'sp', 1),
  ('chest',      null, 'Treasure Chest','container', 'chest',     '{}',                        30, 8,   '{}', null,  5, 'gp', 25),
  ('spell_book', null, 'Spell Book',    'container', 'spellbook', array['spell']::text[],      10, 1,   array['lore']::text[], null, 50, 'gp', 3),
  ('quiver',     null, 'Quiver',        'container', 'quiver',    array['ammunition']::text[], 10, 1,   '{}', null,  1, 'gp', 1);

-- Coins. A dummy until currency is its own subsystem - three rows that
-- exist so the purse has something to accept and everything else to
-- refuse. 5e weight: fifty coins to the pound.
insert into public.items
  (key, game_id, name, kind, slots, content_tags, price, denom, weight)
values
  ('coin_gp', null, 'Gold Piece',   'loot', 0.2, array['coin']::text[], 1, 'gp', 0.02),
  ('coin_sp', null, 'Silver Piece', 'loot', 0.2, array['coin']::text[], 1, 'sp', 0.02),
  ('coin_cp', null, 'Copper Piece', 'loot', 0.2, array['coin']::text[], 1, 'cp', 0.02);

-- Arrows, so the quiver means something and so the `amm` property on
-- five weapons has an answer. Twenty to a bundle is the 5e purchase.
insert into public.items
  (key, game_id, name, kind, slots, content_tags, price, denom, weight)
values
  ('arrow', null, 'Arrow', 'loot', 0.05, array['ammunition']::text[], 1, 'gp', 0.05);
