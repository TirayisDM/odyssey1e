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

/* ================= 247. WALKING, NOT STEPPING ================= */

/// How far a creature actually got.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Walked {
    /// Where it ended up, which is NOT always where it was sent.
    pub reached: Point,
    /// Squares of movement spent, summed step by step - so difficult
    /// ground and a climb are each paid for where they happen.
    pub cost: i64,
    pub steps: usize,
    /// Why it stopped short, or `None` if it arrived.
    ///
    /// **A SENTENCE, NOT A CODE.** It is `step`'s own refusal for the
    /// square it could not enter - "a timber wall is in the way" - and
    /// it is the thing the DM reads.
    pub stopped: Option<String>,
    /// What the square it ENDED IN does to it.
    ///
    /// 247. THE END, NOT THE JOURNEY. A creature that crosses two
    /// squares of deep water and comes out the far side was in the
    /// water, and this does not say so. That is deliberate rather than
    /// overlooked: the only thing anything can act on today is the
    /// state it is now IN, because the drowning rules do not exist and
    /// nothing counts rounds held underwater. When they do, this
    /// becomes a peril per step and the caller sums them.
    pub peril: Option<Peril>,
}

/// Walk toward a square, one step at a time, stopping at the first
/// thing in the way.
///
/// ---------------------------------------------------------------------
/// WHY A LINE AND NOT A ROUTE
/// ---------------------------------------------------------------------
///
/// This is **not** pathfinding. It walks `grid::line` - the Bresenham
/// walk 225 already built for line of sight - and every consecutive
/// pair on that line is adjacent, so each pair is exactly the question
/// [`step`] answers. A wall in the middle stops the creature AT the
/// wall rather than sending it round, and `stopped` says what stopped
/// it.
///
/// A route that goes round corners is MAP.md's Phase 5 and wants its
/// own module with its own tests. **The honest version of the small
/// thing is better than a pretend version of the big one**: a creature
/// that visibly stops against a wall is telling the truth, and one
/// that silently takes a different path than the line the DM drew is
/// not.
///
/// ---------------------------------------------------------------------
/// EACH STEP IS PRICED WHERE IT HAPPENS
/// ---------------------------------------------------------------------
///
/// The cost is summed step by step rather than taken as the distance,
/// so crossing three squares of mud costs six and crossing three of
/// road costs three. Chebyshev means the NUMBER of steps is the
/// distance; it does not mean they cost the same.
///
/// `ground` answers what is at a square - a row, else the room's
/// default, else nothing. Passing it in rather than reaching for it
/// keeps this testable without a database, and means the caller
/// decides how wide a window to fetch.
///
/// ONE LEVEL. `grid::line` is 2D and carries the starting z, so a
/// target on another level is refused rather than quietly flattened
/// into a walk across this one. **A change of height is a step**, and
/// `step` prices it - up a ramp, down a drop, onto a stair.
pub fn walk(
    at: Point,
    toward: Point,
    ground: impl Fn(Point) -> Option<Stack>,
    can: &[Mode],
) -> Result<Walked, String> {
    if at == toward {
        return Err("that is the square you are already in".to_string());
    }
    // THE QUIET WRONG ANSWER THIS PREVENTS: `line` ignores z and keeps
    // the start's, so without this a click on another level would walk
    // the creature sideways across its own floor to the x,y of a
    // square it was never sent to.
    if toward.z != at.z {
        return Err(
            "that is on another level - a change of height is a step, not a walk".to_string(),
        );
    }

    let Some(mut from) = ground(at) else {
        return Err("the square it is standing on is not drawn".to_string());
    };

    let mut here = at;
    let mut cost = 0i64;
    let mut steps = 0usize;
    let mut peril = None;
    let mut stopped = None;

    // `line` yields the starting square first, so it is skipped.
    for next in grid::line(at, toward).skip(1) {
        let Some(to) = ground(next) else {
            stopped = Some(
                "nothing is drawn there - a missing square is unmapped, not open floor".to_string(),
            );
            break;
        };
        match step(&from, here, &to, next, can) {
            Ok(s) => {
                cost += s.cost;
                steps += 1;
                peril = s.peril;
                here = next;
                from = to;
            }
            Err(e) => {
                stopped = Some(e);
                break;
            }
        }
    }

    // 217'S SHAPE. A walk that moved nothing is not a success with a
    // note attached - it is a refusal, and the reason is the only
    // useful part of it.
    if steps == 0 {
        return Err(stopped.unwrap_or_else(|| "it did not move".to_string()));
    }

    Ok(Walked {
        reached: here,
        cost,
        steps,
        stopped,
        peril,
    })
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

    /// What this creature can do besides walk: nothing.
    ///
    /// 247. Renamed from `afoot()` when [`walk`] became a function in
    /// this module - a helper shadowing the thing under test is how a
    /// test ends up passing against itself.
    fn afoot() -> &'static [Mode] {
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
            afoot(),
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
            afoot(),
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
            afoot(),
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
            afoot(),
        )
        .unwrap();
        assert_eq!(s.cost, 1);
    }

    #[test]
    fn a_cover_can_make_good_ground_slow() {
        let snowy = Stack::of(Terrain::Road).under(Terrain::Snow);
        let s = step(&ground(Terrain::Road), here(), &snowy, Point::at(6, 5), afoot()).unwrap();
        assert_eq!(s.cost, 2);
    }

    /* ---------------- things in the way ---------------- */

    #[test]
    fn a_wall_is_named_rather_than_numbered() {
        let wall = Stack::of(Terrain::StoneFloor).under(Terrain::StoneWall);
        let e = step(&ground(Terrain::StoneFloor), here(), &wall, Point::at(6, 5), afoot())
            .unwrap_err();
        assert!(e.contains("stone wall"), "{}", e);
    }

    #[test]
    fn the_cover_is_the_blocker_and_not_the_floor_under_it() {
        // "a stone floor is in the way" would be a true sentence about
        // the wrong thing.
        let wall = Stack::of(Terrain::WoodFloor).under(Terrain::TimberWall);
        let e =
            step(&ground(Terrain::WoodFloor), here(), &wall, Point::at(6, 5), afoot()).unwrap_err();
        assert!(e.contains("timber wall"), "{}", e);
        assert!(!e.contains("floor"), "{}", e);
    }

    #[test]
    fn a_shut_door_stops_you_and_a_broken_one_does_too() {
        for d in [Terrain::DoorClosed, Terrain::DoorBroken] {
            let door = Stack::of(Terrain::Doorway).under(d);
            assert!(
                step(&ground(Terrain::StoneFloor), here(), &door, Point::at(6, 5), afoot()).is_err(),
                "{}",
                d.key()
            );
        }
        let open = Stack::of(Terrain::Doorway).under(Terrain::DoorOpen);
        assert!(step(&ground(Terrain::StoneFloor), here(), &open, Point::at(6, 5), afoot()).is_ok());
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
            afoot(),
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
            afoot(),
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
            afoot(),
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
            afoot(),
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
            afoot(),
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
            afoot(),
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
            afoot(),
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
            afoot(),
        )
        .unwrap();
        assert_eq!(up.cost, 1);

        let onto = step(
            &ground(Terrain::Stairs),
            here(),
            &ground(Terrain::StoneFloor),
            Point::new(5, 6, 1),
            afoot(),
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
            afoot(),
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
            afoot(),
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
            afoot(),
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
            afoot(),
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
            afoot(),
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
            afoot(),
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
            afoot(),
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
            afoot(),
        )
        .unwrap();
        assert_eq!(s.peril, None);
        assert!(ground(Terrain::Lava).footing().hazard.is_some());
    }

    /* ============== 247. walking a line ============== */

    /// A map from a list, so a test can describe a corridor and nothing
    /// else. Anything not listed is UNMAPPED, which is 226's rule and
    /// the reason a walk can stop at the edge of what is drawn.
    fn room(squares: &[(i32, i32, Terrain)]) -> impl Fn(Point) -> Option<Stack> + '_ {
        move |p: Point| {
            squares
                .iter()
                .find(|(x, y, _)| *x == p.x && *y == p.y)
                .map(|(_, _, t)| Stack::of(*t))
        }
    }

    fn strip(n: i32, t: Terrain) -> Vec<(i32, i32, Terrain)> {
        (0..n).map(|x| (x, 0, t)).collect()
    }

    #[test]
    fn a_walk_down_a_clear_line_arrives() {
        let g = strip(5, Terrain::Dirt);
        let w = walk(Point::at(0, 0), Point::at(4, 0), room(&g), afoot()).unwrap();
        assert_eq!(w.reached, Point::at(4, 0));
        assert_eq!(w.steps, 4);
        assert_eq!(w.cost, 4);
        assert_eq!(w.stopped, None, "it got there");
    }

    #[test]
    fn each_step_is_priced_where_it_happens() {
        // Three squares of mud cost six, not three. Chebyshev says how
        // MANY steps; it does not say what they cost.
        let g = strip(4, Terrain::Mud);
        let w = walk(Point::at(0, 0), Point::at(3, 0), room(&g), afoot()).unwrap();
        assert_eq!(w.steps, 3);
        assert_eq!(w.cost, 6);
    }

    #[test]
    fn a_mixed_line_costs_the_sum_of_its_squares() {
        let g = vec![
            (0, 0, Terrain::Road),
            (1, 0, Terrain::Road),
            (2, 0, Terrain::Mud),
            (3, 0, Terrain::Road),
        ];
        let w = walk(Point::at(0, 0), Point::at(3, 0), room(&g), afoot()).unwrap();
        assert_eq!(w.steps, 3);
        assert_eq!(w.cost, 1 + 2 + 1, "road, mud, road");
    }

    #[test]
    fn a_wall_stops_it_at_the_wall() {
        let mut g = strip(5, Terrain::Dirt);
        g[3] = (3, 0, Terrain::Dirt);
        let g: Vec<_> = g
            .into_iter()
            .map(|(x, y, t)| if x == 3 { (x, y, Terrain::TimberWall) } else { (x, y, t) })
            .collect();
        let w = walk(Point::at(0, 0), Point::at(4, 0), room(&g), afoot()).unwrap();
        assert_eq!(w.reached, Point::at(2, 0), "it stands against the wall");
        assert_eq!(w.steps, 2);
        // The sentence is `step`'s own, so the two cannot drift apart.
        assert!(w.stopped.unwrap().contains("in the way"));
    }

    #[test]
    fn a_wall_in_the_very_first_square_is_a_refusal_not_a_walk() {
        // 217: a walk that moved nothing is not a success with a note.
        let g = vec![(0, 0, Terrain::Dirt), (1, 0, Terrain::TimberWall)];
        let e = walk(Point::at(0, 0), Point::at(1, 0), room(&g), afoot()).unwrap_err();
        assert!(e.contains("in the way"), "{}", e);
    }

    #[test]
    fn it_stops_at_the_edge_of_what_is_drawn() {
        // A MISSING ROW IS UNMAPPED, NOT OPEN FLOOR - 226. Walking off
        // the end of a corridor must stop, not continue into nothing.
        let g = strip(3, Terrain::Dirt);
        let w = walk(Point::at(0, 0), Point::at(6, 0), room(&g), afoot()).unwrap();
        assert_eq!(w.reached, Point::at(2, 0));
        assert!(w.stopped.unwrap().contains("unmapped"));
    }

    #[test]
    fn a_diagonal_line_is_walked_one_square_at_a_time() {
        let g: Vec<_> = (0..4).map(|i| (i, i, Terrain::Dirt)).collect();
        let w = walk(Point::at(0, 0), Point::at(3, 3), room(&g), afoot()).unwrap();
        assert_eq!(w.steps, 3);
        assert_eq!(w.cost, 3, "a diagonal costs one - grid.rs decided that");
    }

    #[test]
    fn every_pair_on_the_line_is_a_legal_step() {
        // The property the whole function rests on: Bresenham never
        // jumps, so each consecutive pair is adjacent and `step` is
        // always being asked a question it can answer. A knight's-move
        // line would make this fail.
        let g: Vec<_> = (0..8)
            .flat_map(|x| (0..8).map(move |y| (x, y, Terrain::Dirt)))
            .collect();
        for (tx, ty) in [(7, 3), (3, 7), (7, 7), (1, 6), (6, 1)] {
            let w = walk(Point::at(0, 0), Point::at(tx, ty), room(&g), afoot()).unwrap();
            assert_eq!(w.reached, Point::at(tx, ty), "to ({},{})", tx, ty);
            assert_eq!(
                w.steps as i32,
                tx.max(ty),
                "steps should be the Chebyshev distance to ({},{})",
                tx,
                ty
            );
        }
    }

    #[test]
    fn walking_into_deep_water_ends_in_peril() {
        let g = vec![
            (0, 0, Terrain::Dirt),
            (1, 0, Terrain::Dirt),
            (2, 0, Terrain::DeepWater),
        ];
        let w = walk(Point::at(0, 0), Point::at(2, 0), room(&g), afoot()).unwrap();
        assert_eq!(w.reached, Point::at(2, 0), "it is allowed in - stride's rule");
        assert_eq!(w.peril, Some(Peril::Drowning));
    }

    #[test]
    fn a_swimmer_crosses_the_same_water_without_peril() {
        let g = vec![(0, 0, Terrain::Dirt), (1, 0, Terrain::DeepWater)];
        let w = walk(Point::at(0, 0), Point::at(1, 0), room(&g), &[Mode::Swim]).unwrap();
        assert_eq!(w.peril, None);
    }

    #[test]
    fn coming_out_the_far_side_of_the_water_reports_no_peril() {
        // RECORDED, NOT LIKED. The creature was in the water and this
        // says it is not now. See the note on `Walked::peril`: the end
        // state is the only thing anything can act on until the
        // drowning rules exist.
        let g = vec![
            (0, 0, Terrain::Dirt),
            (1, 0, Terrain::DeepWater),
            (2, 0, Terrain::Dirt),
        ];
        let w = walk(Point::at(0, 0), Point::at(2, 0), room(&g), afoot()).unwrap();
        assert_eq!(w.reached, Point::at(2, 0));
        assert_eq!(w.peril, None);
    }

    #[test]
    fn another_level_is_refused_rather_than_flattened() {
        // Without this, `line` would keep the start's z and walk the
        // creature sideways across its own floor to the x,y of a
        // square it was never sent to.
        let g = vec![(0, 0, Terrain::Dirt), (1, 0, Terrain::Dirt)];
        let e = walk(
            Point::at(0, 0),
            Point::new(1, 0, 1),
            room(&g),
            afoot(),
        )
        .unwrap_err();
        assert!(e.contains("another level"), "{}", e);
    }

    #[test]
    fn the_square_you_are_in_is_not_a_walk() {
        let g = vec![(0, 0, Terrain::Dirt)];
        let e = walk(Point::at(0, 0), Point::at(0, 0), room(&g), afoot()).unwrap_err();
        assert!(e.contains("already in"), "{}", e);
    }

    #[test]
    fn standing_on_nothing_is_refused_before_anything_moves() {
        let g = vec![(1, 0, Terrain::Dirt)];
        let e = walk(Point::at(0, 0), Point::at(1, 0), room(&g), afoot()).unwrap_err();
        assert!(e.contains("not drawn"), "{}", e);
    }

    #[test]
    fn there_is_no_budget_yet_so_a_long_walk_is_allowed() {
        // RECORDED SO IT IS NOT MISTAKEN FOR AN OVERSIGHT. A speed
        // exists on `species` and nothing reads it; until it does, one
        // click can cross a room. The cost is RETURNED, which is what
        // the next cycle spends.
        let g = strip(40, Terrain::Dirt);
        let w = walk(Point::at(0, 0), Point::at(39, 0), room(&g), afoot()).unwrap();
        assert_eq!(w.steps, 39);
        assert_eq!(w.cost, 39);
        assert_eq!(w.stopped, None);
    }
}
