//! Who is holding what: turning an entity id into something readable.
//!
//! 031 made `objects.holder_id` point at an ENTITY rather than at a
//! character, and 032 and 033 added the other two kinds. That was the
//! right shape - a sword in a chest in a room needs no special case -
//! and it left one thing genuinely hard: saying, on screen, where a
//! thing is.
//!
//! An entity id names nothing. Resolving it means asking which of three
//! tables owns that id, and the answer can be any of them:
//!
//!   a character   Rodnar is carrying it
//!   a container   it is in the iron chest, which is itself an object
//!   a location    it is lying on the floor of the Frostvalley Inn
//!
//! THE CONTAINER CASE IS WHY THIS IS NOT A JOIN. A container is an
//! object that also has an entity, so the thing that names the holder
//! lives in the very table being resolved. One pass over the objects
//! builds half the index and the other half comes from characters and
//! locations - which is a fold, not a query, and belongs here where it
//! can be tested rather than in a command that cannot.
//!
//! AND WHERE A THING ULTIMATELY IS. `resolve` answers one step - the
//! coin is in the purse - and `root_of` walks the rest of the way: the
//! purse is in the backpack, the backpack is on Rodnar, so the coin is
//! ultimately Rodnar's. That walk is what decides whether two things
//! are within reach of each other, which is the first question putting
//! one inside the other has to answer.
//!
//! NOTHING IS EVER DROPPED. Two holders can be invisible to the caller:
//! RLS hides a character in another game, and a container the DM has
//! since destroyed leaves rows pointing at an id that no longer names
//! anything. Both come back as `Unknown` rather than being filtered
//! away, for the same reason `locations::arrange` makes a root out of a
//! parent it cannot see: a screen that silently omits what it cannot
//! explain is worse than one that says it does not know.

use serde::{Deserialize, Serialize};

/* ============================ TYPES ============================ */

/// An object as the manager screen asks for it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Obj {
    pub id: String,
    pub item_key: String,
    /// Null on the interchangeable ones - see objects.rs. A named thing
    /// never merges into a stack.
    pub name: Option<String>,
    pub quantity: i64,
    pub equipped: bool,
    /// Attuned, which is a fact about the CARRIER rather than about
    /// where the thing is - three across everything somebody holds, at
    /// any depth. Carried here because this is the only loader that
    /// walks a whole game's objects, which is what counting them needs.
    pub attuned: bool,
    /// The entity it rests in. None is nowhere at all, which 033 kept as
    /// a real answer rather than a gap.
    pub holder_id: Option<String>,
    /// Its OWN entity, present only when this object is a container.
    /// That is what makes this table both the question and half the
    /// answer.
    pub entity_id: Option<String>,
    /// 036. None means take it from the catalogue, which is what almost
    /// every object does.
    pub size_override: Option<String>,
    /// 036, containers only. None means ask the type - and the TYPE's
    /// own None is what means unrestricted. Two different absences, one
    /// behind the other.
    pub holds_size_override: Option<String>,
}

/// What the CATALOGUE says about a key. The half of an object that is
/// true of every one of them.
// NOT Eq, because slots is an f64 and a float has no total equality.
// PartialEq is what the tests compare with and is honest about it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kind {
    pub key: String,
    pub size: String,
    pub holds_size: Option<String>,
    pub weight: Option<String>,
    /// How much room one of these takes up inside something. Fractional
    /// on purpose - a coin is 0.2 of a slot, so a 5-slot purse holds 25.
    pub slots: f64,
    /// How much room it HAS, when it is a container. None is an
    /// unfinished catalogue row rather than "bottomless", which is 032's
    /// decision and the opposite of what None means for `holds_size`.
    pub capacity_slots: Option<f64>,
}

/// Something that can hold: a character, a container or a place.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Holder {
    pub entity_id: String,
    /// "character", "container" or "location". The same vocabulary
    /// `entities.kind` uses, so a reader comparing the two sees one
    /// word rather than two spellings of it.
    pub kind: String,
    pub name: String,
}

/// An object with its holder said out loud.
// Not Eq: see Kind. Slot fills are floats because a coin is 0.2.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Located {
    pub id: String,
    pub item_key: String,
    pub name: Option<String>,
    pub quantity: i64,
    pub equipped: bool,
    /// True when this object is itself a container - the screen offers
    /// to look inside.
    pub is_container: bool,
    /// Its own entity, when it is a container. What `holder_id` points
    /// at for everything inside it, which is how a screen can tell
    /// "this container" from "the container this is already in".
    pub entity_id: Option<String>,
    /// "character", "container", "location", "nowhere" or "unknown".
    pub holder_kind: String,
    /// What to print. "nowhere" and "unknown" carry their own words so
    /// a caller never has to decide what a blank means.
    pub holder_name: String,
    /// Kept so a click can act on the holder rather than re-derive it.
    pub holder_id: Option<String>,
    /// How big this one is, the override applied. Empty when the
    /// catalogue has no row for the key - which is a broken object
    /// rather than a sizeless one, and says so by being blank.
    pub size: String,
    /// The largest thing it takes, if it takes anything and there is a
    /// limit. None on a container means no limit; None on anything else
    /// means it is not a container.
    pub holds_size: Option<String>,
    /// Pounds for ONE of them. The screen multiplies by quantity,
    /// because a stack's weight is a rendering and not a fact.
    pub weight: Option<String>,
    /// WHERE THIS ULTIMATELY IS, as one comparable string - see
    /// Root::token. Two objects sharing it are within reach of each
    /// other, which is what lets a picker offer only the containers a
    /// thing could actually go into. EMPTY MEANS OFFER NOTHING.
    pub reach: String,
    /// The same root in words, for saying so.
    pub reach_name: String,
    /// Containers only: how much room is in use, by the SAME
    /// arithmetic that refuses the next thing - containers::slot_total.
    /// A gauge computed any other way is a gauge that reads half empty
    /// while the container says no.
    ///
    /// Direct contents only, which is what `fits` counts too: a purse
    /// inside a pack costs the pack the purse's own slots, not the
    /// slots of what is in the purse.
    pub used_slots: Option<f64>,
    /// Containers only: how much room there is. None IS A FAULT rather
    /// than "infinite" - see Kind - and the screen says so instead of
    /// drawing an empty gauge.
    pub capacity_slots: Option<f64>,
}

