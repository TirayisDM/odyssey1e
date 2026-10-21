//! WHAT A SQUARE IS MADE OF, and what that costs to cross.
//!
//! 225. THE PHYSICS HALF. `grid.rs` holds the arithmetic - how far, what
//! touches, what is on the line - and none of it cares what the ground
//! is. This file is the part that cares, and it is therefore the part
//! that has to agree with 5e.
//!
//! ---------------------------------------------------------------------
//! ONE VOCABULARY, SURFACE AND INTERIOR TOGETHER
//! ---------------------------------------------------------------------
//!
//! The ported `TerrainType` had eighteen variants and every one of them
//! was a dungeon: stone, worked walls, rubble, grates, doorways. **It
//! cannot express a field.** Tirayis has a calendar, seasons and a
//! world above ground, and a vocabulary that can only say "corridor"
//! would have needed a second one the first time anybody walked outside
//! - which is the two-vocabulary fault this project keeps finding, laid
//! down deliberately in advance.
//!
//! So both, in one closed list, decided once.
//!
//! ---------------------------------------------------------------------
//! COST IS AN ENUM, NOT A MULTIPLIER
//! ---------------------------------------------------------------------
//!
//! The ported code carried `movement_cost: f32` - 0.25, 1.5, 2.0 - and
//! a second, unreachable `TerrainMovement` with its own
//! `cost_multiplier: f32`. Two float systems for one fact, neither of
//! them 5e.
//!
//! **5e HAS EXACTLY TWO ANSWERS AND A REFUSAL.** Ordinary ground costs
//! one square. Difficult terrain "costs 1 extra foot for every foot" -
//! which on a grid is two, not 1.5 and not 2.0-as-a-coefficient. A wall
//! costs nothing because you do not enter it. A float invites a third
//! answer nobody wrote down, and rounding it is where the bugs live.
//!
//! ---------------------------------------------------------------------
//! WHAT IS DELIBERATELY NOT HERE
//! ---------------------------------------------------------------------
//!
//! **BREAKABLE, DIGGABLE, FLAMMABLE, max_integrity, tool_required.**
//! The ported table had all five and this project has no object
//! integrity model to hang them on - 218 made a crate a row, and
//! nothing yet gives a wall hit points. They are good rules with no
//! caller, and a rule with no caller is a rule nobody is checking.
//! They come back when something can swing a pickaxe.
//!
//! **challenge_modifier.** It was a percentile-system number and this
//! project resolves on d20 against a DC. Porting it would have carried
//! a second resolution system in through a side door.
//!
//! **A PIT'S FALL.** `Pit` is enterable and carries no damage here,
//! because falling damage depends on HOW FAR - which is a z difference
//! and a movement rule, not a constant property of the ground.

use std::fmt;

/// What it costs to cross a square.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Going {
    /// One square per square.
    Normal,
    /// 5e's "1 extra foot for every foot" - two squares per square.
    Difficult,
    /// Not enterable at all. A wall, not an expensive floor.
    Blocked,
}

impl Going {
    /// Squares of movement spent entering one square of this.
    ///
    /// 225. THE ONE STEP COST, which MAP.md asked for by name after
    /// finding the rule living in whoever happened to call `spend`.
    /// `None` is "you cannot go there" and is not a number, because a
    /// wall priced at `u32::MAX` is a wall somebody will eventually
    /// afford.
    pub fn step_cost(self) -> Option<i64> {
        match self {
            Going::Normal => Some(1),
            Going::Difficult => Some(2),
            Going::Blocked => None,
        }
    }
}

/// How a creature has to be moving to enter a square at all.
///
/// `None` on a [`Footing`] means walking is enough, which is almost
/// everywhere. This is not a cost - a swim speed crossing deep water
/// pays the ordinary rate - it is a GATE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Swim,
    Climb,
    Fly,
}

/// Something that hurts you for being there.
///
/// Dice as a FORMULA STRING, because `dice.rs` already parses those and
/// a second numeric damage representation is the fault this file keeps
/// refusing. `kind` is a `resist::TYPES` word so resistance applies
/// without translation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hazard {
    pub dice: &'static str,
    pub kind: &'static str,
}

/// Everything crossing a square depends on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Footing {
    pub going: Going,
    /// What you must be doing to enter. `None` means walking.
    pub needs: Option<Mode>,
    pub blocks_sight: bool,
    /// Is this the fabric of the room rather than its floor?
    ///
    /// SEPARATE FROM `blocks_sight` ON PURPOSE, and `IronGrate` is why:
    /// it stops a body and not an eye. Folding them into one flag would
    /// make every grate either a window you can walk through or a wall
    /// you cannot see past.
    pub is_wall: bool,
    pub hazard: Option<Hazard>,
}

/// The closed list. Surface, water, worked, hazard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Terrain {
    // ---- surface ----
    Grass,
    Dirt,
    Sand,
    Snow,
    Ice,
    Scree,
    Rock,
    Forest,
    Marsh,
    Road,
    // ---- water ----
    ShallowWater,
    DeepWater,
    // ---- worked ----
    StoneFloor,
    WoodFloor,
    StoneWall,
    WoodWall,
    IronGrate,
    Doorway,
    // ---- hazard ----
    Lava,
    Pit,
    Rubble,
    Vines,
}

