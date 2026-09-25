-- =====================================================================
-- 054_an_action_remembers_its_round.sql
-- odyssey1e — a swing knows which round it belonged to
-- =====================================================================
--
-- A character can swing again and again, and nothing anywhere can say
-- so. Not because the rule is missing - because the EVIDENCE is.
--
-- 012 built the action as the turn-sized unit and said so in its
-- header: "a miss spends an initiative slot the same as a hit". 051
-- finally built the slot. But an action records `created_at` and
-- nothing else about when it happened, and a timestamp cannot answer
-- "which round was this" - the round moves when a DM presses a button,
-- not when the clock ticks. Two swings a second apart can be in
-- different rounds and two an hour apart can be in the same one, if
-- the table broke for lunch.
--
-- So the round is stored, because it is a FACT ABOUT THE TABLE and not
-- a consequence of any row - the same test 051 applied to itself, and
-- the same one 010 applied to hit points. Everything downstream stays
-- derived: how many actions a creature has taken this round, whether
-- that is more than one, and what the log looks like grouped by round
-- are all worked out in Rust on the way to the screen.
--
-- ---------------------------------------------------------------------
-- A TRIGGER FILLS IT, NOT THE CLIENT
-- ---------------------------------------------------------------------
--
-- The same call 001 made for character_name and roller_name, for the
-- same reason: a client cannot forget it and cannot fake it. A swing
-- claiming to be in round 1 while the encounter is on round 4 would be
-- undetectable, and the count that depends on it would be quietly
-- wrong - which is this codebase's most expensive defect class, and it
-- is named in STATUS.md.
--
-- It also costs no extra round trip. `swing` does not load the
-- encounter today, and making it do so would put a select in front of
-- every attack to fetch a number the database already has in hand.
--
-- NULL IS AN HONEST ANSWER, and there are two of them:
--
--   an action with no encounter    a skill check in a tavern
--   an action written before this  every row already in the table
--
-- Neither is round 0. Round 0 means the order has not started, which
-- 051 chose deliberately over 1, and backfilling history to 0 would
-- claim 60-odd existing actions happened before a fight began. They
-- happened; nobody was counting rounds. That is what NULL says.
--
-- ---------------------------------------------------------------------
-- WHAT IS DELIBERATELY NOT HERE
-- ---------------------------------------------------------------------
--
-- NO LIMIT. Nothing refuses a second action, exactly as 051 refuses
-- nothing about turn order, and for the same reason: a DM grants a
-- second swing constantly - Extra Attack at level 5, a haste spell, an
-- action surge, a legendary action, or simply because it is more fun.
-- A rig that refuses is a rig they fight.
--
-- What this makes possible is the screen SAYING so: "2 actions this
-- round" beside a creature that has taken two. Counting is what was
-- missing; refusing is a separate decision, and if it is ever taken it
-- belongs in Rust where it can be tested, not in a check constraint.
--
-- NO ACTION ECONOMY. 5e splits a turn into an action, a bonus action,
-- a reaction and free interactions, and nothing in this schema knows
-- which of those a technique costs. Adding `action_cost` to techniques
-- is its own piece of work with its own seed data, and inventing the
-- classification here - guessing that every technique is an action -
-- would put 193 wrong answers in the database. One honest count beats
-- four fabricated ones.
-- =====================================================================

alter table public.actions
  add column round integer;

comment on column public.actions.round is
  'Which round of the encounter this happened in, stamped at insert by actions_stamp_round. NULL means there was no round to record: an action outside an encounter, or one written before 054. Not derivable afterwards - the round advances when the table says so, not with the clock, so created_at cannot answer it.';

-- Counting a creature's actions in the current round is the whole
-- point, and it is always asked per encounter.
create index actions_round_idx
  on public.actions(encounter_id, round)
  where encounter_id is not null;

-- ---------------------------------------------------------------------
-- The stamp.
--
-- SECURITY DEFINER with an empty search_path, like every other trigger
-- function since 002, and EXECUTE revoked below - a trigger function
-- is called by the trigger, never by a client.
--
-- It only fills a round that arrived NULL. A DM correcting the log
-- afterwards - "that was actually round 3" - is an UPDATE, and this
-- trigger is INSERT only, so it does not fight them.
-- ---------------------------------------------------------------------

create or replace function public.stamp_action_round()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
begin
  if new.round is null and new.encounter_id is not null then
    select e.round into new.round
      from public.encounters e
     where e.id = new.encounter_id;
  end if;
  return new;
end;
$$;

comment on function public.stamp_action_round() is
  'Copies the encounter''s current round onto an action as it is written. The client never sends it, for the same reason snapshot_roll_names exists: a round the caller supplies is a round the caller can get wrong, and a count built on it would be calmly, invisibly false.';

create trigger actions_stamp_round
  before insert on public.actions
  for each row execute function public.stamp_action_round();

revoke all on function public.stamp_action_round() from public, anon, authenticated;
