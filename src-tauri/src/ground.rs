//! WHAT IS AT THIS SQUARE.
//!
//! 233. One question with three possible answers, asked in one place so
//! that the renderer, a step and the fog cannot disagree about the same
//! square.
//!
//! ```text
//!   a row is drawn there          ->  that
//!   no row, inside a declared room, and the room has a default  ->  the default
//!   otherwise                     ->  nothing. The square is unmapped.
//! ```
//!
//! ---------------------------------------------------------------------
//! WHY A DEFAULT AT ALL
//! ---------------------------------------------------------------------
//!
//! A forty-by-thirty room is TWELVE HUNDRED SQUARES, and storing twelve
//! hundred rows that all say `stone_floor` makes a mockery of the
//! sparseness 226 chose rows for - "a party entering a dark room pulls
//! a handful of rows, not sixteen hundred".
//!
//! A default says "this room is stone floor unless stated", and **only
//! the exceptions get rows**: the walls, the doorway, the pit.
//!
//! ---------------------------------------------------------------------
//! AND IT CHANGES WHAT A MISSING ROW MEANS, WHICH IS A RECORDED DECISION
//! ---------------------------------------------------------------------
//!
//! 226 says plainly: **a missing row is unmapped, not empty floor.**
//! That is still true where there is no default. Where a room declares
//! one, a missing row inside it is now THE DEFAULT - and the two states
//! have to stay tellable apart, because `step_to` refuses onto undrawn
//! ground and falling will want to know there is nothing underneath.
//!
//! So this returns an `Option`. `None` still means unmapped, and it
//! still means it everywhere a default does not reach.
//!
//! ---------------------------------------------------------------------
//! A ROW OUTSIDE THE ROOM IS STILL REAL
//! ---------------------------------------------------------------------
//!
//! 232 decided that shrinking a map does not delete what falls outside
//! it: "a tile outside the declared room should look WRONG, not
//! vanish." So a drawn square wins wherever it is, and the bounds are
//! consulted only to decide whether a default reaches an UNDRAWN one.

use crate::terrain::Stack;

/// How big a location says its map is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    pub width: Option<i32>,
    pub height: Option<i32>,
}

impl Bounds {
    /// A map nobody has measured.
    pub const UNMEASURED: Bounds = Bounds {
        width: None,
        height: None,
    };

    pub fn of(width: Option<i32>, height: Option<i32>) -> Self {
        Bounds { width, height }
    }

    /// Has anybody said how big this is?
    pub fn measured(&self) -> bool {
        self.width.is_some() || self.height.is_some()
    }

    /// MAY a square exist here?
    ///
    /// An unmeasured map allows anything, because nobody has said where
    /// it stops. This is the question `paint_tiles` asks.
    pub fn allows(&self, x: i32, y: i32) -> bool {
        self.width.is_none_or(|w| x >= 0 && x < w)
            && self.height.is_none_or(|h| y >= 0 && y < h)
    }

    /// Is this square INSIDE A ROOM somebody has drawn the walls of?
    ///
    /// ---------------------------------------------------------------
    /// 233. THE SAME SHAPE AS `allows` AND THE OPPOSITE ANSWER WHEN
    /// NOTHING IS DECLARED, which is the whole reason there are two.
    /// ---------------------------------------------------------------
    ///
    /// `allows` is permissive about an unmeasured map: nobody said
    /// where it stops, so paint where you like. `encloses` is strict
    /// about exactly the same map: nobody said where it stops, so a
    /// default cannot reach anywhere.
    ///
    /// **FOLDING THEM INTO ONE FUNCTION WOULD GIVE AN UNMEASURED ROOM
    /// INFINITE GROUND.** Every square in every direction would resolve
    /// to the default, a creature could walk to x = 500, and the map
    /// would have no edge at all. The distinction looks pedantic until
    /// it is the difference between a tavern and a plain.
    pub fn encloses(&self, x: i32, y: i32) -> bool {
        match (self.width, self.height) {
            (Some(w), Some(h)) => x >= 0 && x < w && y >= 0 && y < h,
            _ => false,
        }
    }
}

