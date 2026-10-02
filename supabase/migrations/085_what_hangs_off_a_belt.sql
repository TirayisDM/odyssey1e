-- 085. WHAT HANGS OFF A BELT, AND WHAT IS WORN.
--
-- The five items 084's ladder needs and the catalogue did not have.
--
-- THE SHEATH NEEDED NO NEW MACHINERY. "A hip option for put-in for
-- small weapons" is a container with `holds_size = 'sm'`, and both
-- halves of that have been in `items` since 032 and 036. `put_in`
-- works on it unchanged; `containers::admits_size` already refuses a
-- greatsword. The only new thing is the row.
--
-- RING, AMULET AND HELM CARRY `worn_slot`, because they are the three
-- the ladder cannot deduce - all small pieces of equipment, with
-- nothing about their size or kind to tell one from another.
--
-- GENERIC ON PURPOSE. A plain ring and a plain amulet, so a DM can
-- name one "Ring of Protection" with 049's override and have it be
-- that. Seeding a dozen magic items would be guessing at a campaign.

insert into items
  (key, name, kind, size, weight, price, denom, slots,
   holds_size, capacity_slots, worn_slot, description)
values
  ('hip_pouch', 'Hip Pouch', 'container', 'sm', 1, 5, 'sp', 1,
   'tiny', 3, null,
   'A stiffened leather pouch on a belt loop, flapped and buckled. Holds what you want to reach without stopping.'),

  ('sheath', 'Sheath', 'container', 'sm', 0.5, 2, 'sp', 0.5,
   'sm', 1, null,
   'A fitted leather scabbard on a belt hanger, stitched to one blade and useless to another. Keeps an edge off your leg and out of the weather.'),

  ('ring', 'Ring', 'equipment', 'tiny', 0, 5, 'gp', 0.1,
   null, null, 'ring',
   'A plain band, worn on a finger. What it does, if anything, is not written on it.'),

  ('amulet', 'Amulet', 'equipment', 'tiny', 1, 5, 'gp', 0.2,
   null, null, 'amulet',
   'A disc or token on a cord, worn at the throat. A symbol to some, an ornament to others, and occasionally neither.'),

  ('helm', 'Helm', 'equipment', 'sm', 4, 10, 'gp', 1,
   null, null, 'head',
   'A shaped steel cap with a padded liner and a chin strap. It does not change what a blade does to the rest of you.')
on conflict do nothing;
