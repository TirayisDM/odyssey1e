-- =====================================================================
-- 014_write_action_hp.sql
-- odyssey1e — the write that takes the hit points off
-- =====================================================================
--
-- 013 added the links and hp_events. This teaches write_action to use
-- them, so a swing and the damage it did still land in one transaction.
--
-- WHY THE FUNCTION HAD TO CHANGE RATHER THAN THE CALLER.
--
-- An hp_event points at the damage ROLL that caused it, and that roll's
-- id does not exist until the insert runs. A caller cannot supply it.
-- Doing it in two calls - write the action, read back the damage roll,
-- then write the event - reopens exactly the hole 012 closed: a swing
-- that landed with no damage recorded, or damage recorded against a
-- swing that failed to write.
--
-- So the function captures the id from its own RETURNING and attaches
-- the event itself. The data-modifying CTE runs whether or not the
-- outer select matches, so a swing with no damage roll simply leaves
-- dmg_id null and writes no event.
--
-- IT STILL DECIDES NOTHING. Rust has already worked out whether damage
-- applies, what it came to, and whom it lands on. The delta arrives
-- signed and is stored as given. This function is a transactional
-- writer; the rules stayed in Rust.
--
-- DROP AND RECREATE, NOT CREATE OR REPLACE. Adding a third parameter
-- with a default would leave the two-argument version in place as an
-- overload, and PostgREST would then have two candidates for the same
-- RPC name. One function, one signature.
-- =====================================================================

drop function if exists public.write_action(jsonb, jsonb);

create function public.write_action(p_action jsonb, p_rolls jsonb, p_hp jsonb)
returns uuid
language plpgsql
as $$
declare
  a_id   uuid;
  dmg_id uuid;
begin
  insert into public.actions (
    game_id, character_id, encounter_id, actor_id,
    target_actor_id, target_challenge_id, request, label, key)
  values (
    (p_action->>'game_id')::uuid,
    nullif(p_action->>'character_id','')::uuid,
    nullif(p_action->>'encounter_id','')::uuid,
    nullif(p_action->>'actor_id','')::uuid,
    nullif(p_action->>'target_actor_id','')::uuid,
    nullif(p_action->>'target_challenge_id','')::uuid,
    p_action->>'request',
    p_action->>'label',
    coalesce(p_action->>'key','custom')
  )
  returning id into a_id;

  -- jsonb_populate_recordset maps each object onto the rolls rowtype, so
  -- the column list here is the only place the shape is written down and
  -- a typo in a key name lands as NULL rather than as a silent shift.
  with ins as (
    insert into public.rolls (
      game_id, character_id, owner_uid, action_id, role,
      request, label, mode, formula, detail, total, natural_roll,
      face_outcome, crit_min, fumble_max, narrative, status,
      target_value, target_kind, target_label, success, reason, margin
    )
    select
      v.game_id, v.character_id, v.owner_uid, a_id, v.role,
      v.request, v.label, coalesce(v.mode, 'normal'), v.formula, v.detail,
      v.total, v.natural_roll, v.face_outcome, v.crit_min, v.fumble_max,
      v.narrative, coalesce(v.status, 'resolved'),
      v.target_value, v.target_kind, v.target_label, v.success, v.reason, v.margin
    from jsonb_populate_recordset(null::public.rolls, p_rolls) v
    returning id, role
  )
  select id into dmg_id from ins where role = 'damage' limit 1;

  -- No damage, no event. A miss writes an action and one roll and stops.
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

  return a_id;
end;
$$;

comment on function public.write_action(jsonb, jsonb, jsonb) is
  'Write an action, its rolls, and the hit points it cost, or none of it. Decides nothing - Rust has already resolved every number, including whether damage applies and whom it lands on. The damage roll''s id cannot be known by the caller, so the function captures it from its own RETURNING and attaches the event; doing that in two calls would reopen the hole 012 closed. SECURITY INVOKER on purpose.';

revoke all on function public.write_action(jsonb, jsonb, jsonb) from public;
grant execute on function public.write_action(jsonb, jsonb, jsonb) to authenticated;
