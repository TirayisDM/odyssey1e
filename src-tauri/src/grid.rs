//! WHERE A THING IS, and what counts as near it.
//!
//! 225. THE GEOMETRY HALF OF A MAP, and deliberately nothing else. No
//! terrain, no cost, no light, no walls. This file answers three
//! questions - how far apart are two squares, which squares touch this
//! one, and what lies on the line between them - and every one of them
//! has the same answer whatever the terrain turns out to be.
//!
//! THAT SPLIT IS THE POINT. `terrain.rs` will hold what a square is
//! MADE OF and what crossing it costs, and those are rules that argue
//! with 5e. Distance does not argue with anything; it is arithmetic,
//! and keeping it in its own file means the arguments happen in one
//! place rather than leaking into the maths.
//!
//! ---------------------------------------------------------------------
//! A DIAGONAL COSTS ONE, AND THIS IS THE ONLY FILE THAT SAYS SO
//! ---------------------------------------------------------------------
//!
//! MAP.md found three answers coexisting in the ported code and they
//! contradicted each other: `distance_to` used Chebyshev, where a
//! diagonal is 1; `TileGrid::adjacent` returned all eight neighbours
//! undifferentiated; and the Apps Script pathfinder was four-directional
//! and would route around a step the distance function called adjacent.
//!
//! **CHEBYSHEV, AND ONE FUNCTION.** It is the Player's Handbook default
//! - a diagonal is five feet, the same as a straight step - and the
//! 5-10-5 alternative in the DMG is explicitly an optional rule. Pick
//! the default, write it once, and let everything call it.
//!
//! ---------------------------------------------------------------------
//! SIGNED COORDINATES, WHICH THE PORTED CODE WAS NOT
//! ---------------------------------------------------------------------
//!
//! `TileGrid` used `u16` and had to special-case `x == 0` in every
//! neighbour walk, because `0 - 1` on an unsigned integer is not a
//! square off the edge of the map, it is 65535. That is a bug waiting
//! for a room whose origin is at the corner.
//!
//! These are `i32` for x and y and `i16` for z, which are exactly
//! Postgres `int` and `smallint` - the types MAP.md's `tiles` table
//! already proposes. A coordinate that cannot survive the round trip to
//! the database is not a coordinate this project can use.
//!
//! ---------------------------------------------------------------------
//! Z IS A HEIGHT, IN THE SAME UNIT AS X AND Y
//! ---------------------------------------------------------------------
//!
//! One z is one square is five feet. Not a layer index, not a bounded
//! range, and negative is legal - a pit bottom is z = -1 the way a
//! bridge deck ten feet up is z = 2.

/// How many feet one square is, everywhere.
///
/// 225. THE ONE PLACE THIS NUMBER LIVES. 5e talks in feet - a speed of
/// 30, a reach of 10, a torch that is bright for 20 - and a grid talks
/// in squares. Something has to convert, and if two things convert they
/// will eventually disagree by a factor of five.
pub const FEET_PER_SQUARE: i64 = 5;

/// Squares as the feet a rulebook would print.
pub fn feet(squares: i64) -> i64 {
    squares * FEET_PER_SQUARE
}

/// Feet as squares, rounding DOWN.
///
/// DOWN, NOT NEAREST, and it matters at the edge of a reach. A reach of
/// 7 feet is one square, not two: the rule is "can you touch it", and
/// two feet short of the next square is short. Rounding to nearest
/// would hand out a free square of reach to anything measured in odd
/// numbers.
pub fn squares(feet: i64) -> i64 {
    feet.div_euclid(FEET_PER_SQUARE)
}

/// A square on a map, at a height.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Point {
    pub x: i32,
    pub y: i32,
    pub z: i16,
}

impl Point {
    /// A square on the ground floor, which is most of them.
    pub fn at(x: i32, y: i32) -> Self {
        Point { x, y, z: 0 }
    }

    pub fn new(x: i32, y: i32, z: i16) -> Self {
        Point { x, y, z }
    }
}

/// How many squares apart, counting a diagonal as one.
///
/// 225. CHEBYSHEV IN THREE DIMENSIONS - the largest of the three axis
/// differences. In two dimensions this is the Player's Handbook rule;
/// extending it to z rather than inventing a second rule for height is
/// the same decision taken once, and it keeps "adjacent" meaning one
/// thing whether the other creature is beside you or hovering a square
/// above your head.
pub fn distance(a: Point, b: Point) -> i64 {
    let dx = (a.x as i64 - b.x as i64).abs();
    let dy = (a.y as i64 - b.y as i64).abs();
    let dz = (a.z as i64 - b.z as i64).abs();
    dx.max(dy).max(dz)
}

/// The same distance a rulebook would print.
pub fn distance_in_feet(a: Point, b: Point) -> i64 {
    feet(distance(a, b))
}

/// Touching - one square away by the rule above, and not the same square.
///
/// NOT THE SAME SQUARE, said explicitly. Two creatures sharing a square
/// are a real situation (a swarm, a mount, a grapple) and whatever the
/// rule for them is, it is not "adjacent".
pub fn is_adjacent(a: Point, b: Point) -> bool {
    a != b && distance(a, b) == 1
}

