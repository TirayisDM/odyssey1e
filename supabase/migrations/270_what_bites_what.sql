-- 270. WHAT BITES WHAT
--
-- Dave: "You need shovels, picks, digging bars, chisels hammers to dig
-- (or magic or creature features)". 269 gave work a price in hours and
-- left a hole it named out loud: bare hands dug stone. This closes it.
--
-- THE CATALOGUE HAD NO TOOLS AT ALL. 115 weapons, 13 armour, 9
-- instruments, 9 pieces of equipment that are lanterns and rope - not
-- one shovel, pick, crowbar or mason's kit. A party could fell a forest
-- with a battleaxe and could not dig a hole, because nothing in the game
-- could scoop. That asymmetry is the shape of this whole migration.
--
-- THE RULE IS NOT "TOOL / NO TOOL", it is what will bite what, and that
-- is a fact about the MATERIAL - so it lives in terrain.rs beside `ac`,
-- `hp` and `threshold`. Three words cover the table: a point for stone,
-- ice and iron; an edge for timber, wood and growing things; a scoop for
-- earth, for fill, and for anything already broken into debris.
--
-- WHAT IS HERE IS THE OTHER HALF: which THINGS provide which bite. That
-- is a fact about an item and belongs on the catalogue, the way
-- `weapon_class` and `armor_category` do.

/* ---------- ONE: a tool is a kind of thing ---------- */

-- 074 and 218 both widened this list; 115 says an applied migration is a
-- record rather than a draft, so the list is restated whole rather than
-- edited where it was first written.
alter table public.items drop constraint items_kind_check;
alter table public.items add constraint items_kind_check
  check (kind = any (array['weapon', 'armor', 'equipment', 'consumable',
                           'loot', 'container', 'instrument', 'fixture',
                           'tool']));

/* ---------- TWO: what it bites ---------- */

alter table public.items add column if not exists bites text[];

-- AN ARRAY, LIKE `damage_types` AND `properties` BESIDE IT, because a
-- mattock genuinely IS a pick and a shovel on one handle and forcing it
-- to pick a side would be a lie about a real tool. Refusing against a
-- list costs the same line as refusing against a value.
alter table public.items
  add constraint a_bite_is_a_word_the_rules_know
  check (bites is null or bites <@ array['point', 'edge', 'scoop']);

comment on column public.items.bites is
  'Which bites this thing provides, from terrain::Bite - point, edge, scoop. The vocabulary is a RULE and lives in Rust; this is the copy Postgres needs to refuse nonsense, and terrain.rs checks it with the_bite_words_and_the_migration_cannot_drift rather than trusting it. NULL means this thing will not work any material - which is almost everything, a lantern included. NOT restricted to kind=tool on purpose: a warhammer is a point and a greataxe is an edge, so a party already carrying the right weapon can dig and build with no special case anywhere.';

/* ---------- THREE: the things themselves ---------- */

