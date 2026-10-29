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
/// What a thing is made of.
///
/// 264. SO THIRTEEN HAND-WRITTEN NUMBERS BECOME FIVE FACTS AND A
/// MULTIPLIER. Hit points used to be assigned per WORD, which meant a
/// stone wall and a stone floor could disagree about stone and nothing
/// would notice. A material is asked once and every word made of it
/// agrees by construction.
///
/// `hp` IS FOR A FIVE-FOOT SPAN - one wall square's worth. That is the
/// unit the whole table is in, and it is why a pit works: filling five
/// feet of hole costs what raising five feet of wall costs, because
/// both are five feet of the same arithmetic.
///
/// THE THRESHOLD BELONGS HERE AND DOES NOT SCALE. Dave's ruling: it is
/// how much force the STUFF shrugs off, not how much of it there is.
/// Half a slab of stone is still stone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Material {
    pub ac: i64,
    /// Hit points for a five-foot span.
    pub hp: i64,
    pub threshold: i64,
}

impl Material {
    const fn of(ac: i64, hp: i64, threshold: i64) -> Self {
        Material { ac, hp, threshold }
    }

    /// Masonry and bedrock. One material, because `stone_wall` and
    /// `rock_wall` have never differed by a point.
    pub const STONE: Material = Material::of(17, 60, 10);
    /// Beams and standing timber.
    pub const TIMBER: Material = Material::of(15, 40, 5);
    /// Planks and boards.
    pub const WOOD: Material = Material::of(15, 30, 5);
    pub const ICE: Material = Material::of(13, 30, 5);
    pub const IRON: Material = Material::of(19, 30, 10);
    /// Growing things that are in the way rather than holding anything
    /// up. No threshold - there is nothing in a bramble to shrug a blow
    /// off with.
    pub const GREENERY: Material = Material::of(15, 15, 0);
    /// Packed earth.
    pub const EARTH: Material = Material::of(12, 30, 0);
    /// Sand, scree, mud, marsh - anything that will not hold an edge.
    pub const LOOSE: Material = Material::of(10, 20, 0);
    /// WHAT IT COSTS TO FILL FIVE FEET OF VOLUME.
    ///
    /// 262. Dave's ruling: a pit and a depth of water are destroyed by
    /// FILLING them, at the same rate as building. Sixty is `STONE`'s
    /// span, which is the point - four squares of rubble fill a pit
    /// exactly as four raise a wall.
    ///
    /// ONE MATERIAL FOR BOTH, because the number is about the volume
    /// and not about what is currently in it.
    pub const FILL: Material = Material::of(10, 60, 0);
}

/// How much of a material a square holds.
///
/// 264. A WALL IS A SPAN AND A FLOOR IS A SLAB. Dave's rule - a surface
/// is half - with the one case that already existed as the proof:
/// `ice_wall` was 30 and `ice` was 15 before any of this was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bulk {
    /// A five-foot span: a wall square, or five feet of depth.
    Span,
    /// Half of one. A floor, or water you can stand up in.
    Slab,
    /// 265. A QUARTER, WHICH IS WHAT DESTRUCTION GIVES BACK.
    ///
    /// 262: debris is 25% of what it came from, so a square of rubble
    /// is a quarter span of its material. The consequence Dave kept
    /// rather than rounding away: **four squares of rubble rebuild one
    /// wall**, and you cannot un-break a wall from its own debris.
    Quarter,
}

impl Bulk {
    pub fn of(self, hp: i64) -> i64 {
        match self {
            Bulk::Span => hp,
            Bulk::Slab => hp / 2,
            Bulk::Quarter => hp / 4,
        }
    }
}

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

/// What becomes of the layer itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Remains {
    /// Cannot be attacked. Grass, a lawn, a hole in a wall.
    Indestructible,
    /// It turns into something else in place: a pit filled becomes
    /// ground, deep water drained becomes shallow.
    Becomes(Terrain),
    /// It is simply gone - vines cut away, a grate cut out, a door
    /// broken open, a wall collapsed. The layer becomes empty.
    Clears,
    /// It was a lid. What was beneath it is now the surface.
    ///
    /// ONLY MEANINGFUL ON A SURFACE, and only when that square has a
    /// `sub`. Ice over water becomes water; ice over a road becomes the
    /// road; ice over nothing is a `Pit` - which 226 refused to answer
    /// and 262 answered.
    Reveals,
}

/// What is left when something is destroyed: the layer, and the mess.
///
/// 265. A PAIR, BECAUSE DESTROYING ONE THING DOES TWO THINGS.
///
/// This was a single choice until now, and it could not say what
/// digging a floor actually does: the floor OPENS, revealing what is
/// under it, AND the spoil lands on top of the hole. One answer had to
/// be dropped, and the dropped one was always the debris - which is
/// the half 262's construction economy is built out of.
///
/// IT ALSO SIMPLIFIED THE WALLS. `Leaves(Rubble)` used to mean "the
/// cover layer is replaced by rubble", which bundled two ideas into one
/// variant and then could not be reused for a floor. A wall now
/// `Clears` and leaves `Rubble` behind, which is the same square and a
/// truer sentence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Destruction {
    pub remains: Remains,
    /// What lands on the square, on top of whatever is left. `None` is
    /// a clean break - cut vines, shattered ice, a door off its hinges.
    pub spoil: Option<Terrain>,
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
    /// 265. STONE DEBRIS, and the word keeps its old name on purpose.
    ///
    /// The fork could have made this `rubble_stone` and left `rubble`
    /// to mean nothing in particular. It would have renamed every row
    /// that already says `rubble`, thrown away the art, and bought a
    /// tidier spelling with a migration over live data - for a word
    /// whose picture is grey chunks of masonry and which only walls
    /// and floors have ever produced.
    Rubble,
    /// Splintered beams, from timber and from felled trees.
    RubbleTimber,
    /// Broken boards.
    RubbleWood,
    /// Dug spoil - loose earth, sand, the bed of a drained marsh.
    RubbleEarth,
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
/// -------------------------------------------------------------------
/// 269. WORK IS TIME, EVERYWHERE
/// -------------------------------------------------------------------
///
/// Dave, from having done it: **a five-foot dig in stone is five hours
/// of work, and at most three people can work at it.**
///
/// Stone is sixty hit points a span, so five hours divides into ONE
/// CONSTANT: a hit point of work is five minutes. Twelve an hour. Every
/// other duration in the game then falls out of the material table
/// without a second number - a stone floor is two and a half hours, a
/// dirt floor an hour and a quarter, a timber wall three and twenty,
/// filling a pit five hours, shifting a heap of rubble an hour and a
/// quarter.
///
/// **IT REPRICES 268.** That cycle made building cost one ACTION per
/// heap - four actions and twenty-four seconds for a wall - and said
/// its best property was needing no rate constant at all. It cannot
/// both be true that a heap is an action and that a heap is an hour and
/// a quarter, so the action framing goes. What 268 keeps is the heap
/// CAP: you still cannot place stone you do not have. That was never
/// the rate, it only looked like one.
///
/// The consequence Dave accepted: **no raising a barricade mid-fight.**
/// Which is true to life, and does not touch his ruling that a partial
/// wall blocks - interrupted work is exactly when that matters.
///
/// AND IT CHECKS AGAINST ITSELF. A stone wall is five hours of masonry,
/// and the four heaps that feed it are an hour and a quarter each: five
/// hours of hauling to supply five hours of building. Nothing arranged
/// that - it is 262's four-to-one meeting Dave's rate.
pub const PER_POINT: i64 = 50;