/// Is `b` within `reach` squares of `a`?
pub fn within(a: Point, b: Point, reach_squares: i64) -> bool {
    distance(a, b) <= reach_squares
}

/// The eight squares around this one, on the same level.
///
/// EIGHT AND NOT TWENTY-SIX. The 3D neighbourhood is what `distance`
/// implies, and it is not what a creature standing on a floor can step
/// to - stepping up a level is climbing or flying, which is a movement
/// rule and belongs with movement. This returns what "the squares
/// around me" means to somebody standing still.
pub fn neighbours(p: Point) -> Vec<Point> {
    let mut out = Vec::with_capacity(8);
    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            out.push(Point::new(p.x + dx, p.y + dy, p.z));
        }
    }
    out
}

/// Every square within `radius`, on the same level, nearest first.
///
/// A FILLED SQUARE, NOT A CIRCLE, which is what Chebyshev means and
/// what 5e draws for a burst on a grid.
pub fn in_radius(centre: Point, radius: i64) -> Vec<Point> {
    if radius < 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let r = radius as i32;
    for dy in -r..=r {
        for dx in -r..=r {
            out.push(Point::new(centre.x + dx, centre.y + dy, centre.z));
        }
    }
    out.sort_by_key(|p| (distance(centre, *p), p.y, p.x));
    out
}

/// The squares on the straight line from `a` to `b`, both ends included.
///
/// ---------------------------------------------------------------------
/// 225. AN ITERATOR, NOT A `Vec`
/// ---------------------------------------------------------------------
///
/// MAP.md lists `bresenham_line` allocating a `Vec` per call as a defect
/// to fix on the way in rather than port faithfully. It is called once
/// per ray per light source per frame, and almost every caller stops
/// early - the first wall ends the question. An iterator lets a caller
/// stop without having built the rest of the line.
///
/// IT WALKS IN THE PLANE OF `a`. Line of sight through a height
/// difference is a harder question than this file is ready to answer -
/// it needs to know what a wall's height is - so z is carried through
/// unchanged and three-dimensional sight is deliberately not pretended
/// at. See the test that pins this.
pub fn line(a: Point, b: Point) -> Line {
    let dx = (b.x - a.x).abs();
    let dy = -(b.y - a.y).abs();
    Line {
        at: a,
        to: b,
        sx: if a.x < b.x { 1 } else { -1 },
        sy: if a.y < b.y { 1 } else { -1 },
        dx,
        dy,
        err: dx + dy,
        done: false,
    }
}

/// The walk produced by [`line`].
pub struct Line {
    at: Point,
    to: Point,
    sx: i32,
    sy: i32,
    dx: i32,
    dy: i32,
    err: i32,
    done: bool,
}

impl Iterator for Line {
    type Item = Point;

