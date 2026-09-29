-- =====================================================================
-- 060_an_action_knows_whose_turn_it_was.sql
-- odyssey1e — the other half of the moment 054 started recording
-- =====================================================================
--
-- The Inn fight has seven actions in round 1, spanning two days. The
-- turn pointer sat on Garn through all of them and the round never
-- moved, because the round only moves when a DM presses Next. So
-- "acted 3 times this round" was true and useless: the round was a
-- container holding the whole fight.
--
-- 051 decided the order GATES NOTHING and that stands - a held
-- action, a surprise round, a reaction and a DM simply allowing it are
-- all ordinary, and a rig that refuses is a rig they fight. What was
-- missing is not a refusal. It is the screen being able to SAY that
-- Merchant 1 swung three times while it was Garn's turn.
--
-- ---------------------------------------------------------------------
-- WHY IT HAS TO BE STORED
-- ---------------------------------------------------------------------
--
-- Whose turn it was when a blow landed is not derivable afterwards.
-- The pointer moves; comparing an old action against the CURRENT turn
-- holder answers a different question and answers it confidently,
-- which is the worst combination. Exactly 054's argument about the
-- round, and 010's about hit points: a fact about a moment has to be
-- written down in that moment.
--
-- Stamped by the same trigger, which now fills both and is named for
-- what it does rather than for the first half of it. The old one is
-- dropped rather than left beside it - two triggers writing to one row
-- is the thing this codebase spends whole migrations undoing.
--
-- NULL IS AN HONEST ANSWER AND MEANS THREE THINGS, all of them "there
-- was no turn to be out of":
--
--   no encounter          a skill check in a tavern
--   round 0               nobody has taken a turn yet - 051 keeps
--                         that distinct from round 1 on purpose
--   written before 060    the seven actions above, and every older one
--
-- None of them is "out of turn", and the screen must not read them as
-- such. An action is out of turn only where a turn was actually being
-- held by somebody else.
--
-- ---------------------------------------------------------------------
-- WHAT IS DELIBERATELY NOT HERE
-- ---------------------------------------------------------------------
--
-- NO REFUSAL, again. Nothing checks this column on the way in, nothing
-- blocks a swing, and no constraint mentions it. It is evidence, not a
-- rule - the same standing this table's `round` has.
--
-- NO AUTO-ADVANCE. The alternative considered was handing the turn on
-- when the current creature acts, and it was declined because it
-- fights Extra Attack: a level 5 fighter's second swing would arrive
-- after the turn had already moved and would read as out of turn. The
-- turn moves when the table says it moves.
-- =====================================================================

alter table public.actions
  add column turn_actor_id uuid
    references public.encounter_actors(id) on delete set null;

comment on column public.actions.turn_actor_id is
  'Whose turn it was when this happened, stamped at insert. NULL where there was no turn to hold: outside an encounter, before the order started, or written before 060. Not derivable afterwards - the pointer moves, so comparing an old action to the current turn holder answers a different question.';

-- Asked as "was this creature the one holding the turn", per encounter.
create index actions_turn_idx
  on public.actions(encounter_id, turn_actor_id)
  where turn_actor_id is not null;

-- ---------------------------------------------------------------------
-- One trigger, both facts.
-- ---------------------------------------------------------------------

create or replace function public.stamp_action_moment()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
begin
  if new.encounter_id is not null then
    if new.round is null then
      select e.round into new.round
        from public.encounters e
       where e.id = new.encounter_id;
    end if;
    if new.turn_actor_id is null then
      select e.turn_actor_id into new.turn_actor_id
        from public.encounters e
       where e.id = new.encounter_id;
    end if;
  end if;
  return new;
end;
$$;

comment on function public.stamp_action_moment() is
  'Copies the encounter''s round and current turn holder onto an action as it is written. Replaces stamp_action_round, which did the first half. The client sends neither, for the reason snapshot_roll_names exists: a moment the caller describes is a moment the caller can get wrong.';

drop trigger if exists actions_stamp_round on public.actions;
drop function if exists public.stamp_action_round();

create trigger actions_stamp_moment
  before insert on public.actions
  for each row execute function public.stamp_action_moment();

revoke all on function public.stamp_action_moment() from public, anon, authenticated;
