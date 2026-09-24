-- =====================================================================
-- 045_weapons_take_room.sql
-- odyssey1e — a pike and a dagger stop costing the same
-- =====================================================================
--
-- Every one of the fifty-seven weapons takes exactly one slot, because
-- 032 set real slot values for the things its container economy needed
-- - coins, arrows, the containers themselves - and left `slots` at its
-- column default of 1 for everything else. That was not a decision
-- about weapons; it was the absence of one.
--
-- So 036's size ladder did all the gating and slots did none. A
-- backpack that holds Medium correctly refuses a pike, and would then
-- accept twenty longswords - or twenty daggers, at the same price.
--
-- SLOTS ARE BULK. WEIGHT IS MASS. That separation is the whole of this
-- migration, and it is now clean:
--
--   size   -> slots   how much ROOM it takes up in a container
--   weight -> carry   how much it WEIGHS on the person holding it
--
-- carry.rs exists as of yesterday and reads weight for encumbrance, so
-- the second half has somewhere to live. Before that, weight was a
-- column nothing read and there was a standing temptation to make slots
-- do both jobs.
--
-- DERIVED FROM SIZE, NOT FROM WEIGHT, and the longbow is why. It is 2
-- pounds and six feet long; a pike is 18 pounds and eighteen feet. In a
-- pack they are the same problem - a long awkward thing that does not
-- fit - and they are nothing alike on a scale. Room is about shape.
-- A weight-derived slot cost would make the longbow as packable as a
-- dagger, which is exactly backwards.
--
-- THE LADDER DOUBLES, because the sizes do. Two Small things in the
-- room of one Medium, two Medium in one Large. It is the simplest rule
-- that makes the ladder mean something, and a fractional slot is
-- already how this schema works - a coin is 0.2 because a 5-slot purse
-- holds twenty-five of them.
--
--   tiny 0.25   sm 0.5   med 1   lg 2   huge 4   grg 8
--
-- A backpack is 20 slots and stops at Medium, so: eighty daggers, forty
-- shortswords, twenty longswords, and no pike at all. A chest is 30 and
-- stops at Large, so fifteen pikes.
--
-- WHAT IS DELIBERATELY NOT TOUCHED. Coins at 0.2 and arrows at 0.05 are
-- 032's own numbers, calibrated against the purse and the quiver rather
-- than against this ladder, and they are currencies and ammunition
-- rather than things you pack. Changing them would re-price every purse
-- in play to fix a consistency nobody is bothered by.
--
-- WHAT THIS LEAVES CROOKED, said plainly rather than discovered later:
-- ARMOUR AND GEAR ARE STILL AT THE DEFAULT 1. Plate is Large and 65
-- pounds and costs one slot, while a pike is Large and costs two. That
-- is the same species of inconsistency this migration removes, one
-- table over, and the fix is this same CASE with `kind` widened. It was
-- left out because the ask was weapons and because armour deserves a
-- look of its own - a suit of plate is bulky worn and bulkier packed,
-- and it may want more than the ladder gives it.
--
-- Nothing is in play with a weapon inside a container - checked before
-- this was written - so no container goes over its capacity as a
-- result. Weapons get CHEAPER at Tiny and Small and dearer at Large,
-- and had anything been packed, the dear direction is the one that
-- could have overfilled a chest.
-- =====================================================================

update public.items
   set slots = case size
                 when 'tiny' then 0.25
                 when 'sm'   then 0.5
                 when 'med'  then 1
                 when 'lg'   then 2
                 when 'huge' then 4
                 when 'grg'  then 8
               end
 where kind = 'weapon';

comment on column public.items.slots is
  'How much ROOM one of these takes. Fractional on purpose: a coin purse is 5 slots and holds 25 coins, so a coin is 0.2 - the same arithmetic the earlier game used. For weapons this is derived from size on a doubling ladder (045): tiny 0.25, sm 0.5, med 1, lg 2. Bulk, not mass - weight is a separate column and belongs to encumbrance, which is a rule about the person carrying it rather than about the sack. Armour and general gear are still at the default of 1 and have not had this look yet.';
