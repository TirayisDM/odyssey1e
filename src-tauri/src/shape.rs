//! 254. WHICH PIECE OF A WALL A SQUARE IS.
//!
//! Dave, having drawn a corner: *"it needs to be its own cover type as
//! it is wall section but a corner that also needs to be faceable"*.
//!
//! It does need to be its own something. **It does not need to be its
//! own terrain word**, and the difference is the whole of this module.
//!
//! ---------------------------------------------------------------------
//! WHY NOT A TERRAIN WORD
//! ---------------------------------------------------------------------
//!
//! A corner and a straight run of the same wall are **mechanically
//! identical**: the same `Blocked` footing, the same AC 17 and 60 hit
//! points, the same `Leaves(Rubble)` when it comes down, the same
//! blocked line of sight. Every rule in this codebase would treat them
//! the same.
//!
//! `terrain.rs` is a vocabulary where every word earns its place by
//! meaning something different TO A RULE. A corner means something
//! different only to the eye.
//!
//! And it multiplies. `stone_wall_corner` implies the same for timber,
//! rock, wooden and ice - five words, seventeen edits each in
//! `terrain.rs`, and three CHECK constraints in 226 that are applied
//! and therefore need a migration to rewrite. Then ends and
//! T-junctions would be fifteen more.
//!
//! One `shape` on the square says it once, for every material, for
//! every shape there will ever be.
//!
//! ---------------------------------------------------------------------
//! THIS MODULE IS NOT RULES, AND THAT IS WHY IT IS NOT IN terrain.rs
//! ---------------------------------------------------------------------
//!
//! `terrain.rs` holds the physics. **Nothing here changes what a
//! square DOES** - not its footing, not its hit points, not whether
//! you can see through it. A shape changes which PNG gets drawn and
//! nothing else, and putting it beside `Footing` would invite somebody
//! to give it a rule later because of the company it keeps.
//!
//! It is still validated in Rust rather than by a CHECK, which is
//! 238's split: *"the database constrains the RANGE because that is a
//! fact about the type. It leaves the vocabulary alone."* A shape has
//! no range, so the database holds free text and `paint_tiles` refuses
//! an unknown word with a sentence - because a typo that silently fell
//! back to the plain tile is this project's named defect.
//!
//! ---------------------------------------------------------------------
//! THE VOCABULARY FOLLOWS THE ART
//! ---------------------------------------------------------------------
//!
//! Three words, because three sprites exist. A shape nobody has drawn
//! would store as a word the renderer cannot honour and fall back to
//! the plain tile without saying so - 236 settled this for tiles, art
//! arrives one at a time, and the vocabulary arrives WITH it rather
//! than ahead of it.
//!
//! `end` was added within the hour: the first draft of this module
//! listed `corner` and `t` because those were the two files on disk,
//! and `stone_wall_end.png` appeared while the wiring was being
//! written. That is the rule working rather than failing. `cross` is
//! the obvious next one and is still not here.

/// A piece of wall that is not a plain straight run.
///
/// `None` - an absent shape - is the ordinary case and means the plain
/// tile. That is every square painted before 254.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// Two runs meeting at a right angle.
    Corner,
    /// Three runs meeting - a side wall joining a straight run.
    ///
    /// Spelled `t` in the database and in a filename, lower case like
    /// every other key in this project. Dave wrote `stone_wall_T`; the
    /// capital is the only thing that changed.
    Tee,
    /// Where a run of wall stops - a stub with one open side.
    End,
}

/// Every shape, for a picker that must not keep its own copy.
pub const ALL: [Shape; 3] = [Shape::Corner, Shape::Tee, Shape::End];

impl Shape {
    /// As it is stored, and as the filename suffix.
    ///
    /// `stone_wall` + `corner` is `stone_wall_corner.png`. The
    /// renderer builds the name; nothing stores a path.
    pub fn key(self) -> &'static str {
        match self {
            Shape::Corner => "corner",
            Shape::Tee => "t",
            Shape::End => "end",
        }
    }

    /// Read a stored shape, or `None` for an absent one.
    ///
    /// AN EMPTY STRING IS ABSENT, not an error. A picker set to "plain"
    /// sends "" and means it.
    pub fn of_key(k: &str) -> Result<Option<Shape>, String> {
        let k = k.trim();
        if k.is_empty() {
            return Ok(None);
        }
        ALL.iter()
            .find(|s| s.key() == k)
            .copied()
            .map(Some)
            .ok_or_else(|| {
                format!(
                    "{} is not a wall shape - try {}",
                    k,
                    ALL.iter().map(|s| s.key()).collect::<Vec<_>>().join(" or ")
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shape_round_trips_through_its_key() {
        for s in ALL {
            assert_eq!(Shape::of_key(s.key()).unwrap(), Some(s), "{}", s.key());
        }
    }

    #[test]
    fn nothing_is_the_ordinary_case() {
        // A plain straight wall, and every square painted before 254.
        assert_eq!(Shape::of_key("").unwrap(), None);
        assert_eq!(Shape::of_key("   ").unwrap(), None);
    }

    #[test]
    fn the_capital_t_dave_wrote_is_lower_case_here() {
        // Keys in this project are lower case. `stone_wall_T` was the
        // filename he had; `t` is the key.
        assert_eq!(Shape::Tee.key(), "t");
        assert_eq!(Shape::of_key("t").unwrap(), Some(Shape::Tee));
    }

    #[test]
    fn an_unknown_shape_is_refused_with_the_list() {
        // NOT SILENTLY IGNORED. A typo that fell back to the plain
        // tile would be an ordinary-looking square that is quietly
        // wrong - which is the defect this project is named around.
        let e = Shape::of_key("korner").unwrap_err();
        assert!(e.contains("korner"), "{}", e);
        assert!(e.contains("corner"), "it should say what IS allowed: {}", e);
    }

    #[test]
    fn a_shape_not_yet_drawn_is_not_in_the_vocabulary() {
        // `cross` is the obvious next one and is refused until
        // somebody draws it - see the module note. This test is the
        // reminder to move the line when it arrives, which is exactly
        // what happened to `end` an hour after this module was
        // written.
        assert!(Shape::of_key("cross").is_err());
    }

    #[test]
    fn surrounding_space_is_not_part_of_a_key() {
        assert_eq!(Shape::of_key(" corner ").unwrap(), Some(Shape::Corner));
    }
}
