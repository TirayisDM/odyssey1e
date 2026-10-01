-- 074. SOMETHING TO PLAY IT ON.
--
-- Nine instruments, and a new `kind` to hold them.
--
-- A KIND RATHER THAN A PROPERTY, because an instrument is a category
-- of thing the way a weapon is, not a trait a thing has. The perform
-- screen has to ask "what can this bard play" and that is a question
-- about kind - `items_kind_idx` has existed since 008 and already
-- answers it. A property would have meant scanning every item a
-- character owns and filtering in Rust.
--
-- NOTHING BREAKS ON AN UNFAMILIAR KIND, which is why this is cheap.
-- `equipment::is_proficient` ends in a catch-all, the sheet's three
-- inventory divisions define the third as the LEFTOVERS rather than as
-- a list, and `armor_class` reads `kind == "armor"`. A lute lands in
-- General, grants no AC, and is carried like anything else. 075 is
-- what then teaches proficiency to notice it.
--
-- `foc` ON EVERY ONE OF THEM. That is 5e as written - a bard may use
-- an instrument as a spellcasting focus - and it is the "channel or
-- totem" half of what these are for. HONEST CAVEAT: nothing reads
-- `foc` today, because nothing in this engine casts anything. The
-- `spells` table has been seeded since 006 and no Rust touches it. The
-- property is here so the catalogue is not lying about what a lute is,
-- the same way 027 stored versatile dice four migrations before
-- anything could roll them.
--
-- STATS DELIBERATELY FLAT. 5e prices these from 2 gp to 35 gp and
-- weighs them from 1 lb to 10 lb, and the spread buys nothing: no rule
-- anywhere distinguishes a drum from a viol. Dave's instruction was
-- "simple basic instruments, similar stats", so they sit in a narrow
-- band - 1 to 5 lb, 1 to 35 gp - close enough to the book to be
-- recognisable and uniform enough that choosing one is a character
-- decision rather than an optimisation.
--
-- Four of these are not in the PHB list: harp, mouth harp, cymbals and
-- tambourine. They break nothing - an instrument is an instrument to
-- every rule here - and the campaign asked for them.

-- THE CONSTRAINT CAUGHT THIS, which is the point of it. 008 closed
-- `kind` to six values and the first insert below was refused outright
-- rather than quietly creating a seventh category nothing would ever
-- look for. Widening it deliberately is the whole difference between a
-- new kind and a typo.
alter table items drop constraint if exists items_kind_check;
alter table items add constraint items_kind_check
  check (kind in ('weapon', 'armor', 'equipment', 'consumable',
                  'loot', 'container', 'instrument'));

insert into items
  (key, name, kind, size, weight, price, denom, slots, properties, description)
values
  ('flute', 'Flute', 'instrument', 'tiny', 1, 2, 'gp', 0.5, array['foc'],
   'A slim tube of boxwood with six finger holes, carried crosswise in a belt sheath. The first instrument most players learn and the easiest to keep dry.'),

  ('mouth_harp', 'Mouth Harp', 'instrument', 'tiny', 1, 1, 'gp', 0.25, array['foc'],
   'A small iron frame with a sprung tongue, held against the teeth and struck with a finger. The note never changes; everything is done with the shape of the mouth around it.'),

  ('tambourine', 'Tambourine', 'instrument', 'tiny', 1, 3, 'gp', 0.5, array['foc'],
   'A shallow hoop of bent ash with a skin head and loose pairs of jingles set into the rim. Struck, shaken or brushed, and audible over a crowd that has stopped listening.'),

  ('pipes', 'Pipes', 'instrument', 'sm', 2, 12, 'gp', 1, array['foc'],
   'Seven reeds of falling length bound side by side with waxed cord. Breath across the tops rather than into them, which takes a season to learn and a lifetime to do well.'),

  ('horn', 'Horn', 'instrument', 'sm', 2, 3, 'gp', 1, array['foc'],
   'A curved ox horn, hollowed and polished, with a cut mouthpiece. Few notes and all of them carry - as much a signal as a song.'),

  ('drum', 'Drum', 'instrument', 'sm', 3, 6, 'gp', 1, array['foc'],
   'A hide head laced over a shallow wooden shell, slung at the hip and played with a short beater. It sets a marching pace as readily as a dance.'),

  ('cymbals', 'Cymbals', 'instrument', 'sm', 3, 5, 'gp', 1, array['foc'],
   'A pair of hammered bronze discs with leather thongs through the boss. One note, held as long as the ringing lasts, and no way at all to play one quietly.'),

  ('lute', 'Lute', 'instrument', 'med', 2, 35, 'gp', 2, array['foc'],
   'A pear-shaped body of thin staves under a spruce face, with a neck that angles sharply back. Light to the point of fragile, expensive to replace, and the instrument a bard is expected to own.'),

  ('harp', 'Harp', 'instrument', 'med', 5, 30, 'gp', 2, array['foc'],
   'A lap harp of perhaps thirty gut strings in a carved willow frame. Heavy for its size, slow to tune, and the sound a room goes quiet for.')
on conflict do nothing;
