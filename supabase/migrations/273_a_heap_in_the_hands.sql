/* ===================================================================
   273. A HEAP IN THE HANDS
   ===================================================================

   Dave: "hauling next - build it".

   271 built the pour and said out loud what it was for: hauling is the
   only caller that can overflow a ring, and it is the one that makes
   the pour worth having - a collapse never fills its own square twice,
   but a party moving a wall across a room does it all day.

   THE CONVERSION, AND IT IS NEVER A COPY. A heap is terrain while it
   is on the ground and an object while it is in somebody's hands, and
   it is never both at once: `take_rubble` subtracts from the square's
   cover and inserts the object in the same breath, `tip_rubble` does
   it the other way. Two homes for one quantity would be the
   two-places problem with a barrow in it.

   AND THE ITEM KEY IS THE TERRAIN WORD, unchanged. `rubble` on the
   floor and `rubble` in the pack are the same string, so nothing
   anywhere has to translate between a map vocabulary and a catalogue
   one. The drift guard in terrain.rs - the_weights_and_the_migration
   _cannot_drift - checks that every debris word has a row here, which
   is what makes a fifth kind of rubble fail loudly rather than
   silently refuse to be picked up.

   WHY THESE ROWS CARRY A WEIGHT PER HIT POINT. `objects.quantity` is
   the heap's HIT POINTS, because that is what the whole of 262-271
   counts in, and `holders::stack_weight` already multiplies a unit
   weight by a quantity for every arrow and ration in the game. So a
   heap of fifteen rubble weighs 500 lb on the carry line through
   machinery that was already there, with no new code path and no
   special case for "but it is terrain". That is the entire reason the
   number here is 33.3333 and not 500.

   KIND IS 'loot', NOT A NEW WORD. Nothing in Rust branches on `kind`;
   it is the catalogue's shelf label, and 'loot' is its none-of-the-
   above. A tenth kind that the whole catalogue would have to learn
   for four rows of broken stone is not worth the restated CHECK.

   =================================================================== */

/* ---------- THE FOUR HEAPS ----------

   POUNDS PER HIT POINT, derived from terrain::Material::weight - a
   five-foot span's pounds over a five-foot span's hit points:

     stone     2000 / 60 = 33.3333    a heap of 15 is 500 lb
     timber     800 / 40 = 20         a heap of 10 is 200 lb
     wood       400 / 30 = 13.3333    a heap of  7 is  93 lb
     loose     1200 / 20 = 60         a heap of  5 is 300 lb

   THE COLUMN ORDER IS PART OF THE CONTRACT. `weighed_words` in
   terrain.rs reads the fourth value of each row, so this insert names
   its columns in that order on purpose and the whole of each row's
   first four fields stays on one line.

   AND LOOSE IS HEAVIER PER POINT THAN STONE, which looks wrong and is
   not: hit points are how hard a thing is to shift and pounds are how
   much of it there is. A barrow of spoil is twice a barrow of masonry
   and a quarter of the work.                                        */

/* ---------- FIRST: `rubble` WAS ALREADY TAKEN ----------

   AND THIS IS THE CYCLE'S OWN NEAR-MISS, CAUGHT BY LOOKING. 218 built
   searchable scenery and gave one of its fixtures the key `rubble` - a
   pile of fallen stone with twenty slots to rummage through - eight
   cycles before 226 made `rubble` a word in the terrain vocabulary.
   Two meanings, one key.

   The first version of this file inserted over it with `on conflict do
   nothing`, which did exactly what it says: nothing. A heap of masonry
   in a pack would have come back weighing NOTHING, encumbering nobody,
   and looking precisely like an ordinary inventory row - and the drift
   guard in terrain.rs would have stayed green, because it reads this
   FILE and not the table. A fault wearing the costume of an ordinary
   answer, in the one cycle written to prevent them.

   THE TERRAIN WORD WINS, and not by seniority. `rubble` is named in
   three CHECK constraints over `tiles` - 226, 263, 265 - and in every
   row of every map anybody has painted; the fixture is one catalogue
   row with no objects of it anywhere. So the scenery is renamed and
   keeps its name on screen.

   AND THE SWEEP AT THE BOTTOM IS THE REAL FIX. A rename closes this
   collision; the `do $check$` block closes the NEXT one, because a
   swallowed row now fails the migration instead of shipping quietly. */

update public.items
   set key = 'rubble_pile'
 where key = 'rubble'
   and game_id is null
   and kind = 'fixture';

comment on column public.items.key is
  'Stable slug, unique per partition. 273: it is also the shared vocabulary between the map and the catalogue - the four debris rows carry the same keys terrain::Terrain does, so nothing translates between a map word and an item key. That is why 218''s rubble fixture became rubble_pile: one key cannot mean both a heap you can carry and a heap you can search.';

