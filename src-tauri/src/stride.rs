//! ONE STEP, AND WHAT IT COSTS.
//!
//! 230. The smallest piece of movement there is: I am standing here, I
//! want that square next to me, may I and what does it cost.
//!
//! IT IS DELIBERATELY ONE STEP. No budget - how much movement is left
//! is a creature's business, not a square's. No path - a route is many
//! of these and wants its own module. No falling damage - refusing a
//! long drop is honest, and working out what it costs you is a rule
//! about bodies rather than ground. No `leads_to` - leaving a location
//! is a different question from crossing one.
//!
//! This is the module four terrain words have been owing their meaning
//! to since 226: `Ramp` and `Stairs` do nothing until something asks
//! what a change of height costs, and `Pit` and `Ledge` do nothing
//! until something falls.
//!
//! ---------------------------------------------------------------------
//! A CLIMB ADDS ONE SQUARE. IT DOES NOT DOUBLE.
//! ---------------------------------------------------------------------
//!
//! The rule reads: "each foot of movement costs 1 extra foot (2 extra
//! feet in difficult terrain)". So on ordinary ground a five-foot climb
//! is ten feet - two squares - and in difficult terrain it is fifteen,
//! which is three.
//!
//! **THAT IS `base + 1`, NOT `base * 2`.** Doubling gets the ordinary
//! case right by coincidence and the difficult case wrong, which is the
//! worst way to be wrong: it looks correct everywhere anybody checks
//! first.
//!
//! ---------------------------------------------------------------------
//! DEEP WATER IS NOT A WALL
//! ---------------------------------------------------------------------
//!
//! A creature with no swim speed may walk into deep water. **It is a
//! legal move and a terrible one** - they begin drowning, and that is
//! the game working rather than the engine refusing. Lava is already
//! this shape: enterable, and an awful idea.
//!
//! So `Mode::Swim` is not a gate. It changes WHAT HAPPENS, not whether
//! you may, and `step` says so by handing back a [`Peril`] beside the
//! cost.
//!
//! A HAZARD AND A PERIL ARE DIFFERENT THINGS, and the distinction is
//! worth keeping: a hazard hurts ANYBODY in the square - lava burns the
//! fire-swimmer and the fool alike - and lives on the terrain. A peril
//! is what the square does to YOU because of what you cannot do. The
//! caller reads the destination's hazard for the first and this for the
//! second.

use crate::grid::{self, Point};
use crate::terrain::{Going, Mode, Stack};

/// What a square does to a creature that should not be there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Peril {
    /// Deep water, and no swim speed. The move is allowed; what
    /// follows is the drowning rules, which are not this module's.
    Drowning,
}

/// A step that may be taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    /// Squares of movement spent.
    pub cost: i64,
    /// What the creature has got itself into, if anything.
    pub peril: Option<Peril>,
}

/// May this creature step from one square to the next, and at what cost?
///
/// `can` is what the creature can do BESIDES WALK - a swim speed, a fly
/// speed. Most creatures hand in nothing.
///
/// ADJACENCY IS CHECKED HERE because it is the one precondition that
/// makes the rest meaningful, and `grid::is_adjacent` is 3D Chebyshev -
/// so a diagonal step up one level is a single step, and anything two
/// levels away is not a step at all.
pub fn step(
    from: &Stack,
    at: Point,
    to: &Stack,
    onto: Point,
    can: &[Mode],
) -> Result<Step, String> {
    if at == onto {
        return Err("that is the square you are already in".to_string());
    }

    let dz = onto.z as i64 - at.z as i64;
    if !grid::is_adjacent(at, onto) {
        // A MORE USEFUL REFUSAL THAN "not adjacent" when the squares
        // are side by side and the difference is all height: that is
        // the commonest way to meet this, and "not next to each other"
        // would read as nonsense about two squares that plainly are.
        if at.x == onto.x && at.y == onto.y {
            return match dz > 0 {
                true => Err(format!("that is {} feet up - a climb, not a step", grid::feet(dz))),
                false => Err(format!("that is {} feet down - a fall, not a step", grid::feet(-dz))),
            };
        }
        return Err("that square is not next to this one".to_string());
    }

    let footing = to.footing();
    let Some(base) = footing.going.step_cost() else {
        return Err(format!("{} is in the way", blocker(to)));
    };

    // THE CLIMB. Either square may be the ramp: you walk UP a ramp and
    // you walk ONTO one, and a flight of stairs is no use if only the
    // bottom step counts.
    //
    // 238. AND IT MUST RUN DOWN THE RAMP'S OWN AXIS. Until now a ramp
    // eased a climb in EVERY direction, which made walking east across
    // a north-south ramp as cheap as walking up it. A slope crossed
    // sideways is a slope, not a staircase.
    let dx = onto.x - at.x;
    let dy = onto.y - at.y;
    let ramped = |s: &Stack| s.surface.links_levels() && s.facing.along(dx, dy);
    let eased = ramped(from) || ramped(to);
    let cost = match dz {
        1 if !eased => base + 1,
        // Up a ramp, along the flat, or down a single square - a drop
        // of five feet costs nothing but the square you land in.
        _ => base,
    };

    // A GATE THAT IS NOT A GATE. See the module note: a creature
    // without the mode may still go, and finds out why it should not
    // have.
    let peril = match footing.needs {
        Some(need) if !can.contains(&need) => peril_of(need),
        _ => None,
    };

    Ok(Step { cost, peril })
}

