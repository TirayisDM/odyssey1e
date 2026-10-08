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

use std::collections::HashSet;

use crate::creature_io::{self, Creature, Envelope, KitItem, Known};
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

/* ========================= EXPORT AND IMPORT =========================

   125. A template belongs to a game - 123 took that decision - so the
   way one gets to another game, or into a backup, is a file.

   KEYS TRAVEL, IDS DO NOT. Nothing written here carries a uuid; a
   creature is a name, some numbers and a pile of keys into catalogues.
   That is what makes it portable and it is also the whole problem,
   which `creature_io::vet` is the answer to.
   ===================================================================== */

fn as_str(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(|x| x.as_str()).filter(|s| !s.is_empty()).map(str::to_string)
}
fn as_i64(v: &Value, k: &str) -> Option<i64> {
    v.get(k).and_then(|x| x.as_i64())
}
fn as_strs(v: &Value, k: &str) -> Vec<String> {
    v.get(k)
        .and_then(|x| x.as_array())
        .map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

/// Everything one entity holds, and what is inside it.
fn read_kit(token: &str, holder: &str, depth: usize) -> Result<Vec<KitItem>, String> {
    if depth >= 16 || holder.is_empty() {
        return Ok(Vec::new());
    }
    let rows = supabase::rest_get(
        token,
        "objects",
        &[
            (
                "select",
                "entity_id,item_key,quantity,name,slot,attuned,proficient_override,uses_spent,uses_max,grants",
            ),
            ("holder_id", &format!("eq.{}", holder)),
        ],
    )?;
    let empty = Vec::new();
    let mut out = Vec::new();
    for r in rows.as_array().unwrap_or(&empty) {
        let Some(item_key) = as_str(r, "item_key") else {
            continue;
        };
        let contents = match as_str(r, "entity_id") {
            Some(ent) => read_kit(token, &ent, depth + 1)?,
            None => Vec::new(),
        };
        out.push(KitItem {
            item_key,
            quantity: as_i64(r, "quantity").unwrap_or(1),
            name: as_str(r, "name"),
            slot: as_str(r, "slot"),
            attuned: r.get("attuned").and_then(|v| v.as_bool()).unwrap_or(false),
            proficient_override: r.get("proficient_override").and_then(|v| v.as_bool()),
            uses_spent: as_i64(r, "uses_spent"),
            uses_max: as_i64(r, "uses_max"),
            grants: r.get("grants").filter(|g| !g.is_null()).cloned(),
            contents,
        });
    }
    Ok(out)
}

/// A creature as a file.
///
/// THE WHOLE THING, so a backup is a backup: scores, skills, class
/// levels, the choices made for them, prepared spells, and the kit
/// including the contents of its containers.
///
/// NOT WHAT IS SPENT. Slots, uses, damage and death saves are the state
/// of one afternoon rather than of the creature, and 123 already
/// refuses to copy them when placing one.
#[tauri::command]
pub fn export_creature(state: State<AppState>, template_id: String) -> Result<Value, String> {
    let token = state.token()?;

    let rows = supabase::rest_get(
        &token,
        "characters",
        &[
            (
                "select",
                "id,entity_id,name,level,size,creature_type,hp_max,ac_mode,ac_override,prof_bonus,species_key,class_key,npc_key,description,weapon_profs,armor_profs,tool_profs",
            ),
            ("id", &format!("eq.{}", template_id)),
        ],
    )?;
    let c = rows
        .as_array()
        .and_then(|a| a.first())
        .ok_or_else(|| "no such creature".to_string())?;

    let entity = as_str(c, "entity_id").unwrap_or_default();
    let sat = |t: &str, sel: &str| -> Result<Value, String> {
        supabase::rest_get(
            &token,
            t,
            &[("select", sel), ("character_id", &format!("eq.{}", template_id))],
        )
    };

    let creature = Creature {
        name: as_str(c, "name").unwrap_or_else(|| "a creature".into()),
        level: as_i64(c, "level").unwrap_or(1),
        size: as_str(c, "size"),
        creature_type: as_str(c, "creature_type"),
        hp_max: as_i64(c, "hp_max"),
        ac_mode: as_str(c, "ac_mode"),
        ac_override: as_i64(c, "ac_override"),
        prof_bonus: as_i64(c, "prof_bonus"),
        species_key: as_str(c, "species_key"),
        class_key: as_str(c, "class_key"),
        npc_key: as_str(c, "npc_key"),
        description: as_str(c, "description"),
        weapon_profs: as_strs(c, "weapon_profs"),
        armor_profs: as_strs(c, "armor_profs"),
        tool_profs: as_strs(c, "tool_profs"),
        abilities: serde_json::from_value(sat("character_abilities", "ability,score,save_prof")?)
            .unwrap_or_default(),
        skills: serde_json::from_value(sat("character_skills", "skill_key,prof")?)
            .unwrap_or_default(),
        classes: serde_json::from_value(sat("character_classes", "class_key,level")?)
            .unwrap_or_default(),
        choices: serde_json::from_value(sat(
            "character_choices",
            "class_key,feature_key,pick,choice",
        )?)
        .unwrap_or_default(),
        spells: serde_json::from_value(sat("character_spells", "spell_key,state")?)
            .unwrap_or_default(),
        kit: read_kit(&token, &entity, 0)?,
    };

    serde_json::to_value(Envelope::wrap(creature, Some(stamp())))
        .map_err(|e| format!("could not write that creature out: {}", e))
}

/// Seconds since the epoch, as text.
///
/// A COURTESY, NOT A KEY. Nothing reads it back; it is there so a person
/// looking at three backups can tell which is which, and it avoids
/// taking a date dependency for one string.
fn stamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        .to_string()
}

