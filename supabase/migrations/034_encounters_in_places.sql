-- =====================================================================
-- 034_encounters_in_places.sql
-- odyssey1e — a fight happens somewhere, and can be called off
-- =====================================================================
--
-- Two small changes, one of which is not small at all in its
-- consequences.
--
-- WHERE IT HAPPENS. 033 gave the world places and nothing pointed at
-- one. An encounter is the most obvious thing that does: a fight is in
-- the Pub Floor, and knowing that is what makes "drop it here" mean
-- anything while it is happening. Nullable, because a scratch encounter
-- with no place is a legitimate thing to build.
--
-- ON DELETE SET NULL, not cascade. Deleting the room must not delete
-- the record of the fight that happened in it - the same reasoning 013
-- gave for the target links, and the same reason 030 stopped a delete
-- eating inventory.
--
-- CANCELLED IS NOT ENDED, AND THIS IS THE PART THAT MATTERS.
--
-- `ended` has been doing two jobs: the fight finished, and the DM
-- changed their mind. They read identically in a log a month later and
-- they are about to stop being interchangeable, because ENDED IS THE
-- TRIGGER. Completing an encounter is what will one day review the
-- actions taken in it and award experience from them, and write the
-- journal entry that says what happened.
--
-- A cancelled encounter must award NOTHING. It was called off; nobody
-- earned anything. If the only word available were `ended`, that review
-- would either have to run on encounters that never happened or be
-- skipped by some flag bolted on beside it - which is the same
-- rots-when-it-drifts problem 015 refused with the dying condition.
--
-- So: four states. draft while it is built, active while it is in front
-- of the table, ENDED when it finished and is owed a reckoning,
-- CANCELLED when it was abandoned and is owed none.
--
-- Nothing reads the distinction yet. It is one value in a check
-- constraint today and the hinge of the experience system later, and
-- adding it now means the history will already be true when something
-- comes to count it. The alternative is a migration that has to guess,
-- months from now, which of the old `ended` rows were really abandoned.
-- =====================================================================

alter table public.encounters
  add column location_id uuid references public.locations(id) on delete set null;

comment on column public.encounters.location_id is
  'WHERE THIS HAPPENS. Nullable: a scratch encounter with no place is legitimate. SET NULL on delete, because removing the room must not remove the record of the fight - the same reasoning 013 gave for the target links.';

create index encounters_location_idx
  on public.encounters(location_id) where location_id is not null;

-- ---------------------------------------------------------------------
-- A fourth state.
-- ---------------------------------------------------------------------

alter table public.encounters drop constraint encounters_status_check;
alter table public.encounters
  add constraint encounters_status_check
  check (status in ('draft', 'active', 'ended', 'cancelled'));

comment on column public.encounters.status is
  'draft while the DM builds it, active while it is in front of players, ENDED when it finished, CANCELLED when it was called off. At most one active per game, by partial unique index. Ended and cancelled are deliberately different words: ended is what will trigger the experience review and the journal entry, and a cancelled encounter must earn nobody anything. Nothing reads that distinction yet - recording it now means the history is already true when something comes to count it.';
