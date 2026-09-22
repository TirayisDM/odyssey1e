//! Places: making them, moving them, and seeing what is lying in them.
//!
//! 033 built the schema and nothing could reach it - the same gap dm.rs
//! closed for encounters. This is the reaching.
//!
//! DM-ONLY, AND NOT BECAUSE THIS MODULE SAYS SO. 033 wrote
//! `is_game_dm` into the write policies on `locations`; a player calling
//! any of the writes gets refused by Postgres. Nothing is re-checked
//! here, for the reason dm.rs gives: a second copy of an access rule is
//! a second place for it to be wrong. Reading is different - members
//! read, because the players have to see where they are.
//!
//! THE ONE EXCEPTION IS `drop_here`, and it belongs to everybody. A
//! player putting something down in the room they are standing in is
//! not an act of world-building, and 033's objects policy already says
//! a loose thing is anyone's to move.
//!
//! Thin, per commands/mod.rs. The one derivation in reach - depth and
//! path - is in `src/locations.rs` where it can be tested, and this file
//! only asks for it.

use serde_json::{json, Value};
use tauri::State;

use crate::holders::{self, Located};
use crate::locations::{self, Place, Placed};
use crate::objects;
use crate::supabase::{self, AppState};

/// Turn a policy refusal into a sentence. Same helper as dm.rs, same
/// reason: PostgREST answers an RLS denial with a body no DM would
/// recognise.
fn denied(e: String, what: &str) -> String {
    if e.contains("(401)") || e.contains("(403)") || e.contains("42501") {
        format!("only the DM of this game can {} — you are signed in as a player", what)
    } else {
        e
    }
}

/// Blank is not a parent. An untouched select sends "", and "" is not a
/// uuid - it means top of the world.
fn id_or_null(v: Option<String>) -> Value {
    match v.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => json!(s),
        None => Value::Null,
    }
}

/// Every place in the game, in depth-first order with its depth and the
/// road to it.
///
/// The rows come back flat - 033 stores `parent_id` and nothing else -
/// and `locations::arrange` works out the shape. A player sees what the
/// policy lets them see, and a room whose parent is not in that set
/// comes back as a root rather than being dropped.
#[tauri::command]
pub fn list_locations(state: State<AppState>, game_id: String) -> Result<Vec<Placed>, String> {
    let token = state.token()?;
    let rows = supabase::rest_get(
        &token,
        "locations",
        &[
            ("select", "id,parent_id,name,kind,description"),
            ("game_id", &format!("eq.{}", game_id)),
            ("order", "name.asc"),
        ],
    )?;

    let places: Vec<Place> = serde_json::from_value(rows)
        .map_err(|e| format!("could not read the locations: {}", e))?;
    Ok(locations::arrange(&places))
}

/// Make a place. A blank parent means the top of its own tree.
///
/// Nothing here checks the parent: 033's trigger refuses a cycle and a
/// parent in another game, and both refusals arrive as sentences.
#[tauri::command]
pub fn create_location(
    state: State<AppState>,
    game_id: String,
    name: String,
    kind: String,
    parent_id: Option<String>,
    description: Option<String>,
) -> Result<Value, String> {
    let token = state.token()?;
    if name.trim().is_empty() {
        return Err("a place needs a name".to_string());
    }
    supabase::rest_insert(
        &token,
        "locations",
        &json!({
            "game_id": game_id,
            "name": name.trim(),
            "kind": kind,
            "parent_id": id_or_null(parent_id),
            "description": description,
        }),
    )
    .map_err(|e| denied(e, "build the world"))
}

/// Rename a place, or rewrite its description.
#[tauri::command]
pub fn rename_location(
    state: State<AppState>,
    location_id: String,
    name: String,
    description: Option<String>,
) -> Result<Value, String> {
    let token = state.token()?;
    if name.trim().is_empty() {
        return Err("a place needs a name".to_string());
    }
    supabase::rest_update(
        &token,
        "locations",
        &[("id", &format!("eq.{}", location_id))],
        &json!({ "name": name.trim(), "description": description }),
    )
    .map_err(|e| denied(e, "rename a place"))
}

/// Move a place under a different parent, or out to the top.
///
/// THE CYCLE CHECK IS NOT HERE. `no_location_cycles` is a trigger, so
/// putting the Inn inside its own cupboard is refused by Postgres with a
/// sentence. Checking it here as well would be the second copy dm.rs
/// warns about, and this is the one that cannot be bypassed.
#[tauri::command]
pub fn move_location(
    state: State<AppState>,
    location_id: String,
    parent_id: Option<String>,
) -> Result<Value, String> {
    let token = state.token()?;
    supabase::rest_update(
        &token,
        "locations",
        &[("id", &format!("eq.{}", location_id))],
        &json!({ "parent_id": id_or_null(parent_id) }),
    )
    .map_err(|e| denied(e, "move a place"))
}