/// What is at this square, if anything.
///
/// `row` is the drawn square, `default` is the location's recipe, and
/// `bounds` is how big the room says it is. `None` means UNMAPPED, with
/// everything 226 said that means: not empty floor, and nothing to
/// stand on.
pub fn at(
    row: Option<Stack>,
    default: Option<Stack>,
    bounds: Bounds,
    x: i32,
    y: i32,
) -> Option<Stack> {
    // A DRAWN SQUARE WINS WHEREVER IT IS - see the module note on 232.
    if row.is_some() {
        return row;
    }
    match bounds.encloses(x, y) {
        true => default,
        false => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::Terrain;

    fn floor() -> Stack {
        Stack::of(Terrain::StoneFloor)
    }

    fn wall() -> Stack {
        Stack::of(Terrain::StoneFloor).under(Terrain::StoneWall)
    }

    fn room() -> Bounds {
        Bounds::of(Some(40), Some(30))
    }

    /* ---------------- which answer wins ---------------- */

    #[test]
    fn a_drawn_square_is_what_is_there() {
        assert_eq!(
            at(Some(wall()), Some(floor()), room(), 5, 5),
            Some(wall()),
            "the row must beat the default"
        );
    }

    #[test]
    fn an_undrawn_square_inside_the_room_is_the_default() {
        // THE WHOLE POINT. Twelve hundred squares, one recipe, and
        // rows only for the walls.
        assert_eq!(at(None, Some(floor()), room(), 5, 5), Some(floor()));
    }

    #[test]
    fn an_undrawn_square_with_no_default_is_unmapped() {
        // 226 unchanged where no recipe reaches.
        assert_eq!(at(None, None, room(), 5, 5), None);
    }

    #[test]
    fn a_default_does_not_leak_outside_the_room() {
        assert_eq!(at(None, Some(floor()), room(), 40, 5), None);
        assert_eq!(at(None, Some(floor()), room(), -1, 5), None);
        assert_eq!(at(None, Some(floor()), room(), 5, 30), None);
    }

    #[test]
    fn a_row_outside_the_room_is_still_real() {
        // 232: shrinking a map does not delete the east wing. It should
        // look WRONG, not vanish.
        assert_eq!(at(Some(wall()), Some(floor()), room(), 99, 99), Some(wall()));
    }

    /* ---------------- the two bounds questions ---------------- */

    #[test]
    fn an_unmeasured_map_allows_everything_and_encloses_nothing() {
        // THE DISTINCTION THIS MODULE EXISTS TO HOLD. Same map, same
        // square, opposite answers, and both are right.
        let b = Bounds::UNMEASURED;
        assert!(b.allows(500, 500), "nobody said where it stops");
        assert!(!b.encloses(500, 500), "so a default reaches nowhere");
        assert!(!b.measured());
    }

    #[test]
    fn an_unmeasured_map_grows_no_infinite_ground() {
        // Folding the two questions together would make every square
        // in every direction resolve to the default.
        assert_eq!(at(None, Some(floor()), Bounds::UNMEASURED, 0, 0), None);
        assert_eq!(at(None, Some(floor()), Bounds::UNMEASURED, 500, 500), None);
    }

    #[test]
    fn the_far_corner_is_inside_and_the_next_square_is_not() {
        // 0..width-1, so 39 and 29 are the edges of a 40x30 room.
        let b = room();
        assert!(b.encloses(39, 29));
        assert!(!b.encloses(40, 29));
        assert!(!b.encloses(39, 30));
        assert!(b.encloses(0, 0), "(0,0) is the corner, not outside it");
    }

    #[test]
    fn a_negative_coordinate_is_outside_a_measured_room() {
        let b = room();
        assert!(!b.encloses(-1, 0));
        assert!(!b.allows(-1, 0));
    }

    #[test]
    fn half_a_declaration_bounds_half_the_map() {
        // Odd and legal: 232's trigger checks the axes separately too.
        let b = Bounds::of(Some(10), None);
        assert!(b.measured());
        assert!(b.allows(5, 9999), "the height was never declared");
        assert!(!b.allows(10, 0));
        // But a default needs a ROOM, and half a declaration is not one.
        assert!(!b.encloses(5, 5));
        assert_eq!(at(None, Some(floor()), b, 5, 5), None);
    }

    #[test]
    fn allows_and_encloses_agree_once_a_room_is_fully_declared() {
        // The two only differ where something is undeclared. Pinned, so
        // that a future change to one is a deliberate change to both.
        let b = room();
        for (x, y) in [(0, 0), (39, 29), (40, 0), (0, 30), (-1, -1), (5, 5)] {
            assert_eq!(
                b.allows(x, y),
                b.encloses(x, y),
                "({}, {}) disagrees in a fully measured room",
                x,
                y
            );
        }
    }
}
