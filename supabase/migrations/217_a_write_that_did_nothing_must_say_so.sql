-- 217. A WRITE THAT DID NOTHING MUST SAY SO.
--
-- 216 found one of these and promised the pass: "not convert all 54,
-- but find the writes a player can actually reach and make those ask."
-- This is that pass, and it turned up more than a lie.
--
-- THE SHAPE, PROVED AGAIN AT THE TOP OF THE PASS:
--
--     a blocked INSERT raises 42501.
--     a blocked UPDATE or DELETE matches no row and returns success.
--
-- So every UPDATE and DELETE a non-DM can reach is a place the engine
-- can believe something happened that did not. Four were reachable.
-- Three are fixed here; the fourth is in `effects.rs`.
--
--
-- ONE. A TRADE DESTROYED THE GOODS.
--
-- `objects` may be written by the DM, by whoever owns the HOLDER, or by
-- anyone in the game when the thing is on the ground. `trade_move`
-- counted none of its writes, and its merge branch is two statements:
--
--     update objects set quantity = quantity + n where id = dst;
--     delete from objects where id = src.id;
--
-- Bryn's player hands three potions to Falon, who already has one. The
-- first statement is filtered - Falon is not theirs - and matches
-- nothing. The second is their own row and succeeds. Proved against
-- live data inside a rolled-back block:
--
--     Bryn  3 -> row deleted
--     Falon 1 -> 1
--     *** THE POTIONS WERE DESTROYED ***
--
-- THIS IS NOT AN RLS BUG. The policy did exactly what it says. The
-- function was reading "no error" as "it worked" across a pair of
-- statements that only make sense together.
--
-- The fix is to count, NOT to re-state the policy in PL/pgSQL. A
-- function that decides for itself who may trade is a second copy of
-- the rule, and the day the policy changes the two disagree. So each
-- statement asks whether it touched anything and names what it could
-- not move. The whole RPC is one transaction, so a raise anywhere in
-- the loop rolls back every move in it - there is no half-trade.
--
--
-- TWO. A DEATH SAVE THAT NEVER COUNTED.
--
-- `write_action`'s vitals clause updates `characters`, which is
-- owner-or-DM. A player pressing "death save" for a dying character
-- that is not theirs writes the roll - `rolls` is open to members - and
-- then silently fails to move the counters. The log fills with saves
-- and the character neither stabilises nor dies.
--
-- THIS IS 016 ARRIVING FOR THE THIRD TIME. That migration exists
-- because a death save write failed silently; 216 was the same shape on
-- the clock; this is the same shape on the same subject.
--
--
-- THREE. YOU COULD START IT AND NOT STOP IT.
--
-- `effects: members apply` lets anyone in the game put an effect on
-- anyone. `effects: dm or bearer ends` lets only the DM or the person
-- WEARING it take it off. Those two do not meet, and concentration
-- falls through the gap: Bryn's player casts Bless on Falon, then drops
-- concentration, and `drop_concentration` updates a row on Falon.
--
--     cast (INSERT on someone else): ALLOWED
--     drop (UPDATE on someone else): 0 rows
--     *** THE SPELL RUNS ON FOREVER, SILENTLY ***
--
-- Unlike the other two this one IS the policy being wrong, so it is the
-- policy that changes. The rule it was missing: **you may end what you
-- are sustaining.** `source_character_id` has been on the row since the
-- table was built and nothing had asked it who it was.
--
-- Note which half was already right - a player still cannot end a
-- stranger's effect they had no hand in. This adds the one person with
-- a claim on it besides the bearer, and no more.
--
--
-- ON THE LEDGER. This one was first applied with the SQL alone,
-- before this header was written, so its `schema_migrations` row held
-- less than the file and a digest check would have flagged it. The row
-- was then set to this file and the digest compared against it, which
-- is the only way that check is worth having - the check IS that the
-- file and the row agree.

begin;

/* ---------- THREE: you may end what you are sustaining ---------- */

drop policy if exists "effects: dm or bearer ends" on public.effects;
-- And the new name too, so this file can be run twice. 115 says never
-- edit an applied migration; it says nothing against one that does not
-- mind being re-read, and the alternative is a ledger whose text is not
-- the text that ran.
drop policy if exists "effects: dm, bearer or source ends" on public.effects;