/// How many can swing at one face. Dave's cap, and a physical one.
pub const CREW: i64 = 3;

/// How many workers may share one job.
///
/// REFUSED RATHER THAN CAPPED SILENTLY. A fourth digger who was quietly
/// ignored would make a job take longer than the party was told, and
/// the DM would have no way to see why.
pub fn crew(workers: i64) -> Result<i64, String> {
    match workers {
        n if n < 1 => Err("somebody has to do the work".to_string()),
        n if n > CREW => Err(format!(
            "{} cannot work one square - {} is as many as can swing at a face",
            n, CREW
        )),
        n => Ok(n),
    }
}

/// What a shift of `ticks` by `workers` earns, in hit points of work.
///
/// ROUNDED DOWN, which is why four minutes of digging achieves nothing.
/// A point is five minutes; part of one is part of a point, and the
/// alternative is a free point for anybody who clocks on and off.
pub fn earned(ticks: i64, workers: i64) -> i64 {
    match ticks <= 0 || workers <= 0 {
        true => 0,
        false => ticks.saturating_mul(workers) / PER_POINT,
    }
}

/// What `points` of work costs, in ticks.
///
/// ROUNDED UP, and in the other direction from [`earned`] on purpose.
/// The clock is charged for work actually done, so a job that finishes
/// early is not billed for the shift that was asked for - but neither
/// is a point ever had for less than it costs.
pub fn costs(points: i64, workers: i64) -> i64 {
    match points <= 0 || workers <= 0 {
        true => 0,
        false => {
            let ticks = points.saturating_mul(PER_POINT);
            (ticks + workers - 1) / workers
        }
    }
}

/// What one shift of work moved.
///
/// `wanted` IS WHAT IS STILL MISSING, which each caller works out for
/// itself - a volume counts DOWN to nothing and a wall counts UP to
/// full, and the two cannot share a direction. They share this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Work {
    /// How much went in. Never more than the heap had, never more than
    /// the work still wanted.
    pub placed: i64,
    /// Did this action finish it?
    pub done: bool,
}

impl Work {
    /// `supply` is whatever is scarcest: the time the shift bought, or
    /// the debris there is to place. 269 made the first of those a
    /// number; 268 had only the second.
    pub fn of(supply: i64, wanted: i64) -> Work {
        let placed = supply.min(wanted).max(0);
        Work {
            placed,
            // A SHIFT THAT MOVED NOTHING FINISHES NOTHING. `placed ==
            // wanted` alone would call an empty heap meeting a finished
            // wall a completed day's work.
            done: placed > 0 && placed == wanted,
        }
    }
}