/// Remove a place.
///
/// 033 makes `parent_id` ON DELETE RESTRICT, so a place that still
/// contains one refuses to go - empty it first. What is LYING in it is a
/// different matter: deleting the room takes its entity with it and
/// everything resting on the floor SET NULLs back to nowhere, the same
/// way destroying a chest spills it.
#[tauri::command]
pub fn delete_location(state: State<AppState>, location_id: String) -> Result<(), String> {
    let token = state.token()?;
    supabase::rest_delete(&token, "locations", &[("id", &format!("eq.{}", location_id))])
        .map(|_| ())
        .map_err(|e| {
            if e.contains("23503") || e.to_lowercase().contains("foreign key") {
                "something is inside that place — empty it first".to_string()
            } else {
                denied(e, "remove a place")
            }
        })
}

/// What is lying in a place.
///
/// Objects whose holder IS this location - not what is inside a chest
/// that is in the room, which belongs to the chest. One step, because
/// "what can I see on the floor" is a different question from "what is
/// in this building somewhere".
#[tauri::command]
pub fn location_contents(
    state: State<AppState>,
    location_id: String,
) -> Result<Vec<objects::Stack>, String> {
    let token = state.token()?;
    let rows = supabase::rest_get(
        &token,
        "locations",
        &[
            ("select", "entity_id"),
            ("id", &format!("eq.{}", location_id)),
        ],
    )?;
    let entity = rows
        .as_array()
        .and_then(|a| a.first())
        .and_then(|r| r.get("entity_id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "no such place, or it is not visible to you".to_string())?;

    objects::load_held(&token, entity)
}

/// Put a LOOSE thing into a place.
///
/// Not `drop_here`, which drops something out of a holder's hands and
/// refuses an object nobody is holding. This one is for the other case:
/// a thing that is already nowhere, being given somewhere to be.
///
/// `quantity` of None means all of it. A partial move leaves the
/// remainder exactly where it was, which for a loose stack is nowhere -
/// a real place since 033, and the reason this can split at all.
#[tauri::command]
pub fn place_object(
    state: State<AppState>,
    object_id: String,
    location_id: String,
    quantity: Option<i64>,
) -> Result<Value, String> {
    let token = state.token()?;
    let obj = objects::load_object(&token, &object_id)?;
    if obj.holder_id.is_some() {
        return Err("somebody is holding that — drop it instead".to_string());
    }
    let here = location_entity(&token, &location_id)?;

    // IT SPLITS NOW, like every other way of moving a stack. The
    // original comment said there was nothing to split because a loose
    // stack has no holder to keep the remainder - but nowhere IS where
    // the remainder stays, which 033 settled and this had not caught up
    // with.
    let (keep, moved) = objects::split(obj.quantity, quantity)?;
    if keep == 0 {
        return supabase::rest_update(
            &token,
            "objects",
            &[("id", &format!("eq.{}", object_id))],
            &json!({ "holder_id": here }),
        );
    }

    supabase::rest_update(
        &token,
        "objects",
        &[("id", &format!("eq.{}", object_id))],
        &json!({ "quantity": keep }),
    )?;
    supabase::rest_insert(
        &token,
        "objects",
        &json!({
            "game_id": obj.game_id,
            "holder_id": here,
            "item_key": obj.item_key,
            "quantity": moved,
            // Travels with the split - see move_into.
            "size_override": obj.size_override,
        }),
    )
}

/// A location's entity - the thing `objects.holder_id` actually points
/// at. Three commands ask for it, so it is asked once.
fn location_entity(token: &str, location_id: &str) -> Result<String, String> {
    let rows = supabase::rest_get(
        token,
        "locations",
        &[
            ("select", "entity_id"),
            ("id", &format!("eq.{}", location_id)),
        ],
    )?;
    rows.as_array()
        .and_then(|a| a.first())
        .and_then(|r| r.get("entity_id"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| "no such place, or it is not visible to you".to_string())
}

/// Put something down HERE rather than nowhere.
///
/// `drop_object` sets the holder to NULL because until 033 there was
/// nowhere for a dropped thing to be. There is now, and this is the same
/// event with a destination.
///
/// NOT DM-ONLY. A player putting a torch on the floor of the room they
/// are standing in is not world-building, and 033's objects policy
/// already lets a member move anything loose.
///
/// The split rule is `objects::split`, the same one `drop_object` uses:
/// dropping three of seven rations leaves four held and puts three on
/// the floor, because the seven were never seven objects.
#[tauri::command]
pub fn drop_here(
    state: State<AppState>,
    object_id: String,
    location_id: String,
    quantity: Option<i64>,
) -> Result<Value, String> {
    let token = state.token()?;

    let rows = supabase::rest_get(
        &token,
        "locations",
        &[
            ("select", "entity_id"),
            ("id", &format!("eq.{}", location_id)),
        ],
    )?;
    let here = rows
        .as_array()
        .and_then(|a| a.first())
        .and_then(|r| r.get("entity_id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "no such place, or it is not visible to you".to_string())?
        .to_string();

    let obj = objects::load_object(&token, &object_id)?;
    if obj.holder_id.is_none() {
        return Err("nobody is holding that".to_string());
    }
    let (keep, moved) = objects::split(obj.quantity, quantity)?;

    if keep == 0 {
        // The whole row travels, name and charges with it - which is why
        // it is moved rather than deleted and re-inserted.
        return supabase::rest_update(
            &token,
            "objects",
            &[("id", &format!("eq.{}", object_id))],
            // equipped and attuned are cleared by 031's trigger, because
            // a sword on the floor is not worn.
            &json!({ "holder_id": here }),
        );
    }

    supabase::rest_update(
        &token,
        "objects",
        &[("id", &format!("eq.{}", object_id))],
        &json!({ "quantity": keep }),
    )?;

    supabase::rest_insert(
        &token,
        "objects",
        &json!({
            "game_id": obj.game_id,
            "holder_id": here,
            "item_key": obj.item_key,
            "quantity": moved,
            // Travels with the split - see move_into.
            "size_override": obj.size_override,
        }),
    )
}

/* ============================ THE SCENE ============================ */
//
// Three questions of one place: who is here, what is happening here,
// what is lying here. The third was `location_contents` from the day
// 033 landed; these are the other two.
//
// NONE OF THEM IS DM-ONLY. A place a member can see is a place they can
// ask about, and 001's read policies already decide that. The world
// panel is behind amDM() as a courtesy, not as the guard.

/// Everyone in the game and where they are standing.
///
/// THE WHOLE ROSTER, not just the ones in a given room - and NPCs
/// included, unlike `list_characters`, which filters them out so a
/// player's own list is not full of goblins. Here they are the point:
/// most of who is in a room is monsters.
///
/// One call rather than one per place. The scene filters by
/// `location_id` on the way to the screen, and the same list fills the
/// picker that moves somebody here - which has to offer people who are
/// somewhere ELSE, or nobody could ever walk between two rooms.
#[tauri::command]
pub fn who_is_where(state: State<AppState>, game_id: String) -> Result<Value, String> {
    let token = state.token()?;
    supabase::rest_get(
        &token,
        "characters",
        &[
            ("select", "id,name,token_name,is_npc,dead,is_active,location_id,entity_id"),
            ("game_id", &format!("eq.{}", game_id)),
            ("order", "is_npc.asc,name.asc"),
        ],
    )
}

/// What is happening in this place.
///
/// 034 put `location_id` on encounters and only the Run tab ever read
/// it. Asked here rather than filtered out of the list the Run tab
/// already loaded, so that looking at a room does not depend on having
/// opened another tab first.
#[tauri::command]
pub fn encounters_here(state: State<AppState>, location_id: String) -> Result<Value, String> {
    let token = state.token()?;
    supabase::rest_get(
        &token,
        "encounters",
        &[
            ("select", "id,name,status,location_id,created_at"),
            ("location_id", &format!("eq.{}", location_id)),
            ("order", "created_at.desc"),
        ],
    )
}

/// Put a person in a place, or take them out of every place.
///
/// NOT DM-ONLY EITHER, and that is 001's decision rather than this
/// one: "characters: owner or dm updates" has said since the first
/// migration that a character is moved by the player who owns it or by
/// the DM. Walking into the next room is exactly what that policy
/// describes, and it needed no change to cover it.
///
/// A blank location is nowhere in particular, which 035 keeps as a real
/// answer rather than a gap - a character between scenes is not
/// misfiled. The same-game trigger refuses a room in another campaign,
/// and that refusal arrives as a sentence.
#[tauri::command]
pub fn move_character(
    state: State<AppState>,
    character_id: String,
    location_id: Option<String>,
) -> Result<Value, String> {
    let token = state.token()?;
    supabase::rest_update(
        &token,
        "characters",
        &[("id", &format!("eq.{}", character_id))],
        &json!({ "location_id": id_or_null(location_id) }),
    )
    .map_err(|e| {
        // NOT `denied`, which says "only the DM can". That is the wrong
        // sentence here: a player moving their OWN character is allowed,
        // so a refusal means somebody else's.
        if e.contains("(401)") || e.contains("(403)") || e.contains("42501") {
            "that is not your character to move — its owner or the DM can".to_string()
        } else {
            e
        }
    })
}

/* ========================= THE MANAGER ========================= */

/// Every object in the game, with its holder said out loud.
///
/// THE ONLY VIEW THAT CROSSES ALL THREE KINDS OF HOLDER. Everything
/// before it asked a narrower question - what is in this chest, what is
/// on this floor, what is this character carrying - and each of those
/// already knew the answer because it supplied the holder. This one
/// starts from the objects and has to work backwards.
///
/// Four reads and a fold. The fold is `holders::resolve`, which lives in
/// src/ with tests because a container is an object and therefore both
/// the question and half the answer - see its header.
///
/// RLS decides what comes back, as always. A character in another game
/// is invisible, and an object held by one reads as "somewhere
/// unaccounted for" rather than disappearing.
#[tauri::command]
pub fn list_objects(state: State<AppState>, game_id: String) -> Result<Vec<Located>, String> {
    let token = state.token()?;
    let (objects, holders, types) = holders::load_world(&token, &game_id)?;
    Ok(holders::resolve(&objects, &holders, &types))
}
