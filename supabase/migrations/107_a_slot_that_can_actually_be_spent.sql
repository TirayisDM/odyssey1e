-- 107. A SLOT THAT CAN ACTUALLY BE SPENT.
--
-- 106 showed slots and said on the tab that they were never spent,
-- because casting wanted a path that did not exist. The tab now wants
-- "how many are left at this level" on every header, and that is not a
-- display question - it needs the spending to be real.
--
-- ONLY THE SPENDING IS STORED. How many a character HAS comes from
-- prayers::slots_at and their cleric level, worked out on every read -
-- so a level-up widens the pool without anything being migrated, which
-- is the same contract character_uses has had since 092.
--
-- BY LEVEL, NOT BY SPELL. A slot is a slot: a 3rd-level slot can carry
-- a 1st-level spell and often should. Tying expenditure to the spell
-- cast would make upcasting unrepresentable, which is the mistake 101
-- had to undo in the catalogue.
--
-- A LONG REST RETURNS THEM ALL AND A SHORT REST RETURNS NONE, for a
-- cleric. That is not universal - a warlock's Pact Magic comes back on
-- a short rest - which is why the rule lives in Rust beside the class
-- rather than in a column here.

create table if not exists character_slots (
  character_id uuid not null references characters(id) on delete cascade,
  slot_level   integer not null,
  spent        integer not null default 0,
  primary key (character_id, slot_level),
  constraint character_slots_level_range check (slot_level between 1 and 9),
  constraint character_slots_not_negative check (spent >= 0)
);

comment on table character_slots is
  'How many spell slots of each level a character has spent since their last long rest. Only the SPENDING is stored - how many they HAVE is prayers::slots_at of their cleric level, derived on every read, so a level-up widens the pool with nothing to migrate.';
comment on column character_slots.slot_level is
  'The slot''s level, 1 to 9 - not the spell''s. A 3rd-level slot can carry a 1st-level spell and often should, and tying expenditure to the spell cast would make upcasting unrepresentable.';

alter table character_slots enable row level security;

drop policy if exists "character_slots: read with character" on character_slots;
create policy "character_slots: read with character" on character_slots
  for select using (exists (
    select 1 from characters c
    where c.id = character_slots.character_id and is_game_member(c.game_id)));

drop policy if exists "character_slots: owner or dm writes" on character_slots;
create policy "character_slots: owner or dm writes" on character_slots
  for insert with check (exists (
    select 1 from characters c
    where c.id = character_slots.character_id
      and (c.owner_uid = auth.uid() or is_game_dm(c.game_id))));

drop policy if exists "character_slots: owner or dm updates" on character_slots;
create policy "character_slots: owner or dm updates" on character_slots
  for update using (exists (
    select 1 from characters c
    where c.id = character_slots.character_id
      and (c.owner_uid = auth.uid() or is_game_dm(c.game_id))))
  with check (exists (
    select 1 from characters c
    where c.id = character_slots.character_id
      and (c.owner_uid = auth.uid() or is_game_dm(c.game_id))));

drop policy if exists "character_slots: owner or dm deletes" on character_slots;
create policy "character_slots: owner or dm deletes" on character_slots
  for delete using (exists (
    select 1 from characters c
    where c.id = character_slots.character_id
      and (c.owner_uid = auth.uid() or is_game_dm(c.game_id))));