pub const ALL: [Terrain; 36] = [
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
    Terrain::RubbleTimber,
    Terrain::RubbleWood,
    Terrain::RubbleEarth,
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
            RubbleTimber => "rubble_timber",
            RubbleWood => "rubble_wood",
            RubbleEarth => "rubble_earth",
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
            // 265. Every kind of debris walks the same: a heap you
            // pick your way over. What it is MADE of decides what it is
            // worth, not what it costs to cross.
            Snow | Brush | Vines | Rubble | RubbleTimber | RubbleWood | RubbleEarth => {
                Footing::plain(Difficult)
            }

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

    /// What this is made of, and how much of it there is.
    ///
    /// 264. THE ONLY PLACE A WORD MEETS A MATERIAL. `None` is "there is
    /// nothing here to break" - a hole in a wall, the lip of a drop, a
    /// lawn - and is checked against `destruction()` by a test, so the
    /// two tables cannot drift.
    pub fn makeup(self) -> Option<(Material, Bulk)> {
        use Bulk::*;
        use Terrain::*;
        Some(match self {
            // ---- things that stand up ----
            StoneWall | RockWall => (Material::STONE, Span),
            TimberWall | Trees => (Material::TIMBER, Span),
            WoodenWall => (Material::WOOD, Span),
            IceWall => (Material::ICE, Span),
            IronGrate => (Material::IRON, Span),
            Brush | Vines => (Material::GREENERY, Span),

            // ---- things you stand on: half a span, by Dave's rule ----
            Rock | StoneFloor | Road | Stairs | Ramp => (Material::STONE, Slab),
            WoodFloor => (Material::WOOD, Slab),
            Dirt => (Material::EARTH, Slab),
            Sand | Scree | Mud | Marsh => (Material::LOOSE, Slab),
            // THE NUMBER THAT PROVED THE RULE. 30 halved is 15, which
            // is what `ice` was before the table existed.
            Ice => (Material::ICE, Slab),

            // ---- volumes, destroyed by filling them ----
            Pit | DeepWater => (Material::FILL, Span),
            // Ankle deep rather than over your head.
            ShallowWater => (Material::FILL, Slab),

            // ---- nothing to break ----
            //
            // A DOORWAY IS A HOLE IN A WALL; there is no doorway there
            // to attack, only the wall it was cut in. A LEDGE is the lip
            // of a drop, which is a fact about the floor rather than a
            // substance. LAVA is not ruled yet - filling it is a
            // question nobody has answered, and answering it quietly
            // here would be inventing a rule.
            Doorway | Ledge | Lava => return None,
            // 265. DEBRIS IS A QUANTITY NOW. Its hit points ARE its
            // volume: attacking it subtracts, and spending it to build
            // will subtract from the same number. One mechanism, two
            // narrations.
            Rubble => (Material::STONE, Quarter),
            RubbleTimber => (Material::TIMBER, Quarter),
            RubbleWood => (Material::WOOD, Quarter),
            // 265. SPOIL IS LOOSE BY DEFINITION, which is why this is
            // `LOOSE` and not `EARTH` even though packed earth is what
            // most of it is dug out of. Earth that has been dug is no
            // longer packed earth, and the sweep below catches the
            // alternative: debris must never be worth MORE per span
            // than the thing it came out of, or digging a sandbank
            // would quietly mint better fill than the sand was.
            RubbleEarth => (Material::LOOSE, Quarter),

            // Cover that is in the way and nothing more.
            Grass | Snow => return None,
            // A DOOR IS NOT A SLAB OF ANYTHING. Twenty-five where wood
            // is thirty, and it has moving parts - see `integrity`.
            DoorOpen | DoorClosed | DoorBroken => return None,
        })
    }

    /// Can this be attacked, and how tough is it?
    ///
    /// 264. DERIVED FROM `makeup`, with doors as the single exception
    /// the table does not fit.
    pub fn integrity(self) -> Option<Integrity> {
        use Terrain::*;
        // A door is 25 where wood is 30, and it is the only thing in
        // the vocabulary with a MECHANISM - which is what `Wear::Broken`
        // needs to mean anything. Both reasons are about this being a
        // fitting rather than a quantity of material.
        if matches!(self, DoorOpen | DoorClosed | DoorBroken) {
            return Some(Integrity {
                ac: 15,
                hp: 25,
                threshold: 5,
                mechanism: true,
            });
        }
        let (m, bulk) = self.makeup()?;
        Some(Integrity {
            ac: m.ac,
            hp: bulk.of(m.hp),
            threshold: m.threshold,
            mechanism: false,
        })
    }

    /// What is left when it is destroyed.
    pub fn destruction(self) -> Destruction {
        use Remains::*;
        use Terrain::*;
        let (remains, spoil) = match self {
            // ---- things that stand up, and come down ----
            //
            // 265. CLEARS PLUS SPOIL, where this used to say
            // `Leaves(Rubble)`. The wall is gone and its debris is what
            // is left - two facts, and the second one is now the same
            // fact a dug floor produces.
            StoneWall | RockWall => (Clears, Some(Rubble)),
            TimberWall | Trees => (Clears, Some(RubbleTimber)),
            WoodenWall => (Clears, Some(RubbleWood)),
            // Ice shatters. There is nothing left to pile up, which is
            // why a frozen wall gives you nothing to build with.
            IceWall => (Clears, None),
            // Cut away, cut out, broken open. Clean breaks.
            Vines | Brush | IronGrate | DoorOpen | DoorClosed | DoorBroken => (Clears, None),

            // 262. DEBRIS REDUCES TO NOTHING. Dave's words. Spending it
            // is subtracting from its hit points, and at zero the
            // square is clear.
            Rubble | RubbleTimber | RubbleWood | RubbleEarth => (Clears, None),

            // ---- ground, dug through ----
            //
            // 264 made every floor a lid; 265 gives the digging its
            // spoil. The hole AND the heap, which is the whole reason
            // this became a pair.
            Rock | Road | StoneFloor | Ramp | Stairs => (Reveals, Some(Rubble)),
            WoodFloor => (Reveals, Some(RubbleWood)),
            Dirt | Mud | Sand | Scree | Marsh => (Reveals, Some(RubbleEarth)),
            // A lid of ice leaves nothing behind either.
            Ice => (Reveals, None),

            // ---- volumes, destroyed by filling them ----
            //
            // These CONSUME debris rather than making it, which is why
            // none of them has spoil. A pit does not uncover what is
            // beneath it; it stops being a pit.
            Pit => (Becomes(Rock), None),
            DeepWater => (Becomes(ShallowWater), None),
            ShallowWater => (Becomes(Mud), None),

            // ---- nothing to break ----
            Grass | Snow | Doorway | Ledge | Lava => (Indestructible, None),
        };
        Destruction { remains, spoil }
    }

    /// What this leaves lying on the square, and how much of it.
    ///
    /// 262. A QUARTER OF WHAT IT CAME FROM, which is where four-to-one
    /// comes from: a stone wall is 60 and gives back 15, so rebuilding
    /// one takes four squares of debris. Destruction is lossy, and
    /// that is the point - breaching a wall is a decision rather than
    /// a loan.
    ///
    /// THE WORD GIVES THE MAXIMUM AND THE SQUARE STORES THE AMOUNT.
    /// `rubble` is a quarter span of stone, so 15; a stone FLOOR is
    /// half a span and gives 7 of that 15. Both numbers are derived -
    /// nothing has to remember what a heap used to be.
    pub fn spoil(self) -> Option<(Terrain, i64)> {
        let word = self.destruction().spoil?;
        let amount = self.integrity()?.hp / 4;
        Some((word, amount.max(1)))
    }

    /* ====== 268. THE OTHER DIRECTION ====== */

    /// Is this word a heap of debris - something spendable?
    ///
    /// A QUARTER SPAN IS WHAT DEBRIS IS. 265 made that the definition
    /// rather than a list, so this asks the definition. A fifth kind of
    /// rubble would be spendable the day it is added and nothing here
    /// would need editing.
    pub fn is_debris(self) -> bool {
        matches!(self.makeup(), Some((_, Bulk::Quarter)))
    }

    /// Is this ground a volume - destroyed by being FILLED rather than
    /// broken open?
    ///
    /// 262: *"you destroy a pit by using rubble"*. The answer was
    /// already in the table - a pit, deep water and shallow water are
    /// the three words whose `Remains` is `Becomes`, because filling
    /// one IS destroying it. Construction needed no second table for
    /// volumes; it needed to notice that one.
    pub fn is_volume(self) -> bool {
        matches!(self.destruction().remains, Remains::Becomes(_))
    }

    /// Could a heap of `self` raise `wall`?
    ///
    /// ---------------------------------------------------------------
    /// 268. CONSTRUCTION RAISES WALLS, AND ONLY WALLS
    /// ---------------------------------------------------------------
    ///
    /// Two conditions, and together they are one sentence: **debris
    /// raises a BUILT thing of its own material.**
    ///
    /// THE MATERIAL RULE IS THE WHOLE PRICE. It is the same comparison
    /// 265's fork is swept by, read backwards: masonry raises masonry,
    /// splintered beams raise timber, boards raise board walls.
    /// Shattered granite cannot become a timber wall and nobody had to
    /// write that down.
    ///
    /// AND IT NEEDS NO EXCEPTION FOR DUG EARTH. `rubble_earth` is
    /// `LOOSE`, and no cover in the vocabulary is made of loose earth -
    /// so spoil raises nothing, by the same rule that lets masonry
    /// raise a wall, rather than by a line saying so. The same is true
    /// of ice from the other end: an ice wall is `ICE` and nothing
    /// leaves `ICE` debris, so a shattered one is gone for good.
    ///
    /// WALLS, NOT EVERY COVER OF THE RIGHT STUFF. `trees` are `TIMBER`
    /// too, and a heap of splintered beams must not grow one: trees are
    /// grown, a grate is forged, a door is hung. What rubble builds is
    /// masonry and carpentry, which is what `is_wall` already names.
    pub fn may_raise(self, wall: Terrain) -> bool {
        if !self.is_debris() || !wall.is_wall() {
            return false;
        }
        match (self.makeup(), wall.makeup()) {
            (Some((mine, _)), Some((theirs, _))) => mine == theirs,
            _ => false,
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

    /// Can a blow even land on that layer, with the square as it is?
    ///
    /// ---------------------------------------------------------------
    /// 266. A WALL HAS TO COME DOWN BEFORE THE FLOOR UNDER IT
    /// ---------------------------------------------------------------
    ///
    /// Dave, ruling on the behaviour 265 left unruled: *"wall should
    /// have to be destroyed first"*.
    ///
    /// THE RULE IS NOT ABOUT WALLS. Stated as one it would be a second
    /// rule about covers, competing with 262's - which says a lid goes
    /// down the hole with the floor it was lying on. Stated once, with
    /// the edge in the right place, both are the same sentence:
    ///
    ///   **The floor is out of reach while something stands on it that
    ///   could have been attacked in its own right.**
    ///
    /// A cover with hit points is in the way. A cover without them is a
    /// lid, and rides down. Snow and grass are the only covers with no
    /// integrity, and that is now precisely WHY they ride down rather
    /// than an exception to a rule about walls. The edge is one the
    /// player can already see: the layer picker offers exactly the
    /// layers with an AC. If you could have targeted it, you must.
    ///
    /// IT ALSO CLOSES A PRICE. A stone wall is sixty and the stone
    /// floor under it is thirty, so undermining was half price to
    /// remove a wall AND it erased the fifteen rubble instead of
    /// yielding it - the cheap way to delete a wall was to delete the
    /// ground and get nothing for it. See
    /// `undermining_undercut_the_wall_it_went_under`.
    ///
    /// REFUSED WHOLE, not at the last blow. You cannot reach the floor
    /// at all while the wall stands; letting someone whittle it to one
    /// and then discover it can never drop is a trap, and a trap is a
    /// fault wearing the costume of an ordinary answer.
    pub fn reachable(&self, layer: Layer) -> Result<(), String> {
        match layer {
            // Nothing stands on a cover. It is the top of the square by
            // definition, which is also why spoil lands there.
            Layer::Cover => Ok(()),

            Layer::Surface => match self.cover {
                Some(c) if c.integrity().is_some() => Err(format!(
                    "{} stands on it - destroy that first",
                    c.key()
                )),
                _ => Ok(()),
            },

            // THE SAME RULE, NOT A SPECIAL CASE. A surface always
            // exists and always has hit points, so the sub is always
            // under something attackable. It is doubly out of reach -
            // `tiles` has no column to damage it in either - but this
            // answer would stand even once it has one.
            Layer::Sub => Err(format!(
                "{} lies over it - destroy that first",
                self.surface.key()
            )),
        }
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
        use Remains::*;
        let Some(word) = (match layer {
            Layer::Sub => self.sub,
            Layer::Surface => Some(self.surface),
            Layer::Cover => self.cover,
        }) else {
            return self;
        };
        let d = word.destruction();

        match layer {
            // THE SUB IS OUT OF REACH. There is no hit points column
            // for it - `tiles` has two and the square has three layers -
            // so nothing can have damaged it and this cannot be called
            // in earnest yet. Unchanged rather than a panic: 262 has
            // already agreed it becomes reachable, and the day the
            // column lands this arm is where the rule goes.
            Layer::Sub => return self,

            Layer::Cover => match d.remains {
                Indestructible => return self,
                Becomes(into) => self.cover = Some(into),
                // A cover has nothing under it but the floor, which is
                // already there, so a lid and a clean break are the
                // same thing up here.
                Clears | Reveals => self.cover = None,
            },

            Layer::Surface => match d.remains {
                Indestructible => return self,
                Becomes(into) => self.surface = into,
                // 263. A FLOOR THAT SIMPLY GOES IS A HOLE. No surface
                // word reaches this arm today; the answer is written
                // down so that the day one does, it is not invented in
                // a hurry.
                Clears => {
                    self.surface = Terrain::Pit;
                    self.cover = None;
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
                Reveals => {
                    self.surface = self.sub.unwrap_or(Terrain::Pit);
                    self.sub = None;
                    self.cover = None;
                }
            },
        }

        // 265. AND THE SPOIL LANDS ON TOP, whichever layer was broken.
        //
        // AFTER the cover has been cleared, never before: digging a
        // floor takes what was lying on it down the hole and then piles
        // the diggings where it stood. Doing this first would bury the
        // old cover under the new one and keep both.
        if let Some(mess) = d.spoil {
            self.cover = Some(mess);
        }
        self
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
        assert_eq!(before, 36);
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
    fn a_lawn_cannot_be_attacked_but_a_road_can_be_dug_up() {
        // 264. THE ROAD MOVED SIDES. It was indestructible because no
        // ground had hit points; it is masonry at half a span now, and
        // digging through it opens what is under it. Grass is still
        // scenery - there is nothing there to break.
        assert_eq!(Terrain::Grass.integrity(), None);
        assert_eq!(Terrain::Grass.destruction().remains, Remains::Indestructible);

        let road = Terrain::Road.integrity().unwrap();
        assert_eq!(road.hp, 30, "half of stone's span");
        assert_eq!(road.ac, Material::STONE.ac);
        assert_eq!(Terrain::Road.destruction().remains, Remains::Reveals);
    }

    /* ============ 264. THE MATERIAL TABLE ============ */

    #[test]
    fn the_table_reproduces_every_number_that_was_hand_written() {
        // THE TEST THAT EARNS THE REFACTOR ITS KEEP.
        //
        // 262 agreed the material table was worth having only if it
        // gave back the numbers that were already there rather than
        // quietly restating thirteen of them. These are those numbers,
        // copied from the hand-written `integrity` that this replaced.
        for (t, ac, hp, threshold) in [
            (Terrain::StoneWall, 17, 60, 10),
            (Terrain::RockWall, 17, 60, 10),
            (Terrain::TimberWall, 15, 40, 5),
            (Terrain::WoodenWall, 15, 30, 5),
            (Terrain::IceWall, 13, 30, 5),
            (Terrain::Trees, 15, 40, 5),
            (Terrain::Brush, 15, 15, 0),
            (Terrain::Vines, 15, 15, 0),
            (Terrain::IronGrate, 19, 30, 10),
            (Terrain::DoorOpen, 15, 25, 5),
            (Terrain::DoorClosed, 15, 25, 5),
            (Terrain::DoorBroken, 15, 25, 5),
            // ICE KEEPS ITS AC AND ITS HIT POINTS. Only the threshold
            // moved, which is the next test.
            (Terrain::Ice, 13, 15, 5),
        ] {
            let i = t.integrity().unwrap_or_else(|| panic!("{} lost its integrity", t.key()));
            assert_eq!((i.ac, i.hp), (ac, hp), "{}", t.key());
            assert_eq!(i.threshold, threshold, "{} threshold", t.key());
        }
    }

    #[test]
    fn exactly_one_number_changed_and_it_was_ruled() {
        // Ice as a floor had a threshold of 0 while `ice_wall` had 5 -
        // the single value in the table the material rule could not
        // derive. Dave's ruling: a threshold is how much force the
        // STUFF shrugs off, so it does not halve when the slab does.
        assert_eq!(Terrain::Ice.integrity().unwrap().threshold, 5);
        assert_eq!(
            Terrain::Ice.integrity().unwrap().threshold,
            Terrain::IceWall.integrity().unwrap().threshold,
            "one material, one threshold"
        );
    }

    #[test]
    fn a_floor_is_half_the_wall_and_they_share_everything_else() {
        // The rule, and the pair that demonstrated it before it was
        // written down.
        let wall = Terrain::IceWall.integrity().unwrap();
        let floor = Terrain::Ice.integrity().unwrap();
        assert_eq!(floor.hp * 2, wall.hp);
        assert_eq!(floor.ac, wall.ac);
        assert_eq!(floor.threshold, wall.threshold);

        // And it is a rule rather than a coincidence of ice.
        for (span, slab) in [
            (Terrain::StoneWall, Terrain::StoneFloor),
            (Terrain::WoodenWall, Terrain::WoodFloor),
        ] {
            assert_eq!(
                slab.integrity().unwrap().hp * 2,
                span.integrity().unwrap().hp,
                "{}",
                slab.key()
            );
        }
    }

    #[test]
    fn one_material_cannot_disagree_with_itself() {
        // The fault the table exists to make impossible: stone as a
        // wall and stone as a floor differing about stone.
        for a in ALL {
            for b in ALL {
                let (Some((ma, _)), Some((mb, _))) = (a.makeup(), b.makeup()) else {
                    continue;
                };
                if ma == mb {
                    let (ia, ib) = (a.integrity().unwrap(), b.integrity().unwrap());
                    assert_eq!(ia.ac, ib.ac, "{} and {}", a.key(), b.key());
                    assert_eq!(ia.threshold, ib.threshold, "{} and {}", a.key(), b.key());
                }
            }
        }
    }

    #[test]
    fn filling_five_feet_costs_what_raising_five_feet_costs() {
        // 262. Dave's ruling, as arithmetic: a pit is sixty because a
        // span of stone is sixty, so the four squares of rubble that
        // raise a wall are the four that fill a hole.
        assert_eq!(Material::FILL.hp, Material::STONE.hp);
        assert_eq!(Terrain::Pit.integrity().unwrap().hp, 60);
        assert_eq!(Terrain::DeepWater.integrity().unwrap().hp, 60);
        // Water you can stand up in is half the volume.
        assert_eq!(Terrain::ShallowWater.integrity().unwrap().hp, 30);
    }

    #[test]
    fn a_volume_filled_is_not_a_volume_revealed() {
        // A pit does not uncover what is beneath it - it STOPS BEING a
        // pit. That is the difference between breaking a lid and
        // filling a hole, and it is why they are different words.
        assert_eq!(Terrain::Pit.destruction().remains, Remains::Becomes(Terrain::Rock));
        let hole = Stack::of(Terrain::Pit).over(Terrain::Lava);
        let filled = hole.destroyed(Layer::Surface);
        assert_eq!(filled.surface, Terrain::Rock);
        assert_eq!(filled.sub, Some(Terrain::Lava), "what was under it is still under it");
    }

    #[test]
    fn deep_water_drains_in_two_steps() {
        let deep = Stack::of(Terrain::DeepWater);
        let shallow = deep.destroyed(Layer::Surface);
        assert_eq!(shallow.surface, Terrain::ShallowWater);
        assert_eq!(shallow.footing().needs, None, "you can stand up in it");
        let bed = shallow.destroyed(Layer::Surface);
        assert_eq!(bed.surface, Terrain::Mud);
    }

    #[test]
    fn three_things_are_still_waiting_on_a_ruling() {
        // NAMED RATHER THAN FORGOTTEN. Each of these has a reason to be
        // `None` today and a reason it might not stay that way.
        //
        // A DOORWAY is a hole in a wall - there is no doorway to
        // attack, only the wall it was cut in. A LEDGE is the lip of a
        // drop, a fact about a floor rather than a substance. LAVA has
        // no ruling about filling it, and inventing one quietly here
        // would be exactly the kind of fact nobody agreed to.
        for t in [Terrain::Doorway, Terrain::Ledge, Terrain::Lava] {
            assert_eq!(t.makeup(), None, "{}", t.key());
            assert_eq!(t.integrity(), None, "{}", t.key());
        }
    }

    /* ====== 265. THE PAIR, AND WHAT DEBRIS IS WORTH ====== */

    #[test]
    fn digging_a_floor_opens_it_and_leaves_the_diggings() {
        // THE CASE THE PAIR EXISTS FOR. A single-choice `Destruction`
        // could say the hole or the heap and had to drop one; this is
        // both, and the dropped one was always the half 262's whole
        // construction economy is made of.
        let floor = Stack::of(Terrain::StoneFloor).over(Terrain::Rock);
        let dug = floor.destroyed(Layer::Surface);
        assert_eq!(dug.surface, Terrain::Rock, "what was under it is the floor now");
        assert_eq!(dug.sub, None);
        assert_eq!(dug.cover, Some(Terrain::Rubble), "and the spoil is on top of it");
    }

    #[test]
    fn what_was_lying_there_goes_down_the_hole_before_the_spoil_lands() {
        // ORDER MATTERS AND IT IS NOT ARBITRARY. 262 ruled a lid takes
        // its cover with it; the spoil arrives afterwards. Piling first
        // would bury the snow and keep both.
        let snowy = Stack::of(Terrain::Dirt)
            .over(Terrain::Rock)
            .under(Terrain::Snow);
        let dug = snowy.destroyed(Layer::Surface);
        assert_eq!(dug.cover, Some(Terrain::RubbleEarth), "spoil, not snow");
    }

    #[test]
    fn a_wall_says_the_same_square_in_two_facts_now() {
        // `Leaves(Rubble)` and `Clears` + spoil are the same square.
        // The point of the change is that the second sentence can also
        // be said by a floor, and the first never could.
        let wall = Stack::of(Terrain::StoneFloor).under(Terrain::StoneWall);
        let after = wall.destroyed(Layer::Cover);
        assert_eq!(after.cover, Some(Terrain::Rubble));
        assert_eq!(after.surface, Terrain::StoneFloor, "debris lands ON the floor");
    }

    #[test]
    fn debris_is_a_quarter_of_what_it_came_from() {
        // 262. Dave kept the consequence rather than rounding it away:
        // a stone wall is 60 and gives back 15, so FOUR SQUARES OF
        // RUBBLE REBUILD ONE WALL and you cannot un-break a wall from
        // its own debris.
        let (word, amount) = Terrain::StoneWall.spoil().unwrap();
        assert_eq!(word, Terrain::Rubble);
        assert_eq!(amount, 15);
        assert_eq!(Terrain::StoneWall.integrity().unwrap().hp, amount * 4);
    }

    #[test]
    fn the_word_gives_the_maximum_and_the_square_stores_the_amount() {
        // A stone WALL is a span and fills a heap; a stone FLOOR is
        // half a span and half fills one. Both are `rubble`, whose
        // maximum is a quarter span either way - so nothing has to
        // remember what a heap used to be.
        let full = Terrain::StoneWall.spoil().unwrap().1;
        let half = Terrain::StoneFloor.spoil().unwrap().1;
        assert_eq!(full, Terrain::Rubble.integrity().unwrap().hp);
        assert_eq!(half, 7, "half a span, a quarter of it");
        assert!(half < full);
    }

    #[test]
    fn no_heap_is_ever_fuller_than_a_heap_can_be() {
        // THE SWEEP. Spoil is stored in `cover_hp`, and a number above
        // the word's own maximum would make a square whose panel reads
        // "18 / 15" - a remainder larger than the whole.
        for t in ALL {
            let Some((word, amount)) = t.spoil() else { continue };
            let max = word
                .integrity()
                .unwrap_or_else(|| panic!("{} leaves {}, which has no hit points", t.key(), word.key()))
                .hp;
            assert!(
                amount <= max,
                "{} leaves {} of {}, but {} holds {}",
                t.key(),
                amount,
                word.key(),
                word.key(),
                max
            );
            assert!(amount >= 1, "{} leaves nothing measurable", t.key());
        }
    }

    #[test]
    fn debris_reduces_to_nothing() {
        // 262, Dave's words. Its hit points ARE its volume: attacking
        // it and spending it are the same subtraction, and at zero the
        // square is clear. It used to be indestructible, which made a
        // breached wall a dead end - debris nobody could ever shift.
        for t in [
            Terrain::Rubble,
            Terrain::RubbleTimber,
            Terrain::RubbleWood,
            Terrain::RubbleEarth,
        ] {
            assert!(t.integrity().is_some(), "{}", t.key());
            assert_eq!(t.destruction().remains, Remains::Clears, "{}", t.key());
            assert_eq!(t.destruction().spoil, None, "{} must not breed", t.key());
            assert!(t.may(Layer::Cover) && !t.may(Layer::Surface), "{}", t.key());
        }
        let heap = Stack::of(Terrain::Dirt).under(Terrain::Rubble);
        assert_eq!(heap.destroyed(Layer::Cover).cover, None);
    }

    #[test]
    fn what_shatters_or_is_cut_leaves_nothing_to_build_with() {
        // Not every destruction is a quarry. Ice shatters, brambles are
        // cut away, a grate is cut out, a door comes off its hinges.
        for t in [
            Terrain::IceWall,
            Terrain::Ice,
            Terrain::Vines,
            Terrain::Brush,
            Terrain::IronGrate,
            Terrain::DoorClosed,
        ] {
            assert_eq!(t.spoil(), None, "{}", t.key());
        }
    }

    #[test]
    fn filling_a_volume_consumes_debris_rather_than_making_it() {
        // A pit and a depth of water are the OTHER side of the economy:
        // they are destroyed by being filled, so none of them leaves
        // anything behind.
        for t in [Terrain::Pit, Terrain::DeepWater, Terrain::ShallowWater] {
            assert_eq!(t.spoil(), None, "{}", t.key());
            assert!(matches!(t.destruction().remains, Remains::Becomes(_)), "{}", t.key());
        }
    }

    #[test]
    fn every_material_that_leaves_debris_leaves_its_own() {
        // The fork, as a rule rather than a list. Two promises:
        //
        // DEBRIS IS A QUARTER SPAN of something, always - that is what
        // makes four-to-one a rule instead of a number.
        //
        // AND IT NEVER UPGRADES THE MATERIAL. Shattered granite cannot
        // become a timber wall, and nobody had to write that down; the
        // vocabulary refuses it. Going the other way is allowed and
        // happens: dug earth is loose spoil, which is worth less per
        // span than the packed ground it came out of.
        for t in ALL {
            let (Some((mine, _)), Some((word, _))) = (t.makeup(), t.spoil()) else {
                continue;
            };
            let (theirs, bulk) = word.makeup().unwrap();
            assert_eq!(bulk, Bulk::Quarter, "{} is not a quarter span", word.key());
            assert!(
                theirs.hp <= mine.hp,
                "{} leaves {}, which is worth more per span than it was",
                t.key(),
                word.key()
            );
        }

        // And the promise stated plainly for the case it was made for.
        for (wall, debris) in [
            (Terrain::StoneWall, Terrain::Rubble),
            (Terrain::TimberWall, Terrain::RubbleTimber),
            (Terrain::WoodenWall, Terrain::RubbleWood),
        ] {
            let (m, _) = wall.makeup().unwrap();
            assert_eq!(wall.spoil().unwrap().0, debris);
            assert_eq!(debris.makeup().unwrap().0, m, "{}", wall.key());
        }
    }

    /* ====== 268. SPENDING A HEAP ====== */

    /* ====== 269. WHAT WORK COSTS ====== */

    #[test]
    fn five_hours_to_cut_five_feet_of_stone() {
        // DAVE'S NUMBER, WHICH IS WHERE THE RATE COMES FROM. Everything
        // else in this module is derived; this one was measured with a
        // pick.
        let stone = Terrain::StoneWall.integrity().unwrap().hp;
        assert_eq!(stone, 60);
        assert_eq!(costs(stone, 1), 5 * crate::clock::HOUR);
    }

    #[test]
    fn a_point_of_work_is_five_minutes() {
        assert_eq!(PER_POINT, 5 * crate::clock::MINUTE);
        assert_eq!(earned(crate::clock::HOUR, 1), 12);
    }

    #[test]
    fn every_other_duration_falls_out_of_the_material_table() {
        // NO SECOND NUMBER ANYWHERE. These are not written down - they
        // are hit points the table already carried, priced at Dave's
        // rate. If one of them ever reads wrong, the material is wrong.
        let hours = |t: Terrain| costs(t.integrity().unwrap().hp, 1);
        let h = crate::clock::HOUR;
        assert_eq!(hours(Terrain::StoneFloor), 5 * h / 2, "a stone floor");
        assert_eq!(hours(Terrain::Dirt), 5 * h / 4, "a dirt floor");
        assert_eq!(hours(Terrain::TimberWall), 10 * h / 3, "a timber wall");
        assert_eq!(hours(Terrain::Pit), 5 * h, "filling a pit");
        assert_eq!(hours(Terrain::Rubble), 5 * h / 4, "shifting a heap");
    }

    #[test]
    fn a_crew_of_three_and_no_more() {
        assert_eq!(crew(1), Ok(1));
        assert_eq!(crew(3), Ok(3));
        assert!(crew(4).is_err(), "a fourth has nowhere to stand");
        assert!(crew(0).is_err());
        // Three cuts Dave's five hours to one hour forty.
        assert_eq!(costs(60, 3), crate::clock::HOUR + 40 * crate::clock::MINUTE);
    }

    #[test]
    fn part_of_a_point_is_not_a_point() {
        // Four minutes of digging achieves nothing, and the man who
        // clocks on and off all morning achieves nothing all morning.
        assert_eq!(earned(4 * crate::clock::MINUTE, 1), 0);
        assert_eq!(earned(5 * crate::clock::MINUTE, 1), 1);
        assert_eq!(earned(9 * crate::clock::MINUTE, 1), 1);
    }

    #[test]
    fn the_two_directions_never_give_work_away() {
        // `earned` rounds down and `costs` rounds up, which is a trap
        // unless they agree at the boundary: time billed for a job must
        // always buy that job back.
        for points in 1..=60 {
            for workers in 1..=CREW {
                let ticks = costs(points, workers);
                assert!(
                    earned(ticks, workers) >= points,
                    "{} points by {} cost {} ticks and bought {}",
                    points,
                    workers,
                    ticks,
                    earned(ticks, workers)
                );
            }
        }
    }

    #[test]
    fn a_wall_is_five_hours_of_masonry_and_five_hours_of_hauling() {
        // 262'S FOUR-TO-ONE MEETING DAVE'S RATE, and nothing arranged
        // it. The wall wants sixty points of building; the four heaps
        // that supply it are fifteen points of shifting each.
        let wall = Terrain::StoneWall.integrity().unwrap().hp;
        let heap = Terrain::Rubble.integrity().unwrap().hp;
        assert_eq!(costs(wall, 1), costs(heap, 1) * (wall / heap));
    }

    #[test]
    fn a_wall_takes_four_shifts_because_it_takes_four_heaps() {
        // 268'S NUMBER, STILL TRUE WITH 269'S CLOCK ON IT. Four heaps
        // and therefore four trips, whatever each trip costs in hours.
        // The arithmetic lives here and not in the command for the
        // usual reason: a command that wants a test is a rule wearing a
        // wrapper.
        let heap = Terrain::Rubble.integrity().unwrap().hp;
        let full = Terrain::StoneWall.integrity().unwrap().hp;
        let mut has = 0;
        let mut actions = 0;
        while has < full {
            let w = Work::of(heap, full - has);
            has += w.placed;
            actions += 1;
            assert_eq!(w.done, has == full);
            assert!(actions <= 9, "a wall that never finishes");
        }
        assert_eq!(actions, 4);
    }

    #[test]
    fn a_half_empty_heap_buys_half_a_shift() {
        // The cap is what the heap HAS, not what a heap holds - a floor
        // yields seven and a wall yields fifteen, so a wall rebuilt
        // from dug floors takes more trips.
        let (_, dug) = Terrain::StoneFloor.spoil().unwrap();
        assert_eq!(dug, 7);
        let w = Work::of(dug, 60);
        assert_eq!(w.placed, 7);
        assert!(!w.done);
    }

    #[test]
    fn the_last_action_places_only_what_is_missing() {
        // No overfilling, which is what would put a wall above its own
        // maximum and make a panel read "63 of 60".
        let w = Work::of(15, 4);
        assert_eq!(w.placed, 4);
        assert!(w.done);
    }

    #[test]
    fn an_empty_heap_finishes_nothing() {
        // `placed == wanted` alone would call this a completed day's
        // work, and the square would be written as finished for free.
        assert_eq!(Work::of(0, 0), Work { placed: 0, done: false });
        assert_eq!(Work::of(0, 15), Work { placed: 0, done: false });
    }

    #[test]
    fn filling_a_pit_is_the_same_arithmetic_downward() {
        // A volume counts down and a wall counts up, which is the one
        // thing the two arms do not share - so `Work` is given what is
        // MISSING and never has to know which direction it is.
        let heap = Terrain::Rubble.integrity().unwrap().hp;
        let mut left = Terrain::Pit.integrity().unwrap().hp;
        let mut actions = 0;
        while left > 0 {
            let w = Work::of(heap, left);
            left -= w.placed;
            actions += 1;
        }
        assert_eq!(actions, 4, "a pit is a wall's worth of filling");
        assert!(Terrain::Pit.is_volume());
    }


    #[test]
    fn debris_raises_a_built_thing_of_its_own_material() {
        // 262's "use instead of attack", as a rule rather than a list.
        assert!(Terrain::Rubble.may_raise(Terrain::StoneWall));
        assert!(Terrain::Rubble.may_raise(Terrain::RockWall));
        assert!(Terrain::RubbleTimber.may_raise(Terrain::TimberWall));
        assert!(Terrain::RubbleWood.may_raise(Terrain::WoodenWall));
        // Across the grain, never.
        assert!(!Terrain::Rubble.may_raise(Terrain::TimberWall));
        assert!(!Terrain::RubbleWood.may_raise(Terrain::StoneWall));
        assert!(!Terrain::RubbleTimber.may_raise(Terrain::WoodenWall));
    }

    #[test]
    fn a_heap_of_beams_does_not_grow_a_tree() {
        // `trees` ARE timber - the same material, the same span, so the
        // material rule alone would let a pile of splintered beams
        // become a living tree. Trees are grown, a grate is forged, a
        // door is hung; what rubble builds is masonry and carpentry.
        assert_eq!(
            Terrain::Trees.makeup().unwrap().0,
            Terrain::TimberWall.makeup().unwrap().0
        );
        assert!(!Terrain::RubbleTimber.may_raise(Terrain::Trees));
        for t in [Terrain::IronGrate, Terrain::DoorClosed, Terrain::Brush, Terrain::Vines] {
            for heap in ALL.iter().filter(|h| h.is_debris()) {
                assert!(!heap.may_raise(t), "{} raised {}", heap.key(), t.key());
            }
        }
    }

    #[test]
    fn dug_earth_builds_nothing_without_a_rule_saying_so() {
        // THE EXCEPTION THAT IS NOT ONE. Spoil is `LOOSE`, and no cover
        // in the vocabulary is made of loose earth - so it raises
        // nothing BY the material rule rather than by a line excluding
        // it. Dirt fills holes. That is what dirt is for.
        for t in ALL {
            assert!(!Terrain::RubbleEarth.may_raise(t), "{}", t.key());
        }
        assert!(Terrain::Pit.is_volume());
    }

    #[test]
    fn a_shattered_ice_wall_is_gone_for_good() {
        // From the other end of the same rule: an ice wall is ICE and
        // nothing in the world leaves ICE debris, so no heap can raise
        // one. Nobody wrote that either.
        assert_eq!(Terrain::IceWall.spoil(), None);
        for t in ALL {
            assert!(!t.may_raise(Terrain::IceWall), "{}", t.key());
        }
    }

    #[test]
    fn only_a_heap_is_spendable() {
        for t in ALL {
            assert_eq!(
                t.is_debris(),
                t.key().starts_with("rubble"),
                "{}",
                t.key()
            );
        }
    }

    #[test]
    fn the_three_volumes_are_the_three_that_are_filled() {
        // Construction needed no second table. These are the words
        // whose destruction is `Becomes`, which is 262 saying filling
        // one IS destroying it.
        for t in ALL {
            assert_eq!(
                t.is_volume(),
                matches!(t, Terrain::Pit | Terrain::DeepWater | Terrain::ShallowWater),
                "{}",
                t.key()
            );
        }
    }

    #[test]
    fn four_heaps_raise_a_wall_and_no_heap_raises_one_alone() {
        // 262's four-to-one, now spent instead of only produced. A
        // heap holds fifteen, a wall wants sixty, and one action moves
        // at most one heap - so a wall is four actions and four heaps,
        // which is the price and the effort agreeing by construction.
        let heap = Terrain::Rubble.integrity().unwrap().hp;
        let wall = Terrain::StoneWall.integrity().unwrap().hp;
        assert_eq!(wall / heap, 4);
        assert!(heap < wall, "a heap must never finish a wall by itself");
    }

    #[test]
    fn a_wall_being_built_is_a_wall() {
        // NO NEW VOCABULARY, Dave's ruling. A half-raised wall is the
        // wall word holding fewer hit points - so it blocks, and a good
        // blow flattens it. The alternative was an `under_construction`
        // word, which would need its own art, its own footing and its
        // own destruction, and would be a second place the wall's own
        // facts lived.
        let hasty = Stack::of(Terrain::StoneFloor).under(Terrain::StoneWall);
        assert!(hasty.walled());
        assert!(hasty.blocks_sight());
        assert!(!hasty.enterable());
    }

    /* ====== 266. WHAT IS IN THE WAY ====== */

    #[test]
    fn a_wall_has_to_come_down_before_the_floor_under_it() {
        // 266, Dave's ruling. Undermining is out: the wall is the thing
        // in the way, so the wall is the thing you break.
        let wall = Stack::of(Terrain::StoneFloor).under(Terrain::StoneWall);
        let no = wall.reachable(Layer::Surface).unwrap_err();
        assert!(no.contains("stone_wall"), "{}", no);
        assert!(no.contains("destroy that first"), "{}", no);
        // And the wall itself is reachable, so the refusal names a way
        // forward rather than a dead end.
        assert!(wall.reachable(Layer::Cover).is_ok());
        let after = wall.destroyed(Layer::Cover);
        assert_eq!(after.cover, Some(Terrain::Rubble));
        // The heap is attackable too, so it is in the way in its turn -
        // clear the breach and then the floor is yours.
        assert!(after.reachable(Layer::Surface).is_err());
        assert!(after.destroyed(Layer::Cover).reachable(Layer::Surface).is_ok());
    }

    #[test]
    fn a_lid_is_not_in_the_way() {
        // 262 AND 266 ARE ONE SENTENCE. Snow cannot be attacked, so
        // nothing can break it first - which is exactly why it is not
        // in the way and why it rides down the hole. Stated as a rule
        // about walls instead, these two would be rival rules about
        // covers and one of them would eventually win by accident.
        let snowy = Stack::of(Terrain::Ice)
            .over(Terrain::DeepWater)
            .under(Terrain::Snow);
        assert!(snowy.reachable(Layer::Surface).is_ok());
        let gone = snowy.destroyed(Layer::Surface);
        assert_eq!(gone.surface, Terrain::DeepWater);
        assert_eq!(gone.cover, None, "the snow went with the ice");
    }

    #[test]
    fn in_the_way_means_exactly_what_the_player_can_target() {
        // THE SWEEP, and the reason the edge sits where it does: the
        // layer picker offers the layers with an AC, so "if you could
        // have targeted it, you must" is a rule the screen already
        // teaches. A cover that was in the way for some OTHER reason
        // would be an invisible one.
        for t in ALL {
            if !t.may(Layer::Cover) {
                continue;
            }
            let square = Stack::of(Terrain::StoneFloor).under(t);
            assert_eq!(
                square.reachable(Layer::Surface).is_err(),
                t.integrity().is_some(),
                "{} is in the way but cannot be attacked, or the reverse",
                t.key()
            );
        }
    }

    #[test]
    fn a_bare_floor_is_always_reachable_and_a_cover_always_is() {
        for t in ALL {
            if t.may(Layer::Surface) {
                assert!(Stack::of(t).reachable(Layer::Surface).is_ok(), "{}", t.key());
            }
            if t.may(Layer::Cover) {
                let square = Stack::of(Terrain::StoneFloor).under(t);
                assert!(square.reachable(Layer::Cover).is_ok(), "{}", t.key());
            }
        }
    }

    #[test]
    fn nothing_reaches_the_sub() {
        // Not because of the missing column - because the surface is
        // over it, and a surface always exists. The answer survives the
        // day `sub_hp` lands.
        let square = Stack::of(Terrain::Dirt).over(Terrain::Rock);
        let no = square.reachable(Layer::Sub).unwrap_err();
        assert!(no.contains("dirt"), "{}", no);
    }

    #[test]
    fn undermining_undercut_the_wall_it_went_under() {
        // THE PRICE THAT MADE IT A BUG RATHER THAN A STYLE CHOICE. The
        // floor under a wall is worth less than the wall, so digging
        // was the cheap way to remove one - and it yielded dug earth
        // instead of the wall's own fifteen rubble, so the cheap way
        // also destroyed the debris. The rule prices that out.
        let floor = Terrain::StoneFloor.integrity().unwrap().hp;
        let wall = Terrain::StoneWall.integrity().unwrap().hp;
        assert!(floor < wall, "{} vs {}", floor, wall);
        assert!(Terrain::StoneFloor.spoil().unwrap().1 < Terrain::StoneWall.spoil().unwrap().1);
    }

    #[test]
    fn anything_destructible_has_integrity_and_the_reverse() {
        // The two tables cannot drift: a thing with hit points must say
        // what it leaves, and a thing that leaves something must be
        // attackable.
        for t in ALL {
            let has = t.integrity().is_some();
            let dies = t.destruction().remains != Remains::Indestructible;
            assert_eq!(has, dies, "{} disagrees with itself", t.key());
        }
    }

    #[test]
    fn a_wall_collapses_onto_the_floor_it_stood_on() {
        // 265. TWO FACTS WHERE THERE WAS ONE. `Leaves(Rubble)` bundled
        // "the wall is gone" with "there is rubble here" into a single
        // variant, which is why it could never describe a dug floor -
        // that one opens AND leaves spoil, and the old shape had to
        // drop one of them.
        let d = Terrain::StoneWall.destruction();
        assert_eq!(d.remains, Remains::Clears);
        assert_eq!(d.spoil, Some(Terrain::Rubble));
        // and rubble is a cover, so the surface is untouched
        assert!(Terrain::Rubble.may(Layer::Cover));
    }

    #[test]
    fn things_that_are_cut_away_leave_nothing() {
        assert_eq!(Terrain::Vines.destruction().remains, Remains::Clears);
        assert_eq!(Terrain::IronGrate.destruction().remains, Remains::Clears);
        assert_eq!(Terrain::DoorClosed.destruction().remains, Remains::Clears);
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
        assert_eq!(Terrain::Doorway.destruction().remains, Remains::Indestructible);
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
        // 264. A ramp is masonry like the floor it joins; what makes it
        // a ramp is `links_levels`, not being unbreakable.
        assert!(Terrain::Ramp.links_levels());
        assert_eq!(Terrain::Ramp.integrity(), Terrain::StoneFloor.integrity());
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
        // 264. THIS USED TO ASK ICE, which had a threshold of 0 while
        // `ice_wall` had 5 - the one number in the whole table that the
        // material rule could not derive. Dave ruled that a threshold
        // belongs to the MATERIAL and does not scale with how much of
        // it there is, so ice is 5 in both layers now and this asks
        // something that genuinely shrugs off nothing.
        let brush = Terrain::Brush.integrity().unwrap();
        assert_eq!(brush.threshold, 0);
        assert_eq!(brush.lands(1), 1);

        assert_eq!(Terrain::Ice.integrity().unwrap().threshold, 5);
    }

    #[test]
    fn ice_reveals_whatever_the_map_says_is_under_it() {
        // AND NAMES NO TERRAIN, which was the bug in 225: ice over a
        // road is not water.
        assert_eq!(Terrain::Ice.destruction().remains, Remains::Reveals);
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
        // 264. DIRT IS DIGGABLE NOW, so this asks lava - which has no
        // ruling about filling it and therefore no hit points - under a
        // lawn, which is scenery.
        let meadow = Stack::of(Terrain::Lava).under(Terrain::Grass);
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
            if t.destruction().remains == Remains::Indestructible {
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
        // Spoil always lands as a COVER, whichever layer was broken -
        // debris sits on top of a square by definition. A word whose
        // spoil cannot be a cover would make an unreadable stack.
        for t in ALL {
            if let Some(into) = t.destruction().spoil {
                assert!(
                    into.may(Layer::Cover),
                    "{} leaves {}, which cannot be a cover",
                    t.key(),
                    into.key()
                );
            }
            // And a layer that BECOMES something stays in its own
            // layer, so what it becomes has to be legal there.
            if let Remains::Becomes(into) = t.destruction().remains {
                if t.may(Layer::Surface) {
                    assert!(
                        into.may(Layer::Surface),
                        "{} becomes {}, which cannot be a surface",
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
        let sql = include_str!("../../supabase/migrations/265_debris_remembers_what_it_was.sql");

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

