-- 092. A GAME KNOWS WHAT TIME IT IS.
--
-- The engine counted rounds inside a fight and had no idea what time it
-- was outside one. "Inspires for 1 hour, does not stack" had nowhere to
-- be measured, which blocked every timed effect: the bard's song,
-- concentration, exhaustion, and the rest cycle that brings Action
-- Surge back.
--
-- ONE COUNTER, IN SIX-SECOND TICKS, because a round IS six seconds and
-- so every 5e duration is a whole number of them with no second time
-- system to keep in step. The arithmetic is clock.rs.
--
-- ONE PER GAME. A party shares a timeline; a clock each would mean
-- reconciling them the moment anybody scouted ahead.
--
-- GAME TIME, NEVER WALL TIME. Nothing reads now(). A session that
-- breaks for an hour has not aged anybody.

alter table games
  add column if not exists tick bigint not null default 0;

comment on column games.tick is
  'The game clock, in six-second ticks - a tick IS a round, which is what lets combat time and travel time be one number. Tick 0 is day 1 at midnight. Moved by combat a round at a time and by the DM in jumps; it only ever goes forward, because a roll is a record and so is the hour it happened in. The arithmetic is clock.rs.';

-- WHEN EACH CHARACTER LAST STARTED A LONG REST. Per character rather
-- than per game, because 5e's limit is per creature: "a character can't
-- benefit from more than one long rest in a 24-hour period". A party
-- that splits its watch does not all sleep at once.
--
-- NULL MEANS NEVER, which is every character today, and is why the
-- first long rest needs no permission.
alter table characters
  add column if not exists last_long_rest bigint;

comment on column characters.last_long_rest is
  'The games.tick at which this character last STARTED a long rest, or NULL for never. 5e allows one per 24 hours and measures from the start, which is the period the rule names. Per character and not per game: the limit is on the creature, and a party that splits its watch does not all sleep at once.';

-- HIT DICE SPENT, per class, because the die is the class's. A Fighter
-- 4 / Bard 1 spends a d10 or a d8 and they are not interchangeable.
alter table character_classes
  add column if not exists hit_dice_spent integer not null default 0,
  add constraint character_classes_dice_spent_sane
    check (hit_dice_spent >= 0 and hit_dice_spent <= level);

comment on column character_classes.hit_dice_spent is
  'How many of THIS class''s hit dice have been spent and not yet recovered. Per class because the die is the class''s - a Fighter 4 / Bard 1 spends a d10 or a d8 and they are not the same thing. A long rest returns half the character''s total, minimum one; a short rest returns none.';

-- HOW MANY TIMES A FEATURE CAN BE USED, and what brings it back.
--
-- AN EXPRESSION RATHER THAN A NUMBER, because almost none of them are
-- one: Action Surge is once then twice from 17, Ki is the monk's level,
-- Bardic Inspiration is a Charisma modifier. Four forms cover nearly
-- everything - see uses.rs.
alter table class_features
  add column if not exists uses text,
  add column if not exists recharge text;

alter table class_features drop constraint if exists class_features_recharge_check;
alter table class_features add constraint class_features_recharge_check
  check (recharge is null or recharge in ('short', 'long', 'day', 'dawn'));

comment on column class_features.uses is
  'How many times before a rest, as an expression: a flat number, `level` (THE CLASS''S level, never the character''s total), an ability modifier like `cha_mod`, or bands like `1@1,2@17`. NULL means no limit worth tracking - Evasion has no number and Second Wind does, and "unlimited" must not read the same as "none left". Evaluated by uses.rs.';
comment on column class_features.recharge is
  'What brings it back: short, long, day, dawn. A SHORT REST RESTORES WHAT A LONG ONE DOES NOT - 5e says "short or long" on Action Surge and means it, and the reverse is not true, which is the whole asymmetry. NULL with a non-null `uses` means it never comes back on its own.';

-- WHAT A CHARACTER HAS SPENT. The counterpart to class_features.uses,
-- and the only part that is not derivable: how many are LEFT is the
-- expression less this.
create table if not exists character_uses (
  character_id uuid not null references characters(id) on delete cascade,
  class_key    text not null,
  feature_key  text not null,
  spent        integer not null default 0,
  primary key (character_id, class_key, feature_key),
  constraint character_uses_not_negative check (spent >= 0)
);

comment on table character_uses is
  'How many times a character has used a feature since it last recharged. The maximum is derived from class_features.uses and their level on every read, so this stores only the spending - which means a level-up raises the ceiling without anything having to be migrated.';

alter table character_uses enable row level security;

drop policy if exists "character_uses: read with character" on character_uses;
create policy "character_uses: read with character" on character_uses
  for select using (exists (
    select 1 from characters c
    where c.id = character_uses.character_id and is_game_member(c.game_id)));

drop policy if exists "character_uses: owner or dm writes" on character_uses;
create policy "character_uses: owner or dm writes" on character_uses
  for insert with check (exists (
    select 1 from characters c
    where c.id = character_uses.character_id
      and (c.owner_uid = auth.uid() or is_game_dm(c.game_id))));

drop policy if exists "character_uses: owner or dm updates" on character_uses;
create policy "character_uses: owner or dm updates" on character_uses
  for update using (exists (
    select 1 from characters c
    where c.id = character_uses.character_id
      and (c.owner_uid = auth.uid() or is_game_dm(c.game_id))))
  with check (exists (
    select 1 from characters c
    where c.id = character_uses.character_id
      and (c.owner_uid = auth.uid() or is_game_dm(c.game_id))));

drop policy if exists "character_uses: owner or dm deletes" on character_uses;
create policy "character_uses: owner or dm deletes" on character_uses
  for delete using (exists (
    select 1 from characters c
    where c.id = character_uses.character_id
      and (c.owner_uid = auth.uid() or is_game_dm(c.game_id))));
