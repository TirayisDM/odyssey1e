-- =====================================================================
-- 021_write_action_character_name.sql
-- odyssey1e — the name was being thrown away in transit
-- =====================================================================
--
-- THE DEFECT. Every roll a monster makes logs as "Someone".
--
--   Someone - Handaxe (Thrown)   1d20 [18] +1 = 19
--   Someone - Scimitar (Melee)   1d20 [8] +4 = 12
--
-- The dice are right, the finesse rule is right, the target and verdict
-- are right. Only the name is wrong, and it is wrong in the one column
-- whose own comment calls it WHOSE ACTION THIS WAS.
--
-- WHY, AND WHY IT SURVIVED A FIX THAT LOOKED CORRECT. 001's trigger
-- derives the name from character_id and falls back to 'Someone' when
-- there is none - which is every NPC, because a monster has no row in
-- characters. 421d08c fixed that for death saves by sending the actor's
-- label as character_name, and fbffc40 generalised it to every roll a
-- monster makes, since the trigger only overwrites a name that arrived
-- null, blank or 'Someone'.
--
-- Both were correct and neither worked, because write_action never
-- carried the column. Its INSERT names its columns explicitly and
-- character_name was not among them, so jsonb_populate_recordset parsed
-- the field, the select ignored it, and the trigger saw a null and
-- supplied 'Someone'. No error anywhere: the row inserted, the value
-- was simply dropped in transit.
--
-- THE LESSON, WHICH IS THE PART WORTH KEEPING. An explicit column list
-- in a writer function is a SECOND schema that has to be kept in step
-- with the first, and nothing checks it. Adding a column to rolls and
-- setting it from Rust is not enough; if it travels through
-- write_action it has to be listed here too, and forgetting is silent.
-- Both fixes were verified by compiling rather than by reading a row
-- back, which is exactly the check that would have caught this.
--
-- The body is otherwise unchanged from 014/016 - this replaces the
-- function to add one column in two places.
-- =====================================================================

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

  with ins as (
    insert into public.rolls (
      game_id, character_id, character_name, owner_uid, action_id, role,
      request, label, mode, formula, detail, total, natural_roll,
      face_outcome, crit_min, fumble_max, narrative, status,
      target_value, target_kind, target_label, success, reason, margin
    )
    select
      v.game_id, v.character_id,
      -- THE ONE ADDITION. coalesce so a caller that sends nothing gets
      -- the 001 trigger's behaviour unchanged: null in, trigger decides.
      -- A monster sends its label and keeps it.
      coalesce(nullif(btrim(v.character_name), ''), 'Someone'),
      v.owner_uid, a_id, v.role,
      v.request, v.label, coalesce(v.mode, 'normal'), v.formula, v.detail,
      v.total, v.natural_roll, v.face_outcome, v.crit_min, v.fumble_max,
      v.narrative, coalesce(v.status, 'resolved'),
      v.target_value, v.target_kind, v.target_label, v.success, v.reason, v.margin
    from jsonb_populate_recordset(null::public.rolls, p_rolls) v
    returning id, role
  )
  select id into dmg_id from ins where role = 'damage' limit 1;

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

comment on function public.write_action(jsonb, jsonb, jsonb, jsonb) is
  'One swing, written atomically: the action, its rolls, the hit point event and any death-save tally land together or not at all. SECURITY INVOKER on purpose - every table it touches is governed by the caller''s own policies. search_path pinned by 017. CARRIES character_name since 021: the explicit column list is a second schema and a column missing from it is dropped in silence.';

-- 017's grants survive a create or replace, but state them rather than
-- assume it: the function is the attack path and a lost grant is every
-- swing failing.
revoke all on function public.write_action(jsonb, jsonb, jsonb, jsonb) from public, anon;
grant execute on function public.write_action(jsonb, jsonb, jsonb, jsonb) to authenticated;

-- ---------------------------------------------------------------------
-- THE ROWS ALREADY WRITTEN UNDER THE DEFECT STAY AS THEY ARE, and not
-- because it would be hard to mend them. The actor is recoverable
-- through the action, so a backfill was written, attempted, and REFUSED
-- by freeze_roll_identity: 001 makes game_id, owner_uid and both
-- snapshotted names immutable on rolls.
--
-- That trigger is right and the backfill was wrong. A snapshot that can
-- be corrected later is not a snapshot, and "the log says Someone for
-- the swings taken before 021" is a true statement about what was
-- recorded. Rewriting it would make the log easier to read and harder
-- to trust, which is the wrong trade for the one artefact of this
-- system that is supposed to be evidence.
--
-- Left here as the reason rather than deleted, so the next person to
-- think of it finds the answer instead of the idea.
-- ---------------------------------------------------------------------
