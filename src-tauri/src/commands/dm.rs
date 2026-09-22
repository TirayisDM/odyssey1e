//! The DM side: building what the players will be rolling against.
//!
//! Everything an encounter needs was authored by hand in SQL until now.
//! The policies for all of it have been in place since 011 - what was
//! missing was a way to reach them, which made this the largest gap
//! between what the schema supports and what anyone could do.
//!
//! EVERY COMMAND HERE IS DM-ONLY, and not because this module says so.
//! 011 wrote `is_game_dm` into the policy on each of these tables, so a
//! player calling any of them gets refused by Postgres. Nothing is
//! re-checked in Rust: a second copy of an access rule is a second place
//! for it to be wrong, and the one that matters is the one the database
//! enforces. What this module does do is fail READABLY - see `denied`.
//!
//! Thin, per commands/mod.rs: take the session, call one thing, hand the
//! result back. The one piece of judgement in here is turning a blank
//! name into a NULL, because that is what makes 018 name the thing.

use serde_json::{json, Value};
use tauri::State;

use crate::supabase::{self, AppState};

/// Blank is not a name.
///
/// The frontend sends "" for an untouched box, and "" is not what 018's
/// trigger looks for - it looks for NULL or whitespace, and a NULL is
/// the honest way to say "nobody gave one". Trimming here means the
/// caller never has to think about it.
fn name_or_null(label: Option<String>) -> Value {
    match label.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => json!(s),
        None => Value::Null,
    }
}

/// Turn a policy refusal into a sentence.
///
/// PostgREST answers an RLS denial with 401/403 and a body that says
/// nothing a DM would recognise. The rule is real and lives in the
/// database; this only translates it, and only when the status says
/// that is what happened.
fn denied(e: String, what: &str) -> String {
    if e.contains("(401)") || e.contains("(403)") || e.contains("42501") {
        format!("only the DM of this game can {} — you are signed in as a player", what)
    } else {
        e
    }
}

/* ============================ STATBLOCKS ============================ */

/// The statblock catalogue: global rows plus this game's own.
///
/// Both, not either. A campaign's ogre sits beside the shared goblin and
/// the DM picks from one list, which is the same nullable-tenancy shape
/// items and narrative packs already use.
#[tauri::command]
pub fn list_npcs(state: State<AppState>, game_id: String) -> Result<Value, String> {
    let token = state.token()?;
    supabase::rest_get(
        &token,
        "npcs",
        &[
            ("select", "key,game_id,name,species,class,ac,hp_max,size,notes"),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
            ("order", "name.asc"),
        ],
    )
}

/// Write a statblock for this campaign.
///
/// game_id is always set and never null: 011's policy admits only
/// campaign rows, because global content is seeded by migration and by
/// nothing else. A DM inventing a goblin variant gets their own row; the
/// shared goblin is not theirs to edit.
///
/// `species` and `class` are what 018 builds a name from, and both are
/// optional - most NPCs are a species and nothing more.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_npc(
    state: State<AppState>,
    game_id: String,
    key: String,
    name: String,
    ac: i64,
    hp_max: i64,
    species: Option<String>,
    class: Option<String>,
    weapon_profs: Option<String>,
    armor_profs: Option<String>,
) -> Result<Value, String> {
    let token = state.token()?;
    if key.trim().is_empty() || name.trim().is_empty() {
        return Err("a statblock needs a key and a name".to_string());
    }
    if ac < 0 || hp_max < 1 {
        return Err("ac cannot be negative and hp must be at least 1".to_string());
    }

    // Refused before the insert rather than stored and discovered later.
    // A bad proficiency code does not fail, it under-grants - see
    // equipment::parse_armor_profs.
    let weapon_profs = crate::equipment::parse_weapon_profs(weapon_profs.as_deref().unwrap_or(""))?;
    let armor_profs = crate::equipment::parse_armor_profs(armor_profs.as_deref().unwrap_or(""))?;

    supabase::rest_insert(
        &token,
        "npcs",
        &json!({
            "game_id": game_id,
            "key": key.trim(),
            "name": name.trim(),
            "species": name_or_null(species),
            "class": name_or_null(class),
            "ac": ac,
            "hp_max": hp_max,
            "weapon_profs": weapon_profs,
            "armor_profs": armor_profs,
        }),
    )
    .map_err(|e| denied(e, "write a statblock"))
}

/* ============================ ENCOUNTERS ============================ */

/// A new encounter, in draft.
///
/// Draft rather than active on purpose: building one in front of the
/// players, half-populated, is how a surprise stops being one. Making it
/// active is a separate decision and a separate command.
#[tauri::command]
pub fn create_encounter(
    state: State<AppState>,
    game_id: String,
    name: String,
) -> Result<Value, String> {
    let token = state.token()?;
    if name.trim().is_empty() {
        return Err("an encounter needs a name".to_string());
    }
    supabase::rest_insert(
        &token,
        "encounters",
        &json!({ "game_id": game_id, "name": name.trim(), "status": "draft" }),
    )
    .map_err(|e| denied(e, "create an encounter"))
}

