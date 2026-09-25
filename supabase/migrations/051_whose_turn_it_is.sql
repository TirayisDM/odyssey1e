-- =====================================================================
-- 051_whose_turn_it_is.sql
-- odyssey1e — the column 011 left unread finally has a turn around it
-- =====================================================================
--
-- THE MOST DEFERRED-TO THING IN THE SCHEMA. Three migrations parked
-- work against this one column and said so:
--
--   011  created `initiative` and wrote "NOTHING READS THIS YET ...
--        players will be prompted to roll on enrollment"
--   012  built actions on the premise that "a miss spends an initiative
--        slot the same as a hit" - the action was always the turn-sized
--        unit, waiting for a turn to sit in
--   015  deferred automatic death saves: "a dying creature saves on its
--        own turn, and turn order does not exist yet ... until
--        initiative arrives the save is rolled on demand"
--
-- ---------------------------------------------------------------------
-- WHAT IS ACTUALLY MISSING IS THE TURN, NOT THE NUMBER
-- ---------------------------------------------------------------------
--
-- `initiative` has existed since 011 and three selects already fetch
-- it. What has never existed anywhere is WHICH ROUND IT IS and WHOSE
-- TURN, and neither is derivable - they are facts about the table, not
-- consequences of the rows. Everything else stays derived, which is the
-- split this schema has held since 010:
--
--   stored    each actor's rolled value        (011, already there)
--   stored    the round, and the current actor (here, because nothing
--                                               implies them)
--   derived   the ORDER - initiative desc, then DEX, then name
--   derived   who is next, and whether the round rolled over
--
-- The order is worked out in `initiative.rs` on the way to the screen,
-- for the reason 033 gave about depth and 015 gave about the dying
-- condition: a stored ordering is one enrolment away from lying.
--
-- ROUND 0 MEANS THE ORDER HAS NOT STARTED, which is why the default is
-- 0 and not 1. An encounter can be built, enrolled and even rolled for
-- without anybody taking a turn, and "round 1 with nobody acting" would
-- be a different and false claim. The same reasoning 011 used for
-- initiative itself: NULL is not zero, and zero is not one.
--
-- SET NULL, NOT CASCADE, on the current actor. A creature removed
-- mid-fight must not delete the encounter it was in - it ends the turn,
-- which `next_turn` handles by starting again from the top of the
-- order. The turn pointer going empty is a recoverable state and is
-- exactly what an interrupted fight looks like.
--
-- ---------------------------------------------------------------------
-- WHAT IS DELIBERATELY NOT HERE
-- ---------------------------------------------------------------------
--
-- NO GATE. Nothing in this migration or above it refuses a roll made
-- out of turn. A DM fudges initiative constantly - a surprise round, a
-- held action, somebody who stepped away - and a rig that refuses is a
-- rig they fight. The order is information; the table is the authority.
-- If that changes, it changes in Rust where it can be tested, not in a
-- constraint.
--
-- NO TIEBREAK COLUMN. 5e breaks a tie on DEX and then on whoever the DM
-- says; the first is already on the character and the second is a
-- conversation. Storing a tiebreak would be storing the answer to a
-- question the DM has not been asked yet.
--
-- NO TURN TIMER, no action economy, no reactions. An action is already
-- the unit 012 built; counting how many a creature has left is a rule
-- about the game and belongs in Rust the day somebody wants it.
-- =====================================================================

alter table public.encounters
  add column round integer not null default 0
    check (round >= 0);

alter table public.encounters
  add column turn_actor_id uuid
    references public.encounter_actors(id) on delete set null;

comment on column public.encounters.round is
  'Which round the fight is in. ZERO MEANS THE ORDER HAS NOT STARTED, which is why the default is not 1 - an encounter can be built, enrolled and rolled for without anybody taking a turn, and "round 1 with nobody acting" would be a false claim. Same reasoning 011 used for initiative: NULL is not zero, and zero is not one.';
comment on column public.encounters.turn_actor_id is
  'Whose turn it is. NULL when the order has not started, or when the creature whose turn it was has been removed - SET NULL rather than cascade, because taking a goblin out of a fight must not delete the fight. next_turn starts again from the top of the order when it finds no current actor. The ORDER itself is derived in initiative.rs and never stored: a stored ordering is one enrolment away from lying.';

create index encounters_turn_idx
  on public.encounters(turn_actor_id) where turn_actor_id is not null;

-- ---------------------------------------------------------------------
-- The turn belongs to a creature in THIS fight.
--
-- The mirror of location_parent_same_game and
-- technique_object_same_game, and it exists for the same reason: a
-- foreign key points at an id and cannot check the pair. A DM running
-- two encounters can see both actors.
-- ---------------------------------------------------------------------

create or replace function public.turn_actor_is_in_the_encounter()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
declare
  belongs uuid;
begin
  if new.turn_actor_id is null then
    return new;
  end if;
  select encounter_id into belongs
    from public.encounter_actors where id = new.turn_actor_id;
  if belongs is distinct from new.id then
    raise exception 'that creature is not in this encounter';
  end if;
  return new;
end;
$$;

comment on function public.turn_actor_is_in_the_encounter() is
  'Refuses a turn pointed at a creature enrolled somewhere else. Mirrors location_parent_same_game, and exists for the same reason: a foreign key points at an id and cannot check the pair.';

create trigger encounters_turn_is_one_of_ours
  before insert or update of turn_actor_id on public.encounters
  for each row execute function public.turn_actor_is_in_the_encounter();