    fn next(&mut self) -> Option<Point> {
        if self.done {
            return None;
        }
        let here = self.at;
        if here.x == self.to.x && here.y == self.to.y {
            self.done = true;
            return Some(here);
        }
        let e2 = 2 * self.err;
        if e2 >= self.dy {
            self.err += self.dy;
            self.at.x += self.sx;
        }
        if e2 <= self.dx {
            self.err += self.dx;
            self.at.y += self.sy;
        }
        Some(here)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_square_is_five_feet_both_ways() {
        assert_eq!(feet(6), 30);
        assert_eq!(squares(30), 6);
    }

    #[test]
    fn a_reach_that_does_not_fill_a_square_does_not_get_one() {
        // THE WHOLE REASON IT ROUNDS DOWN. Seven feet of reach touches
        // the next square and not the one after it.
        assert_eq!(squares(7), 1);
        assert_eq!(squares(9), 1);
        assert_eq!(squares(10), 2);
    }

    #[test]
    fn feet_round_down_on_the_wrong_side_of_zero_too() {
        // div_euclid rather than `/`, which truncates toward zero and
        // would turn -1 foot into 0 squares and -6 into -1.
        assert_eq!(squares(-1), -1);
        assert_eq!(squares(-5), -1);
        assert_eq!(squares(-6), -2);
    }

    #[test]
    fn a_diagonal_is_one_square() {
        // THE DECISION THIS FILE EXISTS TO HOLD.
        let a = Point::at(0, 0);
        assert_eq!(distance(a, Point::at(1, 0)), 1);
        assert_eq!(distance(a, Point::at(1, 1)), 1);
        assert_eq!(distance(a, Point::at(-1, -1)), 1);
    }

    #[test]
    fn a_long_diagonal_is_not_free_twice_over() {
        // Chebyshev, not Manhattan: four diagonal steps is four, not
        // eight, and not five-and-a-half.
        assert_eq!(distance(Point::at(0, 0), Point::at(4, 4)), 4);
        assert_eq!(distance(Point::at(0, 0), Point::at(4, 2)), 4);
    }

    #[test]
    fn height_counts_on_the_same_rule_as_width() {
        // A creature hovering one square up is adjacent. Inventing a
        // separate rule for z is the thing this avoids.
        let ground = Point::at(0, 0);
        assert_eq!(distance(ground, Point::new(0, 0, 1)), 1);
        assert!(is_adjacent(ground, Point::new(1, 1, 1)));
        assert_eq!(distance(ground, Point::new(0, 0, 6)), 6);
    }

    #[test]
    fn a_negative_height_is_a_pit_and_not_an_error() {
        assert_eq!(distance(Point::at(0, 0), Point::new(0, 0, -1)), 1);
    }

    #[test]
    fn a_square_is_not_adjacent_to_itself() {
        // Two creatures in one square are a real situation; whatever
        // the rule is, it is not this one.
        let p = Point::new(3, 4, 2);
        assert!(!is_adjacent(p, p));
        assert_eq!(distance(p, p), 0);
    }

    #[test]
    fn reach_is_measured_the_same_way() {
        let a = Point::at(0, 0);
        assert!(within(a, Point::at(2, 2), 2));
        assert!(!within(a, Point::at(3, 0), 2));
    }

    #[test]
    fn a_coordinate_at_the_origin_has_eight_neighbours_like_any_other() {
        // THE u16 BUG, PINNED. The ported grid special-cased x == 0
        // because 0 - 1 on an unsigned integer is 65535.
        let n = neighbours(Point::at(0, 0));
        assert_eq!(n.len(), 8);
        assert!(n.contains(&Point::at(-1, -1)));
        assert!(n.contains(&Point::at(-1, 0)));
        assert!(!n.contains(&Point::at(0, 0)));
    }

    #[test]
    fn neighbours_stay_on_their_own_level() {
        for p in neighbours(Point::new(0, 0, 3)) {
            assert_eq!(p.z, 3);
        }
    }

    #[test]
    fn every_neighbour_is_adjacent_and_nothing_else_is() {
        let c = Point::at(10, 10);
        for p in neighbours(c) {
            assert!(is_adjacent(c, p), "{:?}", p);
        }
        assert!(!is_adjacent(c, Point::at(12, 10)));
    }

    #[test]
    fn a_radius_is_a_filled_square() {
        // Chebyshev again: radius 1 is the 3x3 block including the
        // centre, which is what 5e draws for a burst on a grid.
        assert_eq!(in_radius(Point::at(0, 0), 1).len(), 9);
        assert_eq!(in_radius(Point::at(0, 0), 2).len(), 25);
        assert_eq!(in_radius(Point::at(0, 0), 0).len(), 1);
    }

    #[test]
    fn a_radius_comes_back_nearest_first() {
        let r = in_radius(Point::at(0, 0), 2);
        assert_eq!(r[0], Point::at(0, 0));
        let d: Vec<i64> = r.iter().map(|p| distance(Point::at(0, 0), *p)).collect();
        assert!(d.windows(2).all(|w| w[0] <= w[1]), "not sorted by distance");
    }

    #[test]
    fn a_negative_radius_is_empty_rather_than_a_panic() {
        assert!(in_radius(Point::at(0, 0), -1).is_empty());
    }

    #[test]
    fn a_line_includes_both_ends() {
        let v: Vec<Point> = line(Point::at(0, 0), Point::at(3, 0)).collect();
        assert_eq!(v.first(), Some(&Point::at(0, 0)));
        assert_eq!(v.last(), Some(&Point::at(3, 0)));
        assert_eq!(v.len(), 4);
    }

    #[test]
    fn a_line_to_itself_is_one_square() {
        let v: Vec<Point> = line(Point::at(2, 2), Point::at(2, 2)).collect();
        assert_eq!(v, vec![Point::at(2, 2)]);
    }

    #[test]
    fn a_diagonal_line_steps_once_per_square() {
        let v: Vec<Point> = line(Point::at(0, 0), Point::at(3, 3)).collect();
        assert_eq!(v.len(), 4);
        assert_eq!(v[1], Point::at(1, 1));
    }

    #[test]
    fn a_line_is_symmetric() {
        // THE PROPERTY THAT MATTERS FOR SIGHT. If A can see B then B
        // can see A, and a line that walked differently in each
        // direction would quietly break that.
        let a = Point::at(0, 0);
        let b = Point::at(5, 2);
        let there: Vec<Point> = line(a, b).collect();
        let mut back: Vec<Point> = line(b, a).collect();
        back.reverse();
        assert_eq!(there, back);
    }

    #[test]
    fn a_line_can_be_abandoned_without_building_the_rest() {
        // THE REASON IT IS AN ITERATOR. A caller looking for the first
        // wall stops at the first wall.
        let first_three: Vec<Point> = line(Point::at(0, 0), Point::at(1000, 0)).take(3).collect();
        assert_eq!(first_three.len(), 3);
        assert_eq!(first_three[2], Point::at(2, 0));
    }

    #[test]
    fn a_line_stays_in_the_plane_it_started_in() {
        // SAID OUT LOUD because it is a limitation, not a feature.
        // Sight through a height difference needs to know how tall a
        // wall is, and nothing here does yet.
        let v: Vec<Point> = line(Point::new(0, 0, 4), Point::new(3, 0, -2)).collect();
        assert!(v.iter().all(|p| p.z == 4), "z was not carried unchanged");
    }
}
