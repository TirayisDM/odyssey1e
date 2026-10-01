-- 073. A CHARACTER IS MORE THAN ONE THING.
--
-- 055 gave a character `class_key` and `level`. One class, and that
-- level column was doing two jobs at once: the class's level and the
-- character's. For a single-classed character they are the same
-- number, which is why it worked and why it will not stretch. A
-- Fighter 5 / Rogue 3 has three levels at once - 5, 3, and the 8 that
-- decides their proficiency bonus - and no one column holds that.
--
-- SO THE FACT MOVES TO ITS OWN TABLE, one row per class. The
-- arithmetic over those rows is in multiclass.rs, where it is tested;
-- this file is the shape it is stored in.
--
-- `characters.class_key` AND `characters.level` STAY, and are kept in
-- step by a trigger. Two reasons, and neither is reluctance:
--
--   A MONSTER HAS A LEVEL AND NO CLASS. 029 made `level` mean HIT DICE
--   for an NPC, and there is no class row to derive that from. The
--   column has to remain the truth for them.
--
--   EVERY SCREEN AND EVERY ROLL READS `level`. The initiative strip,
--   the target list, the proficiency bonus, `instantiate_npc`. Making
--   all of them sum a second table to learn what level somebody is
--   would be a dozen new round trips to answer a question the row can
--   already answer.
--
-- The trigger is what keeps that from becoming the thing this codebase
-- keeps getting bitten by - a fact written down in two places that
-- drift apart. Nothing in the app writes `characters.level` for a
-- classed character any more; the class rows decide and the trigger
-- follows. `set_level` refuses outright for anybody with class rows,
-- so there is no second writer to disagree with.

create table if not exists character_classes (
  character_id uuid not null references characters(id) on delete cascade,
  class_key    text not null,
  level        integer not null default 1,
  added_at     timestamptz not null default now(),
  primary key (character_id, class_key),
  constraint character_classes_level_range check (level between 1 and 20)
);

comment on table character_classes is
  'One row per class a character holds. 073 split this off characters.class_key, which could only hold one. The arithmetic - total level, which class leads, hit points across classes, extra attack - is multiclass.rs.';
comment on column character_classes.class_key is
  'Into classes.key, BY VALUE and with no foreign key, for 055''s reason: the two partial unique indexes that carry global-versus-game tenancy cannot back one. A dangling key leaves the character on one attack rather than emptying a screen.';
comment on column character_classes.level is
  'Levels in THIS class, 1-20. Never the character''s level - that is the sum, and characters.level carries it. A class at zero is a class they do not have, so dropping one is a delete.';
comment on column character_classes.added_at is
  'WHEN THIS CLASS WAS TAKEN, and it decides two rules rather than being a timestamp for curiosity. The earliest row is the STARTING class, which is the one paid a whole hit die for level one - once in a career, not once per class. It is also the tie-break for which class leads when two are equal.';

alter table character_classes enable row level security;

-- The same four policies character_abilities and character_skills
-- carry, pointing at the same two predicates: a game member may read
-- what belongs to a character in their game, and the owner or the DM
-- may write it.
drop policy if exists "character_classes: read with character" on character_classes;
create policy "character_classes: read with character" on character_classes
  for select using (exists (
    select 1 from characters c
    where c.id = character_classes.character_id and is_game_member(c.game_id)));

drop policy if exists "character_classes: owner or dm writes" on character_classes;
create policy "character_classes: owner or dm writes" on character_classes
  for insert with check (exists (
    select 1 from characters c
    where c.id = character_classes.character_id
      and (c.owner_uid = auth.uid() or is_game_dm(c.game_id))));

drop policy if exists "character_classes: owner or dm updates" on character_classes;
create policy "character_classes: owner or dm updates" on character_classes
  for update using (exists (
    select 1 from characters c
    where c.id = character_classes.character_id
      and (c.owner_uid = auth.uid() or is_game_dm(c.game_id))))
  with check (exists (
    select 1 from characters c
    where c.id = character_classes.character_id
      and (c.owner_uid = auth.uid() or is_game_dm(c.game_id))));

drop policy if exists "character_classes: owner or dm deletes" on character_classes;
create policy "character_classes: owner or dm deletes" on character_classes
  for delete using (exists (
    select 1 from characters c
    where c.id = character_classes.character_id
      and (c.owner_uid = auth.uid() or is_game_dm(c.game_id))));

-- THE TRIGGER THAT STOPS THE TWO PLACES DISAGREEING.
--
-- `characters.level` becomes the SUM of the class rows and
-- `characters.class_key` the one with the most levels - ties to the
-- earliest taken, which is what multiclass::primary does in Rust and
-- the reason both are written here rather than in a command. A command
-- can be forgotten; a trigger cannot.
--
-- DELETING THE LAST CLASS LEAVES `level` ALONE. There is nothing to sum
-- and zero is not a level anybody is - a character who drops their only
-- class is classless at the level they reached, which is exactly the
-- state every character made before 055 is in.
create or replace function sync_character_level()
returns trigger
language plpgsql
security definer
set search_path = public
as $$
declare
  who   uuid;
  tally integer;
  lead_key text;
begin
  who := coalesce(new.character_id, old.character_id);

  select sum(level) into tally from character_classes where character_id = who;

  -- `class_key` LAST IN THE ORDER, so two classes added in the same
  -- transaction - which share a default now() - still resolve to one
  -- answer rather than whichever the planner happened to return.
  -- multiclass::primary breaks the same tie the same way, on a slice
  -- the command sorts to match.
  select class_key into lead_key from character_classes
   where character_id = who
   order by level desc, added_at asc, class_key asc
   limit 1;

  update characters c
     set level = coalesce(tally, c.level),
         class_key = lead_key
   where c.id = who;

  return null;
end
$$;

drop trigger if exists character_classes_sync on character_classes;
create trigger character_classes_sync
  after insert or update or delete on character_classes
  for each row execute function sync_character_level();

-- THE BACKFILL. Every character that already has a class gets the row
-- it should have had, at the level it is already carrying. `added_at`
-- is the character's creation time rather than now(), because this
-- class IS the one they started as and the starting-class rule reads
-- that column.
--
-- Monsters included, deliberately: 064 gave them a class too, and it
-- buys them an attack count the same way. Their hit points still come
-- from size - `rederive_hp_max` refuses NPCs outright - so a class row
-- changes nothing about how tough they are.
insert into character_classes (character_id, class_key, level, added_at)
select c.id, c.class_key, greatest(1, least(20, c.level)), c.created_at
  from characters c
 where c.class_key is not null
   and c.class_key <> ''
on conflict (character_id, class_key) do nothing;
