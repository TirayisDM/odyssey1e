//! Where places are in relation to each other.
//!
//! 033 stores one fact about the hierarchy - `parent_id` - and nothing
//! else. No level, no path, on the grounds that a stored depth is one
//! reparent away from lying. This is where the rest is worked out.
//!
//! THE PRIOR ART IS THE ARGUMENT FOR DOING IT HERE. OdysseyAIRPG's scene
//! schema materialises `hierarchy.level` and `hierarchy.path`, and
//! `linkScenes` writes both on every attach. Its 56 live scenes already
//! disagree with the convention its own comment states. Deriving is not
//! a refinement of that design, it is the lesson from it.
//!
//! WHY NOT A RECURSIVE CTE. It would work, and Postgres would do it
//! well. But the answer is a shape for a screen rather than a fact about
//! the world, the input is a few dozen rows, and doing it here means it
//! is a pure function with tests instead of a query nobody can exercise
//! without a database. Same reason `collapse_overrides` is in Rust.
//!
//! A TREE THAT DOES NOT TRUST ITS INPUT. Postgres refuses a cycle -
//! `no_location_cycles` is a trigger on the table - so `arrange` should
//! never meet one. It handles it anyway, because a function that hangs
//! on bad input is worse than one that drops a row, and "impossible"
//! arguments have a way of being about the version of the schema you
//! were looking at when you wrote them.

use serde::{Deserialize, Serialize};

/// A location as the database holds it: one parent, no depth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Place {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub kind: String,
}

/// A location as a screen wants it: in order, with its depth and the
/// road to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Placed {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub kind: String,
    /// Zero at the top of its own tree. Derived, never stored.
    pub depth: i64,
    /// Breadcrumb, root first, including this one.
    pub path: Vec<String>,
}

impl Placed {
    /// "Tirayis > Dunstgja > Frostvalley Inn". For a label, a log line,
    /// or anywhere the shape matters more than the nesting.
    ///
    /// Only the tests call it today - the frontend joins `path` itself,
    /// because a screen wants to choose its own separator. Kept because
    /// it is the one place the written form is stated, and the tests
    /// assert against it rather than against a literal they would have
    /// to keep in step. Same treatment Mode::as_str gets, for the same
    /// reason.
    #[allow(dead_code)]
    pub fn trail(&self) -> String {
        self.path.join(" > ")
    }
}

/// Arrange flat rows into depth-first order, working out depth and path.
///
/// ROOTS ARE NOT ONLY THE PARENTLESS ONES. A row whose parent is missing
/// from the input is a root too - not an error and not a dropped row.
/// That happens legitimately: a player reads the locations they can see,
/// and the room they are standing in may sit inside a building the DM
/// has not revealed. Treating it as an orphan to discard would hide the
/// room; treating it as a root shows it without claiming to know what
/// contains it.
///
/// Siblings come out ordered by name, so the same input always gives the
/// same screen.
pub fn arrange(rows: &[Place]) -> Vec<Placed> {
    let mut out = Vec::with_capacity(rows.len());
    let known: std::collections::HashSet<&str> =
        rows.iter().map(|p| p.id.as_str()).collect();

    // A parent nobody in this set has is a root here, whatever the
    // database thinks. See the note above.
    let mut roots: Vec<&Place> = rows
        .iter()
        .filter(|p| match &p.parent_id {
            None => true,
            Some(pid) => !known.contains(pid.as_str()),
        })
        .collect();
    roots.sort_by(|a, b| a.name.cmp(&b.name));

    for root in roots {
        walk(rows, root, 0, &[], &mut out);
    }

    // A row the walk never reached is in a cycle: every member of one
    // has a parent inside the set, so none of them is a root. The
    // trigger makes that impossible, and it is still better to show the
    // row flat than to lose it or to loop looking for its parent.
    if out.len() < rows.len() {
        let seen: std::collections::HashSet<&str> =
            out.iter().map(|p| p.id.as_str()).collect();
        let mut stranded: Vec<&Place> = rows
            .iter()
            .filter(|p| !seen.contains(p.id.as_str()))
            .collect();
        stranded.sort_by(|a, b| a.name.cmp(&b.name));
        for p in stranded {
            out.push(placed(p, 0, &[]));
        }
    }

    out
}

fn placed(p: &Place, depth: i64, trail: &[String]) -> Placed {
    let mut path = trail.to_vec();
    path.push(p.name.clone());
    Placed {
        id: p.id.clone(),
        parent_id: p.parent_id.clone(),
        name: p.name.clone(),
        kind: p.kind.clone(),
        depth,
        path,
    }
}

