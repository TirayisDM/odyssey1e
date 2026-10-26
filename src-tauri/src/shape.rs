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
//! written. That is the rule working rather than failing.
//!
//! ---------------------------------------------------------------------
//! 255. `t` CAME OUT AGAIN, AND center AND diag WENT IN AHEAD OF THE ART
//! ---------------------------------------------------------------------
//!
//! Dave: *"the t junction isn't currently necessary"*. It never had a
//! sprite, which means it was the one word in here breaking the rule
//! above. Nobody had painted one - checked, not assumed: five corners
//! and no tees - so it came out.
//!
//! `center` and `diag` go in **before** their art, which is the
//! opposite of how `end` arrived and worth being honest about. They
//! were asked for by name, so they are not speculation - but until
//! the PNGs exist the renderer falls back to the plain wall, and a
//! square that silently draws the wrong picture is exactly what the
//! rule above protects against.
//!
//! So `map.js` now NAMES a shape it could not draw, the same way it
//! names a tile of the wrong size. The guard moved from the
//! vocabulary to the screen, which is a better place for it: the
//! vocabulary can now follow the artist instead of trailing them.

/// A piece of wall that is not a plain straight run.
///
/// `None` - an absent shape - is the ordinary case and means the plain
/// tile. That is every square painted before 254.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// Two runs meeting at a right angle.
    Corner,
    /// Where a run of wall stops - a stub with one open side.
    End,
    /// Three runs meeting - a stub dropping from a straight run.
    ///
    /// Spelled `t`, lower case like every other key here, because the
    /// key IS the filename suffix. Dave's file was `stone_wall_T.png`
    /// and is `stone_wall_t.png` now.
    ///
    /// 256. IT WENT OUT AND CAME BACK IN THE SAME HOUR, which is the
    /// rule working both ways rather than a mind being changed twice.
    /// 255 removed it on *"the t junction isn't currently necessary"*
    /// when it had no sprite; `stone_wall_t.png` was drawn minutes
    /// later and Dave asked for it back. The word follows the art.
    Tee,
    /// A run through the MIDDLE of the square rather than along an
    /// edge.
    ///
    /// 255. Dave: *"a center wall section will not align with a wall -
    /// we need another wall view - stone_wall_center that can orient
    /// N-S or E-W"*. The ordinary wall tile is a band along one edge
    /// of the square; this one crosses it.
    ///
    /// TWO DISTINCT PICTURES, NOT FOUR. A band through the middle
    /// turned 180 degrees is the same band, so facings 0 and 2 draw
    /// identically and so do 1 and 3. The picker still offers four
    /// because `facing` is one column for every word and a wall is not
    /// the only thing that turns.
    Center,
    /// A run corner to corner.
    ///
    /// 255. Also two distinct pictures rather than four, for the same
    /// reason: a diagonal turned 180 degrees is itself.
    Diag,
}

/// Every shape, for a picker that must not keep its own copy.
pub const ALL: [Shape; 5] =
    [Shape::Corner, Shape::End, Shape::Tee, Shape::Center, Shape::Diag];

impl Shape {
    /// As it is stored, and as the filename suffix.
    ///
    /// `stone_wall` + `corner` is `stone_wall_corner.png`. The
    /// renderer builds the name; nothing stores a path.
    pub fn key(self) -> &'static str {
        match self {
            Shape::Corner => "corner",
            Shape::End => "end",
            Shape::Tee => "t",
            // Dave's spelling, because the key IS the filename suffix
            // and the file is `stone_wall_center.png`.
            Shape::Center => "center",
            Shape::Diag => "diag",
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
    fn the_t_junction_came_back_with_its_sprite() {
        // 255 removed it when it had no art and was said not to be
        // needed; 256 restored it when `stone_wall_t.png` arrived.
        // Both moves are the same rule - the word follows the art.
        assert_eq!(Shape::of_key("t").unwrap(), Some(Shape::Tee));
    }

    #[test]
    fn a_key_is_the_filename_suffix() {
        // `stone_wall` + `center` IS `stone_wall_center.png`. If these
        // ever drift the renderer asks for a file nobody drew.
        assert_eq!(Shape::Center.key(), "center");
        assert_eq!(Shape::Diag.key(), "diag");
        assert_eq!(Shape::End.key(), "end");
        assert_eq!(Shape::Tee.key(), "t");
        assert_eq!(Shape::Corner.key(), "corner");
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