/// Where a thing ultimately is, once every container between it and
/// the world has been walked through.
///
/// FOUR ANSWERS AND THEY ARE ALL DIFFERENT. Nowhere is a real place -
/// 033 kept it deliberately - and Unknown is the admission that the
/// chain ran into something this caller cannot see. Collapsing those
/// two would make an invisible holder look like an empty field, which
/// is the mistake this module's header is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Root {
    /// Somebody is carrying it, however deep in their kit.
    Character { entity: String, name: String },
    /// It is lying in a place, or in a chest in that place.
    Location { entity: String, name: String },
    /// Held by nobody and in no place. Still a real answer.
    Nowhere,
    /// The chain ran into a holder nobody can name, or a loop.
    Unknown,
}

impl Root {
    /// What to call it in a sentence.
    pub fn name(&self) -> &str {
        match self {
            Root::Character { name, .. } | Root::Location { name, .. } => name,
            Root::Nowhere => "nowhere",
            Root::Unknown => UNKNOWN,
        }
    }

    /// The whole root as one comparable string, for a screen.
    ///
    /// WHY A TOKEN AND NOT A PAIR OF FIELDS. A picker offering the
    /// containers a thing can reach has to ask "same place?", and
    /// asking it with kind-and-id means `within_reach` written a second
    /// time in JavaScript - which is the rule in two places, one of
    /// them untested. One string, one comparison, and the rule stays
    /// here.
    ///
    /// UNKNOWN IS THE EMPTY STRING, and empty never equals empty for
    /// this purpose because the caller is told to treat it as "offer
    /// nothing". That mirrors `within_reach` refusing Unknown even
    /// against itself: two unreadable chains have not been shown to
    /// meet.
    pub fn token(&self) -> String {
        match self {
            Root::Character { entity, .. } => format!("c:{}", entity),
            Root::Location { entity, .. } => format!("l:{}", entity),
            Root::Nowhere => "nowhere".to_string(),
            Root::Unknown => String::new(),
        }
    }

    /// The identity two roots are compared on. Names collide - two
    /// goblins are both "Goblin" - and an entity id does not.
    fn key(&self) -> Option<&str> {
        match self {
            Root::Character { entity, .. } | Root::Location { entity, .. } => Some(entity),
            _ => None,
        }
    }
}

/* ============================ RULES ============================ */

/// What to show for something being held by an id nobody can name.
///
/// NOT AN ERROR AND NOT A FILTER. RLS hides holders in other games, and
/// a destroyed container can leave rows behind for as long as it takes
/// the cascade to run. Either way the object exists and the DM should
/// be able to see it and move it somewhere real.
const UNKNOWN: &str = "somewhere unaccounted for";

