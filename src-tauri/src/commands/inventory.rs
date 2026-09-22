//! Getting things into hands, and out of them.
//!
//! PLUMBING, per the directory's rule. Every decision here is made in
//! `objects.rs` and tested there; what is left is the order of the
//! requests, which is the one thing these commands genuinely own.
//!
//! WHY THERE WAS NO ADD COMMAND UNTIL NOW
//!   There was one by accident. `set_item_equipped` upserted on
//!   (character, item_key), and an upsert on a pair that is not there
//!   INSERTS - so the way to give Rodnar a longsword was to equip a
//!   longsword he did not have. That is not a path anybody designed,
//!   and 026 closed it by taking an object id instead. These commands
//!   are the path that replaces it.
//!
//! ONE COMMAND FOR BOTH SIDES. `give_item` does not ask whether the
//! holder is a PC or an NPC, because since 022 there is nothing to ask:
//! a goblin is a characters row. The DM's reach is the policy's
//! business - `objects: holder or dm writes` lets a DM equip anyone in
//! their campaign and a player only themselves - and none of that is
//! restated here. A rule enforced in two places is a rule that will
//! disagree with itself.

use serde_json::{json, Value};
use tauri::State;

use crate::equipment;
use crate::objects;
use crate::supabase::{self, AppState};

/// Everything that can be added in this campaign: the global catalogue
/// with this game's overrides applied.
///
/// The same collapse `load_loadout` does, and for the same reason - a
/// campaign that redefines `longsword` should offer its longsword, not
/// both.
#[tauri::command]
pub fn list_catalogue(
    state: State<AppState>,
    game_id: String,
) -> Result<Vec<equipment::Item>, String> {
    let token = state.token()?;
    equipment::load_catalogue(&token, &game_id)
}

/// Put something in someone's hands.
///
/// Three steps, and the middle one is the only interesting part: what
/// is already held decides whether this is a new row or a bigger
/// number. `merge_into` makes that call and says why.
///
/// NOT AN UPSERT, deliberately. PostgREST resolves a conflict target
/// against a unique CONSTRAINT, and `objects_stack_idx` is a partial
/// index - it cannot back one, because the whole point of it is that
/// named objects are exempt. Reading first and then deciding is also
/// the only version that can produce the right error: an upsert that
/// silently merged a named axe into a stack would not look like a
/// failure.
#[tauri::command]
pub fn give_item(
    state: State<AppState>,
    character_id: String,
    item_key: String,
    quantity: Option<i64>,
    name: Option<String>,
) -> Result<Value, String> {
    let token = state.token()?;
    let quantity = objects::check_quantity(quantity.unwrap_or(1))?;
    let name = name.as_deref().and_then(objects::clean_name);

    // A named thing is one thing. Seven of them would each want the
    // name, which is the same question `may_name` refuses from the
    // other direction.
    if name.is_some() && quantity != 1 {
        return Err("only one thing can carry a name - give it singly".to_string());
    }

    let sheet = crate::character::load_profile(&token, &character_id)?;

    // A key with no catalogue row would load as an object the sheet
    // then skips - present in the table, absent from the inventory,
    // which is the worst of both. Refuse it here where there is
    // something to say about it.
    if equipment::load_item(&token, &sheet.game_id, &item_key)?.is_none() {
        return Err(format!("no item with key '{}' in this game", item_key));
    }

    let held = objects::load_held(&token, &character_id)?;
    if let Some(id) = objects::merge_into(&held, &item_key, name.as_deref()) {
        let have = held
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.quantity)
            .unwrap_or(0);
        return supabase::rest_update(
            &token,
            "objects",
            &[("id", &format!("eq.{}", id))],
            &json!({ "quantity": have + quantity }),
        );
    }

    supabase::rest_insert(
        &token,
        "objects",
        &json!({
            "game_id": sheet.game_id,
            "character_id": character_id,
            "item_key": item_key,
            "quantity": quantity,
            "name": name,
        }),
    )
}

