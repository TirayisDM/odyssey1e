-- =====================================================================
-- 015_dying.sql
-- odyssey1e — dropping to zero is not dying yet
-- =====================================================================
--
-- 013 let a hit take hit points off. Nothing said what happened when
-- they ran out, so Goblin 1 sat at -7 and carried on being a target at
-- full armour class.
--
-- Zero makes a creature UNCONSCIOUS and dying. Three death saves settle
-- it either way. That is the 5e shape, and this campaign extends it to
-- NPCs, who by the book simply die at zero - a goblin here bleeds out
-- like anyone else. Which is why encounter_actors gains the same two
-- counters characters has carried since 010.
--
-- THE STATE IS DERIVED, NOT STORED. No `status` column saying 'down'.
-- Hit points and two counters already determine it, and a third field
-- restating them is a field that can disagree with them. Same reasoning
-- as the challenge watermark in 011. The one thing that cannot be
-- derived is `dead`, because overflow damage kills WITHOUT leaving any
-- failures behind, so that gets a flag of its own.
--
-- WHAT IS DELIBERATELY NOT A COLUMN: armour class while unconscious.
-- It is zero, and it is computed in death.rs alongside the condition
-- rather than written anywhere. Storing it would be the Attacks tab
-- mistake a fourth time.
--
-- TWO HOUSE RULES, MARKED SO NOBODY LATER TAKES THEM FOR 5e:
--   NPCs roll death saves at all.
--   An unconscious creature has AC 0. By the book it keeps its armour
--   class and attackers gain advantage, with melee hits inside five
--   feet landing as crits.
--
-- ON THEIR ACTION, EVENTUALLY. A dying creature saves on its own turn,
-- and turn order does not exist yet - encounter_actors.initiative has
-- been sitting unread since 011 for exactly this. Until initiative
-- arrives the save is rolled on demand rather than automatically, and
-- the rule that decides the outcome is the same either way.
-- =====================================================================

alter table public.encounter_actors
  add column death_successes smallint not null default 0,
  add column death_failures  smallint not null default 0,
  add column dead            boolean  not null default false;

alter table public.characters
  add column dead boolean not null default false;

alter table public.encounter_actors
  add constraint encounter_actors_death_saves_check
  check (death_successes between 0 and 3 and death_failures between 0 and 3);

comment on column public.encounter_actors.death_successes is
  'Death saving throw successes, 0-3. Three stabilises. On the ACTOR rather than the statblock, because two goblins off one npcs row die separately - the same reason their hit points are keyed here.';
comment on column public.encounter_actors.death_failures is
  'Death saving throw failures, 0-3. Three kills. A natural 1 is worth two, and being hit while down is worth one - two on a crit.';
comment on column public.encounter_actors.dead is
  'Dead by something that left no failures behind: overflow damage, or the DM saying so. Everything else is derived from hit points and the two counters, which is why this is the only stored piece of the condition.';
comment on column public.characters.dead is
  'As encounter_actors.dead. A character keeps their death saves on this row rather than on their actor, the same way they keep their hit points.';

-- ---------------------------------------------------------------------
-- The writer gains one more optional payload.
--
-- A death save is an action, a roll, sometimes a hit point back, and a
-- change to the counters. Splitting that across calls would leave a
-- creature whose save was rolled but not recorded, which is the hole
-- 012 closed and 014 kept closed. Same reasoning, one more argument.
--
-- It still decides nothing. Rust works out the new counts and sends
-- them absolute; this writes what it is given.
-- ---------------------------------------------------------------------

drop function if exists public.write_action(jsonb, jsonb, jsonb);

create function public.write_action(
  p_action jsonb,
  p_rolls  jsonb,
  p_hp     jsonb,
  p_vitals jsonb
)
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

  -- Absolute counts, worked out in Rust. Writing them rather than
  -- incrementing means a retry cannot double-count a failure.
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
  'Write an action, its rolls, the hit points it cost, and any change to somebody''s death saves - or none of it. Decides nothing: Rust resolves every number and sends the death save counts absolute, so a retry cannot double-count a failure. SECURITY INVOKER on purpose.';

revoke all on function public.write_action(jsonb, jsonb, jsonb, jsonb) from public;
grant execute on function public.write_action(jsonb, jsonb, jsonb, jsonb) to authenticated;