/// Every terrain there is, in declaration order.
///
/// A SLICE AND NOT A `match` SOMEWHERE ELSE. The table column is text,
/// and a round trip through Postgres has to be able to name all of them
/// to be checked. See the test that walks this.
pub const ALL: [Terrain; 22] = [
    Terrain::Grass,
    Terrain::Dirt,
    Terrain::Sand,
    Terrain::Snow,
    Terrain::Ice,
    Terrain::Scree,
    Terrain::Rock,
    Terrain::Forest,
    Terrain::Marsh,
    Terrain::Road,
    Terrain::ShallowWater,
    Terrain::DeepWater,
    Terrain::StoneFloor,
    Terrain::WoodFloor,
    Terrain::StoneWall,
    Terrain::WoodWall,
    Terrain::IronGrate,
    Terrain::Doorway,
    Terrain::Lava,
    Terrain::Pit,
    Terrain::Rubble,
    Terrain::Vines,
];

const WALK: Option<Mode> = None;

impl Terrain {
    /// The word stored in `tiles.terrain`.
    ///
    /// SNAKE CASE AND STABLE. These strings are data the moment the
    /// first row is written, so they are spelled out rather than
    /// derived from the variant name - a rename in Rust must not
    /// silently orphan every row in the database. 152's lesson, applied
    /// before there is anything to break.
    pub fn key(self) -> &'static str {
        match self {
            Terrain::Grass => "grass",
            Terrain::Dirt => "dirt",
            Terrain::Sand => "sand",
            Terrain::Snow => "snow",
            Terrain::Ice => "ice",
            Terrain::Scree => "scree",
            Terrain::Rock => "rock",
            Terrain::Forest => "forest",
            Terrain::Marsh => "marsh",
            Terrain::Road => "road",
            Terrain::ShallowWater => "shallow_water",
            Terrain::DeepWater => "deep_water",
            Terrain::StoneFloor => "stone_floor",
            Terrain::WoodFloor => "wood_floor",
            Terrain::StoneWall => "stone_wall",
            Terrain::WoodWall => "wood_wall",
            Terrain::IronGrate => "iron_grate",
            Terrain::Doorway => "doorway",
            Terrain::Lava => "lava",
            Terrain::Pit => "pit",
            Terrain::Rubble => "rubble",
            Terrain::Vines => "vines",
        }
    }

    /// Read a stored word back.
    pub fn of_key(key: &str) -> Option<Terrain> {
        ALL.iter().copied().find(|t| t.key() == key)
    }

    /// What crossing this costs, and what it does to you.
    pub fn footing(self) -> Footing {
        use Going::*;
        use Terrain::*;
        let plain = |going| Footing {
            going,
            needs: WALK,
            blocks_sight: false,
            is_wall: false,
            hazard: None,
        };
        match self {
            // Ordinary ground. A road is the one that is reliably
            // normal whatever the weather, which is what a road is for.
            Grass | Dirt | Rock | Road | StoneFloor | WoodFloor | Doorway => plain(Normal),

            // Difficult terrain, all for the published reason: deep
            // sand, deep snow, loose scree, undergrowth, bog, rubble,
            // and shallow water.
            Sand | Snow | Scree | Forest | Marsh | Rubble | Vines | ShallowWater => {
                plain(Difficult)
            }

            // ICE IS DIFFICULT AND NOTHING ELSE YET. 5e also asks for a
            // DC 10 save or fall prone, and a save belongs with saves -
            // there is nowhere here to roll one.
            Ice => plain(Difficult),

            // Swimming is a GATE, not a surcharge: a creature with a
            // swim speed crosses at the ordinary rate.
            DeepWater => Footing {
                going: Normal,
                needs: Some(Mode::Swim),
                blocks_sight: false,
                is_wall: false,
                hazard: None,
            },

            StoneWall | WoodWall => Footing {
                going: Blocked,
                needs: WALK,
                blocks_sight: true,
                is_wall: true,
                hazard: None,
            },

            // THE WHOLE REASON THE TWO FLAGS ARE SEPARATE. A grate
            // stops a body and not an eye.
            IronGrate => Footing {
                going: Blocked,
                needs: WALK,
                blocks_sight: false,
                is_wall: true,
                hazard: None,
            },

            // Enterable, and a terrible idea. 10d10 fire is the DMG's
            // number for lava.
            Lava => Footing {
                going: Difficult,
                needs: WALK,
                blocks_sight: false,
                is_wall: false,
                hazard: Some(Hazard {
                    dice: "10d10",
                    kind: "fire",
                }),
            },

            // No hazard here on purpose - see the module note. How much
            // a fall hurts depends on how far, which is a z difference.
            Pit => plain(Normal),
        }
    }

    /// Can anything walk, swim or fly into this square at all?
    pub fn enterable(self) -> bool {
        self.footing().going != Going::Blocked
    }

    /// Squares spent entering this one on foot, or `None` if you cannot.
    pub fn step_cost(self) -> Option<i64> {
        self.footing().going.step_cost()
    }

    pub fn blocks_sight(self) -> bool {
        self.footing().blocks_sight
    }

    pub fn is_wall(self) -> bool {
        self.footing().is_wall
    }
}