fn walk(rows: &[Place], node: &Place, depth: i64, trail: &[String], out: &mut Vec<Placed>) {
    // Depth-capped for the same reason holder_character is: a guard
    // against a shape the trigger already forbids, not a rule about how
    // deep a world may go. 033 allows 32.
    if depth > 32 {
        return;
    }
    let here = placed(node, depth, trail);
    let trail_below = here.path.clone();
    out.push(here);

    let mut kids: Vec<&Place> = rows
        .iter()
        .filter(|p| p.parent_id.as_deref() == Some(node.id.as_str()))
        .collect();
    kids.sort_by(|a, b| a.name.cmp(&b.name));

    for k in kids {
        walk(rows, k, depth + 1, &trail_below, out);
    }
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: &str, parent: Option<&str>, name: &str) -> Place {
        Place {
            id: id.into(),
            parent_id: parent.map(str::to_string),
            name: name.into(),
            kind: "structure".into(),
        }
    }

    /// The worked example from 033, five deep.
    fn world() -> Vec<Place> {
        vec![
            p("r", None, "Tirayis"),
            p("g", Some("r"), "Dunstgja"),
            p("i", Some("g"), "Frostvalley Inn"),
            p("f", Some("i"), "Pub Floor"),
            p("c", Some("f"), "Broom Cupboard"),
        ]
    }

    #[test]
    fn depth_counts_from_the_root() {
        let out = arrange(&world());
        let depths: Vec<i64> = out.iter().map(|p| p.depth).collect();
        assert_eq!(depths, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn the_path_is_the_road_to_it() {
        let out = arrange(&world());
        assert_eq!(out[4].trail(), "Tirayis > Dunstgja > Frostvalley Inn > Pub Floor > Broom Cupboard");
        assert_eq!(out[0].trail(), "Tirayis");
    }

    #[test]
    fn a_path_includes_the_place_itself() {
        // Off by one here would make every breadcrumb name its parent.
        let out = arrange(&world());
        assert_eq!(out[2].path.last().map(String::as_str), Some("Frostvalley Inn"));
        assert_eq!(out[2].path.len(), 3);
    }

    #[test]
    fn children_come_under_their_parent_in_order() {
        let rows = vec![
            p("i", None, "Inn"),
            p("z", Some("i"), "Cellar"),
            p("a", Some("i"), "Attic"),
            p("m", Some("i"), "Bar"),
        ];
        let out = arrange(&rows);
        let names: Vec<&str> = out.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["Inn", "Attic", "Bar", "Cellar"]);
    }

    #[test]
    fn separate_trees_both_appear_and_are_ordered() {
        let rows = vec![
            p("b", None, "Beneath"),
            p("a", None, "Above"),
            p("a1", Some("a"), "Roof"),
        ];
        let out = arrange(&rows);
        let names: Vec<&str> = out.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["Above", "Roof", "Beneath"]);
    }

    #[test]
    fn a_parent_outside_the_set_makes_a_root_rather_than_an_orphan() {
        // A player sees the room they are in without seeing the keep it
        // sits inside. Dropping it would hide the room they are standing
        // in; showing it flat is honest about what is known.
        let rows = vec![p("room", Some("unseen-keep"), "Guard Room")];
        let out = arrange(&rows);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].depth, 0);
        assert_eq!(out[0].trail(), "Guard Room");
        // The link is still reported, so a caller can tell the difference
        // between "top of the world" and "parent not visible to you".
        assert_eq!(out[0].parent_id.as_deref(), Some("unseen-keep"));
    }

    #[test]
    fn nothing_is_lost_however_odd_the_input() {
        let rows = world();
        assert_eq!(arrange(&rows).len(), rows.len());
    }

    #[test]
    fn a_cycle_is_shown_flat_rather_than_hung_on() {
        // no_location_cycles makes this unreachable through the app. If
        // it ever arrives anyway, every row still comes out exactly once
        // and the function returns.
        let rows = vec![
            p("a", Some("b"), "A"),
            p("b", Some("a"), "B"),
            p("ok", None, "Ordinary"),
        ];
        let out = arrange(&rows);
        assert_eq!(out.len(), 3);
        let names: Vec<&str> = out.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"A") && names.contains(&"B") && names.contains(&"Ordinary"));
    }

    #[test]
    fn an_empty_world_is_an_empty_list() {
        assert!(arrange(&[]).is_empty());
    }

    #[test]
    fn one_place_is_a_root_at_depth_zero() {
        let out = arrange(&[p("only", None, "The Void")]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].depth, 0);
        assert_eq!(out[0].path, vec!["The Void".to_string()]);
    }
}