/// What this game has, for `vet` to check a file against.
///
/// GLOBAL AND THIS GAME'S, which is the precedence every catalogue uses.
/// A creature referring to a campaign item of somebody else's game is
/// exactly the case the warnings exist for.
fn known_in(token: &str, game_id: &str) -> Result<Known, String> {
    let both = format!("(game_id.is.null,game_id.eq.{})", game_id);
    let keys = |t: &str| -> Result<HashSet<String>, String> {
        let rows = supabase::rest_get(token, t, &[("select", "key"), ("or", &both)])?;
        let empty = Vec::new();
        Ok(rows
            .as_array()
            .unwrap_or(&empty)
            .iter()
            .filter_map(|r| r.get("key").and_then(|v| v.as_str()).map(str::to_string))
            .collect())
    };
    Ok(Known {
        items: keys("items")?,
        spells: keys("spells")?,
        skills: keys("skills")?,
        classes: keys("classes")?,
        species: keys("species")?,
        npcs: keys("npcs")?,
    })
}

/// Write a kit back, a level at a time.
///
/// THE SAME REASON `copy_kit` DOES IT THIS WAY (124): only a container
/// has an `entity_id` and the trigger that mints one runs on insert, so
/// a child's holder cannot be known before the parent row exists.
fn write_kit(
    token: &str,
    game_id: &str,
    holder: &str,
    kit: &[KitItem],
    depth: usize,
) -> Result<(), String> {
    if depth >= 16 {
        return Ok(());
    }
    for it in kit {
        let mut row = json!({
            "game_id": game_id,
            "holder_id": holder,
            "item_key": it.item_key,
            "quantity": it.quantity,
            "attuned": it.attuned,
        });
        if let Some(v) = &it.name {
            row["name"] = json!(v);
        }
        if let Some(v) = &it.slot {
            row["slot"] = json!(v);
        }
        if let Some(v) = it.proficient_override {
            row["proficient_override"] = json!(v);
        }
        if let Some(v) = it.uses_spent {
            row["uses_spent"] = json!(v);
        }
        if let Some(v) = it.uses_max {
            row["uses_max"] = json!(v);
        }
        if let Some(v) = &it.grants {
            row["grants"] = v.clone();
        }

        let made = supabase::rest_insert(token, "objects", &row)?;
        let ent = made
            .as_array()
            .and_then(|a| a.first())
            .and_then(|r| r.get("entity_id"))
            .and_then(|v| v.as_str())
            .map(str::to_string);

        if !it.contents.is_empty() {
            if let Some(e) = ent {
                write_kit(token, game_id, &e, &it.contents, depth + 1)?;
            }
        }
    }
    Ok(())
}

