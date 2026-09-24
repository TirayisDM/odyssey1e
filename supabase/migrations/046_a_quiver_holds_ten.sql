-- =====================================================================
-- 046_a_quiver_holds_ten.sql
-- odyssey1e — two hundred arrows was never anybody's decision
-- =====================================================================
--
-- A quiver is 10 slots and an arrow is 0.05, so it held two hundred.
-- Nobody chose that number; it fell out of two values set separately.
-- The house rule is TEN.
--
-- THE QUIVER IS NOT THE THING THAT WAS WRONG. An arrow at 0.05 slots is
-- a twentieth of a dagger, and they are both Tiny - 036 says so, and it
-- is the only ladder this schema has. The arrow was priced against
-- nothing, the way the coin was priced against the purse, except that
-- no one had worked out what a quiver was supposed to hold.
--
-- So the bulk moves onto the ladder 045 uses and the capacity follows
-- from the answer:
--
--   arrow    0.05 -> 0.25    Tiny, the same as a dagger
--   quiver   10   -> 2.5     ten arrows, which is the rule
--
-- 2.5 is not a round number and that is fine - it is DERIVED. The
-- capacity of a quiver is "ten arrows", and the arrow's bulk is what
-- turns that into slots. Rounding it to 2 would quietly mean eight.
--
-- WHY NOT THE OTHER WAY. Leaving the arrow at 0.05 and setting the
-- quiver to 0.5 also gives ten, and it is the smaller edit. It also
-- leaves a backpack holding four hundred arrows, which is the actual
-- absurdity - the quiver was only where it became visible.
--
-- WHY NOT ARROW = 1, QUIVER = 10. That reads beautifully - ten slots,
-- ten arrows - and prices an arrow as bulky as a longsword. A backpack
-- would take twenty. The number that is easy to read is not the number
-- that is true.
--
-- THE WEIGHT DOES NOT MOVE, and this is the clearest illustration yet
-- of the split 045 drew. An arrow weighs 0.05 pounds because the book
-- says twenty arrows weigh a pound, and that is a fact about a scale.
-- Its bulk is 0.25 because it is a Tiny thing taking up room in a bag.
-- The two numbers disagree by a factor of five and both are right.
--
--   size   -> slots   room in a container
--   weight -> carry   load on the person
--
-- Eighty arrows in a backpack now, not four hundred. A quiver of ten.
--
-- Nothing is in play with either key - checked before this was written
-- - so no container goes over capacity and no player's ammunition
-- changes underneath them.
-- =====================================================================

update public.items set slots = 0.25 where key = 'arrow' and game_id is null;

update public.items set capacity_slots = 2.5 where key = 'quiver' and game_id is null;

comment on column public.items.capacity_slots is
  'How much room a container HAS, in slots. NULL is a FAULT rather than "infinite" - a container whose capacity nobody wrote down is a catalogue row somebody did not finish, and pretending it is bottomless hides that. Derived from what the container is meant to hold where that is the rule: a quiver is 2.5 because it takes ten arrows at a quarter slot each (046), and a coin purse is 5 because it takes twenty-five coins at a fifth.';