/// Say where every object is.
///
/// `extra` carries the holders this table cannot supply itself -
/// characters and locations. Containers are found in `objects` during
/// the same pass, because a container IS one.
///
/// `types` is the catalogue, by key. 036 put size on the TYPE with an
/// optional override on the instance, so the effective answer is a
/// coalesce - and a coalesce is a rule, which is why it happens here
/// with a test rather than on the screen.
pub fn resolve(objects: &[Obj], extra: &[Holder], types: &[Kind]) -> Vec<Located> {
    // The index: everything that can hold, by entity.
    let mut index: Vec<Holder> = extra.to_vec();
    for o in objects {
        if let Some(e) = o.entity_id.as_deref() {
            index.push(Holder {
                entity_id: e.to_string(),
                kind: "container".to_string(),
                name: label(o),
            });
        }
    }

    objects
        .iter()
        .map(|o| {
            let (kind, name) = match o.holder_id.as_deref() {
                // 033's answer, kept deliberately: unheld is a place to
                // be, not a missing value.
                None => ("nowhere".to_string(), "nowhere".to_string()),
                Some(h) => match index.iter().find(|x| x.entity_id == h) {
                    Some(f) => (f.kind.clone(), f.name.clone()),
                    None => ("unknown".to_string(), UNKNOWN.to_string()),
                },
            };
            let t = types.iter().find(|t| t.key == o.item_key);
            // The walk, per object. O(depth) each and the depth is a
            // purse in a pack - cheaper than the round trip that asking
            // the database per link would cost.
            let root = root_of(o.holder_id.as_deref(), objects, &index);

            // HOW FULL, when this is a container. Its direct contents
            // are the objects whose holder is its own entity.
            let (used, capacity) = match o.entity_id.as_deref() {
                None => (None, None),
                Some(mine) => {
                    let inside: Vec<(f64, i64)> = objects
                        .iter()
                        .filter(|x| x.holder_id.as_deref() == Some(mine))
                        .map(|x| {
                            (
                                types
                                    .iter()
                                    .find(|t| t.key == x.item_key)
                                    // A key nobody catalogued takes one
                                    // slot, which is the column default.
                                    // Zero would make an uncatalogued
                                    // hoard look weightless.
                                    .map(|t| t.slots)
                                    .unwrap_or(1.0),
                                x.quantity,
                            )
                        })
                        .collect();
                    (
                        Some(crate::containers::slot_total(&inside)),
                        t.and_then(|t| t.capacity_slots),
                    )
                }
            };
            Located {
                id: o.id.clone(),
                item_key: o.item_key.clone(),
                name: o.name.clone(),
                quantity: o.quantity,
                equipped: o.equipped,
                is_container: o.entity_id.is_some(),
                entity_id: o.entity_id.clone(),
                holder_kind: kind,
                holder_name: name,
                holder_id: o.holder_id.clone(),
                // THE INSTANCE WINS. A giant's dagger is a shortsword to
                // anybody else, and the override is the only way to say
                // so while there is no screen for writing a catalogue
                // row.
                size: o
                    .size_override
                    .clone()
                    .or_else(|| t.map(|t| t.size.clone()))
                    .unwrap_or_default(),
                holds_size: o
                    .holds_size_override
                    .clone()
                    .or_else(|| t.and_then(|t| t.holds_size.clone())),
                weight: t.and_then(|t| t.weight.clone()),
                reach: root.token(),
                reach_name: root.name().to_string(),
                used_slots: used,
                capacity_slots: capacity,
            }
        })
        .collect()
}

/// What to call an object. Its given name if it has one, otherwise the
/// catalogue key it came off.
///
/// The same precedence every screen uses, written once here so the
/// holder label and the row label cannot disagree about what a chest is
/// called.
pub fn label(o: &Obj) -> String {
    match o.name.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(n) => n.to_string(),
        None => o.item_key.clone(),
    }
}

/// Walk up from a holder until the world is reached.
///
/// A container is an object with an entity, so a holder that matches
/// one of THOSE means "keep going" - the purse is in the backpack is on
/// Rodnar. A character or a location ends the walk, and so does running
/// out of chain.
///
/// GUARDED AT 32, the same number `no_location_cycles` uses. A loop
/// should be impossible - `no_container_cycles` refuses one in the
/// database - but a walk that trusts that is a walk that hangs the day
/// it is wrong, and returning Unknown says exactly as much as is known.
pub fn root_of(start: Option<&str>, objects: &[Obj], holders: &[Holder]) -> Root {
    let mut here = match start {
        None => return Root::Nowhere,
        Some(h) => h.to_string(),
    };

    for _ in 0..32 {
        if let Some(h) = holders.iter().find(|x| x.entity_id == here) {
            return match h.kind.as_str() {
                "character" => Root::Character {
                    entity: h.entity_id.clone(),
                    name: h.name.clone(),
                },
                "location" => Root::Location {
                    entity: h.entity_id.clone(),
                    name: h.name.clone(),
                },
                // A container in the index. Keep climbing.
                _ => match objects.iter().find(|o| o.entity_id.as_deref() == Some(&here)) {
                    Some(o) => match o.holder_id.as_deref() {
                        Some(next) => {
                            here = next.to_string();
                            continue;
                        }
                        // A chest resting on nothing at all.
                        None => Root::Nowhere,
                    },
                    None => Root::Unknown,
                },
            };
        }

        // Not in the index at all, but it may still be a container we
        // know from the objects themselves.
        match objects.iter().find(|o| o.entity_id.as_deref() == Some(&here)) {
            Some(o) => match o.holder_id.as_deref() {
                Some(next) => here = next.to_string(),
                None => return Root::Nowhere,
            },
            None => return Root::Unknown,
        }
    }

    Root::Unknown
}