/// Let go of something. It stays in the campaign with no holder.
///
/// `quantity` of None means all of it, which is what dropping one thing
/// means. A partial drop SPLITS: the held row keeps the remainder and a
/// second, unheld row carries what left. That second row is a new
/// object and not the same one moved, because the seven rations were
/// never seven objects to begin with.
///
/// A dropped thing is unequipped on the way out. Nothing enforces that
/// at the database - `equipped` is a fact about a row, not about a
/// holder - so it is stated here, because armour nobody is wearing
/// should not go on contributing to an AC.
#[tauri::command]
pub fn drop_object(
    state: State<AppState>,
    object_id: String,
    quantity: Option<i64>,
) -> Result<Value, String> {
    let token = state.token()?;
    let obj = objects::load_object(&token, &object_id)?;
    if obj.character_id.is_none() {
        return Err("nobody is holding that".to_string());
    }

    let (keep, moved) = objects::split(obj.quantity, quantity)?;

    if keep == 0 {
        // The whole row goes, holder and all. Its name and its charges
        // travel with it, which is the reason it is not deleted and
        // re-inserted.
        return supabase::rest_update(
            &token,
            "objects",
            &[("id", &format!("eq.{}", object_id))],
            &json!({ "character_id": null, "equipped": false, "attuned": false }),
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
            "character_id": null,
            "item_key": obj.item_key,
            "quantity": moved,
            "equipped": false,
        }),
    )
}

/// Pick up something nobody is holding, merging it if it stacks.
///
/// The inverse of a drop, and it goes through the same stacking rule:
/// three rations picked up by someone carrying four become seven, and
/// the row that was on the floor is gone because it was never a thing
/// in its own right.
#[tauri::command]
pub fn take_object(
    state: State<AppState>,
    object_id: String,
    character_id: String,
) -> Result<Value, String> {
    let token = state.token()?;
    let obj = objects::load_object(&token, &object_id)?;
    if obj.character_id.is_some() {
        return Err("somebody is already holding that".to_string());
    }

    let held = objects::load_held(&token, &character_id)?;
    if let Some(id) = objects::merge_into(&held, &obj.item_key, obj.name.as_deref()) {
        let have = held
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.quantity)
            .unwrap_or(0);
        let merged = supabase::rest_update(
            &token,
            "objects",
            &[("id", &format!("eq.{}", id))],
            &json!({ "quantity": have + obj.quantity }),
        )?;
        supabase::rest_delete(&token, "objects", &[("id", &format!("eq.{}", object_id))])?;
        return Ok(merged);
    }

    supabase::rest_update(
        &token,
        "objects",
        &[("id", &format!("eq.{}", object_id))],
        &json!({ "character_id": character_id }),
    )
}

/// Give one object a name of its own, or take the name away.
///
/// Blank means "call it by its type" and is the way back to a plain
/// object - the same reading `clean_name` gives everywhere, and the
/// reason this is not two commands.
///
/// UNNAMING CAN COLLIDE. A plain handaxe beside a newly-plain one is
/// two rows the stack index forbids, and the refusal arrives from
/// Postgres rather than from here. That is the right place for it: the
/// index is the thing that knows, and duplicating the check would mean
/// two answers to maintain.
#[tauri::command]
pub fn rename_object(
    state: State<AppState>,
    object_id: String,
    name: String,
) -> Result<Value, String> {
    let token = state.token()?;
    let obj = objects::load_object(&token, &object_id)?;
    let clean = objects::clean_name(&name);

    if clean.is_some() {
        objects::may_name(obj.quantity)?;
    }

    supabase::rest_update(
        &token,
        "objects",
        &[("id", &format!("eq.{}", object_id))],
        &json!({ "name": clean }),
    )
}

/// Destroy an object outright.
///
/// Separate from dropping, because they are different events and only
/// one of them is reversible. A dropped thing can be picked up; this is
/// the sword that went into the lava.
///
/// A stack goes whole. Spending three of seven rations is a quantity
/// change and belongs to whatever rule spends them, not to a command
/// whose name says destroy.
#[tauri::command]
pub fn destroy_object(state: State<AppState>, object_id: String) -> Result<(), String> {
    let token = state.token()?;
    supabase::rest_delete(&token, "objects", &[("id", &format!("eq.{}", object_id))])
}
