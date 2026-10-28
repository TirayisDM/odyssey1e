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

/* ========================== WHICH WAY ========================== */

/// Which way a tile points. North is up on the screen.
///
/// 238. SOME TILES ARE NOT SYMMETRICAL. A flight of stairs climbs in a
/// direction; a ledge has an edge you fall off; a doorway sits in one
/// wall rather than all four. The art is drawn once, with **the top of
/// the image as the direction of travel**, and turned a quarter at a
/// time.
///
/// A QUARTER TURN AND NOT AN ANGLE. Three hundred and sixty degrees
/// would let somebody store 37, and a square grid has four sides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Facing {
    North,
    East,
    South,
    West,
}

impl Facing {
    /// From the stored quarter turn, clockwise from north.
    pub fn of(quarter: i16) -> Option<Facing> {
        match quarter {
            0 => Some(Facing::North),
            1 => Some(Facing::East),
            2 => Some(Facing::South),
            3 => Some(Facing::West),
            _ => None,
        }
    }

    pub fn quarter(self) -> i16 {
        match self {
            Facing::North => 0,
            Facing::East => 1,
            Facing::South => 2,
            Facing::West => 3,
        }
    }

    /// Does a step with this delta run along the axis this points down?
    ///
    /// ---------------------------------------------------------------
    /// 238. THE WHOLE REASON A FACING IS A RULE AND NOT A PICTURE
    /// ---------------------------------------------------------------
    ///
    /// A ramp built north-south is climbed by walking north or south.
    /// Walking EAST across it is walking along a slope, not up one, and
    /// should cost the climb it always did.
    ///
    /// **BOTH WAYS ALONG THE AXIS COUNT**, because you walk down a ramp
    /// as well as up it - a facing says which way the art points, not
    /// which way you are allowed to go.
    ///
    /// A DIAGONAL WITH A COMPONENT ALONG THE AXIS COUNTS TOO. Stepping
    /// north-east onto a north-facing ramp is still going up the ramp,
    /// and refusing it would make a creature square up first for no
    /// reason anybody could see.
    pub fn along(self, dx: i32, dy: i32) -> bool {
        match self {
            Facing::North | Facing::South => dy != 0,
            Facing::East | Facing::West => dx != 0,
        }
    }
}

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
            // 263. A WALL IS NOT A FLOOR.
            //
            // This read `ground || self.is_wall()`, so a wall was legal
            // as a surface - sixty hit points of ground, by accident,
            // from one arm covering two layers. Agreed out in 262 and
            // pulled forward here because the damage sweep found what
            // it costs: a wall destroyed as a SURFACE becomes rubble,
            // rubble cannot be a surface, and the square could not be
            // read back. Free to change - no row in `tiles` ever had
            // one.
            //
            // A wall stands ON a floor, which is why `Leaves(Rubble)`
            // works: the debris lands on the surface layer, which is
            // still there because the wall was never in it.
            Layer::Surface | Layer::Sub => ground,
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

    /// Does this word mean something different depending which way it
    /// points?
    ///
    /// 238. A ROCK DOES NOT FACE. Turning a symmetrical tile changes
    /// nothing, so a facing on one is noise somebody will eventually
    /// read as meaningful - and the database stores a quarter turn for
    /// every square whether it means anything or not.
    ///
    /// So this is what `paint_tiles` checks before accepting a turn:
    /// the column holds 0..3 for everybody, and only these words are
    /// allowed to put anything but 0 in it.
    ///
    /// ---------------------------------------------------------------------
    /// 252. AND THE FIVE WALLS JOINED THEM, BECAUSE THE ART FACES
    /// ---------------------------------------------------------------------
    ///
    /// Dave, painting: *"the stone_wall will not place if its
    /// orientation is not N"*. 238 listed eight words and every one of
    /// them turns for a reason the RULES can read - a ramp eases a
    /// climb along its own axis, a doorway sits in one wall, a ledge
    /// has an edge you fall off.
    ///
    /// **A WALL'S REASON IS ONLY THAT IT IS DRAWN.** Its tile has a
    /// top: a face, a capstone, a weathered edge. Turning it changes
    /// nothing any rule asks about and everything the eye asks about,
    /// and 238's own test for this list was never "does a rule read
    /// it" - it was *"does this word mean something different
    /// depending which way it points"*. A directional wall does.
    ///
    /// THE REFUSAL WAS CORRECT AND THE LIST WAS WRONG. `paint_tiles`
    /// was doing exactly what it was told; what it was told predated
    /// the art.
    pub fn faces(self) -> bool {
        use Terrain::*;
        matches!(
            self,
            Doorway
                | Ramp
                | Stairs
                | Ledge
                | DoorOpen
                | DoorClosed
                | DoorBroken
                | IronGrate
                | StoneWall
                | TimberWall
                | RockWall
                | WoodenWall
                | IceWall
        )
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
    /// Which way the square points.
    ///
    /// 238. ON THE SQUARE, NOT ON THE WORD. "Stairs" is a kind of
    /// ground; "these stairs climb north" is a fact about one place.
    /// North for everything that does not face, which is most of it.
    pub facing: Facing,
}

