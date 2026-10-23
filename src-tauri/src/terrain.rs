//! WHAT A SQUARE IS MADE OF, in three layers.
//!
//! 226. `grid.rs` holds the arithmetic - how far, what touches, what is
//! on the line - and none of it cares what the ground is. This file is
//! the part that cares, and therefore the part that has to agree with
//! 5e.
//!
//! ---------------------------------------------------------------------
//! SUB, SURFACE, COVER
//! ---------------------------------------------------------------------
//!
//! 225 had one terrain per square and it could not say two things it
//! needed to:
//!
//!   IT SNOWED. `Snow` was a terrain, so a winter map was a DIFFERENT
//!   MAP - you could not say "it snowed" without rewriting every square
//!   and losing the road under it. The calendar has existed since 214
//!   and names four seasons; a vocabulary that cannot survive one of
//!   them is a vocabulary with a season-shaped hole.
//!
//!   A WALL FELL DOWN. Destroying it left `Rubble`, which erased the
//!   floor it had been standing on. 225 wrote that down as a limitation
//!   - "Rubble is the debris, not the floor underneath" - because with
//!   one layer both cannot be true at once.
//!
//! **A WALL SITS ON A FLOOR.** With the wall as a COVER over a surface,
//! both facts hold: the square is blocked while the wall stands, and
//! the floor is still there when it comes down. The same move puts
//! grass on dirt - so burning a field leaves dirt - and trees on
//! ground, which retires `Forest` as a word entirely. A wood is ground
//! with trees on it.
//!
//! ---------------------------------------------------------------------
//! THE LAYERS ARE NOT EQUALS, AND THAT IS THE USEFUL PART
//! ---------------------------------------------------------------------
//!
//! | layer | affects crossing | read when |
//! |---|---|---|
//! | `surface` | yes, it is the ground | always |
//! | `cover` | yes - snow slows, a wall stops | always |
//! | `sub` | **no** | only when the surface is destroyed |
//!
//! Water under ice changes nothing about walking on the ice. It matters
//! the moment the ice breaks and not before, so `sub` takes no part in
//! [`Stack::footing`] at all.
//!
//! ---------------------------------------------------------------------
//! SUB IS AT THE SAME HEIGHT. Z IS NOT.
//! ---------------------------------------------------------------------
//!
//! These answer different questions and neither replaces the other.
//! `sub` is the substrate of THIS square - the water the ice is the lid
//! of, which is at the same height as the ice. `z - 1` is a genuinely
//! lower place, a cellar or a pit floor.
//!
//! **SO THERE IS NO `height` AND NO `depth` HERE.** A ten-foot wall is
//! cover at `z = 0` and again at `z = 1`; a foundation two squares down
//! is `z = -1` and `z = -2`. MAP.md fixed z as a height in five-foot
//! units, and a second number saying how tall something is would be two
//! ways to state one vertical fact.
//!
//! ---------------------------------------------------------------------
//! COST IS AN ENUM, NOT A MULTIPLIER
//! ---------------------------------------------------------------------
//!
//! The reference engine carried `movement_cost: f32` - 0.25, 1.5, 2.0 -
//! and a second, unreachable `TerrainMovement` with its own
//! `cost_multiplier: f32`. Two float systems for one fact, neither of
//! them 5e.
//!
//! **5e HAS TWO ANSWERS AND A REFUSAL.** Ordinary ground costs one
//! square. Difficult terrain "costs 1 extra foot for every foot", which
//! on a grid is two. A wall costs nothing because you do not enter it.

use std::fmt;

/* ======================== WHAT IT COSTS ========================= */

/// What it costs to cross.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Going {
    /// One square per square.
    Normal,
    /// 5e's "1 extra foot for every foot" - two squares per square.
    Difficult,
    /// Not enterable. A wall, not an expensive floor.
    Blocked,
    // ORDERED WORST-LAST ON PURPOSE: `Ord` is what combines the layers,
    // so snow over grass is Difficult and a wall over anything is
    // Blocked, by `max` and not by a match arm per pair.
}

impl Going {
    /// Squares of movement spent entering one square of this.
    ///
    /// `None` is "you cannot go there" and is deliberately not a
    /// number, because a wall priced at `i64::MAX` is a wall somebody
    /// eventually affords.
    pub fn step_cost(self) -> Option<i64> {
        match self {
            Going::Normal => Some(1),
            Going::Difficult => Some(2),
            Going::Blocked => None,
        }
    }
}

/// How a creature must be moving to enter at all.
///
/// A GATE, NOT A SURCHARGE. A creature with a swim speed crosses deep
/// water at the ordinary rate; it simply cannot walk there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Swim,
    Climb,
    Fly,
}

/// Something that hurts you for being there.
///
/// Dice as a FORMULA STRING because `dice.rs` already parses those, and
/// `kind` as a `resist::TYPES` word so resistance applies without
/// translation. A second damage representation is the fault this file
/// keeps refusing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hazard {
    pub dice: &'static str,
    pub kind: &'static str,
}

/// One layer's contribution to crossing a square.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Footing {
    pub going: Going,
    /// What you must be doing to enter. `None` means walking.
    pub needs: Option<Mode>,
    pub blocks_sight: bool,
    pub hazard: Option<Hazard>,
}

impl Footing {
    const fn plain(going: Going) -> Self {
        Footing {
            going,
            needs: None,
            blocks_sight: false,
            hazard: None,
        }
    }
}

