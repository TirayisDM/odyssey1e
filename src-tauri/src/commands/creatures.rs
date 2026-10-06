//! The bestiary: creature templates, and placing one on the board.
//!
//! 123. A TEMPLATE IS A CHARACTER NOW. 022 settled the instance - every
//! actor in an encounter is a `characters` row - and left the TEMPLATE
//! as a second, thinner schema in `npcs`: 23 flat columns against
//! `characters` 44 and eight satellite tables, sharing only ten column
//! names with the thing it makes. There was nowhere on a statblock to
//! put a skill proficiency, a prepared spell, a feature with uses or a
//! multiclass level, so a creature gained the capacity for all of them
//! the instant it was instantiated and arrived with none.
//!
//! A TEMPLATE IS NOT A CREATURE IN THE WORLD. It has no location, never
//! takes a turn, never appears in a target list, never rests and is
//! nobody's audience. Five game-wide reads exclude templates and a
//! trigger refuses to enrol one, because a template in an encounter is
//! not a display mistake - it would roll initiative, take damage and
//! die.
//!
//! `npcs` IS A PUBLISHED REFERENCE and stays one: the Monster Manual,
//! which you copy out of. Importing a statblock makes a template you own
//! and can edit with the same sheet a player character uses.
//!
//! A NEW FILE BECAUSE dm.rs IS 1037 LINES, which is past the ceiling the
//! architecture note sets. A new subsystem gets a new file.

use serde_json::{json, Value};
use tauri::State;

use crate::supabase;
use crate::AppState;

/// Why a refusal happened, in the words a DM can act on.
///
/// The same shaping `dm.rs` does: a policy refusal is "you are not the
/// DM of this game" rather than a Postgres error code.
fn denied(e: String, doing: &str) -> String {
    if e.contains("row-level security") || e.contains("violates") && e.contains("policy") {
        format!("only the DM of this game can {}", doing)
    } else {
        e
    }
}

fn blank_to_null(s: Option<String>) -> Value {
    match s.as_deref().map(str::trim).filter(|x| !x.is_empty()) {
        Some(x) => json!(x),
        None => Value::Null,
    }
}

/// This game's bestiary.
///
/// TEMPLATES ONLY, and only this game's. A template belongs to the game
/// that made it - 123 took that decision deliberately, with export and
/// import as the way one travels rather than a shared pool nobody owns.
#[tauri::command]
pub fn list_creatures(state: State<AppState>, game_id: String) -> Result<Value, String> {
    let token = state.token()?;
    supabase::rest_get(
        &token,
        "characters",
        &[
            (
                "select",
                "id,name,level,size,creature_type,species_key,npc_key,\
                 class_key,hp_max,ac_mode,ac_override,description",
            ),
            ("game_id", &format!("eq.{}", game_id)),
            ("is_template", "is.true"),
            ("order", "name.asc"),
        ],
    )
}

/// Copy a reference statblock into this game as a template.
///
/// THE BOOK IS NOT YOURS TO EDIT. A global `npcs` row is seeded by
/// migration and writable by nobody through the app - 011's policy
/// admits only campaign rows. So reading one into a template you own is
/// the only way to change a goblin, and the goblin everyone shares stays
/// as it was.
#[tauri::command]
pub fn import_statblock(
    state: State<AppState>,
    game_id: String,
    npc_key: String,
    label: Option<String>,
) -> Result<Value, String> {
    let token = state.token()?;
    let key = npc_key.trim();
    if key.is_empty() {
        return Err("pick a statblock to import".to_string());
    }

    let made = supabase::rpc(
        &token,
        "instantiate_npc",
        &json!({
            "p_game_id": game_id,
            "p_npc_key": key,
            "p_label": blank_to_null(label),
            // THE WHOLE DIFFERENCE. 123 gave the function this flag and
            // defaulted it false, so every existing caller still means
            // "put a goblin in the fight".
            "p_as_template": true,
        }),
    )
    .map_err(|e| denied(e, "import a creature"))?;

    match made.as_str() {
        Some(id) => Ok(json!({ "id": id })),
        None => Err(format!("no statblock with key '{}'", key)),
    }
}

/// Put a creature on the board.
///
/// A COPY, NOT A REFERENCE. The thing that walks in is its own
/// individual, so editing the template afterwards never reaches it and
/// two creatures off one template die separately - which is 022's rule
/// about statblocks, now applying to templates for the same reason.
///
/// WHAT COMES WITH IT: scores, skills, class levels, choices, prepared
/// spells and the whole kit including what is inside its containers.
/// WHAT DOES NOT: spent slots, spent uses, damage taken. A creature
/// arrives rested and whole.
///
/// ENROLMENT IS A SEPARATE CALL, deliberately. A DM often wants a
/// creature in the world before a fight exists, and `enrol_actor`
/// already knows how to put an existing character into an encounter -
/// giving this command a second job would duplicate it. The panel makes
/// both calls where it means both; a creature made and not enrolled is
/// a creature standing in the world, which is a legitimate place to be
/// rather than a half-finished write.
#[tauri::command]
pub fn place_creature(
    state: State<AppState>,
    template_id: String,
    game_id: String,
    label: Option<String>,
) -> Result<Value, String> {
    let token = state.token()?;

    let made = supabase::rpc(
        &token,
        "instantiate_character",
        &json!({
            "p_source": template_id,
            "p_game_id": game_id,
            "p_label": blank_to_null(label),
        }),
    )
    .map_err(|e| denied(e, "place a creature"))?;

    match made.as_str() {
        Some(id) => Ok(json!({ "id": id })),
        None => Err("that creature could not be copied".to_string()),
    }
}

/// Take a template out of the bestiary.
///
/// THE CREATURES MADE FROM IT ARE UNTOUCHED, because each one was a copy
/// rather than a reference. Deleting the template a fight was built from
/// does not reach into the fight.
#[tauri::command]
pub fn delete_creature(state: State<AppState>, template_id: String) -> Result<Value, String> {
    let token = state.token()?;

    // REFUSED UNLESS IT IS A TEMPLATE. The id comes off a list that only
    // holds templates, so this is a guard against a mistyped call rather
    // than against a DM - but "delete_creature" pointed at a player's
    // character would be the worst possible way to find that out.
    let rows = supabase::rest_get(
        &token,
        "characters",
        &[
            ("select", "id,name,is_template"),
            ("id", &format!("eq.{}", template_id)),
        ],
    )?;
    let is_template = rows
        .as_array()
        .and_then(|a| a.first())
        .and_then(|r| r.get("is_template"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !is_template {
        return Err("that is not a creature template".to_string());
    }

    supabase::rest_delete(
        &token,
        "characters",
        &[("id", &format!("eq.{}", template_id))],
    )
    .map_err(|e| denied(e, "delete a creature"))?;
    Ok(json!({ "deleted": template_id }))
}
