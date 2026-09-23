-- =====================================================================
-- 040_shops.sql
-- odyssey1e — somebody to buy from, and one purchase that lands whole
-- =====================================================================
--
-- A SHOP IS A CHARACTER, and almost all of it already existed. The
-- shopkeeper is a `characters` row (022), their stock is a container
-- (032), their till is coins (039), they stand in a place (033), and
-- `move_into` already transfers things with a reach check. What was
-- missing was two facts about the person and one atomic transaction.
--
-- markup       what they charge above list. NULL means NOT A MERCHANT,
--              which is why it is nullable rather than defaulting to 1 -
--              every goblin in the game would otherwise be a shop.
-- disposition  how they feel about the customer. NULL reads as neutral.
--
-- Both feed `store::quote`. Condition is the fourth multiplier in that
-- model and gets no column, because nothing in this schema records wear
-- and a column nothing writes is a column that goes stale.
--
-- ---------------------------------------------------------------------
-- WHY THE PURCHASE IS A FUNCTION
--
-- Buying is three moves: the goods change hands, the coins change
-- hands, and the change comes back. Over REST that is three or more
-- round trips with no transaction around them, and the failure between
-- the first and second is a free item.
--
-- 012 settled this shape already for actions - "one swing is one thing:
-- an action owns its rolls, and write_action lands all of them or
-- none". A purchase is the same claim about goods and money.
--
-- THE PRICE IS DECIDED IN RUST AND PASSED IN, which is deliberate and
-- has a cost worth stating. `store.rs` holds the rules and is tested;
-- duplicating the arithmetic here would be the two-places problem this
-- repo keeps meeting. But it means this function TRUSTS its caller
-- about what things cost. That is acceptable while the DM runs the
-- table and unacceptable the day a player's client can call it
-- directly - at which point the quote has to be computed here, or
-- signed, and this comment is the warning that it was foreseen.
--
-- COINS ARE NAMED BY OBJECT, NOT BY KIND. A buyer's gold may be in a
-- purse inside a backpack, and `wallet` already sums at any depth - so
-- the payment plan carries the object ids Rust actually found rather
-- than a key the function would have to go hunting for.
-- =====================================================================

alter table public.characters add column markup numeric;
alter table public.characters add column disposition text
  check (disposition is null or disposition in
    ('allied','friendly','warm','neutral','unfriendly','hostile','sworn_enemy'));

comment on column public.characters.markup is
  'What this shopkeeper charges above list - 1.0 is list price. NULL MEANS NOT A MERCHANT, which is the whole reason it is nullable: a default of 1 would make every goblin a shop. Read by store::quote.';
comment on column public.characters.disposition is
  'How they feel about the customer, which moves the price: allied 0.80 through hostile 1.25, and sworn_enemy refuses to trade at all. NULL reads as neutral - a shopkeeper nobody has an opinion about charges list.';

-- ---------------------------------------------------------------------
-- Moving coins, at whatever depth they are.
-- ---------------------------------------------------------------------

create or replace function public.move_coin(
  p_object uuid,
  p_to     uuid,
  p_count  bigint
)
returns void
language plpgsql
security invoker
set search_path = ''
as $$
declare
  src public.objects%rowtype;
  dst uuid;
begin
  select * into src from public.objects where id = p_object;
  if src.id is null then
    raise exception 'no such coin, or it is not visible to you';
  end if;
  if p_count <= 0 or p_count > src.quantity then
    raise exception 'cannot move % of % coins', p_count, src.quantity;
  end if;

  -- Merge into whatever the destination already has of this kind. The
  -- stack index is unique on (holder_id, item_key) for unnamed things,
  -- so a second plain row would be refused anyway - see 037, where the
  -- same assumption was load-bearing and wrong.
  select id into dst
    from public.objects
   where holder_id = p_to and item_key = src.item_key and name is null;

  if dst is null then
    insert into public.objects (game_id, holder_id, item_key, quantity)
    values (src.game_id, p_to, src.item_key, p_count);
  else
    update public.objects set quantity = quantity + p_count where id = dst;
  end if;

  if p_count = src.quantity then
    delete from public.objects where id = src.id;
  else
    update public.objects set quantity = quantity - p_count where id = src.id;
  end if;
end;
$$;

comment on function public.move_coin(uuid, uuid, bigint) is
  'Move some of a coin stack to another holder, merging into whatever is already there. Named by OBJECT rather than by kind because a buyer''s gold may be inside a purse inside a backpack.';

revoke all on function public.move_coin(uuid, uuid, bigint) from public, anon;
grant execute on function public.move_coin(uuid, uuid, bigint) to authenticated;

-- ---------------------------------------------------------------------
-- One purchase, whole or not at all.
-- ---------------------------------------------------------------------

create or replace function public.buy_object(
  p_object   uuid,
  p_buyer    uuid,
  p_merchant uuid,
  p_pay      jsonb,
  p_change   jsonb
)
returns jsonb
language plpgsql
security invoker
set search_path = ''
as $$
declare
  goods      public.objects%rowtype;
  buyer_ent  uuid;
  merch_ent  uuid;
  e          jsonb;
  moved      bigint := 0;
  given      bigint := 0;
begin
  select * into goods from public.objects where id = p_object;
  if goods.id is null then
    raise exception 'no such thing for sale, or it is not visible to you';
  end if;

  select entity_id into buyer_ent from public.characters where id = p_buyer;
  select entity_id into merch_ent from public.characters where id = p_merchant;
  if buyer_ent is null or merch_ent is null then
    raise exception 'both a buyer and a seller are needed';
  end if;
  if buyer_ent = merch_ent then
    raise exception 'that is already yours';
  end if;

  -- The money first. If the buyer cannot actually pay, nothing has
  -- moved yet and the exception leaves the shelf untouched.
  for e in select * from jsonb_array_elements(coalesce(p_pay, '[]'::jsonb)) loop
    perform public.move_coin((e->>'object')::uuid, merch_ent, (e->>'count')::bigint);
    moved := moved + 1;
  end loop;

  for e in select * from jsonb_array_elements(coalesce(p_change, '[]'::jsonb)) loop
    perform public.move_coin((e->>'object')::uuid, buyer_ent, (e->>'count')::bigint);
    given := given + 1;
  end loop;

  -- And the goods. `equipped` and `attuned` are cleared by 031's
  -- trigger if the shelf was a container; a thing bought is a thing
  -- held, not a thing worn.
  update public.objects set holder_id = buyer_ent where id = p_object;

  return jsonb_build_object(
    'object', p_object,
    'coin_stacks_paid', moved,
    'coin_stacks_returned', given
  );
end;
$$;

comment on function public.buy_object(uuid, uuid, uuid, jsonb, jsonb) is
  'One purchase, whole or not at all: coins out, change back, goods across. The same claim write_action makes about a swing. TRUSTS ITS CALLER ABOUT THE PRICE - store.rs holds those rules and duplicating them here would be the two-places problem - which is safe while the DM runs the table and is not the day a player client calls this directly.';

revoke all on function public.buy_object(uuid, uuid, uuid, jsonb, jsonb) from public, anon;
grant execute on function public.buy_object(uuid, uuid, uuid, jsonb, jsonb) to authenticated;