-- GLOBAL ROWS, game_id null. Prices and weights are the book's where the
-- book has them and the nearest neighbour's where it does not.
insert into public.items (key, name, kind, price, denom, weight, bites, description) values
  ('shovel', 'Shovel', 'tool', 2, 'gp', 5, array['scoop'],
   'A long-handled digging shovel. THE MOST IMPORTANT ROW IN THIS FILE: before it existed nothing in the game could scoop, so no earth could be dug, no pit filled and no debris shifted.'),
  ('spade', 'Spade', 'tool', 2, 'gp', 5, array['scoop'],
   'Shorter and squarer than a shovel, for cutting turf and squaring a trench.'),
  ('miners_pick', 'Miner''s Pick', 'tool', 2, 'gp', 10, array['point'],
   'A two-handed pick for rock. What Dave''s five hours through five feet of stone were measured with.'),
  ('mattock', 'Mattock', 'tool', 3, 'gp', 10, array['point', 'scoop'],
   'A pick on one side and an adze on the other. The reason bites is an array.'),
  ('digging_bar', 'Digging Bar', 'tool', 2, 'gp', 15, array['point'],
   'A solid iron bar for levering and for breaking what a pick glances off.'),
  ('crowbar', 'Crowbar', 'tool', 2, 'gp', 5, array['point'],
   'Pry and lever. Shorter than a digging bar and worse at rock, which the rules do not yet distinguish.'),
  ('sledgehammer', 'Sledgehammer', 'tool', 2, 'gp', 10, array['point'],
   'Two-handed, for breaking masonry.'),
  ('masons_tools', 'Mason''s Tools', 'tool', 10, 'gp', 8, array['point'],
   'Trowel, hammer, chisels, brushes. The kit Dave named as hammers and chisels.'),
  ('carpenters_tools', 'Carpenter''s Tools', 'tool', 8, 'gp', 6, array['edge'],
   'Saw, hammer, chisels, adze, square.'),
  ('woodcutters_axe', 'Woodcutter''s Axe', 'tool', 1, 'gp', 7, array['edge'],
   'A felling axe. Not a weapon - the handaxe and battleaxe rows are the weapons, and they bite the same.'),
  ('handsaw', 'Handsaw', 'tool', 1, 'gp', 2, array['edge'],
   'For planks and limbs rather than trunks.'),
  ('billhook', 'Billhook', 'tool', 1, 'gp', 3, array['edge'],
   'A hooked blade on a handle, for brush and hedging. Brambles come up by hand under Dave''s ruling; this makes it pleasanter, not faster.');

/* ---------- FOUR: the weapons that were already the right shape ---------- */

-- TWELVE ROWS THE EXPORT ALREADY HAD. A warhammer is a point and a
-- greataxe is an edge, and saying so here is why no command needs a
-- special case for "but they are holding a weapon".
update public.items set bites = array['point']
  where game_id is null
    and key in ('war_pick', 'horsemans_pick', 'warhammer', 'light_hammer');

update public.items set bites = array['edge']
  where game_id is null
    and key in ('battleaxe', 'greataxe', 'handaxe', 'bearded_axe',
                'bardiche', 'sickle', 'war_scythe');

/* ---------- FIVE: creature features, which needed no new mechanism ---------- */

-- DAVE NAMED THESE AND THIS PROJECT HAD ALREADY SOLVED THEM. A claw is
-- an `items` row here - `claws`, `bite`, `talons`, `tusks`, `slam` - so
-- a burrowing creature satisfies the rule through the same column a
-- shovel does, with nothing written twice.
--
-- CONSERVATIVE ON PURPOSE. Digging is near-universal among clawed and
-- tusked animals, so those get a scoop. Teeth do NOT get an edge: a
-- beaver felling a timber wall is a beaver speciality and not a property
-- of jaws in general, and a wolf gnawing down a palisade in three hours
-- twenty is exactly the ordinary-looking wrong answer this project keeps
-- hunting. A creature that really does chew through beams wants its own
-- row, not a loosened rule.
update public.items set bites = array['scoop']
  where game_id is null and key in ('claws', 'tusks');

update public.items set bites = array['point']
  where game_id is null and key = 'slam';

/* ---------- WHAT IS DELIBERATELY NOT HERE ----------

   MAGIC. Dave listed it beside tools, and it does not belong beside
   them: `mold earth` and `stone shape` do not help you dig, they remove
   the stone outright. Pricing a spell at five minutes a hit point so it
   could pose as a shovel would be a fault wearing the costume of an
   ordinary answer. Terrain-moving spells want their own cycle and their
   own verb.

   PROFICIENCY. 075 trains a character on an instrument by name and this
   does not join that list. Nobody needs a lesson to use a shovel, and
   269 has no roll in it anywhere.

   A TOOL FOR EACH WORKER. 269's helpers are a NUMBER, not characters, so
   only the named worker's kit can be checked and a crew of three shares
   one pick. Making helpers real characters is its own cycle - and it is
   the same thing transport will want when a cart has to belong to
   somebody.                                                          */