impl fmt::Display for Terrain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.key())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_terrain_is_in_the_list_exactly_once() {
        // The list is what the migration's CHECK will be built from, so
        // a variant missing from it is a terrain the database refuses.
        let mut seen: Vec<&str> = ALL.iter().map(|t| t.key()).collect();
        let before = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), before, "a key is duplicated");
        assert_eq!(before, 22);
    }

    #[test]
    fn every_key_reads_back_to_the_terrain_it_came_from() {
        // THE ROUND TRIP THROUGH POSTGRES, in one test.
        for t in ALL {
            assert_eq!(Terrain::of_key(t.key()), Some(t), "{}", t.key());
        }
    }

    #[test]
    fn a_word_nobody_wrote_is_not_a_terrain() {
        assert_eq!(Terrain::of_key("grasss"), None);
        assert_eq!(Terrain::of_key("Grass"), None, "keys are lower case");
        assert_eq!(Terrain::of_key(""), None);
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
            assert!(!k.starts_with('_') && !k.ends_with('_'), "{}", k);
        }
    }

    #[test]
    fn ordinary_ground_costs_one_and_difficult_costs_two() {
        // 5e's "1 extra foot for every foot", on a grid.
        assert_eq!(Terrain::Grass.step_cost(), Some(1));
        assert_eq!(Terrain::Road.step_cost(), Some(1));
        assert_eq!(Terrain::Marsh.step_cost(), Some(2));
        assert_eq!(Terrain::Snow.step_cost(), Some(2));
    }

    #[test]
    fn a_wall_has_no_price() {
        // NOT A LARGE NUMBER. A wall costed at u32::MAX is a wall
        // somebody eventually affords.
        assert_eq!(Terrain::StoneWall.step_cost(), None);
        assert!(!Terrain::StoneWall.enterable());
        assert!(!Terrain::IronGrate.enterable());
    }

    #[test]
    fn a_grate_stops_a_body_and_not_an_eye() {
        // The one case that justifies two flags instead of one.
        assert!(Terrain::IronGrate.is_wall());
        assert!(!Terrain::IronGrate.blocks_sight());
        assert!(Terrain::StoneWall.is_wall());
        assert!(Terrain::StoneWall.blocks_sight());
    }

    #[test]
    fn only_walls_block_sight() {
        // Pinned so that adding a terrain that blocks sight without
        // being a wall is a deliberate act with a failing test first.
        for t in ALL {
            if t.blocks_sight() {
                assert!(t.is_wall(), "{} blocks sight but is not a wall", t.key());
            }
        }
    }

    #[test]
    fn deep_water_is_a_gate_and_not_a_surcharge() {
        // A creature with a swim speed pays the ordinary rate.
        let f = Terrain::DeepWater.footing();
        assert_eq!(f.needs, Some(Mode::Swim));
        assert_eq!(f.going, Going::Normal);
        assert!(Terrain::DeepWater.enterable());
    }

    #[test]
    fn shallow_water_is_wading_and_needs_no_swim_speed() {
        let f = Terrain::ShallowWater.footing();
        assert_eq!(f.needs, None);
        assert_eq!(f.going, Going::Difficult);
    }

    #[test]
    fn lava_can_be_entered_and_should_not_be() {
        let f = Terrain::Lava.footing();
        assert!(Terrain::Lava.enterable());
        let h = f.hazard.expect("lava does damage");
        assert_eq!(h.dice, "10d10");
        assert_eq!(h.kind, "fire");
    }

    #[test]
    fn every_hazard_names_a_damage_type_the_project_knows() {
        // Against `resist::TYPES` rather than a list of its own, so
        // resistance applies without translation.
        for t in ALL {
            if let Some(h) = t.footing().hazard {
                assert!(
                    crate::resist::TYPES.contains(&h.kind),
                    "{} deals unknown damage type {}",
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
    fn a_pit_carries_no_damage_because_depth_is_not_a_property_of_ground() {
        // SAID AS A TEST because it looks like an omission otherwise.
        // How far you fall is a z difference, not a constant here.
        assert_eq!(Terrain::Pit.footing().hazard, None);
        assert!(Terrain::Pit.enterable());
    }

    #[test]
    fn nothing_blocked_also_claims_a_cost() {
        for t in ALL {
            let f = t.footing();
            assert_eq!(
                f.going == Going::Blocked,
                t.step_cost().is_none(),
                "{} disagrees with itself about being enterable",
                t.key()
            );
        }
    }

    #[test]
    fn a_doorway_is_a_floor_and_not_a_door() {
        // The door itself is a fixture - 218's vocabulary - and arrives
        // with locks in its own pass. The hole in the wall is terrain.
        assert_eq!(Terrain::Doorway.step_cost(), Some(1));
        assert!(!Terrain::Doorway.is_wall());
        assert!(!Terrain::Doorway.blocks_sight());
    }
}