/* ===================== WHAT BREAKS, AND INTO WHAT ===================== */

/// Object AC, hit points and damage threshold.
///
/// **AC IS QUOTED FROM THE DMG SUBSTANCE TABLE** - ice and glass 13,
/// wood and bone 15, stone 17, iron and steel 19. Nothing invented.
///
/// **HP AND THRESHOLD ARE TUNED AND SAY SO.** The DMG sizes objects, not
/// five-foot wall sections, so these are a judgement: a fighter at
/// roughly twenty a round is through timber in two and stone in three.
///
/// **THE THRESHOLD IS THE POINT.** Damage below it does nothing at all,
/// which is what stops a dagger felling a stone wall over twenty
/// patient rounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Integrity {
    pub ac: i64,
    pub hp: i64,
    pub threshold: i64,
    /// Does this thing have moving parts that can jam?
    ///
    /// A DOOR DOES; A WALL DOES NOT. It is the difference between
    /// [`Wear::Broken`] being a state a thing can be in and being a
    /// word with no meaning for it - a wall at a tenth of its hit
    /// points is a BREACH, which is passable, and a door at a tenth is
    /// JAMMED, which is not.
    pub mechanism: bool,
}

/// How far gone something is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wear {
    Intact,
    /// Ninety per cent down. Unusable until repaired, and still in the
    /// way - only reachable by something with a [`Integrity::mechanism`].
    Broken,
    Destroyed,
}

impl Integrity {
    /// The hit points at or below which a mechanism jams.
    ///
    /// 226. DERIVED, NOT STORED. "Broken at ninety per cent down" is
    /// ONE RULE, so it is one division rather than a number written
    /// beside every door - which is a number that would eventually
    /// disagree with the rule it came from.
    pub fn breaks_at(&self) -> Option<i64> {
        match self.mechanism {
            true => Some(self.hp / 10),
            false => None,
        }
    }

    /// What `hp_left` hit points means for this thing.
    pub fn wear(&self, hp_left: i64) -> Wear {
        if hp_left <= 0 {
            return Wear::Destroyed;
        }
        match self.breaks_at() {
            Some(at) if hp_left <= at => Wear::Broken,
            _ => Wear::Intact,
        }
    }

    /// How much of a blow actually lands.
    ///
    /// THE THRESHOLD IS THE WHOLE POINT OF HAVING ONE. Damage under it
    /// does nothing AT ALL rather than being reduced, which is what
    /// stops a dagger felling a stone wall over twenty patient rounds.
    pub fn lands(&self, damage: i64) -> i64 {
        match damage < self.threshold {
            true => 0,
            false => damage,
        }
    }
}

/// What is left when something is destroyed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destruction {
    /// Cannot be attacked. Grass, a road, open water.
    Indestructible,
    /// It collapses into debris that stays in the square: a wall into
    /// rubble. The layer is REPLACED, not emptied.
    Leaves(Terrain),
    /// It is simply gone - vines cut away, a grate cut out, a door
    /// broken open. The layer becomes empty.
    Clears,
    /// It was a lid. What was beneath it is now the surface.
    ///
    /// ONLY MEANINGFUL ON A SURFACE, and only when that square has a
    /// `sub`. Ice over water becomes water; ice over a road becomes the
    /// road; ice over nothing is a question the map answers, not this
    /// file - which is exactly why it was wrong to name a terrain here.
    Reveals,
}

/* ========================== THE LAYERS ========================== */

/// Where a word may sit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    /// Beneath the surface, at the same height. Revealed, never crossed.
    Sub,
    /// The ground itself. Every square has exactly one.
    Surface,
    /// What stands on the ground: snow, grass, a tree, a wall.
    Cover,
}

/* ========================= THE VOCABULARY ========================= */

/// The closed list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Terrain {
    // ---- ground you stand on ----
    Rock,
    Dirt,
    Mud,
    Sand,
    Scree,
    Road,
    StoneFloor,
    WoodFloor,
    Marsh,
    Ice,
    ShallowWater,
    DeepWater,
    Lava,
    Pit,
    Ledge,
    /// A gap in a wall, and the edge between two places.
    ///
    /// NOT MERELY "A FLOOR WITH NO COVER", which is what 226 called it
    /// before removing it. A doorway is where a creature LEAVES one
    /// location for another, and nothing else in the vocabulary can
    /// mark that. The reference engine carried the same idea as
    /// `ExitPoint { x, y, link, width }`.
    ///
    /// WHERE IT LEADS IS A FACT, NOT A RULE, so the link lives on the
    /// tile row. This word only says the square is a threshold.
    Doorway,
    /// Ground that makes a change of height a WALK rather than a climb.
    ///
    /// 226. A RAMP IS CONNECTIVITY, NOT GEOMETRY. The map cannot tilt a
    /// square - z is whole five-foot steps and a slope of less than
    /// five feet is scenery 5e has no mechanic for. What a ramp
    /// actually answers is "is this z-transition a climb or a walk",
    /// and that is a property of the square, not of its shape.
    ///
    /// A HILLSIDE IS TERRACED AND THAT IS CORRECT. Stepping up five
    /// feet of open hill is a climb, which 5e already prices at one
    /// extra foot per foot - our `Difficult`. A ramp is the exception
    /// that makes it ordinary.
    Ramp,
    /// Worked steps. The same rule as [`Terrain::Ramp`], built.
    Stairs,
    // ---- things that stand on it ----
    Grass,
    Snow,
    Brush,
    Trees,
    Vines,
    Rubble,
    DoorOpen,
    DoorClosed,
    DoorBroken,
    IronGrate,
    // ---- walls, which may be any of the three ----
    StoneWall,
    TimberWall,
    RockWall,
    WoodenWall,
    IceWall,
}