/// Whether a thing and a container are close enough for one to go in
/// the other.
///
/// THE TOP-LEVEL GATE, and it runs before what a container accepts, how
/// big it is or how much room is left. Those three ask whether the
/// thing BELONGS in it; this asks whether anybody could put it there at
/// all, and no answer to the other three matters if the chest is in a
/// different building.
///
/// The rule in one line: THE SAME ROOT. A thing held by Rodnar goes in
/// a container held by Rodnar, however deep either sits in his kit. A
/// thing lying in the Frostvalley Inn goes in a chest in the Frostvalley
/// Inn. Nothing crosses between the two.
///
/// TWO THINGS THAT ARE BOTH NOWHERE ARE REACHABLE. That looks odd and
/// it is deliberate: 033 made nowhere a real answer rather than a gap,
/// and a DM tidying what was dropped before there was anywhere to drop
/// it should not have to place both first.
///
/// UNKNOWN IS ALWAYS REFUSED, even against itself. Two chains that both
/// vanish into something nobody can see have not been shown to meet -
/// they have been shown to be unreadable, which is not the same and
/// must not pass for it.
pub fn within_reach(
    thing: &Root,
    container: &Root,
    thing_name: &str,
    container_name: &str,
) -> Result<(), String> {
    if matches!(thing, Root::Unknown) || matches!(container, Root::Unknown) {
        return Err(format!(
            "cannot tell where {} or {} is - one of them is {}",
            thing_name, container_name, UNKNOWN
        ));
    }
    if thing == container || (thing.key().is_none() && thing.key() == container.key()) {
        return Ok(());
    }
    Err(match (thing, container) {
        (Root::Character { .. }, Root::Character { .. }) => format!(
            "{} is carrying {} and {} is carrying {} - one person cannot reach the other's kit",
            thing.name(), thing_name, container.name(), container_name
        ),
        (Root::Nowhere, _) => format!(
            "{} is nowhere - put it somewhere before putting it in {}",
            thing_name, container_name
        ),
        (_, Root::Nowhere) => format!(
            "{} is nowhere - it has to be somewhere before anything goes in it",
            container_name
        ),
        _ => format!(
            "{} is in {} and {} is in {} - they are not in the same place",
            thing_name, thing.name(), container_name, container.name()
        ),
    })
}

/* ============================ LOADING ============================ */

