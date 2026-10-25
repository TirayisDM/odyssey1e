//! 245. WHAT A SAVED MAP KEEPS, AND WHAT IT MUST DROP COMING BACK.
//!
//! 243 put a named copy of a map in `map_versions`. Three decisions in
//! that round trip are not plumbing, so they are here with tests
//! rather than in a command - `commands/mod.rs` is explicit that a
//! command wanting a test is a rule wearing a wrapper.
//!
//!   [`name_for`]   whether a name can be asked for later
//!   [`keep_link`]  a door to a place that no longer exists
//!   [`surplus`]    which squares a load has to REMOVE
//!
//! Each one is a question with a wrong answer that looks like an
//! ordinary answer, which is the test for whether something belongs
//! up here.
//!
//! NO SERDE AND NO HTTP. A square crosses the wire as
//! `commands::tiles::Tile`; this module reasons about coordinates and
//! identifiers, so it can be tested without a database and cannot be
//! tempted into fetching anything.

use std::collections::HashSet;

/// Where a square is: x, y, z. The whole of a square's identity within
/// one map, and the key `one_square_one_row` already indexes.
pub type At = (i32, i32, i16);

/// The longest a version name may be.
///
/// The name IS the identity - 243 made it unique per location - and it
/// has to be readable in a dropdown next to Load and Delete. Eighty
/// characters is a sentence; it is a limit on a label, not on prose.
const LONGEST_NAME: usize = 80;

/// A name a version can be asked for by, or why it cannot be.
///
/// 243 put `check (length(btrim(name)) > 0)` on the column, so a blank
/// name is refused either way. **The point of refusing it here is the
/// sentence.** A constraint violation arrives as
/// `a_version_is_named`, and a DM who typed a space deserves to be
/// told they typed a space.
///
/// TRIMMED, NOT MERELY CHECKED. `"cellar "` and `"cellar"` are the
/// same shelf to a person and two different rows to a unique index, so
/// the trim happens before the uniqueness does. Storing the untrimmed
/// version would make "that name is taken" arrive for a name that
/// appears nowhere on screen.
pub fn name_for(raw: &str) -> Result<String, String> {
    let name = raw.trim();
    if name.is_empty() {
        return Err("a saved map needs a name - it is how you ask for it again".to_string());
    }
    // Counted in characters rather than bytes: a name in any script
    // gets the same allowance, and `len()` would refuse a short Greek
    // name while accepting a long English one.
    let n = name.chars().count();
    if n > LONGEST_NAME {
        return Err(format!(
            "that name is {} characters - {} is the most that fits in the list",
            n, LONGEST_NAME
        ));
    }
    Ok(name.to_string())
}

/// A square's door, if the place it opens on still exists.
///
/// 243. **THE ONE DANGEROUS FIELD.** On `tiles`, `leads_to` is a
/// foreign key with `on delete set null`, so deleting a place quietly
/// empties every door that pointed at it. Inside a version's jsonb
/// there is no foreign key and no cascade, so **a version taken last
/// week can name a location that is gone** - and putting it back would
/// violate the key on insert, or, if somebody reached for
/// `on conflict do nothing`, drop the square and report success.
///
/// So the link is checked against the places that exist NOW. A dead
/// one becomes no link, the square itself survives, and the caller
/// counts the drops and says how many - 217.
///
/// WHY NOT REFUSE THE WHOLE LOAD: because the map is the thing being
/// restored and a door is a detail of one square. A room you cannot
/// have back because one doorway lost its destination is a worse
/// answer than a room with one doorway that goes nowhere - which is
/// exactly the state `on delete set null` leaves the live table in
/// anyway.
pub fn keep_link<'a>(link: Option<&'a str>, live: &HashSet<String>) -> Option<&'a str> {
    link.filter(|id| live.contains(*id))
}