/// Every terrain there is.
///
/// A SLICE, because the migration's CHECK is built from it and a
/// variant missing here is a terrain the database refuses.
pub const ALL: [Terrain; 33] = [
    Terrain::Rock,
    Terrain::Dirt,
    Terrain::Mud,
    Terrain::Sand,
    Terrain::Scree,
    Terrain::Road,
    Terrain::StoneFloor,
    Terrain::WoodFloor,
    Terrain::Marsh,
    Terrain::Ice,
    Terrain::ShallowWater,
    Terrain::DeepWater,
    Terrain::Lava,
    Terrain::Pit,
    Terrain::Ledge,
    Terrain::Doorway,
    Terrain::Ramp,
    Terrain::Stairs,
    Terrain::Grass,
    Terrain::Snow,
    Terrain::Brush,
    Terrain::Trees,
    Terrain::Vines,
    Terrain::Rubble,
    Terrain::DoorOpen,
    Terrain::DoorClosed,
    Terrain::DoorBroken,
    Terrain::IronGrate,
    Terrain::StoneWall,
    Terrain::TimberWall,
    Terrain::RockWall,
    Terrain::WoodenWall,
    Terrain::IceWall,
];

/// The five walls, which are the only words legal in all three layers.
///
/// A WALL IS THE ONE THING WITH A TOP, A MIDDLE AND A FOOTING. You
/// stand ON a battlement (surface), you are stopped BY a wall in your
/// way (cover), and its foundation continues BELOW the ground around it
/// (sub). z tells the three apart: the battlement is simply a higher
/// row.
pub const WALLS: [Terrain; 5] = [
    Terrain::StoneWall,
    Terrain::TimberWall,
    Terrain::RockWall,
    Terrain::WoodenWall,
    Terrain::IceWall,
];