create policy "effects: dm, bearer or source ends"
  on public.effects for update
  using (
    public.is_game_dm(game_id)
    or exists (select 1 from public.characters c
                where c.id = effects.character_id
                  and c.owner_uid = (select public.current_profile()))
    or exists (select 1 from public.characters c
                where c.id = effects.source_character_id
                  and c.owner_uid = (select public.current_profile())))
  with check (
    public.is_game_dm(game_id)
    or exists (select 1 from public.characters c
                where c.id = effects.character_id
                  and c.owner_uid = (select public.current_profile()))
    or exists (select 1 from public.characters c
                where c.id = effects.source_character_id
                  and c.owner_uid = (select public.current_profile())));

/* ---------- ONE: a trade that cannot quietly lose things ---------- */

-- WHOSE PACK IS THIS. A refusal that cannot name the side it refused is
-- the thing being fixed, and a holder may be a person or a place.
create or replace function public.trade_holder_name(p_entity uuid)
returns text
language sql
stable
set search_path = ''
as $$
  select coalesce(
    (select c.name from public.characters c where c.entity_id = p_entity),
    (select l.name from public.locations l where l.entity_id = p_entity),
    'that holder')
$$;

grant execute on function public.trade_holder_name(uuid) to authenticated;

create or replace function public.trade_move(p_object uuid, p_to uuid, p_count bigint)
returns void
language plpgsql
set search_path = ''
as $$
declare
  src public.objects%rowtype;
  dst uuid;
  n   bigint;
  hit bigint;
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
    raise exception 'a named object cannot be split';
  end if;

  if src.name is null then
    select id into dst
      from public.objects
     where holder_id = p_to and item_key = src.item_key and name is null;
  end if;

  if dst is not null then
    -- THE PAIR THAT DESTROYED THINGS. The receiving stack first, and
    -- the giver's row is not touched unless it landed.
    update public.objects set quantity = quantity + n where id = dst;
    get diagnostics hit = row_count;
    if hit = 0 then
      raise exception 'nothing can be put into %''s keeping: only the DM, or whoever owns them, can trade on that side',
        public.trade_holder_name(p_to);
    end if;

    if n = src.quantity then
      delete from public.objects where id = src.id;
    else
      update public.objects set quantity = quantity - n where id = src.id;
    end if;
    get diagnostics hit = row_count;
    if hit = 0 then
      raise exception '%''s % is not yours to give away: only the DM, or whoever owns them, can trade it',
        public.trade_holder_name(src.holder_id), src.item_key;
    end if;
    return;
  end if;

  if n = src.quantity then
    -- This one already raised on the WITH CHECK when the destination
    -- was not writable. It is counted anyway, because the USING side
    -- fails silently and the two failures are the same mistake.
    update public.objects set holder_id = p_to where id = src.id;
  else
    update public.objects set quantity = quantity - n where id = src.id;
  end if;
  get diagnostics hit = row_count;
  if hit = 0 then
    raise exception '%''s % is not yours to give away: only the DM, or whoever owns them, can trade it',
      public.trade_holder_name(src.holder_id), src.item_key;
  end if;

  if n < src.quantity then
    insert into public.objects
      (game_id, holder_id, item_key, quantity, size_override)
    values (src.game_id, p_to, src.item_key, n, src.size_override);
  end if;
end;
$$;

/* ---------- TWO: a death save that cannot quietly not count ---------- */

create or replace function public.write_action(p_action jsonb, p_rolls jsonb, p_hp jsonb, p_vitals jsonb)
returns uuid
language plpgsql
set search_path = ''
as $$
declare
  a_id   uuid;
  dmg_id uuid;
  hit    bigint;
