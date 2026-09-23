-- =====================================================================
-- 041_trade.sql
-- odyssey1e — one exchange, in both directions
-- =====================================================================
--
-- 040's `buy_object` was the asymmetric special case and it is replaced
-- here. It had a buyer and a merchant baked into its signature, goods
-- moving one way and coins moving two - which is half a bidirectional
-- trade with the other half hidden behind role names.
--
-- `trade` is the symmetric version, and it is SIMPLER than what it
-- replaces: two parties, one list of moves. Four things become one
-- function and four callers:
--
--   buy     one side gives coin
--   sell    the same with the columns swapped
--   barter  both sides give goods, the difference settled in coin
--   give    one side gives nothing
--
-- ---------------------------------------------------------------------
-- A VALIDATING EXECUTOR, NOT A RULEBOOK
--
-- Every DECISION is made in Rust and arrives as a plan. Which coins pay
-- for a thing is `currency::pay`; what it costs is `store::quote`;
-- whether a stack merges or splits is `objects::merge_into`. None of
-- that is repeated here, because a rule written in two places is a rule
-- that will disagree with itself - which this repo has now proved four
-- times, most expensively in 028.
--
-- What SQL owns is the two things Rust cannot: that all of it lands or
-- none of it does, and that the plan is not lying. Over REST a trade is
-- six or more round trips with no transaction around them, and the gap
-- between "goods delivered" and "coins taken" is a free item.
--
-- DESTINATIONS ARE 'a' OR 'b', NEVER AN ENTITY ID. The caller names
-- which SIDE of the trade a thing goes to and this resolves it. A plan
-- carrying raw entity ids could quietly send the goods to a third
-- party who was never in the conversation, and the signature would look
-- fine.
--
-- CONSENT IS NOT MODELLED. Both sides' rows move on one party's call,
-- because the DM runs the table and RLS already stops a player touching
-- what is not theirs. A player-to-player trade that neither DM nor
-- policy mediates needs an offer-and-accept handshake, and this is the
-- note saying that was seen rather than missed.
-- =====================================================================

drop function if exists public.buy_object(uuid, uuid, uuid, jsonb, jsonb);
drop function if exists public.move_coin(uuid, uuid, bigint);

-- ---------------------------------------------------------------------
-- Moving some of one thing.
-- ---------------------------------------------------------------------

create or replace function public.trade_move(
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
  n   bigint;
begin
  select * into src from public.objects where id = p_object;
  if src.id is null then
    raise exception 'no such object, or it is not visible to you';
  end if;

  n := coalesce(p_count, src.quantity);
  if n <= 0 or n > src.quantity then
    raise exception 'cannot move % of % %', n, src.quantity, src.item_key;
  end if;
  if n < src.quantity and src.name is not null then
    -- A named thing is one thing - objects.rs::may_name refuses a name
    -- on a stack, so this should be unreachable and is checked because
    -- the alternative is silently minting a second Runt's Axe.
    raise exception 'a named object cannot be split';
  end if;

  -- Merge into what is already there, when both are interchangeable.
  -- NOT a rule - objects_stack_idx is unique on (holder_id, item_key)
  -- for unnamed rows, so a second plain row would be refused anyway.
  -- This is the mechanical consequence of that index.
  if src.name is null then
    select id into dst
      from public.objects
     where holder_id = p_to and item_key = src.item_key and name is null;
  end if;

  if dst is not null then
    update public.objects set quantity = quantity + n where id = dst;
    if n = src.quantity then
      delete from public.objects where id = src.id;
    else
      update public.objects set quantity = quantity - n where id = src.id;
    end if;
    return;
  end if;

  if n = src.quantity then
    -- The whole row travels, so its name, charges and overrides travel
    -- with it. That is why it is moved rather than deleted and remade.
    update public.objects set holder_id = p_to where id = src.id;
  else
    update public.objects set quantity = quantity - n where id = src.id;
    insert into public.objects
      (game_id, holder_id, item_key, quantity, size_override)
    values (src.game_id, p_to, src.item_key, n, src.size_override);
  end if;
end;
$$;

comment on function public.trade_move(uuid, uuid, bigint) is
  'Move some or all of one object to a holder, merging into an interchangeable stack already there. NULL count means the whole row. Refuses to split a named object. The merge is not a rule but the mechanical consequence of objects_stack_idx.';

revoke all on function public.trade_move(uuid, uuid, bigint) from public, anon;
grant execute on function public.trade_move(uuid, uuid, bigint) to authenticated;

-- ---------------------------------------------------------------------
-- The whole exchange, or none of it.
-- ---------------------------------------------------------------------

create or replace function public.trade(
  p_a     uuid,
  p_b     uuid,
  p_moves jsonb
)
returns jsonb
language plpgsql
security invoker
set search_path = ''
as $$
declare
  a_ent uuid;
  b_ent uuid;
  m     jsonb;
  dest  uuid;
  n     bigint := 0;
begin
  if p_a = p_b then
    raise exception 'a trade needs two sides';
  end if;

  select entity_id into a_ent from public.characters where id = p_a;
  select entity_id into b_ent from public.characters where id = p_b;
  if a_ent is null or b_ent is null then
    raise exception 'both sides of a trade must be creatures you can see';
  end if;

  for m in select * from jsonb_array_elements(coalesce(p_moves, '[]'::jsonb)) loop
    dest := case m->>'to'
              when 'a' then a_ent
              when 'b' then b_ent
              else null
            end;
    if dest is null then
      raise exception 'a move must go to side a or side b, not %', coalesce(m->>'to', 'nowhere');
    end if;
    perform public.trade_move(
      (m->>'object')::uuid,
      dest,
      case when m->>'count' is null then null else (m->>'count')::bigint end
    );
    n := n + 1;
  end loop;

  return jsonb_build_object('moves', n);
end;
$$;

comment on function public.trade(uuid, uuid, jsonb) is
  'One exchange between two creatures, whole or not at all - buy, sell, barter and give are all this with a different plan. Moves name a SIDE rather than an entity, so a plan cannot quietly deliver to a third party. Replaces 040s buy_object, which had a buyer and a merchant in its signature and was the asymmetric special case.';

revoke all on function public.trade(uuid, uuid, jsonb) from public, anon;
grant execute on function public.trade(uuid, uuid, jsonb) to authenticated;
