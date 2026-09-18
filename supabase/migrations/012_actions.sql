-- =====================================================================
-- 012_actions.sql
-- odyssey1e — one swing is one thing
-- =====================================================================
--
-- An attack is two rolls. A to-hit, and then damage. Until now they
-- would have been two unrelated rows, and that breaks three things at
-- once: "that should not have hit" has nothing to undo, a narrator
-- handed two disconnected rolls writes two disconnected sentences, and
-- nothing can say which damage belonged to which swing.
--
-- An action is what owns them.
--
-- EVERY ROLL GETS ONE. A lone Insight check is a one-roll action. The
-- alternative - actions only where there are several rolls - makes a
-- miss structurally different from a hit and a check different from
-- both, and every consumer then has two shapes to handle. A miss is
-- still an action: it happened, it is worth narrating, and it spends a
-- slot in the initiative order exactly as a hit does.
--
-- THE ACTION IS A TRANSACTION BOUNDARY, NOT JUST A LABEL.
--
-- The dice already roll on the device. A whole swing - to-hit, the
-- verdict, the damage, the doubled dice on a crit - is worked out in
-- Rust before anything crosses the network, so it can be written ONCE.
-- `write_action` is that write: the action and all of its rolls land
-- together or not at all.
--
-- The alternative is an insert per roll, where any one can fail and
-- leave a goblin that took damage from a swing that does not exist.
-- That is the precise failure the old Discord path had - rows marked
-- DONE whether or not the post was accepted - and it is worth spending
-- a function to make it unrepresentable.
--
-- write_action DECIDES NOTHING. Rust has already resolved every number
-- in the payload. It is a transactional writer, not a rules engine, so
-- "rules live in Rust, facts live in Postgres" still holds. It is also
-- SECURITY INVOKER - the default - so every insert inside it is subject
-- to the same RLS as an insert from outside, and this adds nothing to
-- the SECURITY DEFINER surface 002 spent effort narrowing.
--
-- THE NARRATIVE STAYS ON THE ROLL, for now. Each roll keeps its canned
-- line, which is what narrative_lines was built for and what already
-- works. actions.narrative is where a generated account of the whole
-- swing will go when there is a generator to write one - one paragraph
-- about a swing rather than two sentences about two dice. Both can
-- coexist; the generated one is an alternative, not a replacement.
--
-- NULLABLE action_id, because fifty-odd rolls already exist without
-- one and rewriting history to fit a new table would be the opposite of
-- what this codebase does with records.
-- =====================================================================

create table public.actions (
  id            uuid primary key default gen_random_uuid(),
  game_id       uuid not null references public.games(id) on delete cascade,
  character_id  uuid references public.characters(id) on delete set null,
  encounter_id  uuid references public.encounters(id) on delete set null,
  owner_uid     uuid not null default auth.uid(),
  request       text not null,
  label         text,
  key           text not null default 'custom',
  status        text not null default 'resolved'
                  check (status in ('resolved', 'delivered')),
  narrative     text,
  created_at    timestamptz not null default now()
);

comment on table public.actions is
  'One thing a character did. Owns the rolls it produced: a to-hit and its damage, or a single check. Every roll belongs to one, including a miss - a miss happened, and it spends an initiative slot the same as a hit.';
comment on column public.actions.character_id is
  'SET NULL on delete, not cascade. A deleted character must not take the account of what they did with them - the rolls carry snapshotted names for exactly this reason.';
comment on column public.actions.encounter_id is
  'The encounter this happened in, when there was one. SET NULL so ending and deleting an encounter does not erase its history.';
comment on column public.actions.key is
  'Engine vocabulary, the same as rolls: attack, a skill key, wis_save. What a narrative or prompt lookup is found by at the action level.';
comment on column public.actions.status is
  'resolved once the dice are down. delivered when the table has seen it - the word rolls.status has never been able to earn, because you deliver an account of a swing rather than two dice results.';
comment on column public.actions.narrative is
  'A generated account of the whole action. Empty today: each roll keeps its canned line from narrative_lines, which already works. This is where one paragraph about a swing goes when there is a generator to write it, as an alternative to the canned lines rather than a replacement.';

create index actions_game_idx on public.actions(game_id, created_at desc);
create index actions_encounter_idx
  on public.actions(encounter_id) where encounter_id is not null;

alter table public.rolls add column action_id uuid
  references public.actions(id) on delete cascade;
alter table public.rolls add column role text;

comment on column public.rolls.action_id is
  'The action this roll is part of. NULL on every roll written before 012; new rolls always have one. CASCADE because undoing an action means the swing did not happen, and a damage roll outliving its to-hit is not a record of anything.';
comment on column public.rolls.role is
  'What this roll DOES within its action: to_hit, damage, check. Not the same as the kind of roll - the vocabulary for that is `key`. Without it the meaning of a roll would have to be inferred from insert order, which is not a thing to rely on.';

alter table public.rolls
  add constraint rolls_role_check
  check (role is null or role in ('to_hit', 'damage', 'check'));

-- A role without an action, or an action without a role, is half a
-- record. Legacy rolls have neither, which is the third legal state.
alter table public.rolls
  add constraint rolls_action_role_pair_check
  check ((action_id is null) = (role is null));

create index rolls_action_idx on public.rolls(action_id) where action_id is not null;

alter table public.actions enable row level security;

create policy "actions: members read"
  on public.actions for select using (public.is_game_member(game_id));
create policy "actions: own or dm writes"
  on public.actions for insert
  with check (owner_uid = auth.uid() and public.is_game_member(game_id));
create policy "actions: owner updates while resolved"
  on public.actions for update
  using (owner_uid = auth.uid() or public.is_game_dm(game_id))
  with check (owner_uid = auth.uid() or public.is_game_dm(game_id));
create policy "actions: owner or dm deletes"
  on public.actions for delete
  using (owner_uid = auth.uid() or public.is_game_dm(game_id));

-- ---------------------------------------------------------------------
-- The atomic write.
-- ---------------------------------------------------------------------

create or replace function public.write_action(p_action jsonb, p_rolls jsonb)
returns uuid
language plpgsql
as $$
declare
  a_id uuid;
begin
  insert into public.actions (game_id, character_id, encounter_id, request, label, key)
  values (
    (p_action->>'game_id')::uuid,
    nullif(p_action->>'character_id','')::uuid,
    nullif(p_action->>'encounter_id','')::uuid,
    p_action->>'request',
    p_action->>'label',
    coalesce(p_action->>'key','custom')
  )
  returning id into a_id;

  -- jsonb_populate_recordset maps each object onto the rolls rowtype, so
  -- the column list here is the only place the shape is written down and
  -- a typo in a key name lands as NULL rather than as a silent shift.
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
  from jsonb_populate_recordset(null::public.rolls, p_rolls) v;

  return a_id;
end;
$$;

comment on function public.write_action(jsonb, jsonb) is
  'Write an action and all of its rolls, or none of them. Decides nothing: Rust has already resolved every number in the payload, and this is a transactional writer rather than a rules engine. SECURITY INVOKER on purpose - every insert inside is subject to the same RLS as one from outside, so this adds nothing to the SECURITY DEFINER surface.';

revoke all on function public.write_action(jsonb, jsonb) from public;
grant execute on function public.write_action(jsonb, jsonb) to authenticated;
