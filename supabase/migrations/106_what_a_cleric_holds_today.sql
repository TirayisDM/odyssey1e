-- 106. WHAT A CLERIC HOLDS TODAY.
--
-- A cleric is a PREPARED caster: the list they draw from is the whole
-- cleric list, every day, and what limits them is how many they may
-- hold at once and how many slots they have to spend. So the only
-- thing worth storing is WHICH ONES - everything else is derived from
-- their cleric level and Wisdom on every read, in prayers.rs.
--
-- CANTRIPS LIVE HERE TOO, and the `prepared` column is what tells them
-- apart. A cantrip is KNOWN rather than prepared: it does not come out
-- of the prepared count, it never costs a slot, and a long rest does
-- not change it. One table because they are both "spells this
-- character has chosen", and one column because the difference is real
-- and worth naming rather than inferring from the spell's level.
--
-- NO SLOT TRACKING YET. Spending a slot is a use like any other and
-- 092's `character_uses` already counts those; wiring it wants the
-- cast path, which does not exist. The slots are shown and not yet
-- spent, which is the honest state and is said on the tab.

create table if not exists character_prayers (
  character_id uuid not null references characters(id) on delete cascade,
  spell_key    text not null,
  prepared     boolean not null default true,
  chosen_at    timestamptz not null default now(),
  primary key (character_id, spell_key)
);

comment on table character_prayers is
  'Which spells a cleric has chosen. Only the CHOICE is stored - how many they may hold, how many slots they have and what their save DC is are all derived from their cleric level and Wisdom in prayers.rs, so a level-up moves every one of them without anything being migrated.';
comment on column character_prayers.prepared is
  'TRUE for a prepared spell, FALSE for a cantrip - which is known rather than prepared, does not come out of the prepared count, never costs a slot, and survives a long rest. The difference is real and worth a column rather than being inferred from the spell''s level.';
comment on column character_prayers.spell_key is
  'Into spells.key, BY VALUE and with no foreign key - 004''s reason, the two partial unique indexes that carry tenancy cannot back one. A dangling key shows as a missing spell rather than emptying the tab.';

alter table character_prayers enable row level security;

drop policy if exists "character_prayers: read with character" on character_prayers;
create policy "character_prayers: read with character" on character_prayers
  for select using (exists (
    select 1 from characters c
    where c.id = character_prayers.character_id and is_game_member(c.game_id)));

drop policy if exists "character_prayers: owner or dm writes" on character_prayers;
create policy "character_prayers: owner or dm writes" on character_prayers
  for insert with check (exists (
    select 1 from characters c
    where c.id = character_prayers.character_id
      and (c.owner_uid = auth.uid() or is_game_dm(c.game_id))));

drop policy if exists "character_prayers: owner or dm updates" on character_prayers;
create policy "character_prayers: owner or dm updates" on character_prayers
  for update using (exists (
    select 1 from characters c
    where c.id = character_prayers.character_id
      and (c.owner_uid = auth.uid() or is_game_dm(c.game_id))))
  with check (exists (
    select 1 from characters c
    where c.id = character_prayers.character_id
      and (c.owner_uid = auth.uid() or is_game_dm(c.game_id))));

-- PUTTING A SPELL DOWN IS A DELETE, unlike a choice or an effect: a
-- prepared list is what you hold TODAY and changes at every long rest.
-- There is no history worth keeping in which spells you held last
-- Tuesday, which is what makes this different from character_choices.
drop policy if exists "character_prayers: owner or dm deletes" on character_prayers;
create policy "character_prayers: owner or dm deletes" on character_prayers
  for delete using (exists (
    select 1 from characters c
    where c.id = character_prayers.character_id
      and (c.owner_uid = auth.uid() or is_game_dm(c.game_id))));