/// draft -> active -> ended.
///
/// At most one active per game, enforced by a partial unique index
/// rather than by hope. Hitting it is not a bug and should not read like
/// one: the DM has an encounter already in front of the table and has to
/// say what happens to it first.
#[tauri::command]
pub fn set_encounter_status(
    state: State<AppState>,
    encounter_id: String,
    status: String,
) -> Result<Value, String> {
    let token = state.token()?;
    if !["draft", "active", "ended"].contains(&status.as_str()) {
        return Err("status must be draft, active or ended".to_string());
    }

    supabase::rest_update(
        &token,
        "encounters",
        &[("id", &format!("eq.{}", encounter_id))],
        &json!({ "status": status }),
    )
    .map_err(|e| {
        if e.contains("encounters_one_active_per_game_idx") || e.contains("23505") {
            "another encounter is already active in this game — end it first".to_string()
        } else {
            denied(e, "change an encounter")
        }
    })
}

/* ============================ ENROLMENT ============================ */

/// Put a creature in the encounter.
///
/// Exactly one of `npc_key` and `character_id`, never both and never
/// neither. That was a schema constraint until 022; it is now a rule
/// about the REQUEST - which kind of thing is being enrolled - because
/// the row that results always has a character behind it either way.
///
/// AN NPC IS INSTANTIATED, NOT REFERENCED. `instantiate_npc` copies the
/// statblock's scores, kit, AC and hit points onto a NEW character, so
/// the goblin that walks in is an individual and later edits to the type
/// never reach it. That is the whole of 022, and the reason this command
/// calls a function rather than writing a row: a half-built goblin with
/// scores but no weapons must not be able to exist.
///
/// A BLANK NAME IS STILL THE POINT. Leave it empty and 018 names the
/// actor, and `instantiate_npc` gives the character the same name.
#[tauri::command]
pub fn enrol_actor(
    state: State<AppState>,
    encounter_id: String,
    npc_key: Option<String>,
    character_id: Option<String>,
    label: Option<String>,
) -> Result<Value, String> {
    let token = state.token()?;

    let npc = npc_key.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let chr = character_id.as_deref().map(str::trim).filter(|s| !s.is_empty());
    match (npc, chr) {
        (Some(_), Some(_)) => {
            return Err("an actor is a statblock or a character, never both".to_string())
        }
        (None, None) => return Err("choose a statblock or a character to enrol".to_string()),
        _ => {}
    }

    // The encounter names the game, and instantiation needs it.
    let encs = supabase::rest_get(
        &token,
        "encounters",
        &[
            ("select", "id,game_id"),
            ("id", &format!("eq.{}", encounter_id)),
        ],
    )?;
    let game_id = encs
        .as_array()
        .and_then(|a| a.first())
        .and_then(|e| e.get("game_id"))
        .and_then(|g| g.as_str())
        .ok_or_else(|| "that encounter is not visible to you".to_string())?
        .to_string();

    // A statblock becomes an individual; a character is already one.
    let resolved = match npc {
        Some(k) => {
            let made = supabase::rpc(
                &token,
                "instantiate_npc",
                &json!({
                    "p_npc_key": k,
                    "p_game_id": game_id,
                    "p_label": name_or_null(label.clone()),
                }),
            )
            .map_err(|e| denied(e, "enrol an actor"))?;
            match made.as_str() {
                Some(id) => id.to_string(),
                None => return Err(format!("could not make an individual from '{}'", k)),
            }
        }
        None => chr.unwrap_or_default().to_string(),
    };

    supabase::rest_insert(
        &token,
        "encounter_actors",
        &json!({
            "encounter_id": encounter_id,
            "character_id": resolved,
            // PROVENANCE since 022. Nothing is read through it; it
            // records which type this individual came from.
            "npc_key": npc,
            // NULL, deliberately. See 018.
            "label": name_or_null(label),
        }),
    )
    .map_err(|e| denied(e, "enrol an actor"))
}

/// Take one out again.
///
/// A hard delete rather than `active = false`, because a mis-click
/// during setup is not history worth keeping. Anything that has already
/// ROLLED is a different matter — the rolls reference it, and the DM
/// should deactivate rather than delete. Nothing stops them here; the
/// foreign keys will, and that refusal is the honest answer.
#[tauri::command]
pub fn remove_actor(state: State<AppState>, actor_id: String) -> Result<(), String> {
    let token = state.token()?;
    supabase::rest_delete(
        &token,
        "encounter_actors",
        &[("id", &format!("eq.{}", actor_id))],
    )
    .map_err(|e| denied(e, "remove an actor"))
}

