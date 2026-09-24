-- =====================================================================
-- 047_armour_and_gear_take_room.sql
-- odyssey1e — the rest of the catalogue stops costing one slot each
-- =====================================================================
--
-- 045 priced the weapons and said in its own header what it was
-- leaving crooked: plate is Large and sixty-five pounds and costs one
-- slot, while a pike is Large and costs two. This is that, plus two
-- defects the same look turned up.
--
-- ---------------------------------------------------------------------
-- TWO BUGS FIRST, because they are bugs rather than calibration.
-- ---------------------------------------------------------------------
--
-- A PLATINUM PIECE DOES NOT FIT IN A COIN PURSE. 039 added `coin_pp`
-- after 036 had already sized the catalogue, so it took the column
-- default of Medium while every other coin is Tiny. A purse is
-- holds_size Tiny, so `admits_size` refuses platinum from the one
-- container built for coins. Nobody hit it because no platinum has
-- been minted yet.
--
-- That is the hazard 036 built in and did not name: a DEFAULT that is
-- reasonable for most rows is silent when it is wrong, and every
-- migration adding an item after it inherits the gap. 042 avoided it by
-- writing sizes per row; 039 did not.
--
-- RING MAIL IS HEAVY ARMOUR FILED WITH THE MEDIUM SUITS. 036 put plate,
-- splint and chain mail in its Large list and missed it, so a forty
-- pound suit of heavy armour is Medium and packs into a backpack that
-- refuses the fifty-five pound one beside it. Same misfiling as the
-- trident in 044.
--
-- ---------------------------------------------------------------------
-- ARMOUR IS NOT PRICED ON THE LADDER, AND THAT IS THE POINT OF 045'S
-- WARNING.
-- ---------------------------------------------------------------------
--
-- The size ladder measures the LONGEST DIMENSION, which is the right
-- question for a weapon: a pike and a longbow are both awkward because
-- they are long. Armour is not long, it is a BUNDLE - a person-shaped
-- volume of metal that does not fold. Two slots for a suit of plate
-- would let a treasure chest hold fifteen of them.
--
-- So armour is priced by `armor_category`, which is the column that
-- already states how much armour it is:
--
--   lgt  2   padded, leather, studded leather
--   med  3   hide, chain shirt, scale mail, breastplate, half plate
--   hvy  5   ring mail, chain mail, splint, plate
--   shl  2   a shield is flat but wide
--
-- A chest is 30 slots and takes Large, so six suits of plate. A
-- backpack is 20 and stops at Medium, so ten leathers or six
-- breastplates - and no plate at all, which is the size rule doing its
-- job.
--
-- WEIGHT IS STILL NOT DOING THIS. Half plate and scale mail are both 40
-- and 45 pounds and both cost 3, while ring mail is 40 pounds and costs
-- 5 - because ring mail is a heavy suit and the other two are not. The
-- category is a statement about coverage and rigidity, which is what
-- fills a bag. Weight remains carry.rs's business.
--
-- ---------------------------------------------------------------------
-- GEAR GOES ON THE LADDER, because gear is ordinary things.
-- ---------------------------------------------------------------------
--
--   tiny 0.25   sm 0.5   med 1   lg 2
--
-- Equipment, consumables and loot, EXCEPT the two 032 calibrated
-- against a container rather than against a ladder: coins stay at 0.2
-- because a five-slot purse holds twenty-five, and the arrow stays at
-- 046's 0.25 because a quiver holds ten.
--
-- CONTAINERS KEEP WHAT 032 CHOSE where it chose anything. A chest is 8
-- slots and a coin purse is 0.5 - both deliberate, both different from
-- the default, both left alone. The quiver and the spell book were
-- sitting at the default 1 and are Small, so they join the ladder at
-- 0.5. The backpack and the priest's pack are Medium and were already
-- 1, which is where the ladder puts them anyway.
--
-- ---------------------------------------------------------------------
-- NOTHING OVERFLOWS, and it was checked against the database rather
-- than against memory - the first draft of this paragraph described
-- what was packed two days ago and was wrong by the time it was
-- written. Four containers hold anything:
--
--   Expedition Backpack   13.2 -> 6.45 of 20
--   backpack               6.0 -> 5.0  of 20
--   Rodnar's Purse         5.0 -> 5.0  of 5   (coins, unchanged)
--   coin_purse             4.0 -> 4.0  of 5   (coins, unchanged)
--
-- Every change here is CHEAPER except armour, and no armour is inside
-- anything. Rodnar's purse stays exactly full, which is where it was.
-- =====================================================================

-- --------------------------------------------------------------- bugs
update public.items set size = 'tiny' where key = 'coin_pp' and game_id is null;
update public.items set size = 'lg'   where key = 'ring_mail' and game_id is null;

-- ------------------------------------------------------------- armour
update public.items
   set slots = case armor_category
                 when 'lgt' then 2
                 when 'med' then 3
                 when 'hvy' then 5
                 when 'shl' then 2
               end
 where kind = 'armor' and armor_category is not null;

-- --------------------------------------------------------------- gear
update public.items
   set slots = case size
                 when 'tiny' then 0.25
                 when 'sm'   then 0.5
                 when 'med'  then 1
                 when 'lg'   then 2
                 when 'huge' then 4
                 when 'grg'  then 8
               end
 where kind in ('equipment', 'consumable', 'loot')
   -- 032's and 046's calibrations, priced against a purse and a quiver
   -- rather than against this ladder.
   and key not in ('coin_cp', 'coin_sp', 'coin_gp', 'coin_pp', 'arrow');

-- --------------------------------------------------- the two containers
-- still sitting on the column default. The chest at 8 and the purse at
-- 0.5 are decisions and are not touched.
update public.items
   set slots = 0.5
 where kind = 'container' and key in ('quiver', 'spell_book');
