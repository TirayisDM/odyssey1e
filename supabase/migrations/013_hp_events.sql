-- =====================================================================
-- 013_hp_events.sql
-- odyssey1e — the hit lands
-- =====================================================================
--
-- 012 made a swing one thing. It still could not take a single hit
-- point off anything, because an action snapshots 'Goblin 1' and 'AC
-- 15' as text and holds no reference to WHICH goblin. Text is the right
-- thing for the record and the wrong thing for "take 3 off that one".
--
-- Two halves: the links, then the events they make possible.
--
-- THE LINK GOES ON THE ACTION, THE SNAPSHOT STAYS ON THE ROLL.
--
-- They answer different questions. The action is what was AIMED at
-- something; the roll is what was JUDGED against a number. Keeping them
-- apart means the damage roll - which has no target of its own and never
-- will - can still find the goblin it was for, by asking its action.
--
-- The precedent is already in this schema: rolls.character_id sits
-- beside rolls.character_name. The link is for querying and may dangle;
-- the snapshot is the record and never changes its mind. Both target
-- links are ON DELETE SET NULL for exactly that reason - deleting a
-- goblin must not rewrite what happened to it.
--
-- THE PERFORMER, AS AN ACTOR AND NOT ONLY AS A CHARACTER.
--
-- actions.character_id says who owns the sheet. actions.actor_id says
-- who was swinging, as a participant in the encounter, and those are
-- not the same question. An NPC swinging back is an action with no
-- character behind it at all.
--
-- It is also what an eventual experience and advancement system counts.
-- "How many actions has this actor succeeded at" is a question about a
-- participant, and answering it from character_id alone would miss
-- every NPC and every action taken outside a sheet. Nothing computes XP
-- today; this is the column that would make it possible to, without
-- rewriting history first.
--
-- HP IS AN EVENT LOG, NOT A NUMBER.
--
-- Not `hp = hp - 7` but a row saying Goblin 1, -7, from this roll.
-- Current HP is hp_max plus the sum of the deltas, so:
--
--   undo is deleting a row, not reconstructing what a number used to be
--   a heal is the same shape as damage, with the sign flipped, rather
--     than a second code path
--   a DM correction is a third row with a note, not an edit that erases
--     the thing it corrected
--   the goblin at 3 can always say WHY it is at 3
--
-- The cost is that reading current HP is a sum rather than a lookup.
-- For a fight with a few dozen events that is nothing, and it buys a
-- history that cannot silently disagree with itself. Same instinct as
-- `rolls`, which has always been an event log of things that happened.
--
-- A DELTA, NOT AN AMOUNT. -7 is seven damage and +5 is five healing.
-- One signed column rather than an amount plus a kind, because a kind
-- column invites a row where the two disagree.
-- =====================================================================

alter table public.actions add column actor_id uuid
  references public.encounter_actors(id) on delete set null;
alter table public.actions add column target_actor_id uuid
  references public.encounter_actors(id) on delete set null;
alter table public.actions add column target_challenge_id uuid
  references public.encounter_challenges(id) on delete set null;

comment on column public.actions.actor_id is
  'WHO ACTED, as a participant in the encounter. Not the same question as character_id, which says whose sheet it was: an NPC swinging back is an action with no character behind it. This is also what an experience system would count, since "how many actions has this actor succeeded at" is a question about a participant. NULL outside an encounter.';
comment on column public.actions.target_actor_id is
  'WHAT WAS AIMED AT. A link for querying and for finding the thing to take hit points off; the immutable record of what the roll was judged against is the snapshot on the roll. SET NULL on delete, because deleting a goblin must not rewrite what happened to it.';
comment on column public.actions.target_challenge_id is
  'The challenge attempted, when it was a lock rather than a goblin. Whether that challenge has been passed is DERIVED from the actions pointing at it with a successful roll after encounter_challenges.reset_at - no status column to forget to update.';

-- An action aims at one thing or at nothing. Both set is a target that
-- cannot be described.
alter table public.actions
  add constraint actions_one_target_check
  check (target_actor_id is null or target_challenge_id is null);

create index actions_actor_idx
  on public.actions(actor_id) where actor_id is not null;
create index actions_target_actor_idx
  on public.actions(target_actor_id) where target_actor_id is not null;
create index actions_target_challenge_idx
  on public.actions(target_challenge_id) where target_challenge_id is not null;

-- ---------------------------------------------------------------------

create table public.hp_events (
  id            uuid primary key default gen_random_uuid(),
  game_id       uuid not null references public.games(id) on delete cascade,
  character_id  uuid references public.characters(id) on delete cascade,
  actor_id      uuid references public.encounter_actors(id) on delete cascade,
  delta         integer not null,
  action_id     uuid references public.actions(id) on delete cascade,
  roll_id       uuid references public.rolls(id) on delete set null,
  note          text,
  created_at    timestamptz not null default now(),

  -- One subject, never both, never neither. The same XOR that lets
  -- encounter_actors hold two kinds of participant in one table.
  constraint hp_events_one_subject_check
    check ((character_id is null) <> (actor_id is null))
);

comment on table public.hp_events is
  'Every change to somebody''s hit points, as a log rather than a number. Current HP is hp_max plus the sum of the deltas. Undo is deleting a row; a heal is damage with the sign flipped; a DM correction is another row with a note rather than an edit that erases what it corrected.';
comment on column public.hp_events.delta is
  'SIGNED. -7 is seven damage, +5 is five healing. One column rather than an amount plus a kind, because a kind column invites a row where the two disagree. Zero is allowed and means something was recorded as having no effect, which is a fact worth keeping.';
comment on column public.hp_events.character_id is
  'The player character who took it. CASCADE, unlike the target links on actions: a deleted character has no hit points to account for, and the account of what they DID lives on the action.';
comment on column public.hp_events.actor_id is
  'The encounter participant who took it. Two goblins from one statblock take damage independently because the event names the instance, not the statblock.';
comment on column public.hp_events.action_id is
  'The swing that caused it. NULL for a DM adjustment, which has no action behind it. CASCADE so undoing an action undoes its damage in the same stroke - which is the whole reason the action had to exist first.';
comment on column public.hp_events.roll_id is
  'The damage roll, so the number can be traced to the dice that produced it. SET NULL rather than CASCADE: losing the roll should not silently heal anybody.';
comment on column public.hp_events.note is
  'Why, in the DM''s words. Expected on a manual adjustment and empty on damage, where the action says it.';

create index hp_events_character_idx
  on public.hp_events(character_id) where character_id is not null;
create index hp_events_actor_idx
  on public.hp_events(actor_id) where actor_id is not null;
create index hp_events_game_idx on public.hp_events(game_id, created_at desc);

alter table public.hp_events enable row level security;

-- Damage is public at the table. Everyone sees the goblin drop.
create policy "hp_events: members read"
  on public.hp_events for select using (public.is_game_member(game_id));
create policy "hp_events: members write"
  on public.hp_events for insert
  with check (public.is_game_member(game_id));
create policy "hp_events: dm updates"
  on public.hp_events for update
  using (public.is_game_dm(game_id)) with check (public.is_game_dm(game_id));
-- Undo is a delete, so it has to be reachable. The owner of the action
-- that caused it, or the DM, may take it back.
create policy "hp_events: dm or action owner deletes"
  on public.hp_events for delete
  using (
    public.is_game_dm(game_id)
    or exists (select 1 from public.actions a
               where a.id = action_id and a.owner_uid = auth.uid())
  );
