//! The individual object: getting one, splitting one, naming one.
//!
//! WHY THIS IS NOT IN equipment.rs
//!   equipment.rs answers "what can this character do with what they
//!   are holding" - proficiency, modes, the one-armor rule. It reads
//!   the catalogue and it reads the loadout, and it never changes
//!   either. This file is the other half: the rules for the inventory
//!   CHANGING - a thing arriving, a stack splitting, a name being given.
//!   Separate questions, and equipment.rs was 768 production lines
//!   against an 800 ceiling, which settles where the second one goes.
//!
//! WHAT 026 MADE POSSIBLE AND WHAT IT LEFT OPEN
//!   Before 026 an object was a junction keyed (character, item_key), so
//!   there was one row per kind of thing and "how many" was the only
//!   state it could carry. Adding was an upsert and there was nothing to
//!   decide. Now that the row is an individual, every one of these
//!   questions has two possible answers and something has to choose:
//!
//!   - Seven rations arrive and six are already held. One row or two?
//!   - Three of the seven get dropped. Which row keeps the name?
//!   - A stack gets named. Which of the seven is Runt's?
//!
//!   The answers are below, and they all come from one line in 026: a
//!   stack is an object that happens to be several, and anything that
//!   can be told apart cannot be in one.
//!
//! WHAT IS STILL MISSING
//!   Dropping sets the holder to NULL, because there is nowhere for a
//!   dropped thing to BE - Locations is a stub and there is no floor, no
//!   room and no container. 026 said the same and accepted it for the
//!   same reason: the alternative is deleting things people let go of.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::supabase;

/* ============================ TYPES ============================ */

/// What a stacking decision needs to know about a row that is already
/// held. Not the whole object - the catalogue half is irrelevant to
/// every rule here, and asking for it would mean loading it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stack {
    pub id: String,
    pub item_key: String,
    /// Null on the interchangeable ones. That is the whole test.
    pub name: Option<String>,
    pub quantity: i64,
}

/* ============================ RULES ============================ */

/// The row an incoming item should merge into, or None to insert a new
/// one.
///
/// A NAMED THING NEVER MERGES, in either direction. It does not join a
/// plain stack, because then it would be one of seven identical rations
/// and its name would be a fact about all of them; and a plain thing
/// does not join a named stack, because "Runt's Axe ×2" is not a
/// sentence about anything. This is `objects_stack_idx` stated as a
/// rule instead of as an index - the index refuses the second plain row
/// and this is what decides there should not be one.
pub fn merge_into(held: &[Stack], item_key: &str, incoming_name: Option<&str>) -> Option<String> {
    if incoming_name.is_some() {
        return None;
    }
    held.iter()
        .find(|s| s.item_key == item_key && s.name.is_none())
        .map(|s| s.id.clone())
}