impl Stack {
    pub fn of(surface: Terrain) -> Self {
        Stack {
            sub: None,
            surface,
            cover: None,
            facing: Facing::North,
        }
    }

    pub fn facing(mut self, facing: Facing) -> Self {
        self.facing = facing;
        self
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

    /// What this square becomes when one of its layers is destroyed.
    ///
    /// 263. THE FIRST PLACE A `Destruction` IS ACTUALLY APPLIED.
    /// `destruction()` has described what is left since 226 and nothing
    /// has ever acted on it - the panel turned it into English and that
    /// was all. This is the rule that spends it.
    ///
    /// IT RETURNS A NEW STACK RATHER THAN EDITING THE SQUARE. Destroying
    /// a thing is a question about terrain, which is a rule and belongs
    /// here with tests; writing the answer to Postgres is plumbing and
    /// belongs in a command.
    pub fn destroyed(mut self, layer: Layer) -> Stack {
        use Destruction::*;
        match layer {
            // THE SUB IS OUT OF REACH. There is no hit points column
            // for it - `tiles` has two and the square has three layers -
            // so nothing can have damaged it and this cannot be called
            // in earnest yet. Unchanged rather than a panic: 262 has
            // already agreed it becomes reachable, and the day the
            // column lands this arm is where the rule goes.
            Layer::Sub => self,

            Layer::Cover => {
                let Some(cover) = self.cover else { return self };
                match cover.destruction() {
                    Indestructible => self,
                    // The debris stays in the square, ON the floor the
                    // thing stood on - which the surface layer holds and
                    // this does not touch.
                    Leaves(into) => {
                        self.cover = Some(into);
                        self
                    }
                    Clears => {
                        self.cover = None;
                        self
                    }
                    // A cover is not a lid; there is nothing under it
                    // but the floor, which is already there. No word
                    // currently reaches this arm.
                    Reveals => {
                        self.cover = None;
                        self
                    }
                }
            }

            Layer::Surface => match self.surface.destruction() {
                Indestructible => self,
                Leaves(into) => {
                    self.surface = into;
                    self
                }
                // 263. A FLOOR THAT SIMPLY GOES IS A HOLE. No surface
                // word reaches this arm today; the answer is written
                // down so that the day one does, it is not invented in
                // a hurry.
                Clears => {
                    self.surface = Terrain::Pit;
                    self.cover = None;
                    self
                }
                // THE LID COMES OFF.
                //
                // 262, Dave's ruling: the cover goes with it. Snow lying
                // on ice over deep water does not stay lying there when
                // the ice is gone - "cannot be attacked" means nothing
                // can TARGET the snow, not that the snow survives losing
                // what held it up. True of every cover, not just this
                // one, which is why it is one line here rather than a
                // list of exceptions.
                //
                // AND NO SUB MEANS A PIT. 226 said "ice over nothing is
                // a question the map answers, not this file"; 262
                // answered it.
                Reveals => {
                    self.surface = self.sub.unwrap_or(Terrain::Pit);
                    self.sub = None;
                    self.cover = None;
                    self
                }
            },
        }
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

    /* ---------------- which way it points ---------------- */

    #[test]
    fn a_quarter_turn_reads_back_and_nothing_else_does() {
        for q in 0..4 {
            assert_eq!(Facing::of(q).unwrap().quarter(), q);
        }
        assert_eq!(Facing::of(4), None);
        assert_eq!(Facing::of(-1), None);
    }

    #[test]
    fn a_ramps_axis_is_both_ways_and_not_the_other_one() {
        // A facing says which way the art points, not which way you
        // may go - you walk down a ramp as well as up it.
        assert!(Facing::North.along(0, 1));
        assert!(Facing::North.along(0, -1));
        assert!(!Facing::North.along(1, 0));

        assert!(Facing::East.along(1, 0));
        assert!(Facing::East.along(-1, 0));
        assert!(!Facing::East.along(0, 1));
    }

    #[test]
    fn north_and_south_are_one_axis_and_east_and_west_the_other() {
        // Turning a ramp end for end does not change which way it is
        // climbed, so the two pairs must agree.
        for (dx, dy) in [(0, 1), (1, 0), (1, 1), (0, 0)] {
            assert_eq!(Facing::North.along(dx, dy), Facing::South.along(dx, dy));
            assert_eq!(Facing::East.along(dx, dy), Facing::West.along(dx, dy));
        }
    }

    #[test]
    fn a_diagonal_runs_along_both_axes() {
        assert!(Facing::North.along(1, 1));
        assert!(Facing::East.along(1, 1));
    }

    #[test]
    fn standing_still_runs_along_nothing() {
        for f in [Facing::North, Facing::East, Facing::South, Facing::West] {
            assert!(!f.along(0, 0));
        }
    }

    #[test]
    fn only_the_asymmetrical_words_face() {
        // A rock is the same whichever way it is turned, so storing a
        // turn on one is noise somebody will read as meaningful.
        //
        // 252. THE FIVE WALLS JOINED THE LIST. Their art has a top -
        // a face, a capstone, a weathered edge - so a turn changes
        // what you see even though no rule reads it. The ground and
        // the soft covers still do not face.
        for t in ALL {
            let expected = matches!(
                t,
                Terrain::Doorway
                    | Terrain::Ramp
                    | Terrain::Stairs
                    | Terrain::Ledge
                    | Terrain::DoorOpen
                    | Terrain::DoorClosed
                    | Terrain::DoorBroken
                    | Terrain::IronGrate
                    | Terrain::StoneWall
                    | Terrain::TimberWall
                    | Terrain::RockWall
                    | Terrain::WoodenWall
                    | Terrain::IceWall
            );
            assert_eq!(t.faces(), expected, "{}", t.key());
        }
    }

    #[test]
    fn every_wall_can_be_turned() {
        // 252. Stated against WALLS rather than by listing them again,
        // so a sixth wall added to the vocabulary cannot quietly
        // arrive unable to turn - which is the shape of the bug Dave
        // hit, one list having moved on without the other.
        for t in WALLS {
            assert!(t.faces(), "{} is drawn with a top and must turn", t.key());
        }
    }

    #[test]
    fn the_ground_still_does_not_face() {
        // The half of 238 that has not changed: a turn on open ground
        // is noise. This is the guard that stops 252 being read as
        // "facing is free now".
        for t in [
            Terrain::Rock,
            Terrain::Dirt,
            Terrain::Mud,
            Terrain::Sand,
            Terrain::Road,
            Terrain::StoneFloor,
            Terrain::Grass,
            Terrain::Snow,
        ] {
            assert!(!t.faces(), "{} should not turn", t.key());
        }
    }

    #[test]
    fn everything_that_links_levels_also_faces() {
        // A ramp that could not be turned would only ever climb north.
        for t in ALL {
            if t.links_levels() {
                assert!(t.faces(), "{} eases a climb but cannot be turned", t.key());
            }
        }
    }

    #[test]
    fn a_stack_points_north_until_it_is_turned() {
        assert_eq!(Stack::of(Terrain::Rock).facing, Facing::North);
        assert_eq!(
            Stack::of(Terrain::Ramp).facing(Facing::West).facing,
            Facing::West
        );
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
    fn no_word_is_legal_in_all_three() {
        // 263. THE WALLS USED TO BE, and that was the bug. A word is
        // either something you stand ON or something that stands on it;
        // nothing is both.
        for t in ALL {
            let all_three =
                t.may(Layer::Sub) && t.may(Layer::Surface) && t.may(Layer::Cover);
            assert!(!all_three, "{} is legal in every layer", t.key());
        }
        // The two ground layers remain one list - see the `sub` flag on
        // `Word`, which asks for this rather than assuming it.
        for t in ALL {
            assert_eq!(t.may(Layer::Sub), t.may(Layer::Surface), "{}", t.key());
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
            facing: Facing::North,
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

    /* ============ 263. APPLYING A DESTRUCTION ============ */

    #[test]
    fn the_lid_comes_off_and_the_snow_goes_with_it() {
        // Dave's square: deep water, under ice, under snow.
        let frozen = Stack::of(Terrain::Ice)
            .over(Terrain::DeepWater)
            .under(Terrain::Snow);
        let open = frozen.destroyed(Layer::Surface);

        assert_eq!(open.surface, Terrain::DeepWater, "what was beneath is now the floor");
        assert_eq!(open.sub, None, "and there is nothing beneath it any more");
        assert_eq!(
            open.cover, None,
            "262: a lid takes what sat on it - snow does not float"
        );

        // AND THE CROSSING CHANGES WITH IT, which is the whole point of
        // doing this to a stack rather than to a word.
        assert_eq!(frozen.footing().needs, None, "ice is walked on");
        assert_eq!(open.footing().needs, Some(Mode::Swim), "water is swum");
    }

    #[test]
    fn ice_over_nothing_leaves_a_hole() {
        let thin = Stack::of(Terrain::Ice);
        assert_eq!(thin.destroyed(Layer::Surface).surface, Terrain::Pit);
    }

    #[test]
    fn a_wall_becomes_its_own_debris_and_the_floor_is_untouched() {
        let wall = Stack::of(Terrain::StoneFloor).under(Terrain::StoneWall);
        let after = wall.destroyed(Layer::Cover);
        assert_eq!(after.cover, Some(Terrain::Rubble));
        assert_eq!(after.surface, Terrain::StoneFloor, "debris lands ON the floor");
        // A breach is passable; a wall is not.
        assert_eq!(wall.footing().going, Going::Blocked);
        assert_eq!(after.footing().going, Going::Difficult);
    }

    #[test]
    fn a_door_broken_open_leaves_the_doorway_behind() {
        let door = Stack::of(Terrain::Doorway).under(Terrain::DoorClosed);
        let after = door.destroyed(Layer::Cover);
        assert_eq!(after.cover, None, "clears - nothing remains of it");
        assert_eq!(after.surface, Terrain::Doorway);
        assert!(!after.footing().blocks_sight);
    }

    #[test]
    fn destroying_what_cannot_be_destroyed_changes_nothing() {
        // The command refuses before it gets here, but a rule that is
        // only correct when it is called correctly is not a rule.
        let meadow = Stack::of(Terrain::Dirt).under(Terrain::Grass);
        assert_eq!(meadow.destroyed(Layer::Cover), meadow);
        assert_eq!(meadow.destroyed(Layer::Surface), meadow);
    }

    #[test]
    fn the_sub_is_out_of_reach() {
        // Two hit points columns, three layers. Until that changes this
        // is unreachable, and saying so is better than a half-rule.
        let s = Stack::of(Terrain::Ice).over(Terrain::DeepWater);
        assert_eq!(s.destroyed(Layer::Sub), s);
    }

    #[test]
    fn everything_destructible_lands_somewhere_legal() {
        // THE SWEEP. A destruction that produces an illegal stack would
        // be a square nothing could read back.
        for t in ALL {
            if t.destruction() == Destruction::Indestructible {
                continue;
            }
            if t.may(Layer::Cover) {
                let s = Stack::of(Terrain::StoneFloor).under(t);
                s.destroyed(Layer::Cover)
                    .legal()
                    .unwrap_or_else(|e| panic!("{} as cover: {}", t.key(), e));
            }
            if t.may(Layer::Surface) {
                let s = Stack::of(t).over(Terrain::Rock);
                s.destroyed(Layer::Surface)
                    .legal()
                    .unwrap_or_else(|e| panic!("{} as surface: {}", t.key(), e));
            }
        }
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
        // 263. THE NEWEST FILE THAT DEFINES THESE LISTS, not the one
        // that first wrote them. 115 says an applied migration is a
        // record rather than a draft, so narrowing the ground layers
        // meant a new file - and the guard has to read whichever file
        // is currently the database's copy of the vocabulary.
        let sql = include_str!("../../supabase/migrations/263_a_wall_is_not_a_floor.sql");

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