impl Terrain {
    /// The word stored in the database.
    ///
    /// SPELLED OUT, NOT DERIVED from the variant name. These strings are
    /// data the moment the first row exists, and a rename in Rust must
    /// not silently orphan every row in Postgres. 152's lesson, applied
    /// before there is anything to break.
    pub fn key(self) -> &'static str {
        use Terrain::*;
        match self {
            Rock => "rock",
            Dirt => "dirt",
            Mud => "mud",
            Sand => "sand",
            Scree => "scree",
            Road => "road",
            StoneFloor => "stone_floor",
            WoodFloor => "wood_floor",
            Marsh => "marsh",
            Ice => "ice",
            ShallowWater => "shallow_water",
            DeepWater => "deep_water",
            Lava => "lava",
            Pit => "pit",
            Ledge => "ledge",
            Doorway => "doorway",
            Ramp => "ramp",
            Stairs => "stairs",
            Grass => "grass",
            Snow => "snow",
            Brush => "brush",
            Trees => "trees",
            Vines => "vines",
            Rubble => "rubble",
            DoorOpen => "door_open",
            DoorClosed => "door_closed",
            DoorBroken => "door_broken",
            IronGrate => "iron_grate",
            StoneWall => "stone_wall",
            TimberWall => "timber_wall",
            RockWall => "rock_wall",
            WoodenWall => "wooden_wall",
            IceWall => "ice_wall",
        }
    }

    pub fn of_key(key: &str) -> Option<Terrain> {
        ALL.iter().copied().find(|t| t.key() == key)
    }

    pub fn is_wall(self) -> bool {
        WALLS.contains(&self)
    }

    /// May this word sit in that layer?
    ///
    /// ---------------------------------------------------------------
    /// A WORD OCCUPIES AS FEW LAYERS AS IT CAN
    /// ---------------------------------------------------------------
    ///
    /// Anything legal in two layers is a modelling choice somebody makes
    /// per square, and two people will make it differently - which is
    /// the two-places problem wearing a new costume. So grass is only
    /// ever a cover and dirt is only ever ground, and the database can
    /// refuse the rest.
    ///
    /// WALLS ARE THE DELIBERATE EXCEPTION, for the physical reason in
    /// [`WALLS`]. `sub` otherwise draws from the same list as `surface`,
    /// because a substrate is just the ground that would be revealed -
    /// one vocabulary, two slots, nothing to keep in sync.
    pub fn may(self, layer: Layer) -> bool {
        use Terrain::*;
        let ground = matches!(
            self,
            Rock | Dirt
                | Mud
                | Sand
                | Scree
                | Road
                | StoneFloor
                | WoodFloor
                | Marsh
                | Ice
                | ShallowWater
                | DeepWater
                | Lava
                | Pit
                | Ledge
                | Doorway
                | Ramp
                | Stairs
        );
        match layer {
            Layer::Surface | Layer::Sub => ground || self.is_wall(),
            Layer::Cover => !ground,
        }
    }

    /// This layer's own contribution to crossing the square.
    pub fn footing(self) -> Footing {
        use Going::*;
        use Terrain::*;
        match self {
            // ---- ordinary ground ----
            Rock | Dirt | Road | StoneFloor | WoodFloor | Pit | Ledge | Doorway | Ramp
            | Stairs => {
                Footing::plain(Normal)
            }

            // ---- difficult, each for the published reason ----
            Mud | Sand | Scree | Marsh | ShallowWater => Footing::plain(Difficult),

            // ICE IS DIFFICULT AND NOTHING ELSE YET. 5e also wants a
            // DC 10 save or fall prone, and there is nowhere here to
            // roll one.
            Ice => Footing::plain(Difficult),

            DeepWater => Footing {
                going: Normal,
                needs: Some(Mode::Swim),
                blocks_sight: false,
                hazard: None,
            },

            // Enterable, and a terrible idea. 10d10 is the DMG's lava.
            Lava => Footing {
                going: Difficult,
                needs: None,
                blocks_sight: false,
                hazard: Some(Hazard {
                    dice: "10d10",
                    kind: "fire",
                }),
            },

            // ---- covers that do not impede ----
            Grass => Footing::plain(Normal),

            // ---- covers that do ----
            Snow | Brush | Vines | Rubble => Footing::plain(Difficult),

            // A tree fills the square. You go round it.
            Trees => Footing {
                going: Blocked,
                needs: None,
                blocks_sight: true,
                hazard: None,
            },

            // AN OPEN DOOR IS A HOLE. It is still a door - it can be
            // attacked, shut, and it is still the thing in the gap -
            // but it stops nothing while it stands open.
            DoorOpen => Footing::plain(Normal),

            // BROKEN BLOCKS EXACTLY AS SHUT DOES, and that is the
            // decision worth writing down. A door at a tenth of its hit
            // points is not a hole, it is JAMMED: the hinges have gone
            // and the leaf is wedged in its frame. You repair it to use
            // it again, or you finish destroying it to get through.
            //
            // The obvious wrong answer is that breaking a door opens
            // it, which would make smashing strictly better than
            // picking the lock and retire every locked door in the
            // game.
            DoorClosed | DoorBroken => Footing {
                going: Blocked,
                needs: None,
                blocks_sight: true,
                hazard: None,
            },

            // THE ONE THAT JUSTIFIES TWO SEPARATE FLAGS: a grate stops
            // a body and not an eye.
            IronGrate => Footing {
                going: Blocked,
                needs: None,
                blocks_sight: false,
                hazard: None,
            },

            // ---- walls ----
            // AS A COVER this blocks. AS A SURFACE it is the top of the
            // wall and you are standing on it, which is why `Stack`
            // reads a surface's `going` and a cover's separately.
            StoneWall | TimberWall | RockWall | WoodenWall | IceWall => Footing {
                going: Blocked,
                needs: None,
                blocks_sight: true,
                hazard: None,
            },
        }
    }

    /// Is a change of height in or out of this square a walk?
    ///
    /// THE WHOLE CONTENT OF `Ramp` AND `Stairs`. Without it they are
    /// floors with different names. The movement rule that reads it
    /// does not exist yet - stepping between z levels is Phase 5, and
    /// the same rule owes answers to falling off a [`Terrain::Ledge`],
    /// dropping into a [`Terrain::Pit`] and breaking through a floor.
    pub fn links_levels(self) -> bool {
        matches!(self, Terrain::Ramp | Terrain::Stairs)
    }

    /// Can this be attacked, and how tough is it?
    pub fn integrity(self) -> Option<Integrity> {
        use Terrain::*;
        let solid = |ac, hp, threshold| {
            Some(Integrity {
                ac,
                hp,
                threshold,
                mechanism: false,
            })
        };
        let works = |ac, hp, threshold| {
            Some(Integrity {
                ac,
                hp,
                threshold,
                mechanism: true,
            })
        };
        match self {
            StoneWall | RockWall => solid(17, 60, 10),
            TimberWall => solid(15, 40, 5),
            WoodenWall => solid(15, 30, 5),
            Trees => solid(15, 40, 5),
            IceWall => solid(13, 30, 5),
            Ice => solid(13, 15, 0),
            Vines | Brush => solid(15, 15, 0),
            // THE SAME DOOR THROUGHOUT. All three words are one leaf of
            // timber in one frame; only its position differs, so they
            // must not differ in how hard it is to break.
            DoorOpen | DoorClosed | DoorBroken => works(15, 25, 5),
            // A grate is barred, not hinged - there is nothing in it to
            // jam, so it is cut out rather than broken.
            IronGrate => solid(19, 30, 10),
            _ => None,
        }
    }

    /// What is left when it is destroyed.
    pub fn destruction(self) -> Destruction {
        use Destruction::*;
        use Terrain::*;
        match self {
            // A wall collapses into its own debris, ON the floor it
            // stood on - which the surface layer still holds.
            StoneWall | RockWall | TimberWall | WoodenWall | IceWall | Trees => Leaves(Rubble),
            // Cut away, cut out, broken open. Nothing remains.
            Vines | Brush | IronGrate | DoorOpen | DoorClosed | DoorBroken => Clears,
            // A lid. The map says what is under it.
            Ice => Reveals,
            _ => Indestructible,
        }
    }
}

impl fmt::Display for Terrain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.key())
    }
}

/* ===================== THE THREE, TOGETHER ====================== */

