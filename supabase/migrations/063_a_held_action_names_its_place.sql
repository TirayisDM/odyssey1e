-- =====================================================================
-- 063_a_held_action_names_its_place.sql
-- odyssey1e — holding is a position in the order, not a trigger
-- =====================================================================
--
-- 062 left the held attack as the last of the five and guessed wrong
-- about what it was. It assumed 5e's Ready: an action converted into a
-- reaction, fired by a trigger somebody has to describe in prose and
-- the app has to watch for.
--
-- IT IS NOT THAT. A held action here DECLARES A POSITION: go after the
-- next one, go after that character, go at the end of the round. That
-- is a statement about the ORDER, which is a thing this app already
-- has, rather than about an event, which is a thing it does not.
--
-- The difference matters because the trigger version needs a watcher
-- and a vocabulary for conditions, and this version needs one column
-- and a sort. 051 built the order; this is a second way to sit in it.
--
-- ---------------------------------------------------------------------
-- THE ROLL IS STILL THE RECORD
-- ---------------------------------------------------------------------
--
-- A hold does NOT rewrite `initiative`. The rolled number is what
-- somebody rolled - 011's whole reason for the column, and this
-- codebase's oldest principle - and a hold is a declaration laid over
-- it. Release the hold and they are back where the dice put them, with
-- nothing to restore because nothing was overwritten.
--
-- ---------------------------------------------------------------------
-- A HOLD LASTS ONE ROUND
-- ---------------------------------------------------------------------
--
-- "End of round" only means anything inside a round, so that is the
-- life of the whole declaration: advancing into a new round clears
-- every hold and everybody is back on their rolled number. A DM may
-- also release one by hand at any point.
--
-- The alternative - a hold that persists until cancelled - would mean
-- a creature who held in round 1 quietly acting last in rounds 2, 3
-- and 4 because nobody remembered to clear it. That is the kind of
-- state that makes a tool untrustworthy at the table.
--
-- ---------------------------------------------------------------------
-- WHAT IS DELIBERATELY NOT HERE
-- ---------------------------------------------------------------------
--
-- NO TRIGGER, NO CONDITION TEXT. "When the goblin steps into the
-- doorway" is a sentence for a DM, not a column, and a `condition text`
-- that nothing reads would be decoration - 054's argument about
-- action costs, and 056's about traits.
--
-- NO REFUSAL, as ever. 051 decided the order informs and never
-- refuses. A hold that points at a creature who has already gone, or
-- at one who is themselves holding, is not rejected - it is resolved
-- as best it can be and shown. See `initiative::order`, where a hold
-- that cannot be placed falls to the end of the round rather than
-- vanishing or looping.
--
-- NO HOLDING AFTER YOURSELF. That one IS refused, by a check
-- constraint, because it is not a declaration anybody could mean.
-- =====================================================================

alter table public.encounter_actors
  add column if not exists held_mode     text,
  add column if not exists held_after_id uuid references public.encounter_actors(id) on delete set null;

comment on column public.encounter_actors.held_mode is
  'Where this creature has declared they will act: after_next, after_actor or end_of_round. NULL is the common case - they act on their rolled initiative. A hold is a declaration laid over the roll and never rewrites it.';
comment on column public.encounter_actors.held_after_id is
  'The creature this one is waiting for, when held_mode is after_actor. ON DELETE SET NULL, which leaves an after_actor hold with nobody to follow - initiative::order drops it to the end of the round rather than losing the creature.';

alter table public.encounter_actors
  drop constraint if exists actors_held_mode_is_a_mode;
alter table public.encounter_actors
  add constraint actors_held_mode_is_a_mode check (
    held_mode is null or held_mode in ('after_next', 'after_actor', 'end_of_round')
  );

-- A target belongs to exactly one mode. Without this a hold could name
-- a creature AND say end of round, and the two would disagree.
alter table public.encounter_actors
  drop constraint if exists actors_held_target_matches_mode;
alter table public.encounter_actors
  add constraint actors_held_target_matches_mode check (
    (held_mode = 'after_actor' and held_after_id is not null)
    or (held_mode is distinct from 'after_actor' and held_after_id is null)
  );

-- THE ONE THING THAT IS REFUSED. Everything else about a hold is
-- resolved as best it can be; waiting for yourself is not a
-- declaration anybody could mean.
alter table public.encounter_actors
  drop constraint if exists actors_do_not_hold_for_themselves;
alter table public.encounter_actors
  add constraint actors_do_not_hold_for_themselves check (
    held_after_id is null or held_after_id <> id
  );

create index if not exists encounter_actors_held_idx
  on public.encounter_actors (encounter_id) where held_mode is not null;

-- ---------------------------------------------------------------------
-- A HOLD POINTS INSIDE ITS OWN FIGHT
-- ---------------------------------------------------------------------
--
-- The foreign key says "some actor row"; it cannot say "one in this
-- encounter". Without this a hold could name a creature from another
-- table's fight, which would resolve to nothing and read as a bug in
-- the sort. Same shape as 031's holder guard and 052's challenge one.

create or replace function public.held_actor_same_encounter()
returns trigger
language plpgsql
security definer
set search_path = public
as $$
declare
  other uuid;
begin
  if new.held_after_id is null then
    return new;
  end if;
  select encounter_id into other
    from public.encounter_actors where id = new.held_after_id;
  if other is distinct from new.encounter_id then
    raise exception 'a hold must name somebody in the same fight';
  end if;
  return new;
end $$;

drop trigger if exists actors_hold_inside_their_fight on public.encounter_actors;
create trigger actors_hold_inside_their_fight
  before insert or update of held_after_id on public.encounter_actors
  for each row execute function public.held_actor_same_encounter();
