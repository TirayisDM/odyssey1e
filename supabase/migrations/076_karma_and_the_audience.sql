-- 076. KARMA, AND WHO IS LISTENING.
--
-- Karma is a universal ability that shows up differently for every
-- class. The bard is the first of them, and Bard Karma is Insight +
-- Performance.
--
-- SO IT IS A PROPERTY OF THE CLASS, NOT OF THE BARD. Writing
-- "ins + prf" into Rust would mean the second class to get Karma is a
-- code change and a release; writing it into a column means the second
-- class is a row. `karma_skills` names which skills sum, and
-- karma.rs reads the character's modifier for each and adds them.
--
-- MODIFIERS, AND EXPERTISE COUNTS. The skill modifier is the ability
-- modifier plus the proficiency bonus, doubled where a character has
-- expertise - which bards get, and which is why a late-career bard
-- will sit pinned at the top of the table. That is deliberate: the
-- ceiling IS the reward.
--
-- PER CLASS MEANS PER CLASS, with 073 in the ground. A Fighter 5 /
-- Bard 3 has one class with a Karma formula and one without, and the
-- Karma that applies is whichever class they are acting as. Nothing
-- sums two classes' Karma together and nothing takes the better of
-- them.

alter table classes
  add column if not exists karma_skills text[] not null default '{}';

comment on column classes.karma_skills is
  'Which skills sum to this class''s Karma, by skills.key. Bard is {ins,prf} - Insight plus Performance. Empty means this class has no Karma expression yet, which is every class but the bard. Summed as MODIFIERS with expertise counted, in karma.rs. Per class and never per character: a Fighter/Bard uses Bard Karma when acting as a bard, and nothing adds two classes together.';

update classes set karma_skills = array['ins', 'prf']
 where key = 'bard' and game_id is null;

-- WHO IS LISTENING, as the other axis of the percentile table.
--
-- The HOPPER chart is `50 + 2 * (karma - rating)`, floored at 01 and
-- printed as 00 at 100 - so a point of audience goodwill and a point
-- of Karma cancel exactly, and only the gap between them matters. The
-- table is a one-dimensional scale wearing a square's face, and
-- karma.rs is where that arithmetic lives.
--
-- A TABLE RATHER THAN AN ENUM, for two reasons that are the same
-- reason. The spacing below is a FIRST GUESS at a feel - Neutral at 8,
-- goodwill compressed, hostility spread, because a friendly room helps
-- less than a hostile one hurts - and retuning a guess should be one
-- UPDATE and no rebuild. And a game that wants "Drunk" or "Royal
-- Court" should get it by inserting a row, which is what the nullable
-- `game_id` is for: NULL is the catalogue everybody reads, and a row
-- with a game_id is that game's own.
create table if not exists audiences (
  id      uuid primary key default gen_random_uuid(),
  game_id uuid references games(id) on delete cascade,
  key     text not null,
  name    text not null,
  rating  integer not null,
  sort    integer not null default 0,
  constraint audiences_rating_range check (rating between 0 and 25)
);

comment on table audiences is
  'How receptive a room is, as the opposing axis of the HOPPER percentile table. HIGHER IS WORSE for the performer - the chart subtracts it from Karma - so Participating is 0 and Hostile is 24. Reference data, tenanted the way items and classes are: game_id NULL is the catalogue every game reads.';
comment on column audiences.rating is
  'The number that goes on the table''s top axis, 0-25. Higher opposes the performer harder. The spacing is a tuning decision rather than a rule, which is why it is data: Neutral sits at 8 and the steps are uneven on purpose, goodwill compressed and hostility spread.';
comment on column audiences.sort is
  'Display order, best room first. Not derivable from rating - two audiences could share a rating and still want a stable order on a dropdown.';

-- The same two partial indexes every tenanted catalogue in this schema
-- carries: a key is unique within the global rows, and unique within
-- each game, and the two do not collide - which is what lets a game
-- override a global key with its own row.
create unique index if not exists audiences_global_key
  on audiences (key) where game_id is null;
create unique index if not exists audiences_game_key
  on audiences (game_id, key) where game_id is not null;

alter table audiences enable row level security;

drop policy if exists "audiences: read global or own game" on audiences;
create policy "audiences: read global or own game" on audiences
  for select using (game_id is null or is_game_member(game_id));

drop policy if exists "audiences: dm writes own game" on audiences;
create policy "audiences: dm writes own game" on audiences
  for insert with check (game_id is not null and is_game_dm(game_id));

drop policy if exists "audiences: dm updates own game" on audiences;
create policy "audiences: dm updates own game" on audiences
  for update using (game_id is not null and is_game_dm(game_id))
  with check (game_id is not null and is_game_dm(game_id));

drop policy if exists "audiences: dm deletes own game" on audiences;
create policy "audiences: dm deletes own game" on audiences
  for delete using (game_id is not null and is_game_dm(game_id));

insert into audiences (key, name, rating, sort) values
  ('participating',  'Participating',  0,  1),
  ('watching',       'Watching',       3,  2),
  ('some_interest',  'Some interest',  5,  3),
  ('neutral',        'Neutral',        8,  4),
  ('busy',           'Busy',          13,  5),
  ('distracted',     'Distracted',    18,  6),
  ('hostile',        'Hostile',       24,  7)
on conflict do nothing;