/// One square's three layers.
///
/// Named `Stack` and not `Tile` because a tile is a ROW - it has a
/// location, coordinates and state. This is only the material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stack {
    /// Beneath, at the same height. Takes no part in crossing.
    pub sub: Option<Terrain>,
    /// The ground. Every square has one.
    pub surface: Terrain,
    /// What stands on it.
    pub cover: Option<Terrain>,
}

impl Stack {
    pub fn of(surface: Terrain) -> Self {
        Stack {
            sub: None,
            surface,
            cover: None,
        }
    }

    pub fn under(mut self, cover: Terrain) -> Self {
        self.cover = Some(cover);
        self
    }

    pub fn over(mut self, sub: Terrain) -> Self {
        self.sub = Some(sub);
        self
    }

    /// Is every layer holding a word allowed in it?
    pub fn legal(&self) -> Result<(), String> {
        if !self.surface.may(Layer::Surface) {
            return Err(format!("{} cannot be a surface", self.surface));
        }
        if let Some(c) = self.cover {
            if !c.may(Layer::Cover) {
                return Err(format!("{} cannot be a cover", c));
            }
        }
        if let Some(s) = self.sub {
            if !s.may(Layer::Sub) {
                return Err(format!("{} cannot be a sub", s));
            }
        }
        Ok(())
    }

    /// What crossing this square actually costs.
    ///
    /// THE COMBINATION, in one place:
    ///
    /// - `going` is the WORST of surface and cover, by `Ord`. Snow over
    ///   grass is difficult; a wall over anything is blocked.
    /// - `blocks_sight` if EITHER does.
    /// - `needs` comes from the TOPMOST SOLID LAYER, which is what makes
    ///   ice over deep water walk rather than swim - and makes it swim
    ///   again the moment the ice is gone.
    /// - `hazard` from either, cover first - standing in lava under a
    ///   fallen tree is still lava.
    /// - `sub` TAKES NO PART. It is not under your feet, it is under
    ///   the floor.
    pub fn footing(&self) -> Footing {
        let s = self.surface.footing();
        let Some(cover) = self.cover else { return s };
        let c = cover.footing();
        Footing {
            going: s.going.max(c.going),
            needs: c.needs.or(s.needs),
            blocks_sight: s.blocks_sight || c.blocks_sight,
            hazard: c.hazard.or(s.hazard),
        }
    }

    pub fn step_cost(&self) -> Option<i64> {
        self.footing().going.step_cost()
    }

    pub fn enterable(&self) -> bool {
        self.footing().going != Going::Blocked
    }

    pub fn blocks_sight(&self) -> bool {
        self.footing().blocks_sight
    }