/// A name, or None for "call it by its type".
///
/// Blank and whitespace mean the same thing as absent. The database
/// agrees - `objects.name` is nullable and null is most swords - so the
/// alternative is a row whose name is a space, which prints as nothing
/// and sorts as something.
pub fn clean_name(raw: &str) -> Option<String> {
    let t = raw.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

/// Splitting a held stack: how many stay, how many go.
///
/// `take` of None means all of them, which is the ordinary case - most
/// things are quantity 1 and dropping one is dropping it.
///
/// Refusing to move zero is not pedantry. It is the difference between
/// an action that did nothing and an action that failed, and a UI that
/// cannot tell those apart will show a success for a drop that did not
/// happen.
pub fn split(have: i64, take: Option<i64>) -> Result<(i64, i64), String> {
    let take = take.unwrap_or(have);
    if take <= 0 {
        return Err("moving none of something is not moving it".to_string());
    }
    if take > have {
        return Err(format!("only {} to move, not {}", have, take));
    }
    Ok((have - take, take))
}

/// Whether this row can be given a name.
///
/// It cannot if there are several in it, and the reason is that the
/// question has no answer: name seven rations and the name is not
/// about any of them. The caller's move is to split one off first,
/// which is why `split` exists and why this error says so.
pub fn may_name(quantity: i64) -> Result<(), String> {
    if quantity > 1 {
        return Err(format!(
            "there are {} of these - split one off before naming it",
            quantity
        ));
    }
    Ok(())
}

/// How many to add, refused if it is not a number of things.
///
/// Zero is not a quantity and negative is not one either. The column is
/// checked `>= 0`, which permits an empty stack - a real state, because
/// a stack can be spent down to nothing - but not a way to arrive at
/// one.
pub fn check_quantity(n: i64) -> Result<i64, String> {
    if n < 1 {
        return Err("a quantity is at least one".to_string());
    }
    Ok(n)
}

/* ============================ NETWORK ============================ */

/// One object by id: who holds it, what type it is, what it is called.
///
/// The equip path needs this now that 026 gave objects an identity. It
/// used to be handed a (character, item_key) pair, which named a type
/// and trusted the caller to have picked a real one; an id names the
/// thing itself, and the holder comes back from the row rather than
/// from whoever asked.
pub struct ObjectRow {
    /// WHERE IT IS: a character's entity, a container's entity, or None
    /// for nowhere. See 031 - one column, because two would rot.
    pub holder_id: Option<String>,
    /// WHAT IT IS, when it can hold. None for an arrow.
    pub entity_id: Option<String>,
    pub game_id: String,
    pub item_key: String,
    pub name: Option<String>,
    pub quantity: i64,
    /// 036. Carried so that splitting a stack passes it to the half
    /// that travels - a DM's edit must not evaporate on a move.
    pub size_override: Option<String>,
}

pub fn load_object(token: &str, object_id: &str) -> Result<ObjectRow, String> {
    let rows = supabase::rest_get(
        token,
        "objects",
        &[
            ("select", "holder_id,entity_id,game_id,item_key,name,quantity,size_override"),
            ("id", &format!("eq.{}", object_id)),
        ],
    )?;
    // An object a policy hides is indistinguishable from one that was
    // never there, and deliberately so - see supabase::error_message.
    let r = rows
        .as_array()
        .and_then(|a| a.first())
        .cloned()
        .ok_or_else(|| "no such object, or it is not visible to you".to_string())?;
    Ok(ObjectRow {
        holder_id: str_or_none(&r, "holder_id"),
        entity_id: str_or_none(&r, "entity_id"),
        game_id: as_text(&r, "game_id"),
        item_key: as_text(&r, "item_key"),
        name: str_or_none(&r, "name"),
        quantity: r.get("quantity").and_then(|x| x.as_i64()).unwrap_or(1),
        size_override: str_or_none(&r, "size_override"),
    })
}

/// The character this holder IS, or None when it is a container.
///
/// DIRECT, deliberately. Postgres has `holder_character`, which walks
/// up through containers and answers "whose is this ultimately" - the
/// question a POLICY asks. This asks the narrower one the equip rule
/// needs: is this thing in somebody's hands, rather than somewhere in
/// their luggage.
pub fn character_holding(token: &str, holder_id: &str) -> Result<Option<String>, String> {
    let rows = supabase::rest_get(
        token,
        "characters",
        &[("select", "id"), ("entity_id", &format!("eq.{}", holder_id))],
    )?;
    Ok(rows
        .as_array()
        .and_then(|a| a.first())
        .map(|r| as_text(r, "id")))
}

/// Everything directly inside ONE HOLDER, in the shape the stacking
/// rule wants. A character's entity gives what they are carrying; a
/// chest's gives what is in the chest. Not recursive: what is in the
/// backpack is the backpack's, not theirs. Deliberately not the loadout: deciding where seven rations go
/// needs no catalogue row, and loading one would turn an add into three
/// requests.
pub fn load_held(token: &str, holder_id: &str) -> Result<Vec<Stack>, String> {
    let rows = supabase::rest_get(
        token,
        "objects",
        &[
            ("select", "id,item_key,name,quantity"),
            ("holder_id", &format!("eq.{}", holder_id)),
        ],
    )?;
    Ok(rows
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|r| Stack {
            id: as_text(r, "id"),
            item_key: as_text(r, "item_key"),
            name: str_or_none(r, "name"),
            quantity: r.get("quantity").and_then(|x| x.as_i64()).unwrap_or(1),
        })
        .collect())
}

fn as_text(v: &Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

fn str_or_none(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(id: &str, key: &str, name: Option<&str>, qty: i64) -> Stack {
        Stack {
            id: id.to_string(),
            item_key: key.to_string(),
            name: name.map(|s| s.to_string()),
            quantity: qty,
        }
    }

    fn held() -> Vec<Stack> {
        vec![
            stack("a", "rations", None, 6),
            stack("b", "handaxe", None, 1),
            stack("c", "handaxe", Some("Runt's Axe"), 1),
        ]
    }

    #[test]
    fn a_plain_thing_joins_the_plain_stack() {
        assert_eq!(merge_into(&held(), "rations", None).as_deref(), Some("a"));
    }

    #[test]
    fn a_named_thing_joins_nothing() {
        assert_eq!(merge_into(&held(), "handaxe", Some("Biter")), None);
    }

    #[test]
    fn a_plain_thing_does_not_join_a_named_one() {
        // Two handaxes are held and only one of them is nameless. The
        // named one must not absorb the new axe just because it matches
        // on type.
        assert_eq!(merge_into(&held(), "handaxe", None).as_deref(), Some("b"));
    }

    #[test]
    fn nothing_to_join_means_a_new_row() {
        assert_eq!(merge_into(&held(), "longsword", None), None);
    }

    #[test]
    fn a_name_that_is_only_spaces_is_no_name() {
        assert_eq!(clean_name("   "), None);
        assert_eq!(clean_name(""), None);
        assert_eq!(clean_name("  Runt's Axe "), Some("Runt's Axe".to_string()));
    }

    #[test]
    fn dropping_without_a_number_drops_the_lot() {
        assert_eq!(split(7, None), Ok((0, 7)));
    }

    #[test]
    fn a_partial_drop_leaves_the_remainder() {
        assert_eq!(split(7, Some(3)), Ok((4, 3)));
    }

    #[test]
    fn more_than_is_held_cannot_be_moved() {
        assert!(split(2, Some(3)).is_err());
    }

    #[test]
    fn moving_none_is_refused_rather_than_ignored() {
        assert!(split(7, Some(0)).is_err());
        assert!(split(7, Some(-1)).is_err());
    }

    #[test]
    fn one_of_a_kind_can_be_named() {
        assert!(may_name(1).is_ok());
        // Zero is an empty stack, which is a real state and not a
        // reason to refuse a name.
        assert!(may_name(0).is_ok());
    }

    #[test]
    fn a_stack_cannot_be_named() {
        assert!(may_name(7).is_err());
    }

    #[test]
    fn a_quantity_is_at_least_one() {
        assert_eq!(check_quantity(1), Ok(1));
        assert!(check_quantity(0).is_err());
        assert!(check_quantity(-4).is_err());
    }
}