begin
  insert into public.actions (
    game_id, character_id, encounter_id, actor_id,
    target_actor_id, target_challenge_id, request, label, key, cost)
  values (
    (p_action->>'game_id')::uuid,
    nullif(p_action->>'character_id','')::uuid,
    nullif(p_action->>'encounter_id','')::uuid,
    nullif(p_action->>'actor_id','')::uuid,
    nullif(p_action->>'target_actor_id','')::uuid,
    nullif(p_action->>'target_challenge_id','')::uuid,
    p_action->>'request',
    p_action->>'label',
    coalesce(p_action->>'key','custom'),
    -- NULL LEAVES IT TO `stamp_action_cost`, which is what every caller
    -- before this one relied on.
    nullif(p_action->>'cost','')
  )
  returning id into a_id;

  with ins as (
    insert into public.rolls (
      game_id, character_id, character_name, owner_uid, action_id, role,
      request, label, mode, formula, detail, total, natural_roll,
      face_outcome, crit_min, fumble_max, narrative, status,
      target_value, target_kind, target_label, success, reason, margin
    )
    select
      v.game_id, v.character_id,
      coalesce(nullif(btrim(v.character_name), ''), 'Someone'),
      v.owner_uid, a_id, v.role,
      v.request, v.label, coalesce(v.mode, 'normal'), v.formula, v.detail,
      v.total, v.natural_roll, v.face_outcome, v.crit_min, v.fumble_max,
      v.narrative, coalesce(v.status, 'resolved'),
      v.target_value, v.target_kind, v.target_label, v.success, v.reason, v.margin
    from jsonb_populate_recordset(null::public.rolls, p_rolls) v
    returning id, role
  )
  -- EITHER ROLE. 013 made hit points a log of signed deltas so that a
  -- heal needed no second mechanism; this is the one lookup that had
  -- assumed the sign.
  select id into dmg_id from ins where role in ('damage', 'heal') limit 1;

  if p_hp is not null and p_hp <> 'null'::jsonb then
    insert into public.hp_events (
      game_id, character_id, actor_id, delta, action_id, roll_id, note)
    values (
      (p_hp->>'game_id')::uuid,
      nullif(p_hp->>'character_id','')::uuid,
      nullif(p_hp->>'actor_id','')::uuid,
      (p_hp->>'delta')::integer,
      a_id,
      dmg_id,
      nullif(p_hp->>'note','')
    );
  end if;

  if p_vitals is not null and p_vitals <> 'null'::jsonb then
    if p_vitals ? 'actor_id' and nullif(p_vitals->>'actor_id','') is not null then
      update public.encounter_actors
         set death_successes = coalesce((p_vitals->>'death_successes')::smallint, death_successes),
             death_failures  = coalesce((p_vitals->>'death_failures')::smallint, death_failures),
             dead            = coalesce((p_vitals->>'dead')::boolean, dead)
       where id = (p_vitals->>'actor_id')::uuid;
      get diagnostics hit = row_count;
    elsif p_vitals ? 'character_id' and nullif(p_vitals->>'character_id','') is not null then
      update public.characters
         set death_successes = coalesce((p_vitals->>'death_successes')::smallint, death_successes),
             death_failures  = coalesce((p_vitals->>'death_failures')::smallint, death_failures),
             dead            = coalesce((p_vitals->>'dead')::boolean, dead)
       where id = (p_vitals->>'character_id')::uuid;
      get diagnostics hit = row_count;
    else
      -- Vitals were offered and addressed to nobody. Before this they
      -- fell past both branches in silence, which is how an NPC's death
      -- saves would have gone nowhere if 022 had not made every actor a
      -- character.
      raise exception 'a vitals write must name an actor or a character';
    end if;

    -- THE WHOLE ACTION GOES BACK. A roll that was recorded beside
    -- counters that did not move is worse than no roll: the log shows
    -- three failed saves against a character who is still on two.
    if hit = 0 then
      raise exception 'only the DM, or whoever owns them, can record that creature''s death saves';
    end if;
  end if;

  return a_id;
end;
$$;

/* ---------- and prove it, before this transaction is allowed to end ---------- */

do $check$
declare
  g uuid; p1 uuid; bryn_e uuid; falon_e uuid; src uuid; dst uuid;
  blew boolean := false; msg text;
begin
  select c.entity_id, c.game_id into bryn_e, g
    from public.characters c where c.name='Bryn' and not c.is_npc;
  select c.entity_id into falon_e
    from public.characters c where c.name='Falon' and c.game_id=g;
  select id into p1 from public.profiles where display_name='Player One';

  if g is null or falon_e is null or p1 is null then
    raise notice 'fixture characters absent - skipping the live check';
    return;
  end if;

  insert into public.objects (game_id, holder_id, item_key, quantity)
    values (g, bryn_e, 'check-217-potion', 3) returning id into src;
  insert into public.objects (game_id, holder_id, item_key, quantity)
    values (g, falon_e, 'check-217-potion', 1) returning id into dst;

  perform set_config('role','authenticated',true);
  perform set_config('request.jwt.claims',
                     json_build_object('sub',p1,'role','authenticated')::text,true);
  begin
    perform public.trade_move(src, falon_e, null);
  exception when others then
    blew := true; msg := sqlerrm;
  end;
  perform set_config('role','postgres',true);

  delete from public.objects where id in (src, dst);

  if not blew then
    raise exception '217 DID NOT HOLD: the trade still completed in silence';
  end if;
  raise notice '217 holds - the trade now refuses: %', msg;
end
$check$;

commit;
