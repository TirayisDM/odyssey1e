-- 156. `write_action` CARRIES THE COST, AND A HEAL IS A ROLL THAT MOVES
-- HIT POINTS.
--
-- Two things stop a healing spell going through the one writer that
-- lands an action, its rolls and its hit point event together or not at
-- all - which is 012's whole rule and the reason that function exists.
--
-- ---------------------------------------------------------------------
-- ONE: THE COST IS DROPPED IN TRANSIT
-- ---------------------------------------------------------------------
--
-- `write_action`'s INSERT names its columns and `cost` is not among
-- them, so a caller cannot say what an action costs. It has never shown,
-- because `stamp_action_cost` fills a null from the key and every
-- caller so far wanted exactly what it derives: an attack costs an
-- attack and everything else costs an action.
--
-- A SPELL IS WHERE THAT BREAKS. Healing Word and Spiritual Weapon are
-- BONUS actions and the key is 'spell', so the trigger would say
-- "action" and quietly take a cleric's whole turn for a bonus-action
-- spell. `cast_prayer` avoids this today by writing the action itself
-- with a plain REST insert - and so cannot use this function, and so
-- cannot write a heal atomically with it.
--
-- THIS IS 021'S LESSON AGAIN, word for word: "an explicit column list in
-- a writer function is a SECOND schema that has to be kept in step with
-- the first, and nothing checks it." 021 found it with `character_name`
-- on `rolls`. Same function, same fault, different column.
--
-- STILL NULL WHEN NOT SUPPLIED, so the trigger keeps deriving it and
-- every existing caller behaves exactly as before.
--
-- ---------------------------------------------------------------------
-- TWO: AN HP EVENT COULD ONLY POINT AT DAMAGE
-- ---------------------------------------------------------------------
--
-- `hp_events.roll_id` was set from `select id from ins where role =
-- 'damage'`, so a heal's event would have been written with a null roll
-- and the log could not say which dice did it. A heal is the same shape
-- as damage with the sign the other way up - 013 made hit points a log
-- of SIGNED deltas precisely so that this needed no second mechanism -
-- so the lookup takes either role.
--
-- 'heal' IS A NEW ROLE and nothing else in the schema constrains the
-- column, so no check needs widening.
--
--   ^ THAT SENTENCE IS WRONG AND 157 CORRECTS IT. `rolls_role_check`
--     allowed only to_hit, damage and check, so the first heal would
--     have failed the insert and taken the whole cast with it. Left as
--     written per 115: an applied migration is a record, not a draft.
--     Found by a rolled-back probe, which is why it cost a paragraph
--     rather than a session.

create or replace function public.write_action(
  p_action jsonb,
  p_rolls  jsonb,
  p_hp     jsonb,
  p_vitals jsonb
)
returns uuid
language plpgsql
security invoker
set search_path = ''
as $$
declare
  a_id   uuid;
  dmg_id uuid;
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
    elsif p_vitals ? 'character_id' and nullif(p_vitals->>'character_id','') is not null then
      update public.characters
         set death_successes = coalesce((p_vitals->>'death_successes')::smallint, death_successes),
             death_failures  = coalesce((p_vitals->>'death_failures')::smallint, death_failures),
             dead            = coalesce((p_vitals->>'dead')::boolean, dead)
       where id = (p_vitals->>'character_id')::uuid;
    end if;
  end if;

  return a_id;
end;
$$;
