/* ===================================================================
   288. A MINER IS NOT A GIANT
   ===================================================================

   Dave, reading Tarren's sheet: *"he is listed as an Untgar but it
   looks like some of his attributes and species traits are coming
   from Untgaroth"*.

   ONE FIELD, AND HE FOUND IT FROM THE OUTSIDE. Everything else on
   that sheet is right. Tarren's row says `species_key: untgar`,
   `size: med`, `creature_type: null`, and all six stored scores are
   flat 10s - so the 12 Constitution and 11 Intelligence the sheet
   shows are Unt'gar's +2/+1 being applied exactly as 056 designed.
   The Unt'gar species row matches Dave's own species document field
   for field: Con +2 / Int +1, Medium, 25 feet, History, four to five
   feet, Darkvision 120, Mineral Sense, Stonecunning, Trade Savvy,
   Environmental Resilience, Common / Hågenfolk / one other, mature at
   25 and live to 500, lawful neutral. Nothing is borrowed. The
   lookup cannot borrow anything either - `load_species_map` asks for
   the key exactly and both screens match on `key ===`.

   Except `creature_type`, which said `giant`. The same word as the
   Unt'garoth, on a four-foot Hågenfolk miner, printed into the sheet
   header beside the name: "Tarren - giant". The Fjell'gar, in the
   same height band, are `humanoid`.

   THE WORSE HALF IS THAT NO MIGRATION EVER WROTE THIS COLUMN. 122
   added `creature_type` to `species`, `characters` and `npcs`, and
   set a value on exactly one of them - `npcs`, to humanoid. Species
   got the column, the check constraint and the comment, and not one
   row. So all four live values were typed straight into the
   database: a rebuild from these migrations produces four NULLs, the
   repo has never recorded what any people IS, and the wrong word sat
   in the one place nobody reviews. That is the fault worth fixing.
   The typo is a consequence of it.

   IT IS NOT COSMETIC. `creature::is_creature` files anything that is
   not humanoid-or-null as a creature rather than a person - 140's
   line between a merchant and a goblin. Tarren escapes it today only
   because `who_is_where` reads `characters.creature_type`, which is
   null, and not the species behind it. The moment anything stamps a
   resolved type onto the row, an Unt'gar player character lands on
   the monster list.

   THE UNT'GAROTH KEEP `giant`, AND THAT IS DAVE'S CALL, not an
   inheritance: *"Unt'gar to humanoid but Untgaroth is a Large
   creature"*. Their document calls them towering humanoids and the
   mechanics say otherwise - Large, Powerful Build, forged by the
   Mithar'itra as labourers - so the word stands on purpose now
   rather than by accident.

   ALL FOUR ARE WRITTEN, not just the broken one, because the point
   is the record and not the repair. `where game_id is null` so a
   game's own override is never touched; keyed by slug so it is
   idempotent and a rebuilt database lands on the same four rows.
   =================================================================== */

update public.species set creature_type = 'humanoid'
 where key = 'untgar'    and game_id is null;

update public.species set creature_type = 'humanoid'
 where key = 'fjellgar'  and game_id is null;

update public.species set creature_type = 'humanoid'
 where key = 'nyook'     and game_id is null;

update public.species set creature_type = 'giant'
 where key = 'untgaroth' and game_id is null;

/* A PEOPLE HAS TO SAY WHAT IT IS. 122 left NULL meaning "nobody has
   stated it", which is the right answer for a character and the wrong
   one for a species: `collapse` replaces a whole row, so a people
   with no type hands every character of it an unstated type, and
   `is_creature` then reads them as people by default rather than by
   decision. There are four peoples and a fifth is a seed away, so
   the cheapest guard is the one that fails the migration that forgets
   - the same argument as `security_doors()`, one layer down. */
do $$
declare
  missing text;
begin
  select string_agg(key, ', ' order by key) into missing
    from public.species
   where creature_type is null;

  if missing is not null then
    raise exception
      'species with no creature_type: %. Every people states what '
      'it is - see 288.', missing;
  end if;
end $$;
