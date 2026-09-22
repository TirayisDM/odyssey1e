//! Putting things in things, and taking them out again.
//!
//! PLUMBING. Every decision is in `containers.rs` and tested there; the
//! order of the requests is all that lives here.
//!
//! ONE MOVE, TWO DIRECTIONS. `put_in` and `take_out` are the same
//! operation with the destination swapped - an object's holder changes
//! - so they share `move_into`, and the checks that differ are the ones
//! about what a CONTAINER will accept. A character's hands accept
//! anything; that is the only asymmetry, and it is why a character is
//! not just a container with a permissive tag list.

use serde_json::{json, Value};
use tauri::State;

use crate::containers;
use crate::objects;
use crate::supabase::{self, AppState};

/// What is inside, in the same shape an inventory comes back in.
///
/// Deliberately `load_loadout`, which renders as an `Owned` row and so
/// reuses the panel the player's own inventory already uses. It derives
/// proficiency against empty lists on the way, which is meaningless for
/// a chest and harmless - nothing in a chest is being wielded.
#[tauri::command]
pub fn list_contents(
    state: State<AppState>,
    container_id: String,
) -> Result<Vec<crate::equipment::Owned>, String> {
    let token = state.token()?;
    let c = objects::load_object(&token, &container_id)?;
    let entity = c
        .entity_id
        .ok_or_else(|| "that is not a container".to_string())?;
    crate::equipment::load_loadout(&token, &entity, &c.game_id, &[], &[], false)
}

/// Put something into a container.
#[tauri::command]
pub fn put_in_container(
    state: State<AppState>,
    object_id: String,
    container_id: String,
) -> Result<Value, String> {
    let token = state.token()?;
    if object_id == container_id {
        return Err("a container cannot be put inside itself".to_string());
    }

    let obj = objects::load_object(&token, &object_id)?;
    let con = objects::load_object(&token, &container_id)?;
    let into = con
        .entity_id
        .clone()
        .ok_or_else(|| "that is not a container".to_string())?;

    let profile = containers::load_profile(&token, &con.game_id, &con.item_key)?
        .ok_or_else(|| "that is not a container".to_string())?;
    let name = con.name.clone().unwrap_or_else(|| con.item_key.clone());

    // What is already inside decides how much room is left, so the
    // contents are read before either check.
    let inside = objects::load_held(&token, &into)?;
    let mut keys: Vec<String> = inside.iter().map(|s| s.item_key.clone()).collect();
    keys.push(obj.item_key.clone());
    let bulks = containers::load_bulk(&token, &con.game_id, &keys)?;

    let find = |k: &str| {
        bulks
            .iter()
            .find(|b| b.key == k)
            .cloned()
            .ok_or_else(|| format!("no item with key '{}' in this game", k))
    };

    let incoming = find(&obj.item_key)?;
    let mut contents: Vec<(containers::Bulk, i64)> = Vec::new();
    for s in &inside {
        contents.push((find(&s.item_key)?, s.quantity));
    }

    // THREE OBJECTIONS, WORST-FIRST. They are not the same question
    // and any one of them is enough:
    //
    //   admits       a purse takes coins, and that is not a coin
    //   admits_size  nothing bigger than tiny goes in, and that is large
    //   fits         it would go in, but there is no room left
    //
    // Size before slots because it is the objection a person reaches
    // for and it gives the better sentence. Arithmetic about how much
    // room a greatsword needs in a coin purse answers a question nobody
    // asked.
    containers::admits(&profile, &incoming, &name)?;
    containers::admits_size(&profile, &incoming, &name)?;
    containers::fits(&profile, &contents, &incoming, obj.quantity, &name)?;

    move_into(&token, &obj, &object_id, &inside, Some(&into))
}

/// Take something out of a container and into somebody's hands.
///
/// No acceptance check: hands take anything. The one thing worth
/// refusing is taking from nowhere, which would silently create gear.
#[tauri::command]
pub fn take_from_container(
    state: State<AppState>,
    object_id: String,
    character_id: String,
) -> Result<Value, String> {
    let token = state.token()?;
    let obj = objects::load_object(&token, &object_id)?;
    let taker = crate::character::load_profile(&token, &character_id)?;
    let held = objects::load_held(&token, &taker.entity_id)?;
    move_into(&token, &obj, &object_id, &held, Some(&taker.entity_id))
}

/// Move an object to a holder, merging into a stack already there.
///
/// The merge is the whole reason this is shared: ten gold into a purse
/// holding five is fifteen in one row, not a second row the stack index
/// would refuse anyway.
fn move_into(
    token: &str,
    obj: &objects::ObjectRow,
    object_id: &str,
    destination_contents: &[objects::Stack],
    holder: Option<&str>,
) -> Result<Value, String> {
    if let Some(id) = objects::merge_into(
        destination_contents,
        &obj.item_key,
        obj.name.as_deref(),
    ) {
        let have = destination_contents
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.quantity)
            .unwrap_or(0);
        let merged = supabase::rest_update(
            token,
            "objects",
            &[("id", &format!("eq.{}", id))],
            &json!({ "quantity": have + obj.quantity }),
        )?;
        // The row that moved is gone, because it was never a thing in
        // its own right - the same reasoning `take_object` uses.
        supabase::rest_delete(token, "objects", &[("id", &format!("eq.{}", object_id))])?;
        return Ok(merged);
    }

    supabase::rest_update(
        token,
        "objects",
        &[("id", &format!("eq.{}", object_id))],
        &json!({ "holder_id": holder }),
    )
}
