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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kind {
    pub key: String,
    pub size: String,
    pub holds_size: Option<String>,
    pub weight: Option<String>,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Located {
    pub id: String,
    pub item_key: String,
    pub name: Option<String>,
    pub quantity: i64,
    pub equipped: bool,
    /// True when this object is itself a container - the screen offers
    /// to look inside.
    pub is_container: bool,
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
            Located {
                id: o.id.clone(),
                item_key: o.item_key.clone(),
                name: o.name.clone(),
                quantity: o.quantity,
                equipped: o.equipped,
                is_container: o.entity_id.is_some(),
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

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(id: &str, key: &str, holder: Option<&str>) -> Obj {
        Obj {
            id: id.into(),
            item_key: key.into(),
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
            Kind { key: "dagger".into(), size: "tiny".into(), holds_size: None,
                   weight: Some("1".into()) },
            Kind { key: "greatsword".into(), size: "lg".into(), holds_size: None,
                   weight: Some("6".into()) },
            Kind { key: "chest".into(), size: "lg".into(),
                   holds_size: Some("lg".into()), weight: Some("25".into()) },
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

}