/// Stop targeting something without deleting it.
///
/// The right move for a creature that has fled or been captured: it
/// leaves the target list, its rolls keep pointing at a row that still
/// exists, and the DM can bring it back.
#[tauri::command]
pub fn set_actor_active(
    state: State<AppState>,
    actor_id: String,
    active: bool,
) -> Result<Value, String> {
    let token = state.token()?;
    supabase::rest_update(
        &token,
        "encounter_actors",
        &[("id", &format!("eq.{}", actor_id))],
        &json!({ "active": active }),
    )
    .map_err(|e| denied(e, "change an actor"))
}

/* ============================ CHALLENGES ============================ */

/// An authored difficulty: a lock, a stuck door, a warded chest.
///
/// A challenge supplies a DC exactly as an actor supplies an AC, and the
/// roll path cannot tell them apart. `skill_key` is guidance for the
/// card and not a restriction — the DM may allow anything against it,
/// and nothing enforces the suggestion.
#[tauri::command]
pub fn add_challenge(
    state: State<AppState>,
    encounter_id: String,
    label: String,
    dc: i64,
    skill_key: Option<String>,
) -> Result<Value, String> {
    let token = state.token()?;
    if label.trim().is_empty() {
        return Err("a challenge needs a label — it is what the card will say".to_string());
    }
    if dc < 1 {
        return Err("a DC of zero is not a challenge".to_string());
    }

    supabase::rest_insert(
        &token,
        "encounter_challenges",
        &json!({
            "encounter_id": encounter_id,
            "label": label.trim(),
            "dc": dc,
            "skill_key": name_or_null(skill_key),
        }),
    )
    .map_err(|e| denied(e, "add a challenge"))
}

#[tauri::command]
pub fn set_challenge_active(
    state: State<AppState>,
    challenge_id: String,
    active: bool,
) -> Result<Value, String> {
    let token = state.token()?;
    supabase::rest_update(
        &token,
        "encounter_challenges",
        &[("id", &format!("eq.{}", challenge_id))],
        &json!({ "active": active }),
    )
    .map_err(|e| denied(e, "change a challenge"))
}

/* ============================ ROSTER ============================ */

/// Everything enrolled, including the inactive — what the DM manages, as
/// opposed to `list_targets`, which is what a player can aim at.
///
/// Two different questions, so two different reads. A fled goblin is
/// absent from one and present in the other, and collapsing them would
/// mean the DM could never bring it back.
#[tauri::command]
pub fn list_roster(state: State<AppState>, encounter_id: String) -> Result<Value, String> {
    let token = state.token()?;
    supabase::rest_get(
        &token,
        "encounter_actors",
        &[
            (
                "select",
                "id,label,npc_key,character_id,active,dead,initiative,\
                 name_base,name_ordinal,death_successes,death_failures,enrolled_at",
            ),
            ("encounter_id", &format!("eq.{}", encounter_id)),
            ("order", "enrolled_at.asc"),
        ],
    )
}

/// The challenges in an encounter, active or not. Same reasoning as
/// `list_roster`.
#[tauri::command]
pub fn list_challenges(state: State<AppState>, encounter_id: String) -> Result<Value, String> {
    let token = state.token()?;
    supabase::rest_get(
        &token,
        "encounter_challenges",
        &[
            ("select", "id,label,dc,skill_key,active,created_at"),
            ("encounter_id", &format!("eq.{}", encounter_id)),
            ("order", "created_at.asc"),
        ],
    )
}

/// Which skills a challenge can suggest, for the picker on the form.
///
/// The same catalogue the sheet resolver reads, filtered the same way:
/// globals plus this game's overrides. No quoting needed - the only
/// interpolated value is a uuid, not a free-text name.
#[tauri::command]
pub fn list_skill_keys(state: State<AppState>, game_id: String) -> Result<Value, String> {
    let token = state.token()?;
    supabase::rest_get(
        &token,
        "skills",
        &[
            ("select", "key,name,ability"),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
            ("order", "sort_order.asc"),
        ],
    )
}

/* ============================ A MONSTER ACTS ============================ */

