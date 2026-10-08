-- 171. A CAMPAIGN THAT IS NOT GOLD HEAVY.
--
-- Dave's rates, and they are not a tweak to 170 - they change what the
-- unit of a spellbook IS.
--
-- ---------------------------------------------------------------------
-- SILVER, NOT GOLD
-- ---------------------------------------------------------------------
--
--   ink, one vial        5 sp     (was 25 gp)
--   quill and penknife   2 sp     (unchanged)
--   blank scroll        10 sp     (was 10 gp)
--
-- THE RATIO IS UNTOUCHED AND THAT IS THE POINT. `scribe::to_copy` still
-- says two vials per spell level; only the price of a vial moved, so a
-- first-level spell costs 10 sp and a ninth costs 90. The rule never
-- knew the price and still does not - 170 said the cost IS the
-- materials, and this is that decision paying off: a campaign's economy
-- is eight rows in `items`, not a constant in Rust.
--
-- 170'S HEADER IS WRONG ABOUT THE PUBLISHED FIGURE from here on. It
-- says two vials at 25 gp "comes to the 50 gp per level the book
-- prints", which was true of the catalogue it shipped with and is not
-- true of this one. Left as written per 115; this is the correction.
--
-- ---------------------------------------------------------------------
-- A BOOK HOLDS SPELL LEVELS, NOT SPELLS
-- ---------------------------------------------------------------------
--
-- This is the real change. 170 said one spell takes one slot and the
-- standard book holds ten of them. Dave's books are measured in SPELL
-- LEVELS: a Tome holds fifty, and that is five ninth-level spells or
-- fifty cantrips, not fifty spells.
--
-- THAT IS A BETTER UNIT and it is why `capacity_slots` could not carry
-- it. That column is the CONTAINER system's volume measure - what 036
-- walks to decide whether a backpack is full - and spell levels are not
-- a volume. 170 reused it and called it "the page count now", which was
-- a pun that would have collided the first time somebody put a real
-- object in a book. `spell_levels` is its own column and the books hold
-- no gear.
--
-- A CANTRIP TAKES ONE LEVEL OF ROOM, which is this file's decision and
-- not Dave's: level 0 would make cantrips free and a book would hold
-- infinitely many. A page is a page. Same floor `scribe::to_copy`
-- already applies to time and ink.
--
-- ---------------------------------------------------------------------
-- FOUR BOOKS, AND THE OLD ONE BECOMES THE MIDDLE OF THEM
-- ---------------------------------------------------------------------
--
-- `spell_book` has been in the catalogue since 027 and nothing has ever
-- owned one - no object, no creature's kit - so it is free to reshape.
-- It becomes the Mage's, which is the closest to what it was, rather
-- than being deleted and leaving a key that once meant something.
--
--   spell_book_adventure   Adventure Spell Book   20 sp  sm   2.5 lb  10
--   spell_book_acolyte     Acolyte's Spell Book   30 sp  med  5 lb    20
--   spell_book             Mage's Spell Book      50 sp  med  5 lb    30
--   tome_of_spells         Tome of Spells        100 sp  med  10 lb   50
--
-- WEIGHT AND CAPACITY ARE DAVE'S; `slots` IS NOT. He gave pounds and
-- spell levels and said nothing about how much room one takes in a
-- pack, so these are scaled off the old book's 0.5 and are the one
-- number here to argue with.

alter table public.items
  add column if not exists spell_levels integer;

alter table public.items
  drop constraint if exists items_spell_levels_check;
alter table public.items
  add constraint items_spell_levels_check
  check (spell_levels is null or spell_levels >= 1);

comment on column public.items.spell_levels is
  'How many SPELL LEVELS a book can hold - a 3rd-level spell takes 3 '
  'and a cantrip takes 1. NULL for anything that is not a spellbook. '
  'Deliberately not capacity_slots, which is the container system''s '
  'volume measure. See 171.';

-- ---- the rates -------------------------------------------------------

update public.items set price = 5,  denom = 'sp' where game_id is null and key = 'ink_vial';
update public.items set price = 10, denom = 'sp' where game_id is null and key = 'scroll_blank';
-- quill is already 2 sp.

-- ---- the books -------------------------------------------------------

insert into public.items
  (key, name, kind, size, weight, slots, price, denom,
   accepts, capacity_slots, spell_levels, content_tags, description)
values
('spell_book_adventure','Adventure Spell Book','container','sm',2.5,0.5,20,'sp',
 array['spell'], null, 10, array['lore'],
 'Thin, stitched and meant to be carried - the book a wizard owns because they could not afford a better one, and the one they actually take out of the city.'),
('spell_book_acolyte','Acolyte''s Spell Book','container','med',5,1,30,'sp',
 array['spell'], null, 20, array['lore'],
 'The standard issue of a school that expects you to come back. Twice the room of a travelling book and twice the weight.'),
('spell_book','Mage''s Spell Book','container','med',5,1,50,'sp',
 array['spell'], null, 30, array['lore'],
 'Properly bound, properly sized, and the first book that does not run out halfway through a career.'),
('tome_of_spells','Tome of Spells','container','med',10,1.5,100,'sp',
 array['spell'], null, 50, array['lore'],
 'Ten pounds of vellum and board. Nobody carries one by choice - it lives on a desk, and what a wizard takes travelling is copied out of it.')
on conflict (key) where game_id is null do update set
  name = excluded.name, kind = excluded.kind, size = excluded.size,
  weight = excluded.weight, slots = excluded.slots,
  price = excluded.price, denom = excluded.denom,
  accepts = excluded.accepts, capacity_slots = excluded.capacity_slots,
  spell_levels = excluded.spell_levels, content_tags = excluded.content_tags,
  description = excluded.description;