/// Which squares a load must REMOVE: the ones on the map now that the
/// version does not contain.
///
/// 245. A LOAD REPLACES, AND REPLACING IS NOT WRITING.
///
/// The version's own squares are written over whatever is there, so
/// they need no deleting. What needs deleting is everything ELSE -
/// otherwise loading a four-square version onto a forty-square map
/// leaves thirty-six squares that no version ever contained, the map
/// looks like neither version, and nothing anywhere said so.
///
/// **THIS IS WHY A LOAD IS NOT AN UPSERT AND STOPS.** The surplus is
/// the whole difference between "load" and "merge", and it is computed
/// rather than assumed because `delete everything first` is the other
/// way to get it and leaves the map EMPTY if the write that follows
/// fails.
///
/// Order is the order the map was read in, so the delete requests go
/// out in index order rather than scattered.
pub fn surplus(current: &[At], version: &[At]) -> Vec<At> {
    let keep: HashSet<At> = version.iter().copied().collect();
    current
        .iter()
        .copied()
        .filter(|at| !keep.contains(at))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn live(ids: &[&str]) -> HashSet<String> {
        ids.iter().map(|s| s.to_string()).collect()
    }

    /* ---------------- name_for ---------------- */

    #[test]
    fn a_name_is_a_name() {
        assert_eq!(name_for("the cellar").unwrap(), "the cellar");
    }

    #[test]
    fn surrounding_space_is_not_part_of_the_name() {
        // Two rows to a unique index, one shelf to a person.
        assert_eq!(name_for("  cellar  ").unwrap(), "cellar");
        assert_eq!(name_for("cellar ").unwrap(), name_for(" cellar").unwrap());
    }

    #[test]
    fn a_blank_name_is_refused_with_a_sentence() {
        let e = name_for("   ").unwrap_err();
        assert!(e.contains("needs a name"), "{}", e);
        assert!(name_for("").is_err());
        assert!(name_for("\t\n").is_err());
    }

    #[test]
    fn inner_space_is_left_alone() {
        assert_eq!(name_for("before the dragons").unwrap(), "before the dragons");
    }

    #[test]
    fn a_name_the_list_cannot_show_is_refused() {
        let long = "a".repeat(LONGEST_NAME + 1);
        let e = name_for(&long).unwrap_err();
        assert!(e.contains("81 characters"), "{}", e);
        assert!(name_for(&"a".repeat(LONGEST_NAME)).is_ok(), "the limit itself is allowed");
    }

    #[test]
    fn the_limit_counts_characters_not_bytes() {
        // Eighty Greek letters is two bytes each. A byte limit would
        // refuse this and accept eighty English ones, which is the
        // same name in two scripts getting two different answers.
        let greek = "λ".repeat(LONGEST_NAME);
        assert_eq!(greek.len(), LONGEST_NAME * 2);
        assert!(name_for(&greek).is_ok());
    }

    /* ---------------- keep_link ---------------- */

    #[test]
    fn a_door_to_a_place_that_exists_survives() {
        let places = live(&["undercroft", "stair-head"]);
        assert_eq!(keep_link(Some("undercroft"), &places), Some("undercroft"));
    }

    #[test]
    fn a_door_to_a_deleted_place_becomes_no_door() {
        // The fault this exists for: an FK violation on restore, or a
        // silently dropped square.
        let places = live(&["undercroft"]);
        assert_eq!(keep_link(Some("the-old-mine"), &places), None);
    }

    #[test]
    fn a_square_with_no_door_is_not_a_problem() {
        assert_eq!(keep_link(None, &live(&["undercroft"])), None);
        assert_eq!(keep_link(None, &live(&[])), None);
    }

    #[test]
    fn nowhere_left_to_go_drops_every_door() {
        assert_eq!(keep_link(Some("undercroft"), &live(&[])), None);
    }

    /* ---------------- surplus ---------------- */

    #[test]
    fn a_smaller_version_removes_what_it_does_not_contain() {
        let current = [(0, 0, 0), (1, 0, 0), (2, 0, 0), (3, 0, 0)];
        let version = [(0, 0, 0), (1, 0, 0)];
        assert_eq!(surplus(&current, &version), vec![(2, 0, 0), (3, 0, 0)]);
    }

    #[test]
    fn a_square_in_both_is_written_over_not_removed() {
        let current = [(0, 0, 0)];
        let version = [(0, 0, 0)];
        assert!(surplus(&current, &version).is_empty());
    }

    #[test]
    fn a_bigger_version_removes_nothing() {
        let current = [(0, 0, 0)];
        let version = [(0, 0, 0), (1, 0, 0), (2, 0, 0)];
        assert!(surplus(&current, &version).is_empty());
    }

    #[test]
    fn an_empty_version_removes_the_whole_map() {
        // 243 allows `[]`: a room with a recipe and nothing drawn in
        // it. Loading it must actually give you that, not the map you
        // were trying to get rid of.
        let current = [(0, 0, 0), (1, 0, 0)];
        assert_eq!(surplus(&current, &[]), vec![(0, 0, 0), (1, 0, 0)]);
    }

    #[test]
    fn an_empty_map_has_no_surplus() {
        assert!(surplus(&[], &[(0, 0, 0)]).is_empty());
        assert!(surplus(&[], &[]).is_empty());
    }

    #[test]
    fn a_level_is_part_of_where_a_square_is() {
        // The same x and y on another floor is ANOTHER SQUARE. A
        // surplus computed on x and y alone would wipe the cellar when
        // you loaded the ground floor.
        let current = [(0, 0, 0), (0, 0, -1), (0, 0, 1)];
        let version = [(0, 0, 0)];
        assert_eq!(surplus(&current, &version), vec![(0, 0, -1), (0, 0, 1)]);
    }

    #[test]
    fn negative_coordinates_are_ordinary() {
        let current = [(-2, -2, 0), (0, 0, 0)];
        let version = [(-2, -2, 0)];
        assert_eq!(surplus(&current, &version), vec![(0, 0, 0)]);
    }

    #[test]
    fn the_map_is_walked_in_the_order_it_arrived() {
        // `list_tiles` reads `order=z,y,x`, which is the index. Keeping
        // that order means the delete requests follow it too.
        let current = [(5, 0, 0), (1, 0, 0), (3, 0, 0)];
        assert_eq!(surplus(&current, &[]), vec![(5, 0, 0), (1, 0, 0), (3, 0, 0)]);
    }

    #[test]
    fn a_duplicate_in_the_map_is_reported_as_many_times_as_it_appears() {
        // `one_square_one_row` makes this impossible in the database,
        // so this records what the function does rather than asking
        // for it: it filters, it does not de-duplicate. A caller that
        // invents duplicates gets them back.
        let current = [(0, 0, 0), (0, 0, 0)];
        assert_eq!(surplus(&current, &[]), vec![(0, 0, 0), (0, 0, 0)]);
    }
}