/// What this NPC can attack with, as the roster's buttons.
///
/// Derived from the statblock's loadout rather than listed anywhere: a
/// weapon offers one entry per mode it has, so a handaxe - `thr`, so
/// melee and thrown - offers two. The `request` is the same string a
/// player would type, which is why the same resolver answers both.
#[tauri::command]
pub fn list_npc_attacks(state: State<AppState>, actor_id: String) -> Result<Value, String> {
    let token = state.token()?;
    let sheet = match crate::encounter::load_actor_sheet(&token, &actor_id)? {
        Some(s) => s,
        // No such actor, or not visible to this account.
        None => return Ok(json!([])),
    };

    let mut out = Vec::new();
    for owned in &sheet.loadout {
        if owned.item.kind != "weapon" {
            continue;
        }
        for mode in &owned.modes {
            out.push(json!({
                "request": crate::attack::weapon_request_name(&owned.item.name, *mode),
                "weapon": owned.item.name,
                "mode": mode.as_str(),
                "proficient": owned.proficient,
            }));
        }
    }

    // Techniques the statblock's weapons offer, gated by the level 019
    // gave it. Same list a character would get, same gate.
    for t in &sheet.techniques {
        if t.min_level <= sheet.level {
            out.push(json!({
                "request": t.roll_name,
                "weapon": t.name,
                "mode": t.mode.as_str(),
                "technique": true,
            }));
        }
    }

    Ok(json!(out))
}

/// A monster swings.
///
/// The entire body of this is `swing`, the same function `roll_named`
/// calls. 019 chose scores and real gear over a table of stored numbers
/// precisely so this command could be four lines and no rule could ever
/// differ between a goblin's attack and Rodnar's.
///
/// `actor_id` is both who swung and the sheet to build - unlike
/// roll_named, where the sheet is a character and the actor is a
/// separate question.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn npc_attack(
    state: State<AppState>,
    actor_id: String,
    request: String,
    mode: String,
    target_value: Option<i64>,
    target_kind: Option<String>,
    target_label: Option<String>,
    encounter_id: Option<String>,
    target_id: Option<String>,
    target_row: Option<String>,
    target_character_id: Option<String>,
) -> Result<Value, String> {
    let session = state
        .current()?
        .ok_or_else(|| "not signed in".to_string())?;

    let sheet = crate::encounter::load_actor_sheet(&session.access_token, &actor_id)?
        .ok_or_else(|| "that actor is not in the encounter, or is not visible to you".to_string())?;

    crate::swing(
        &session,
        &sheet,
        crate::Swing {
            request,
            mode,
            target_value,
            target_kind,
            target_label,
            encounter_id,
            target_id,
            target_row,
            target_character_id,
            actor_id: Some(actor_id),
        },
    )
    .map_err(|e| denied(e, "roll for an NPC"))
}

/* ============================ SEEING ONE ============================ */

/// Everything about one creature in the fight.
///
/// THIS IS THE SAME SHEET A PLAYER GETS. 022 made a monster a character,
/// so `load_sheet` answers for a goblin exactly as it answers for
/// Rodnar - scores, proficiency bonus, armour class, hit points, the
/// kit with its modes and proficiency already derived, and any
/// techniques its weapons offer. There is no NPC sheet loader any more
/// and no second set of rules to keep in step.
///
/// The actor's own facts ride alongside, because they belong to this
/// APPEARANCE rather than to the creature: which type it came from,
/// whether it is hidden, its initiative, and the per-encounter
/// overrides.
#[tauri::command]
pub fn view_actor(state: State<AppState>, actor_id: String) -> Result<Value, String> {
    let token = state.token()?;

    let rows = supabase::rest_get(
        &token,
        "encounter_actors",
        &[
            (
                "select",
                "id,character_id,npc_key,label,initiative,active,\
                 ac_override,hp_override,name_base,name_ordinal,enrolled_at",
            ),
            ("id", &format!("eq.{}", actor_id)),
        ],
    )?;
    let actor = rows
        .as_array()
        .and_then(|a| a.first())
        .cloned()
        .ok_or_else(|| "no such actor, or it is not visible to you".to_string())?;

    let character_id = actor
        .get("character_id")
        .and_then(|c| c.as_str())
        .ok_or_else(|| "that actor has no character".to_string())?;

    let sheet = crate::character::load_sheet(&token, character_id)?;
    let hp = crate::encounter::load_actor_vitals(&token, &actor_id)?;

    Ok(json!({
        "actor": actor,
        "sheet": sheet,
        // Current hit points are DERIVED from the damage log, not stored,
        // so they do not live on the sheet - see 013.
        "hp_current": hp.as_ref().map(|v| v.hp_current),
        "hp_max": hp.as_ref().and_then(|v| v.hp_max),
    }))
}

/// Give a creature a name.
///
/// One call, because the actor's label and the character's name are one
/// fact and writing them separately opens a window where the roster and
/// the roll log disagree. See 025.
#[tauri::command]
pub fn rename_actor(
    state: State<AppState>,
    actor_id: String,
    name: String,
) -> Result<Value, String> {
    let token = state.token()?;
    if name.trim().is_empty() {
        return Err("a name cannot be blank".to_string());
    }
    supabase::rpc(
        &token,
        "rename_actor",
        &json!({ "p_actor_id": actor_id, "p_name": name.trim() }),
    )
    .map_err(|e| denied(e, "rename a creature"))
}
