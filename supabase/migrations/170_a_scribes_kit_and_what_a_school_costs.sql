-- 170. A SCRIBE'S KIT, AND WHAT A SCHOOL COSTS.
--
-- 150 built the wizard and left the book empty. 161's own header says
-- it plainly - "nothing writes `book`, so a player wizard has an empty
-- book and no way to fill it" - and then 161-167 put 206 wizard spells
-- in the catalogue, which turned a footnote into the whole feature.
--
-- `scribe.rs` is the arithmetic and is already written and tested. This
-- is the part of it that is MADE OF THINGS.
--
-- ---------------------------------------------------------------------
-- THE COST IS THE MATERIALS, NOT A NUMBER BESIDE THEM
-- ---------------------------------------------------------------------
--
-- The book charges "2 hours and 50 gp per spell level" to copy a spell,
-- and says the 50 gp is the fine inks and the components burnt through
-- while working it out. So nothing here charges 50 gp. It consumes the
-- INK, and the ink is what costs the money:
--
--   two vials per spell level, at 25 gp a vial = 50 gp per level
--
-- A ninth-level spell wants eighteen vials and 450 gp, which is the
-- published figure arrived at by buying something real. A wizard who
-- already has ink does not pay twice; a wizard with a full purse and no
-- ink in a wilderness cannot copy anything, which is the entire point of
-- making it a thing rather than a price.
--
-- ---------------------------------------------------------------------
-- THE BOOK IS ALREADY HERE, AND ITS CAPACITY IS NOW A RULE
-- ---------------------------------------------------------------------
--
-- `spell_book` has existed since 027 as a CONTAINER with
-- `accepts = {spell}` and `capacity_slots = 10` - somebody's intent,
-- never implemented, and no item has ever been tagged `spell`.
--
-- THAT COLUMN IS THE PAGE COUNT NOW. One spell takes one slot, so the
-- standard book holds ten and a wizard who wants more buys another
-- book. That reuses the column exactly as it stands rather than adding
-- a second idea of capacity beside it, and it makes "spell book(s)"
-- plural for a reason.
--
-- ---------------------------------------------------------------------
-- PER LEVEL AND PER SCHOOL, AND THE SCHOOL PART IS DATA
-- ---------------------------------------------------------------------
--
-- Level drives the cost and school bends it. `scribe_schools.pct` is a
-- percentage - 100 is the book's own rate - and `reagent_item_key` is
-- the jar a scribe empties for that school.
--
-- SEEDED AT 100 WITH NO REAGENT, which is deliberate and is not the
-- same as not having built it. Every school starts at the published
-- rate because that is the only number anybody can defend yet, and
-- which jar a conjurer empties is a question about Dave's world rather
-- than about 5e. The mechanism is here so tuning is eight UPDATEs and
-- not a Rust change.
--
-- ---------------------------------------------------------------------
-- WHAT IS DELIBERATELY NOT HERE
-- ---------------------------------------------------------------------
--
-- A WRITTEN SCROLL. `scroll_blank` is vellum with nothing on it. A
-- scroll that HOLDS a spell needs somewhere to record which spell, and
-- that is the same open question as where a book's contents live - see
-- STATUS. Adding a half-answer now would be the thing 027 did with
-- `accepts = {spell}`: an intention in the schema that nothing reads,
-- which reads like a feature for a year.
--
-- THE QUILL DOES NOT WEAR OUT. 093 gave objects `uses_spent` and
-- `uses_max` and this could use them, but `items` has no way to declare
-- a default, so wear would be plumbing rather than data. It is a tool
-- you must have, and replacing a worn one is a rule nobody asked for
-- yet.

insert into items
  (key, name, kind, size, weight, slots, price, denom, content_tags, description)
values
('ink_vial','Vial of Fine Ink','consumable','tiny',0.5,0.1,25,'gp',array['scribing'],
 'Iron gall and lamp black ground fine enough to hold a sigil. Two vials go into every level of a spell copied, and they are most of what copying one costs.'),
('quill','Quill and Penknife','equipment','tiny',0,0.1,2,'sp',array['scribing'],
 'A cut feather and the small blade that keeps it cut. Not used up by the work - but nothing is written without one.'),
('scroll_blank','Blank Scroll','consumable','tiny',0.2,0.1,10,'gp',array['scribing'],
 'A sheet of prepared vellum, ruled and sized, waiting for a spell. Blank: what makes a scroll worth anything is written on it afterwards.')
on conflict (key) where game_id is null do update set
  name = excluded.name, kind = excluded.kind, size = excluded.size,
  weight = excluded.weight, slots = excluded.slots,
  price = excluded.price, denom = excluded.denom,
  content_tags = excluded.content_tags, description = excluded.description;

-- ---------------------------------------------------------------------
-- WHAT EACH SCHOOL COSTS
-- ---------------------------------------------------------------------

create table if not exists public.scribe_schools (
  school            text primary key,
  -- 100 IS THE BOOK'S OWN RATE. `scribe::to_copy` multiplies the
  -- level-driven hours and ink by this and rounds UP, so a cheap school
  -- never makes a spell free.
  pct               integer not null default 100 check (pct >= 1),
  -- The jar a scribe empties for this school, over and above the ink.
  -- NULL means the ink is the whole of it.
  reagent_item_key  text,
  reagent_quantity  integer not null default 1 check (reagent_quantity >= 1),
  note              text
);

comment on table public.scribe_schools is
  'Per-school tuning for copying a spell: a percentage applied to the '
  'level-driven time and ink, and an optional reagent. Seeded at 100 '
  'with no reagent so every school starts at the published rate. See 170.';

insert into public.scribe_schools (school) values
  ('abjuration'), ('conjuration'), ('divination'), ('enchantment'),
  ('evocation'), ('illusion'), ('necromancy'), ('transmutation')
on conflict (school) do nothing;

alter table public.scribe_schools enable row level security;

-- READ BY ANYBODY SIGNED IN, WRITTEN BY NOBODY THROUGH THE API. This is
-- reference data like `spells` and `items`: it is tuned by a migration,
-- not by a player, and there is no game_id on it because a percentage
-- that differs per game is a thing to want before it is a thing to
-- build.
drop policy if exists scribe_schools_read on public.scribe_schools;
create policy scribe_schools_read on public.scribe_schools
  for select to authenticated using (true);