/* ---------- THEN: the four heaps ---------- */

insert into public.items (key, name, kind, weight, description) values
  ('rubble', 'Broken Stone', 'loot', 33.3333,
   'Masonry and bedrock, broken up. Quantity is HIT POINTS of debris, not a count of lumps - fifteen is one square''s worth and weighs 500 lb. Four of them raise a wall.'),
  ('rubble_timber', 'Splintered Timber', 'loot', 20,
   'Beams and standing timber, broken up. Ten is one square''s worth, 200 lb.'),
  ('rubble_wood', 'Broken Boards', 'loot', 13.3333,
   'Planks and boards, broken up. Seven is one square''s worth - a quarter of thirty rounded down, which is why a heap of boards is 93 lb rather than 100.'),
  ('rubble_earth', 'Spoil', 'loot', 60,
   'Dug earth, sand, scree. Five is one square''s worth and weighs 300 lb: the lightest heap to break and the heaviest to carry. Raises nothing - no wall in the vocabulary is made of loose earth - so backfill is the only thing to do with it.')
on conflict do nothing;

/* ---------- AND PROVE IT, BEFORE THIS COMMITS ----------

   218'S HABIT, AND THIS FILE IS THE ARGUMENT FOR IT. `on conflict do
   nothing` is kept so the migration can be replayed, which means it
   can still swallow a row - so the swallowing is what gets checked.
   Every debris word must end up with exactly one global row, of the
   right kind, carrying a weight. The numbers themselves are checked
   against the Rust table by terrain.rs's
   the_weights_and_the_migration_cannot_drift; what cannot be checked
   from there is whether the row arrived at all.                     */

do $check$
declare
  n int;
  bad text;
begin
  select count(*) into n
    from public.items
   where game_id is null
     and key in ('rubble', 'rubble_timber', 'rubble_wood', 'rubble_earth');
  if n <> 4 then
    raise exception 'expected four debris rows, found %', n;
  end if;

  select string_agg(key || ' (' || kind || ', weight ' ||
                    coalesce(weight::text, 'NULL') || ')', ', ')
    into bad
    from public.items
   where game_id is null
     and key in ('rubble', 'rubble_timber', 'rubble_wood', 'rubble_earth')
     and (kind <> 'loot' or weight is null or weight <= 0);
  if bad is not null then
    raise exception 'a heap that cannot be carried or weighed: %', bad;
  end if;

  -- AND THE FIXTURE IS STILL THERE UNDER ITS NEW KEY. A rename that
  -- lost the scenery would be a different silent fault.
  select count(*) into n
    from public.items
   where game_id is null and key = 'rubble_pile' and kind = 'fixture';
  if n <> 1 then
    raise exception '218''s rubble fixture went missing in the rename';
  end if;
end
$check$;

comment on column public.items.weight is
  'Pounds for ONE of them. 273: for the four debris rows this is pounds per HIT POINT, because objects.quantity holds a heap''s hit points and holders::stack_weight multiplies the two - which is how a heap encumbers through the same arithmetic as a quiver of arrows. The span weights are a RULE and live in terrain::Material::weight; these four numbers are the copy Postgres needs, and terrain.rs checks them with the_weights_and_the_migration_cannot_drift rather than trusting them.';

/* ---------- WHAT IS DELIBERATELY NOT HERE ----------

   NO HIT POINTS AND NO ARMOUR CLASS ON THE OBJECT HALF. Dave asked
   whether rubble has them and the answer is that `objects` has neither
   column for anything - not a sword, not a door, not a cart. Giving
   four rows of debris their own would be inventing "objects have hit
   points" sideways, in the one cycle least able to think it through.
   On the ground a heap has both, because there it is terrain and 264
   gave it them. In the hands it is a weight and a quantity, which is
   all a pack has ever held.

   NO `bites`. A heap is not a tool. It is what a tool is pointed at,
   and 270's column says which tool - asked of the heap already, by
   `build_square`, because spending debris is shovel work whatever the
   debris used to be.

   NO WEIGHT FOR ICE OR IRON OR GREENERY ROWS, because there are no
   such heaps: ice shatters, a grate is cut out, brambles are cleared.
   Those three materials have span weights in Rust and nothing in the
   vocabulary turns them into something you could lift. That is a
   table being complete rather than a gap.

   NO CART, NO BARROW, NO SLED. Everything here is one body's hands,
   and a vehicle that multiplies what a body can carry is a thing with
   a capacity, a speed and somewhere to be parked. It is a cycle, and
   it is the obvious next one.                                       */