/// PostgREST sends `numeric` as a JSON STRING, not a number, so a plain
/// `as_f64` returns None on every slot and capacity in the catalogue.
/// The same trap containers::as_f64 exists for, and the same fix.
fn num(v: Option<&serde_json::Value>) -> Option<f64> {
    let v = v?;
    v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

/// Every object in a game, everything that can hold one, and what the
/// catalogue says about each key.
///
/// FOUR READS AND NO MORE, whoever is asking. A walk up a chain of
/// containers is a query per link if it is done one link at a time, and
/// the answers do not change between links - so the whole board is
/// loaded once and walked in memory.
///
/// RLS decides what comes back. A holder in another game is simply
/// absent, and `root_of` reports that as Unknown rather than guessing.
pub fn load_world(
    token: &str,
    game_id: &str,
) -> Result<(Vec<Obj>, Vec<Holder>, Vec<Kind>), String> {
    let rows = crate::supabase::rest_get(
        token,
        "objects",
        &[
            ("select", "id,item_key,name,quantity,equipped,attuned,holder_id,entity_id,size_override,holds_size_override"),
            ("game_id", &format!("eq.{}", game_id)),
            ("order", "item_key.asc,acquired_at.asc"),
        ],
    )?;
    let objects: Vec<Obj> =
        serde_json::from_value(rows).map_err(|e| format!("could not read the objects: {}", e))?;

    // The two kinds of holder that are not objects. Containers are
    // found among the objects themselves - see the module header.
    let people = crate::supabase::rest_get(
        token,
        "characters",
        &[
            ("select", "name,token_name,entity_id"),
            ("game_id", &format!("eq.{}", game_id)),
        ],
    )?;
    let places = crate::supabase::rest_get(
        token,
        "locations",
        &[
            ("select", "name,entity_id"),
            ("game_id", &format!("eq.{}", game_id)),
        ],
    )?;

    let mut holders: Vec<Holder> = Vec::new();
    for (rows, kind) in [(&people, "character"), (&places, "location")] {
        for r in rows.as_array().map(|a| a.as_slice()).unwrap_or(&[]) {
            let entity = match r.get("entity_id").and_then(|v| v.as_str()) {
                Some(e) => e,
                None => continue,
            };
            // The short name where there is one - a roll card says
            // "Rodnar", and so should the line saying who is holding
            // the sword.
            let name = r
                .get("token_name")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .or_else(|| r.get("name").and_then(|v| v.as_str()))
                .unwrap_or("unnamed");
            holders.push(Holder {
                entity_id: entity.to_string(),
                kind: kind.to_string(),
                name: name.to_string(),
            });
        }
    }

    let rows = crate::supabase::rest_get(
        token,
        "items",
        &[
            ("select", "key,game_id,size,holds_size,weight,slots,capacity_slots"),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
            // This campaign's row first, so the de-duplication below
            // keeps the override - the same precedence
            // collapse_overrides applies, done by the sort.
            ("order", "game_id.desc"),
        ],
    )?;
    let mut types: Vec<Kind> = Vec::new();
    for r in rows.as_array().map(|a| a.as_slice()).unwrap_or(&[]) {
        let key = r.get("key").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if types.iter().any(|t| t.key == key) {
            continue;
        }
        types.push(Kind {
            key,
            size: r.get("size").and_then(|v| v.as_str()).unwrap_or("med").to_string(),
            holds_size: r.get("holds_size").and_then(|v| v.as_str()).map(str::to_string),
            // numeric arrives as a string; see Located::weight.
            weight: r.get("weight").and_then(|v| v.as_str()).map(str::to_string),
            // THESE TWO ARE PARSED, unlike weight, because they are
            // summed and compared rather than printed. PostgREST sends
            // numeric as a JSON string, so as_f64 alone returns None on
            // every one of them - the trap containers::as_f64 documents.
            slots: num(r.get("slots")).unwrap_or(1.0),
            capacity_slots: num(r.get("capacity_slots")),
        });
    }

    Ok((objects, holders, types))
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(id: &str, key: &str, holder: Option<&str>) -> Obj {
        Obj {
            id: id.into(),
            item_key: key.into(),
            attuned: false,
            name: None,
            quantity: 1,
            equipped: false,
            holder_id: holder.map(str::to_string),
            entity_id: None,
            size_override: None,
            holds_size_override: None,
        }
    }

    /// The catalogue these tests measure against. Small on purpose: a
    /// key MISSING from it is a case worth keeping reachable, because
    /// an object whose type nobody catalogued is a real thing to render.
    fn catalogue() -> Vec<Kind> {
vec![
            kind("dagger", "tiny", None, 1.0, None, Some("1")),
            kind("greatsword", "lg", None, 2.0, None, Some("6")),
            kind("chest", "lg", Some("lg"), 8.0, Some(30.0), Some("25")),
            kind("backpack", "med", Some("med"), 1.0, Some(20.0), Some("5")),
            // THE RATIO THE FRACTIONS EXIST FOR: a purse is 5 slots and
            // holds 25 coins, so a coin is 0.2. Straight from 032.
            kind("coin_purse", "tiny", Some("tiny"), 0.5, Some(5.0), Some("1")),
            kind("coin_gp", "tiny", None, 0.2, None, Some("0.02")),
            // Deliberately no capacity: an unfinished catalogue row is
            // a real state and 032 calls it a fault rather than
            // bottomless.
            kind("priests_pack", "sm", Some("sm"), 1.0, None, Some("5")),
        ]
    }

    fn person(entity: &str, name: &str) -> Holder {
        Holder {
            entity_id: entity.into(),
            kind: "character".into(),
            name: name.into(),
        }
    }

    fn place(entity: &str, name: &str) -> Holder {
        Holder {
            entity_id: entity.into(),
            kind: "location".into(),
            name: name.into(),
        }
    }

    #[test]
    fn a_carried_thing_names_its_carrier() {
        let out = resolve(&[obj("o1", "dagger", Some("e-rodnar"))], &[person("e-rodnar", "Rodnar")], &catalogue());
        assert_eq!(out[0].holder_kind, "character");
        assert_eq!(out[0].holder_name, "Rodnar");
    }

    #[test]
    fn a_thing_on_the_floor_names_the_room() {
        let out = resolve(&[obj("o1", "handaxe", Some("e-inn"))], &[place("e-inn", "Frostvalley Inn")], &catalogue());
        assert_eq!(out[0].holder_kind, "location");
        assert_eq!(out[0].holder_name, "Frostvalley Inn");
    }

    // 033 kept "nowhere" as a real answer rather than a gap, and the
    // screen must say the word rather than print a blank.
    #[test]
    fn unheld_is_nowhere_and_says_so() {
        let out = resolve(&[obj("o1", "torch", None)], &[], &catalogue());
        assert_eq!(out[0].holder_kind, "nowhere");
        assert_eq!(out[0].holder_name, "nowhere");
        assert_eq!(out[0].holder_id, None);
    }

    // THE CASE THIS MODULE EXISTS FOR. The chest is an object in the
    // same list, and it names the holder of the coin.
    #[test]
    fn a_container_is_found_among_the_objects_it_holds() {
        let mut chest = obj("o-chest", "chest", Some("e-inn"));
        chest.entity_id = Some("e-chest".into());
        chest.name = Some("iron chest".into());
        let coin = obj("o-coin", "gp", Some("e-chest"));

        let out = resolve(&[chest, coin], &[place("e-inn", "Frostvalley Inn")], &catalogue());

        // The chest itself is on the floor...
        assert_eq!(out[0].holder_name, "Frostvalley Inn");
        assert!(out[0].is_container);
        // ...and the coin is in the chest, by its GIVEN name.
        assert_eq!(out[1].holder_kind, "container");
        assert_eq!(out[1].holder_name, "iron chest");
    }

    #[test]
    fn a_container_in_a_container_resolves_to_the_inner_one() {
        let mut pack = obj("o-pack", "backpack", Some("e-rodnar"));
        pack.entity_id = Some("e-pack".into());
        let mut purse = obj("o-purse", "pouch", Some("e-pack"));
        purse.entity_id = Some("e-purse".into());
        let coin = obj("o-coin", "gp", Some("e-purse"));

        let out = resolve(&[pack, purse, coin], &[person("e-rodnar", "Rodnar")], &catalogue());

        assert_eq!(out[0].holder_name, "Rodnar");
        assert_eq!(out[1].holder_name, "backpack");
        assert_eq!(out[2].holder_name, "pouch");
    }

    // An unnamed container falls back to its catalogue key, the same
    // precedence the row label uses. If these two disagreed, a chest
    // would be called one thing in the list and another in the holder
    // column of the thing inside it.
    #[test]
    fn an_unnamed_container_is_called_what_it_is() {
        let mut chest = obj("o-chest", "chest", None);
        chest.entity_id = Some("e-chest".into());
        let coin = obj("o-coin", "gp", Some("e-chest"));
        let out = resolve(&[chest, coin], &[], &catalogue());
        assert_eq!(out[1].holder_name, "chest");
    }

    // THE lesson from locations::arrange, applied again. A holder the
    // caller cannot see must not delete the object from the screen -
    // the DM needs to see it precisely so they can put it somewhere
    // real.
    #[test]
    fn a_holder_nobody_can_name_keeps_its_object_visible() {
        let out = resolve(&[obj("o1", "gp", Some("e-ghost"))], &[], &catalogue());
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].holder_kind, "unknown");
        assert_eq!(out[0].holder_name, UNKNOWN);
        // And the id survives, so a fix does not need a second lookup.
        assert_eq!(out[0].holder_id.as_deref(), Some("e-ghost"));
    }

    #[test]
    fn nothing_in_nothing_out() {
        assert!(resolve(&[], &[person("e", "Nobody")], &catalogue()).is_empty());
    }

    // Order is the caller's. The command asks PostgREST to sort, and
    // re-sorting here would quietly override it.
    #[test]
    fn the_order_it_was_given_is_the_order_it_returns() {
        let out = resolve(
            &[obj("a", "rope", None), obj("b", "torch", None), obj("c", "gp", None)],
            &[],
            &catalogue(),
        );
        assert_eq!(
            out.iter().map(|o| o.id.as_str()).collect::<Vec<_>>(),
            ["a", "b", "c"]
        );
    }

    #[test]
    fn a_given_name_wins_over_the_catalogue_key() {
        let mut o = obj("o1", "longsword", None);
        o.name = Some("Dawnbreaker".into());
        assert_eq!(label(&o), "Dawnbreaker");
        // And blank is not a name.
        o.name = Some("   ".into());
        assert_eq!(label(&o), "longsword");
    }
    /* ---------- size and weight, 036 ---------- */

    #[test]
    fn size_comes_from_the_catalogue() {
        let out = resolve(&[obj("o1", "dagger", None)], &[], &catalogue());
        assert_eq!(out[0].size, "tiny");
        assert_eq!(out[0].weight.as_deref(), Some("1"));
    }

    // THE GIANT'S DAGGER. There is no screen for writing a catalogue
    // row, so the instance override is the only way to say that this
    // particular one is not ordinary.
    #[test]
    fn the_instance_overrides_the_type() {
        let mut o = obj("o1", "dagger", None);
        o.size_override = Some("lg".into());
        let out = resolve(&[o], &[], &catalogue());
        assert_eq!(out[0].size, "lg");
        // The WEIGHT still comes from the type - 036 gave no override
        // for it, deliberately, because weight is the book's.
        assert_eq!(out[0].weight.as_deref(), Some("1"));
    }

    // TWO ABSENCES, ONE BEHIND THE OTHER. None on the instance means
    // "ask the type"; the type's own None is what means "no limit".
    #[test]
    fn a_container_limit_falls_through_to_the_type() {
        let mut chest = obj("o-chest", "chest", None);
        chest.entity_id = Some("e-chest".into());
        let out = resolve(&[chest], &[], &catalogue());
        assert_eq!(out[0].holds_size.as_deref(), Some("lg"));
    }

    #[test]
    fn a_giants_backpack_says_so_on_the_instance() {
        let mut chest = obj("o-chest", "chest", None);
        chest.entity_id = Some("e-chest".into());
        chest.holds_size_override = Some("huge".into());
        let out = resolve(&[chest], &[], &catalogue());
        assert_eq!(out[0].holds_size.as_deref(), Some("huge"));
    }

    // An object whose type nobody catalogued is a real thing to render
    // - the same call this module makes about a holder it cannot name.
    // Blank rather than a guess, because "med" here would be inventing
    // a fact rather than reporting one.
    #[test]
    fn a_key_missing_from_the_catalogue_is_blank_not_guessed() {
        let out = resolve(&[obj("o1", "whatsit", None)], &[], &catalogue());
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].size, "");
        assert_eq!(out[0].weight, None);
    }

    /* ---------- reach ---------- */

    /// Rodnar carrying a backpack, with a purse inside it, in the Inn.
    /// The whole chain this walk exists for, in one fixture.
    fn nested() -> (Vec<Obj>, Vec<Holder>) {
        let mut pack = obj("o-pack", "backpack", Some("e-rodnar"));
        pack.entity_id = Some("e-pack".into());
        let mut purse = obj("o-purse", "coin_purse", Some("e-pack"));
        purse.entity_id = Some("e-purse".into());
        let coin = obj("o-coin", "coin_gp", Some("e-purse"));

        let mut chest = obj("o-chest", "chest", Some("e-inn"));
        chest.entity_id = Some("e-chest".into());
        let axe = obj("o-axe", "handaxe", Some("e-inn"));
        let loose = obj("o-loose", "dagger", None);

        (
            vec![pack, purse, coin, chest, axe, loose],
            vec![person("e-rodnar", "Rodnar"), place("e-inn", "Frostvalley Inn")],
        )
    }

    fn kind(
        key: &str,
        size: &str,
        holds: Option<&str>,
        slots: f64,
        capacity: Option<f64>,
        weight: Option<&str>,
    ) -> Kind {
        Kind {
            key: key.into(),
            size: size.into(),
            holds_size: holds.map(str::to_string),
            weight: weight.map(str::to_string),
            slots,
            capacity_slots: capacity,
        }
    }

    fn root(id: &str) -> Root {
        let (objects, holders) = nested();
        let o = objects.iter().find(|o| o.id == id).unwrap().clone();
        root_of(o.holder_id.as_deref(), &objects, &holders)
    }

    // THE WALK. A coin two containers deep is still Rodnar's.
    #[test]
    fn a_coin_in_a_purse_in_a_pack_is_the_carriers() {
        assert_eq!(
            root("o-coin"),
            Root::Character { entity: "e-rodnar".into(), name: "Rodnar".into() }
        );
    }

    #[test]
    fn a_thing_in_a_room_roots_at_the_room() {
        assert_eq!(
            root("o-axe"),
            Root::Location { entity: "e-inn".into(), name: "Frostvalley Inn".into() }
        );
    }

    #[test]
    fn nowhere_is_where_nothing_leads() {
        assert_eq!(root("o-loose"), Root::Nowhere);
    }

    #[test]
    fn a_chain_into_the_unseeable_is_unknown_not_nowhere() {
        let (objects, holders) = nested();
        assert_eq!(root_of(Some("e-ghost"), &objects, &holders), Root::Unknown);
    }

    // A loop cannot happen - no_container_cycles refuses one - but a
    // walk that trusts that is a walk that hangs the day it is wrong.
    #[test]
    fn a_loop_ends_in_unknown_rather_than_forever() {
        let mut a = obj("o-a", "chest", Some("e-b"));
        a.entity_id = Some("e-a".into());
        let mut b = obj("o-b", "chest", Some("e-a"));
        b.entity_id = Some("e-b".into());
        assert_eq!(root_of(Some("e-a"), &[a, b], &[]), Root::Unknown);
    }

    /* ---------- within_reach ---------- */

    #[test]
    fn a_carrier_can_pack_their_own_kit() {
        // The coin is Rodnar's and so is the purse it would go into.
        assert!(within_reach(&root("o-coin"), &root("o-purse"), "coin", "purse").is_ok());
    }

    #[test]
    fn a_thing_on_the_floor_goes_in_a_chest_in_the_same_room() {
        assert!(within_reach(&root("o-axe"), &root("o-chest"), "handaxe", "chest").is_ok());
    }

    // THE RULE, stated by what it refuses. Rodnar cannot drop a coin
    // into a chest across the room without picking it up first.
    #[test]
    fn a_carried_thing_does_not_reach_a_chest_on_the_floor() {
        let e = within_reach(&root("o-coin"), &root("o-chest"), "coin", "chest").unwrap_err();
        assert!(e.contains("not in the same place"), "{}", e);
    }

    #[test]
    fn one_person_cannot_reach_anothers_kit() {
        let mine = Root::Character { entity: "e-a".into(), name: "Rodnar".into() };
        let theirs = Root::Character { entity: "e-b".into(), name: "Runt".into() };
        let e = within_reach(&mine, &theirs, "dagger", "pack").unwrap_err();
        assert!(e.contains("cannot reach"), "{}", e);
    }

    // TWO GOBLINS ARE BOTH CALLED GOBLIN. Compared on the entity, never
    // on the name, or one goblin would be packing the other's kit.
    #[test]
    fn two_holders_sharing_a_name_are_still_two_holders() {
        let one = Root::Character { entity: "e-1".into(), name: "Goblin".into() };
        let two = Root::Character { entity: "e-2".into(), name: "Goblin".into() };
        assert!(within_reach(&one, &two, "dagger", "pack").is_err());
    }

    // 033 made nowhere a real answer, so two things that are both there
    // are as reachable as two things in a room.
    #[test]
    fn two_things_that_are_both_nowhere_reach_each_other() {
        assert!(within_reach(&Root::Nowhere, &Root::Nowhere, "dagger", "chest").is_ok());
    }

    #[test]
    fn something_nowhere_does_not_reach_something_somewhere() {
        let e = within_reach(&root("o-loose"), &root("o-chest"), "dagger", "chest").unwrap_err();
        assert!(e.contains("nowhere"), "{}", e);
    }

    // UNKNOWN IS REFUSED EVEN AGAINST ITSELF. Two unreadable chains
    // have not been shown to meet.
    #[test]
    fn unknown_never_reaches_anything_including_unknown() {
        assert!(within_reach(&Root::Unknown, &Root::Unknown, "a", "b").is_err());
        assert!(within_reach(&Root::Unknown, &root("o-chest"), "a", "chest").is_err());
        assert!(within_reach(&root("o-axe"), &Root::Unknown, "axe", "b").is_err());
    }

    /* ---------- the reach token ---------- */

    // THE PROPERTY A PICKER RELIES ON: things that within_reach admits
    // share a token, and things it refuses do not. If these two ever
    // disagreed, a screen would offer a container the command then
    // refused - or hide one it would have taken.
    #[test]
    fn the_token_agrees_with_the_rule() {
        let (objects, holders) = nested();
        let ids = ["o-coin", "o-purse", "o-pack", "o-axe", "o-chest", "o-loose"];
        for a in ids {
            for b in ids {
                let ra = root(a);
                let rb = root(b);
                let allowed = within_reach(&ra, &rb, a, b).is_ok();
                let same_token = !ra.token().is_empty() && ra.token() == rb.token();
                assert_eq!(allowed, same_token, "{} vs {}", a, b);
            }
        }
        let _ = (objects, holders);
    }

    #[test]
    fn unknown_tokenises_as_nothing() {
        assert_eq!(Root::Unknown.token(), "");
        // And nowhere does NOT, because two things there do reach each
        // other.
        assert_eq!(Root::Nowhere.token(), "nowhere");
    }

    #[test]
    fn resolve_carries_the_reach_of_the_whole_chain() {
        let (objects, holders) = nested();
        let out = resolve(&objects, &holders, &catalogue());
        let coin = out.iter().find(|o| o.id == "o-coin").unwrap();
        let pack = out.iter().find(|o| o.id == "o-pack").unwrap();
        let chest = out.iter().find(|o| o.id == "o-chest").unwrap();
        // The coin is two containers deep and still Rodnar's.
        assert_eq!(coin.reach, pack.reach);
        assert_eq!(coin.reach_name, "Rodnar");
        // The chest is in the room, which is somewhere else entirely.
        assert_ne!(coin.reach, chest.reach);
        assert_eq!(chest.reach_name, "Frostvalley Inn");
    }

    /* ---------- how full, 032's arithmetic ---------- */

    #[test]
    fn a_container_reports_what_is_in_it() {
        let (objects, holders) = nested();
        let out = resolve(&objects, &holders, &catalogue());
        let purse = out.iter().find(|o| o.id == "o-purse").unwrap();
        // One coin at 0.2 of a slot, in a purse that holds 5.
        assert_eq!(purse.used_slots, Some(0.2));
        assert_eq!(purse.capacity_slots, Some(5.0));
    }

    // DIRECT CONTENTS ONLY, which is what `fits` counts. The pack holds
    // the PURSE - half a slot - and not the coin inside it.
    #[test]
    fn a_nested_container_costs_its_own_slots_not_its_contents() {
        let (objects, holders) = nested();
        let out = resolve(&objects, &holders, &catalogue());
        let pack = out.iter().find(|o| o.id == "o-pack").unwrap();
        assert_eq!(pack.used_slots, Some(0.5));
    }

    #[test]
    fn a_thing_that_is_not_a_container_has_no_gauge() {
        let (objects, holders) = nested();
        let out = resolve(&objects, &holders, &catalogue());
        let coin = out.iter().find(|o| o.id == "o-coin").unwrap();
        assert_eq!(coin.used_slots, None);
        assert_eq!(coin.capacity_slots, None);
    }

    // An empty container is 0 of something, not None. The difference
    // matters: None means "not a container", and a screen that read
    // them the same would draw no gauge on an empty chest.
    #[test]
    fn an_empty_container_is_zero_rather_than_nothing() {
        let (objects, holders) = nested();
        let out = resolve(&objects, &holders, &catalogue());
        let chest = out.iter().find(|o| o.id == "o-chest").unwrap();
        assert_eq!(chest.used_slots, Some(0.0));
        assert_eq!(chest.capacity_slots, Some(30.0));
    }

    // 032's unfinished row survives to the screen as a capacity of
    // None, so it can be SAID rather than drawn as an empty bar.
    #[test]
    fn a_container_with_no_capacity_recorded_says_nothing_not_zero() {
        let mut pack = obj("o-p", "priests_pack", None);
        pack.entity_id = Some("e-p".into());
        let out = resolve(&[pack], &[], &catalogue());
        assert_eq!(out[0].used_slots, Some(0.0));
        assert_eq!(out[0].capacity_slots, None);
    }

    // THE GAUGE AND THE REFUSAL AGREE. Twenty-five coins fill a purse
    // exactly, and 0.2 does not survive binary floating point - ten of
    // them sum to 1.9999999999999998. Whatever the sum is, it is the
    // one `fits` compares against, because both go through slot_total.
    #[test]
    fn the_gauge_uses_the_same_sum_the_refusal_does() {
        let mut purse = obj("o-purse", "coin_purse", None);
        purse.entity_id = Some("e-purse".into());
        let mut coins = obj("o-coins", "coin_gp", Some("e-purse"));
        coins.quantity = 25;

        let out = resolve(&[purse, coins], &[], &catalogue());
        let used = out[0].used_slots.unwrap();
        let direct = crate::containers::slot_total(&[(0.2, 25)]);
        assert_eq!(used, direct);
        // Full to the brim, within the hundredth-of-a-slot tolerance
        // `fits` uses for exactly this reason.
        assert!((used - 5.0).abs() < 0.005, "{}", used);
    }

    // A key nobody catalogued takes one slot - the column default -
    // rather than nothing, or an uncatalogued hoard would look
    // weightless in a full chest.
    #[test]
    fn an_uncatalogued_thing_still_takes_room() {
        let mut chest = obj("o-chest", "chest", None);
        chest.entity_id = Some("e-chest".into());
        let odd = obj("o-odd", "whatsit", Some("e-chest"));
        let out = resolve(&[chest, odd], &[], &catalogue());
        assert_eq!(out[0].used_slots, Some(1.0));
    }

}