/// Read a creature file into this game, as a template.
///
/// SKIP WITH A WARNING, NOT REFUSE - which is `creature_io::vet`'s whole
/// job. A creature carrying one unknown trinket is still worth having,
/// and a file that will not open because of a torch is a worse answer
/// than a goblin with no torch. Every skip comes back by name.
///
/// IT ARRIVES AS A TEMPLATE, never as a creature in the world. Placing
/// it stays the separate, deliberate act it already was.
#[tauri::command]
pub fn import_creature(
    state: State<AppState>,
    game_id: String,
    file: String,
) -> Result<Value, String> {
    let token = state.token()?;

    let env: Envelope = serde_json::from_str(&file)
        .map_err(|e| format!("that does not read as a creature file: {}", e))?;
    creature_io::admits(&env.format, env.version)?;

    let known = known_in(&token, &game_id)?;
    let (c, warnings) = creature_io::vet(env.creature, &known);

    let mut row = json!({
        "game_id": game_id,
        "name": c.name,
        "is_npc": true,
        "is_template": true,
        "level": c.level,
    });
    if let Some(v) = &c.size {
        row["size"] = json!(v);
    }
    if let Some(v) = &c.creature_type {
        row["creature_type"] = json!(v);
    }
    if let Some(v) = c.hp_max {
        row["hp_max"] = json!(v);
    }
    if let Some(v) = &c.ac_mode {
        row["ac_mode"] = json!(v);
    }
    if let Some(v) = c.ac_override {
        row["ac_override"] = json!(v);
    }
    if let Some(v) = c.prof_bonus {
        row["prof_bonus"] = json!(v);
    }
    if let Some(v) = &c.species_key {
        row["species_key"] = json!(v);
    }
    if let Some(v) = &c.class_key {
        row["class_key"] = json!(v);
    }
    if let Some(v) = &c.npc_key {
        row["npc_key"] = json!(v);
    }
    if let Some(v) = &c.description {
        row["description"] = json!(v);
    }
    if !c.weapon_profs.is_empty() {
        row["weapon_profs"] = json!(c.weapon_profs);
    }
    if !c.armor_profs.is_empty() {
        row["armor_profs"] = json!(c.armor_profs);
    }
    if !c.tool_profs.is_empty() {
        row["tool_profs"] = json!(c.tool_profs);
    }

    let made = supabase::rest_insert(&token, "characters", &row)
        .map_err(|e| denied(e, "import a creature"))?;
    let first = made
        .as_array()
        .and_then(|a| a.first())
        .ok_or_else(|| "the creature did not come back".to_string())?;
    let cid = first
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "the creature came back without an id".to_string())?
        .to_string();
    let entity = first
        .get("entity_id")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    // ABILITIES ARE UPDATED, NOT INSERTED: a trigger seeds six rows on
    // every new character, the same reason instantiate_character does.
    for a in &c.abilities {
        supabase::rest_update(
            &token,
            "character_abilities",
            &[
                ("character_id", &format!("eq.{}", cid)),
                ("ability", &format!("eq.{}", a.ability)),
            ],
            &json!({ "score": a.score, "save_prof": a.save_prof }),
        )?;
    }
    for s in &c.skills {
        supabase::rest_upsert(
            &token,
            "character_skills",
            &json!({ "character_id": cid, "skill_key": s.skill_key, "prof": s.prof }),
            "character_id,skill_key",
        )?;
    }
    for k in &c.classes {
        supabase::rest_upsert(
            &token,
            "character_classes",
            &json!({ "character_id": cid, "class_key": k.class_key, "level": k.level }),
            "character_id,class_key",
        )?;
    }
    for ch in &c.choices {
        supabase::rest_insert(
            &token,
            "character_choices",
            &json!({
                "character_id": cid,
                "class_key": ch.class_key,
                "feature_key": ch.feature_key,
                "pick": ch.pick,
                "choice": ch.choice
            }),
        )?;
    }
    for p in &c.spells {
        supabase::rest_upsert(
            &token,
            "character_spells",
            &json!({ "character_id": cid, "spell_key": p.spell_key, "state": p.state }),
            "character_id,spell_key",
        )?;
    }
    write_kit(&token, &game_id, &entity, &c.kit, 0)?;

    Ok(json!({
        "id": cid,
        "name": c.name,
        "warnings": warnings,
        "kit": creature_io::count_kit(&c.kit),
    }))
}
