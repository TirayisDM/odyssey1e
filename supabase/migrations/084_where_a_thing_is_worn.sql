-- 084. WHERE A THING IS WORN.
--
-- `objects.equipped` has been a boolean since 008 and says only THAT
-- something is in use, never where. A sheet cannot draw a right hand
-- and a left hand from a bare yes, and it cannot say that a greatsword
-- in one of them empties the other.
--
-- THE LADDER ORGANISES; IT DOES NOT INVENT RULES. 5e has no slot
-- system - what it has is two hands, one suit of armour and three
-- attuned items, and all three already live in carry.rs and
-- equipment.rs. The one addition is the hip, which is a house rule and
-- is marked as one in slots.rs.
--
-- `slot` IS THE EQUIPPED STATE. Non-null means equipped, and `equipped`
-- is dropped rather than kept beside it: a fact in two places is a fact
-- that disagrees with itself, which this codebase has now paid for
-- three times.
--
-- THE BACKFILL HERE WAS WRONG AND 086 FIXES IT. It is left as written
-- because the mistake is worth reading: `equipped` was true on seven
-- things at once, so "everything else in the right hand" put seven
-- things in one hand - not a guess but an impossible state, and one
-- `slots::check` would refuse. See 086.

alter table objects
  add column if not exists slot text;

comment on column objects.slot is
  'WHERE this is worn or held, and the fact that it IS - non-null means equipped, which is why 008''s boolean is gone rather than kept beside it. Values are slots::LADDER: right_hand left_hand hip backpack chest ring_right ring_left amulet head body. The rules about how many fit and what each admits are slots.rs, not this column: a check constraint can say the key is spelled right and cannot say a greatsword leaves no room for a shield.';

alter table objects drop constraint if exists objects_slot_check;
alter table objects add constraint objects_slot_check
  check (slot is null or slot in (
    'right_hand', 'left_hand', 'hip', 'backpack', 'chest',
    'ring_right', 'ring_left', 'amulet', 'head', 'body'));

-- WHERE AN ITEM IS WORN WHEN NEITHER SIZE NOR KIND CAN SAY. A ring, an
-- amulet and a helm are all small pieces of equipment and nothing else
-- distinguishes them, so they state it. NULL for almost everything.
alter table items
  add column if not exists worn_slot text;

comment on column items.worn_slot is
  'The slot this item is worn in, when its size and kind cannot say: ring, amulet, head. NULL for everything else and that is the honest default - a sword is held rather than worn anywhere in particular, and a container is placed by its kind. Read by slots::admits.';

alter table items drop constraint if exists items_worn_slot_check;
alter table items add constraint items_worn_slot_check
  check (worn_slot is null or worn_slot in ('ring', 'amulet', 'head'));

update objects o
   set slot = case
     when i.armor_category = 'shl' then 'left_hand'
     when i.kind = 'armor'         then 'body'
     else 'right_hand'
   end
  from items i
 where i.key = o.item_key
   and o.equipped
   and o.slot is null;

alter table objects drop column if exists equipped;