/// What to name the thing stopping you.
///
/// The COVER first, because that is what is in your way - a wall
/// standing on a floor is a wall, not a floor.
fn blocker(to: &Stack) -> String {
    let blocking = to
        .cover
        .filter(|c| c.footing().going == Going::Blocked)
        .unwrap_or(to.surface);
    format!("a {}", blocking.key().replace('_', " "))
}

fn peril_of(need: Mode) -> Option<Peril> {
    match need {
        Mode::Swim => Some(Peril::Drowning),
        // NOTHING IN THE VOCABULARY ASKS FOR THESE YET. No word means
        // open air, and climbing a cliff is handled by the cost above
        // rather than by a mode, so these have no way to be reached.
        // They get no invented consequence until something can.
        Mode::Climb | Mode::Fly => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::{Facing, Terrain};

    fn ground(t: Terrain) -> Stack {
        Stack::of(t)
    }

    fn here() -> Point {
        Point::at(5, 5)
    }

    fn walk() -> &'static [Mode] {
        &[]
    }

    /* ---------------- along the flat ---------------- */

    #[test]
    fn an_ordinary_step_costs_one() {
        let s = step(
            &ground(Terrain::Dirt),
            here(),
            &ground(Terrain::Dirt),
            Point::at(6, 5),
            walk(),
        )
        .unwrap();
        assert_eq!(s.cost, 1);
        assert_eq!(s.peril, None);
    }

    #[test]
    fn a_diagonal_costs_the_same_as_a_straight_step() {
        // grid.rs decided this; stride must not quietly disagree.
        let s = step(
            &ground(Terrain::Dirt),
            here(),
            &ground(Terrain::Dirt),
            Point::at(6, 6),
            walk(),
        )
        .unwrap();
        assert_eq!(s.cost, 1);
    }

    #[test]
    fn difficult_ground_costs_two() {
        let s = step(
            &ground(Terrain::Dirt),
            here(),
            &ground(Terrain::Marsh),
            Point::at(6, 5),
            walk(),
        )
        .unwrap();
        assert_eq!(s.cost, 2);
    }

    #[test]
    fn the_cost_is_the_destination_not_the_departure() {
        // You pay for where you are going.
        let s = step(
            &ground(Terrain::Marsh),
            here(),
            &ground(Terrain::Road),
            Point::at(6, 5),
            walk(),
        )
        .unwrap();
        assert_eq!(s.cost, 1);
    }

    #[test]
    fn a_cover_can_make_good_ground_slow() {
        let snowy = Stack::of(Terrain::Road).under(Terrain::Snow);
        let s = step(&ground(Terrain::Road), here(), &snowy, Point::at(6, 5), walk()).unwrap();
        assert_eq!(s.cost, 2);
    }

    /* ---------------- things in the way ---------------- */

    #[test]
    fn a_wall_is_named_rather_than_numbered() {
        let wall = Stack::of(Terrain::StoneFloor).under(Terrain::StoneWall);
        let e = step(&ground(Terrain::StoneFloor), here(), &wall, Point::at(6, 5), walk())
            .unwrap_err();
        assert!(e.contains("stone wall"), "{}", e);
    }

    #[test]
    fn the_cover_is_the_blocker_and_not_the_floor_under_it() {
        // "a stone floor is in the way" would be a true sentence about
        // the wrong thing.
        let wall = Stack::of(Terrain::WoodFloor).under(Terrain::TimberWall);
        let e =
            step(&ground(Terrain::WoodFloor), here(), &wall, Point::at(6, 5), walk()).unwrap_err();
        assert!(e.contains("timber wall"), "{}", e);
        assert!(!e.contains("floor"), "{}", e);
    }

    #[test]
    fn a_shut_door_stops_you_and_a_broken_one_does_too() {
        for d in [Terrain::DoorClosed, Terrain::DoorBroken] {
            let door = Stack::of(Terrain::Doorway).under(d);
            assert!(
                step(&ground(Terrain::StoneFloor), here(), &door, Point::at(6, 5), walk()).is_err(),
                "{}",
                d.key()
            );
        }
        let open = Stack::of(Terrain::Doorway).under(Terrain::DoorOpen);
        assert!(step(&ground(Terrain::StoneFloor), here(), &open, Point::at(6, 5), walk()).is_ok());
    }

    /* ---------------- up and down ---------------- */

    #[test]
    fn a_climb_adds_one_square_and_does_not_double() {
        // 5e: "1 extra foot for every foot". Ordinary ground: 1 + 1.
        let s = step(
            &ground(Terrain::Rock),
            here(),
            &ground(Terrain::Rock),
            Point::new(6, 5, 1),
            walk(),
        )
        .unwrap();
        assert_eq!(s.cost, 2);
    }

    #[test]
    fn a_climb_through_difficult_ground_is_three_not_four() {
        // THE CASE DOUBLING GETS WRONG. "2 extra feet in difficult
        // terrain" - so 2 + 1, not 2 * 2.
        let s = step(
            &ground(Terrain::Rock),
            here(),
            &ground(Terrain::Scree),
            Point::new(6, 5, 1),
            walk(),
        )
        .unwrap();
        assert_eq!(s.cost, 3);
    }

    #[test]
    fn a_ramp_makes_a_climb_an_ordinary_step_along_its_own_axis() {
        // THE WHOLE CONTENT OF `links_levels`, finally read - and 238
        // added the half that matters: DOWN THE RAMP'S OWN AXIS.
        let up_the_ramp = step(
            &ground(Terrain::Ramp),
            here(),
            &ground(Terrain::StoneFloor),
            Point::new(5, 6, 1), // northward-southward, matching a north facing
            walk(),
        )
        .unwrap();
        assert_eq!(up_the_ramp.cost, 1);
    }

    #[test]
    fn a_ramp_crossed_sideways_is_a_slope_and_costs_the_climb() {
        // 238. THE CASE THAT WAS WRONG UNTIL THE ART ARRIVED. A ramp
        // built north-south walked EAST across is walking along a
        // slope, not up one.
        let across = step(
            &ground(Terrain::Ramp),
            here(),
            &ground(Terrain::StoneFloor),
            Point::new(6, 5, 1), // eastward, across a north-facing ramp
            walk(),
        )
        .unwrap();
        assert_eq!(across.cost, 2, "crossing a ramp is still a climb");
    }

    #[test]
    fn a_ramp_turned_to_face_the_step_eases_it_again() {
        let turned = Stack::of(Terrain::Ramp).facing(Facing::East);
        let s = step(
            &turned,
            here(),
            &ground(Terrain::StoneFloor),
            Point::new(6, 5, 1),
            walk(),
        )
        .unwrap();
        assert_eq!(s.cost, 1);
    }

    #[test]
    fn a_ramp_is_walked_down_as_well_as_up() {
        // A facing says which way the art points, not which way you
        // are allowed to go.
        let s = step(
            &ground(Terrain::Ramp),
            Point::new(5, 5, 1),
            &ground(Terrain::StoneFloor),
            Point::at(5, 4),
            walk(),
        )
        .unwrap();
        assert_eq!(s.cost, 1);
    }

    #[test]
    fn a_diagonal_with_a_component_along_the_axis_still_counts() {
        // Refusing it would make a creature square up first for a
        // reason nobody could see.
        let s = step(
            &ground(Terrain::Ramp),
            here(),
            &ground(Terrain::StoneFloor),
            Point::new(6, 6, 1),
            walk(),
        )
        .unwrap();
        assert_eq!(s.cost, 1);
    }

    #[test]
    fn stairs_work_from_either_end() {
        // A flight is no use if only the bottom step counts.
        let up = step(
            &ground(Terrain::StoneFloor),
            here(),
            &ground(Terrain::Stairs),
            Point::new(5, 6, 1),
            walk(),
        )
        .unwrap();
        assert_eq!(up.cost, 1);

        let onto = step(
            &ground(Terrain::Stairs),
            here(),
            &ground(Terrain::StoneFloor),
            Point::new(5, 6, 1),
            walk(),
        )
        .unwrap();
        assert_eq!(onto.cost, 1);
    }

    #[test]
    fn stepping_down_one_square_is_free() {
        // Five feet is a drop anybody takes without thinking.
        let s = step(
            &ground(Terrain::Rock),
            here(),
            &ground(Terrain::Dirt),
            Point::new(6, 5, -1),
            walk(),
        )
        .unwrap();
        assert_eq!(s.cost, 1);
    }

    #[test]
    fn two_levels_up_is_a_climb_and_says_so_in_feet() {
        let e = step(
            &ground(Terrain::Rock),
            here(),
            &ground(Terrain::Rock),
            Point::new(5, 5, 2),
            walk(),
        )
        .unwrap_err();
        assert!(e.contains("10 feet up") && e.contains("climb"), "{}", e);
    }

    #[test]
    fn two_levels_down_is_a_fall_and_says_so_in_feet() {
        let e = step(
            &ground(Terrain::Rock),
            here(),
            &ground(Terrain::Rock),
            Point::new(5, 5, -3),
            walk(),
        )
        .unwrap_err();
        assert!(e.contains("15 feet down") && e.contains("fall"), "{}", e);
    }

    #[test]
    fn a_square_across_the_room_is_simply_not_next_to_this_one() {
        let e = step(
            &ground(Terrain::Rock),
            here(),
            &ground(Terrain::Rock),
            Point::at(9, 9),
            walk(),
        )
        .unwrap_err();
        assert!(e.contains("not next to"), "{}", e);
    }

    #[test]
    fn the_square_you_are_standing_in_is_not_a_step() {
        let e = step(
            &ground(Terrain::Rock),
            here(),
            &ground(Terrain::Rock),
            here(),
            walk(),
        )
        .unwrap_err();
        assert!(e.contains("already in"), "{}", e);
    }

    /* ---------------- deep water ---------------- */

    #[test]
    fn a_creature_that_cannot_swim_may_still_walk_into_deep_water() {
        // IT IS A LEGAL MOVE AND A TERRIBLE ONE. Refusing it would be
        // the engine playing the game for them.
        let s = step(
            &ground(Terrain::Dirt),
            here(),
            &ground(Terrain::DeepWater),
            Point::at(6, 5),
            walk(),
        )
        .unwrap();
        assert_eq!(s.cost, 1);
        assert_eq!(s.peril, Some(Peril::Drowning));
    }

    #[test]
    fn a_swimmer_is_in_no_peril_at_all() {
        let s = step(
            &ground(Terrain::Dirt),
            here(),
            &ground(Terrain::DeepWater),
            Point::at(6, 5),
            &[Mode::Swim],
        )
        .unwrap();
        assert_eq!(s.peril, None);
        assert_eq!(s.cost, 1, "a swim speed pays the ordinary rate");
    }

    #[test]
    fn wading_needs_nothing_and_perils_nobody() {
        let s = step(
            &ground(Terrain::Dirt),
            here(),
            &ground(Terrain::ShallowWater),
            Point::at(6, 5),
            walk(),
        )
        .unwrap();
        assert_eq!(s.cost, 2);
        assert_eq!(s.peril, None);
    }

    #[test]
    fn lava_is_a_hazard_and_not_a_peril() {
        // A hazard hurts anybody in the square and lives on the
        // terrain; a peril is what the square does to YOU for what you
        // cannot do. Stepping into lava is neither refused nor perilous
        // in this module's sense - it is simply on fire.
        let s = step(
            &ground(Terrain::Rock),
            here(),
            &ground(Terrain::Lava),
            Point::at(6, 5),
            walk(),
        )
        .unwrap();
        assert_eq!(s.peril, None);
        assert!(ground(Terrain::Lava).footing().hazard.is_some());
    }
}
