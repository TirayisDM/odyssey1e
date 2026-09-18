-- =====================================================================
-- 009_roll_targets.sql
-- odyssey1e — a roll learns what it was trying to beat
-- =====================================================================
--
-- Until now a roll ended at a number and a human decided what it meant.
-- That is the AppSheet shape: the app said `1d20 [14] +5 = 19` and the
-- table worked out whether 19 beat anything. `rolls.status` still says
-- `resolved` for a roll where dice were thrown and nothing was settled.
--
-- A roll can now carry a target, and when it does the engine says
-- whether it was met.
--
-- ONE CONCEPT, NOT TWO. An attack targets an opponent's AC and a check
-- or save targets a DC, but mechanically both are d20 + modifier
-- against a number. `target_kind` records which kind of number it was,
-- because the auto-hit rule below cares; everything else treats them
-- identically.
--
-- WHY THE TARGET IS VISIBLE. It is an ordinary column under the
-- existing roll policies - no DM-only view, no split visibility. A
-- visible target means the dice decided; a hidden one means the DM
-- decided, and this system is being built so the dice decide. The UI
-- need not put the number in lights, but nothing here works to keep it
-- from a player who looks.
--
-- SNAPSHOTS, NOT LINKS. `target_label` stores 'Goblin 1' as text, and
-- `target_value` stores the AC as a number. When that goblin dies and
-- is deleted, or the DM edits its AC mid-fight, last night's rolls must
-- not change their minds about whether they hit. Same rule that already
-- governs character_name and roller_name: a roll is a record.
--
-- TWO VERDICTS, AND THEY ARE NOT THE SAME AXIS.
--
--   face_outcome  crit / fumble / normal, from the raw d20 against the
--                 thresholds in force. dice.rs decides it.
--   success       whether the total met the target. Needs a target.
--
-- A natural 20 hits whatever the AC is, so the two can disagree, and a
-- roll can succeed on the face while its margin is negative. That is
-- why `margin` is stored even when the face decided it - 'hit on a 20,
-- three under the AC' is a fact worth keeping and worth narrating.
--
-- The rule combining them lives in `resolution.rs` and nowhere else.
--
-- WHY THE THRESHOLDS ARE SNAPSHOTTED. Without crit_min and fumble_max
-- on the row, nothing can explain why an 18 was a crit. Techniques
-- carry their own ranges and a DM can change them, so the row records
-- the pair that was actually in force. Null when no d20 was rolled.
--
-- WHAT IS NOT HERE. No opponents, no encounters, no HP. The target is a
-- number someone supplies. Encounters will supply it from a button
-- instead of a keyboard, and that changes where the number comes from,
-- not what a roll does with it.
-- =====================================================================

alter table public.rolls add column target_value  integer;
alter table public.rolls add column target_kind   text;
alter table public.rolls add column target_label  text;
alter table public.rolls add column success       boolean;
alter table public.rolls add column reason        text;
alter table public.rolls add column margin        integer;
alter table public.rolls add column face_outcome  text;
alter table public.rolls add column crit_min      integer;
alter table public.rolls add column fumble_max    integer;

comment on column public.rolls.target_value is
  'The number the total had to meet or beat. NULL means the roll had no target - a damage roll, or a bare d20 the DM asked for. NULL must never be read as failure.';
comment on column public.rolls.target_kind is
  'ac or dc. Only the auto-hit rule distinguishes them: a natural crit hits regardless of AC, and that does not apply to a check. Everything else treats the two alike.';
comment on column public.rolls.target_label is
  'What was being rolled against, as text: Goblin 1, or the DM''s own wording for a DC. Snapshotted, never a link - the target may be deleted or edited and this row must not change its mind.';
comment on column public.rolls.success is
  'Whether the target was met. NULL when there was no target, or while the roll is still pending and has no total yet. Not the same question as face_outcome.';
comment on column public.rolls.reason is
  'WHY the verdict came out that way: auto_hit, auto_miss, met, missed. Stored so a card can say "hit on a natural 20" rather than leaving a player to work out how a total of 12 beat an 18, and so the narrator is told what happened instead of inferring it from two numbers. Always agrees with success - auto_hit and met are the true ones.';
comment on column public.rolls.margin is
  'total - target_value. Kept even when the face decided the outcome, because "hit on a natural 20, three under the AC" is worth knowing and worth narrating.';
comment on column public.rolls.face_outcome is
  'crit, fumble or normal - the verdict on the raw d20 under the thresholds in force. NULL when no single d20 was rolled, which is not the same as normal.';
comment on column public.rolls.crit_min is
  'The crit threshold actually in force, snapshotted so an 18 that crit can explain itself. 20 for an ordinary roll; techniques widen it.';
comment on column public.rolls.fumble_max is
  'The fumble threshold actually in force. 1 ordinarily; 0 means the roll could not fumble.';

-- A target is a pair or it is nothing.
alter table public.rolls
  add constraint rolls_target_pair_check
  check ((target_value is null) = (target_kind is null));

alter table public.rolls
  add constraint rolls_target_kind_check
  check (target_kind is null or target_kind in ('ac','dc'));

-- A verdict requires something to have been decided FROM. success and
-- margin cannot exist without both a target and a total; a label cannot
-- exist without a target to label.
alter table public.rolls
  add constraint rolls_success_needs_target_check
  check (success is null or (target_value is not null and total is not null));

alter table public.rolls
  add constraint rolls_margin_needs_target_check
  check (margin is null or (target_value is not null and total is not null));

-- The reason travels with the verdict or not at all, and its vocabulary
-- is closed: resolution.rs emits exactly these four.
alter table public.rolls
  add constraint rolls_reason_check
  check (reason is null or
         (success is not null
          and reason in ('auto_hit','auto_miss','met','missed')));

alter table public.rolls
  add constraint rolls_label_needs_target_check
  check (target_label is null or target_value is not null);

-- The face verdict requires a face.
alter table public.rolls
  add constraint rolls_face_outcome_check
  check (face_outcome is null or
         (natural_roll is not null and face_outcome in ('crit','fumble','normal')));

-- The same invariant Thresholds::new enforces in Rust, restated where
-- the data lives. Bounds match techniques.crit_min and
-- techniques.fumble_max; the third condition is the one a per-column
-- check cannot express, and without it a single face could be both a
-- crit and a fumble.
alter table public.rolls
  add constraint rolls_thresholds_pair_check
  check ((crit_min is null) = (fumble_max is null));

alter table public.rolls
  add constraint rolls_thresholds_range_check
  check (crit_min is null or
         (crit_min between 2 and 20
          and fumble_max between 0 and 19
          and fumble_max < crit_min));

-- Reading a game's log filtered to decided rolls.
create index rolls_success_idx on public.rolls(game_id, success)
  where success is not null;