    /// Is there a wall standing in this square?
    ///
    /// THE COVER, NOT THE SURFACE. A surface that is a wall is the top
    /// of one, and you are standing on it.
    pub fn walled(&self) -> bool {
        self.cover.is_some_and(|c| c.is_wall())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /* ---------------- the vocabulary ---------------- */

    #[test]
    fn every_terrain_is_in_the_list_exactly_once() {
        let mut keys: Vec<&str> = ALL.iter().map(|t| t.key()).collect();
        let before = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), before, "a key is duplicated");
        assert_eq!(before, 33);
    }

    #[test]
    fn every_key_reads_back_to_the_terrain_it_came_from() {
        for t in ALL {
            assert_eq!(Terrain::of_key(t.key()), Some(t), "{}", t.key());
        }
    }

    #[test]
    fn keys_are_snake_case_and_nothing_else() {
        for t in ALL {
            let k = t.key();
            assert!(
                k.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "{} is not snake case",
                k
            );
        }
    }

    #[test]
    fn timber_wall_and_wooden_wall_are_not_one_letter_apart() {
        // THE RENAME'S WHOLE PURPOSE. `wood_wall` beside `wooden_wall`
        // in a list that becomes a database CHECK is a name somebody
        // mistypes, in Rust and in SQL.
        assert_eq!(Terrain::TimberWall.key(), "timber_wall");
        assert_eq!(Terrain::WoodenWall.key(), "wooden_wall");
        assert_eq!(Terrain::of_key("wood_wall"), None);
    }

    /* ---------------- which layer ---------------- */

    #[test]
    fn a_word_sits_in_as_few_layers_as_it_can() {
        // Ground is never a cover; a cover is never ground.
        assert!(Terrain::Dirt.may(Layer::Surface));
        assert!(!Terrain::Dirt.may(Layer::Cover));
        assert!(Terrain::Grass.may(Layer::Cover));
        assert!(!Terrain::Grass.may(Layer::Surface));
        assert!(!Terrain::Snow.may(Layer::Surface));
    }

    #[test]
    fn sub_draws_from_the_same_list_as_surface() {
        // One vocabulary, two slots, nothing to keep in sync.
        for t in ALL {
            assert_eq!(
                t.may(Layer::Surface),
                t.may(Layer::Sub),
                "{} differs between surface and sub",
                t.key()
            );
        }
    }

    #[test]
    fn only_walls_are_legal_in_all_three() {
        for t in ALL {
            let all_three =
                t.may(Layer::Sub) && t.may(Layer::Surface) && t.may(Layer::Cover);
            assert_eq!(all_three, t.is_wall(), "{}", t.key());
        }
    }

    #[test]
    fn every_word_is_legal_somewhere() {
        for t in ALL {
            assert!(
                t.may(Layer::Surface) || t.may(Layer::Cover),
                "{} can never be placed",
                t.key()
            );
        }
    }

    #[test]
    fn an_illegal_stack_says_which_layer_is_wrong() {
        let bad = Stack {
            sub: None,
            surface: Terrain::Grass,
            cover: None,
        };
        assert!(bad.legal().unwrap_err().contains("cannot be a surface"));

        let bad = Stack::of(Terrain::Dirt).under(Terrain::Rock);
        assert!(bad.legal().unwrap_err().contains("cannot be a cover"));
    }

    /* ---------------- the combination ---------------- */

    #[test]
    fn it_snowed_and_the_road_is_still_there() {
        // THE REASON LAYERS EXIST. A season sets a cover; nothing
        // underneath is lost.
        let winter = Stack::of(Terrain::Road).under(Terrain::Snow);
        assert_eq!(winter.step_cost(), Some(2));
        assert_eq!(winter.surface, Terrain::Road);
        // and when it melts
        let spring = Stack::of(Terrain::Road);
        assert_eq!(spring.step_cost(), Some(1));
    }

    #[test]
    fn a_wall_stands_on_a_floor_and_both_are_true() {
        let wall = Stack::of(Terrain::StoneFloor).under(Terrain::StoneWall);
        assert!(!wall.enterable());
        assert!(wall.blocks_sight());
        assert!(wall.walled());
        assert_eq!(wall.surface, Terrain::StoneFloor);
    }

    #[test]
    fn the_worst_layer_decides_the_cost() {
        assert_eq!(
            Stack::of(Terrain::Dirt).under(Terrain::Grass).step_cost(),
            Some(1)
        );
        assert_eq!(
            Stack::of(Terrain::Dirt).under(Terrain::Brush).step_cost(),
            Some(2)
        );
        assert_eq!(
            Stack::of(Terrain::Mud).under(Terrain::Grass).step_cost(),
            Some(2),
            "a benign cover does not rescue bad ground"
        );
    }

    #[test]
    fn either_layer_can_block_sight() {
        assert!(Stack::of(Terrain::Dirt).under(Terrain::Trees).blocks_sight());
        assert!(!Stack::of(Terrain::Dirt).under(Terrain::Grass).blocks_sight());
    }

    #[test]
    fn ice_over_deep_water_is_walked_on_and_the_water_is_swum() {
        // THE RULE FALLING OUT RATHER THAN BEING SPECIAL-CASED. The
        // topmost solid layer decides how you move.
        let frozen = Stack::of(Terrain::Ice).over(Terrain::DeepWater);
        assert_eq!(frozen.footing().needs, None, "you walk on ice");
        assert_eq!(frozen.step_cost(), Some(2));

        let open = Stack::of(Terrain::DeepWater);
        assert_eq!(open.footing().needs, Some(Mode::Swim));
    }

    #[test]
    fn sub_takes_no_part_in_crossing() {
        // It is not under your feet, it is under the floor.
        let plain = Stack::of(Terrain::StoneFloor);
        let over_lava = Stack::of(Terrain::StoneFloor).over(Terrain::Lava);
        assert_eq!(plain.footing(), over_lava.footing());
    }

    #[test]
    fn a_hazard_on_either_layer_still_burns() {
        let lava = Stack::of(Terrain::Lava);
        assert_eq!(lava.footing().hazard.unwrap().kind, "fire");
        let under_rubble = Stack::of(Terrain::Lava).under(Terrain::Rubble);
        assert!(under_rubble.footing().hazard.is_some());
    }

    #[test]
    fn a_grate_stops_a_body_and_not_an_eye() {
        let g = Stack::of(Terrain::StoneFloor).under(Terrain::IronGrate);
        assert!(!g.enterable());
        assert!(!g.blocks_sight());
    }

    #[test]
    fn standing_on_a_wall_is_not_standing_in_one() {
        // A wall as SURFACE is a battlement; z tells it from the wall
        // below. `walled` reads the cover for exactly this reason.
        let top = Stack::of(Terrain::StoneWall);
        assert!(!top.walled());
        let through = Stack::of(Terrain::Rock).under(Terrain::StoneWall);
        assert!(through.walled());
    }

    /* ---------------- breaking things ---------------- */

    #[test]
    fn ac_comes_from_the_dmg_substance_table() {
        assert_eq!(Terrain::StoneWall.integrity().unwrap().ac, 17);
        assert_eq!(Terrain::TimberWall.integrity().unwrap().ac, 15);
        assert_eq!(Terrain::IceWall.integrity().unwrap().ac, 13);
        assert_eq!(Terrain::IronGrate.integrity().unwrap().ac, 19);
    }

    #[test]
    fn a_threshold_stops_a_dagger_felling_a_wall() {
        // The whole reason threshold is enforced.
        let stone = Terrain::StoneWall.integrity().unwrap();
        assert_eq!(stone.threshold, 10);
        assert!(4 < stone.threshold, "a dagger never gets through stone");
    }

    #[test]
    fn grass_and_roads_cannot_be_attacked() {
        assert_eq!(Terrain::Grass.integrity(), None);
        assert_eq!(Terrain::Road.integrity(), None);
        assert_eq!(Terrain::Grass.destruction(), Destruction::Indestructible);
    }

    #[test]
    fn anything_destructible_has_integrity_and_the_reverse() {
        // The two tables cannot drift: a thing with hit points must say
        // what it leaves, and a thing that leaves something must be
        // attackable.
        for t in ALL {
            let has = t.integrity().is_some();
            let dies = t.destruction() != Destruction::Indestructible;
            assert_eq!(has, dies, "{} disagrees with itself", t.key());
        }
    }

    #[test]
    fn a_wall_collapses_onto_the_floor_it_stood_on() {
        assert_eq!(
            Terrain::StoneWall.destruction(),
            Destruction::Leaves(Terrain::Rubble)
        );
        // and rubble is a cover, so the surface is untouched
        assert!(Terrain::Rubble.may(Layer::Cover));
    }

    #[test]
    fn things_that_are_cut_away_leave_nothing() {
        assert_eq!(Terrain::Vines.destruction(), Destruction::Clears);
        assert_eq!(Terrain::IronGrate.destruction(), Destruction::Clears);
        assert_eq!(Terrain::DoorClosed.destruction(), Destruction::Clears);
    }

    /* ---------------- doorways and doors ---------------- */

    #[test]
    fn a_doorway_is_ground_and_not_a_cover() {
        // It is the gap in the wall, so it is what you walk ON. The
        // door is the thing standing IN it.
        assert!(Terrain::Doorway.may(Layer::Surface));
        assert!(!Terrain::Doorway.may(Layer::Cover));
        assert_eq!(Terrain::Doorway.footing().going, Going::Normal);
    }

    #[test]
    fn a_doorway_cannot_be_attacked_because_it_is_a_hole() {
        assert_eq!(Terrain::Doorway.integrity(), None);
        assert_eq!(Terrain::Doorway.destruction(), Destruction::Indestructible);
    }

    #[test]
    fn a_door_stands_in_a_doorway() {
        let shut = Stack::of(Terrain::Doorway).under(Terrain::DoorClosed);
        assert!(shut.legal().is_ok());
        assert!(!shut.enterable());
        assert!(shut.blocks_sight());

        let open = Stack::of(Terrain::Doorway).under(Terrain::DoorOpen);
        assert_eq!(open.step_cost(), Some(1));
        assert!(!open.blocks_sight());
    }

    #[test]
    fn a_broken_door_still_blocks() {
        // THE DECISION. A door at a tenth of its hit points is jammed,
        // not open. Breaking a door must not be strictly better than
        // picking its lock, or no locked door in the game matters.
        let broken = Stack::of(Terrain::Doorway).under(Terrain::DoorBroken);
        assert!(!broken.enterable());
        assert!(broken.blocks_sight());
        assert_eq!(
            Terrain::DoorBroken.footing(),
            Terrain::DoorClosed.footing(),
            "broken and shut must block alike"
        );
    }

    #[test]
    fn all_three_doors_are_the_same_leaf_of_timber() {
        // Only the position differs, so they must not differ in how
        // hard they are to break.
        let a = Terrain::DoorOpen.integrity().unwrap();
        assert_eq!(Some(a), Terrain::DoorClosed.integrity());
        assert_eq!(Some(a), Terrain::DoorBroken.integrity());
    }

    #[test]
    fn a_ramp_and_stairs_are_ground_you_simply_walk_on() {
        for t in [Terrain::Ramp, Terrain::Stairs] {
            assert!(t.may(Layer::Surface), "{}", t.key());
            assert!(!t.may(Layer::Cover), "{}", t.key());
            assert_eq!(
                t.footing().going,
                Going::Normal,
                "{} should not be difficult",
                t.key()
            );
        }
    }

    #[test]
    fn only_a_ramp_or_stairs_makes_a_change_of_height_a_walk() {
        // Everything else means a climb, which 5e already prices as
        // difficult. A hillside is terraced and that is correct.
        for t in ALL {
            let eases = matches!(t, Terrain::Ramp | Terrain::Stairs);
            assert_eq!(t.links_levels(), eases, "{}", t.key());
        }
    }

    #[test]
    fn a_ramp_is_not_a_doorway_and_neither_is_a_floor() {
        // Three words that all mean "ordinary ground" and each carry
        // one extra fact. None of them may borrow another's.
        assert!(!Terrain::Doorway.links_levels());
        assert!(!Terrain::StoneFloor.links_levels());
        assert_eq!(Terrain::Ramp.integrity(), None);
    }

    /* ---------------- wear ---------------- */

    #[test]
    fn a_mechanism_jams_at_a_tenth_and_a_wall_does_not() {
        let door = Terrain::DoorClosed.integrity().unwrap();
        assert_eq!(door.hp, 25);
        assert_eq!(door.breaks_at(), Some(2));
        assert_eq!(door.wear(25), Wear::Intact);
        assert_eq!(door.wear(3), Wear::Intact);
        assert_eq!(door.wear(2), Wear::Broken);
        assert_eq!(door.wear(0), Wear::Destroyed);

        // A WALL AT A TENTH IS A BREACH, NOT A JAM.
        let wall = Terrain::StoneWall.integrity().unwrap();
        assert_eq!(wall.breaks_at(), None);
        assert_eq!(wall.wear(1), Wear::Intact);
        assert_eq!(wall.wear(0), Wear::Destroyed);
    }

    #[test]
    fn nothing_is_broken_once_it_is_destroyed() {
        for t in ALL {
            if let Some(i) = t.integrity() {
                assert_eq!(i.wear(0), Wear::Destroyed, "{}", t.key());
                assert_eq!(i.wear(-9), Wear::Destroyed, "{}", t.key());
            }
        }
    }

    #[test]
    fn only_doors_have_a_mechanism() {
        for t in ALL {
            if let Some(i) = t.integrity() {
                let is_door = matches!(
                    t,
                    Terrain::DoorOpen | Terrain::DoorClosed | Terrain::DoorBroken
                );
                assert_eq!(i.mechanism, is_door, "{}", t.key());
            }
        }
    }

    #[test]
    fn a_blow_under_the_threshold_does_nothing_at_all() {
        // NOT REDUCED - NOTHING. That is what a threshold is.
        let stone = Terrain::StoneWall.integrity().unwrap();
        assert_eq!(stone.lands(4), 0);
        assert_eq!(stone.lands(9), 0);
        assert_eq!(stone.lands(10), 10);
        assert_eq!(stone.lands(30), 30);
    }

    #[test]
    fn a_thing_with_no_threshold_takes_everything() {
        let ice = Terrain::Ice.integrity().unwrap();
        assert_eq!(ice.threshold, 0);
        assert_eq!(ice.lands(1), 1);
    }

    #[test]
    fn ice_reveals_whatever_the_map_says_is_under_it() {
        // AND NAMES NO TERRAIN, which was the bug in 225: ice over a
        // road is not water.
        assert_eq!(Terrain::Ice.destruction(), Destruction::Reveals);
    }

    #[test]
    fn whatever_a_wall_leaves_is_legal_where_the_wall_was() {
        // A cover must collapse into something that can BE a cover, or
        // the square ends up holding an illegal stack.
        for t in ALL {
            if let Destruction::Leaves(into) = t.destruction() {
                if t.may(Layer::Cover) {
                    assert!(
                        into.may(Layer::Cover),
                        "{} leaves {}, which cannot be a cover",
                        t.key(),
                        into.key()
                    );
                }
            }
        }
    }

    /* ---------------- and what the database knows ---------------- */

    /// Pull a `check (... in ('a','b'))` list out of the migration.
    #[cfg(test)]
    fn constraint_words(sql: &str, name: &str) -> Vec<String> {
        let at = sql
            .find(&format!("constraint {} check", name))
            .unwrap_or_else(|| panic!("no constraint named {}", name));
        // ANCHOR ON `in (`, NOT THE FIRST BRACKET. The first bracket
        // after `check` opens `(surface in (...))`, so slicing from it
        // glues `(surface in (` onto the first word and the guard
        // silently loses one - which is exactly what it did on its
        // first run, and exactly the kind of near-miss it exists to
        // catch.
        let open = sql[at..].find("in (").expect("constraint has no list") + at + 4;
        let close = sql[open..].find(')').expect("unterminated list") + open;
        sql[open..close]
            .split(',')
            .filter_map(|w| {
                let w = w.trim().trim_matches('\'');
                match w.is_empty() || w.contains(' ') || w.contains('\n') {
                    true => None,
                    false => Some(w.to_string()),
                }
            })
            .collect()
    }

    #[test]
    fn the_migration_and_the_enum_cannot_drift() {
        // 226. THE GUARD THAT MAKES LITERAL CHECK LISTS SAFE.
        //
        // The vocabulary is a RULE and lives here; the database needs a
        // copy of it to refuse nonsense. Two copies of one fact is the
        // fault this project keeps finding, so the second one is CHECKED
        // rather than trusted: add a word to the enum and forget the
        // migration and this fails, instead of a map refusing to save
        // in front of a table of players.
        let sql = include_str!("../../supabase/migrations/226_a_square_in_three_layers.sql");

        for (name, layer) in [
            ("a_surface_is_ground", Layer::Surface),
            ("a_sub_is_ground_too", Layer::Sub),
            ("a_cover_stands_on_something", Layer::Cover),
        ] {
            let mut in_sql = constraint_words(sql, name);
            let mut in_rust: Vec<String> = ALL
                .iter()
                .filter(|t| t.may(layer))
                .map(|t| t.key().to_string())
                .collect();
            in_sql.sort();
            in_rust.sort();
            assert_eq!(in_sql, in_rust, "{} disagrees with terrain::ALL", name);
        }
    }

    /* ---------------- what the project already knows ---------------- */

    #[test]
    fn every_hazard_names_a_damage_type_the_project_knows() {
        for t in ALL {
            if let Some(h) = t.footing().hazard {
                assert!(
                    crate::resist::TYPES.contains(&h.kind),
                    "{} deals unknown damage {}",
                    t.key(),
                    h.kind
                );
            }
        }
    }

    #[test]
    fn every_hazard_rolls_a_formula_dice_can_parse() {
        for t in ALL {
            if let Some(h) = t.footing().hazard {
                assert!(
                    crate::dice::roll_formula(h.dice).is_ok(),
                    "{} has unparseable dice {}",
                    t.key(),
                    h.dice
                );
            }
        }
    }

    #[test]
    fn nothing_blocked_also_claims_a_cost() {
        for t in ALL {
            let f = t.footing();
            assert_eq!(
                f.going == Going::Blocked,
                f.going.step_cost().is_none(),
                "{} disagrees with itself about being enterable",
                t.key()
            );
        }
    }

    #[test]
    fn a_pit_and_a_ledge_carry_no_damage_because_depth_is_not_a_property_of_ground() {
        // How far you fall is a z difference, not a constant here.
        assert_eq!(Terrain::Pit.footing().hazard, None);
        assert_eq!(Terrain::Ledge.footing().hazard, None);
    }
}


