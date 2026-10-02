//! Reading a character's class features, and recording a choice.
//!
//! 087. Plumbing. Everything that decides anything is features.rs -
//! which features a character has, what they still owe a decision on,
//! and whether a particular choice may be recorded.
//!
//! WHAT THIS ADDS is the options: a sheet asking "which ability" needs
//! the six codes, and a sheet asking "which skill" for Expertise needs
//! the ones this character is actually proficient with, because
//! doubling a proficiency you do not have is nothing. Those lists are
//! about a PARTICULAR character and so cannot live in the catalogue.

use serde_json::{json, Value};
use tauri::State;

use crate::features::{Choice, Feature};
use crate::supabase::{self, AppState};

/// The six fighting styles the PHB gives a base class.
///
/// A LIST IN RUST RATHER THAN A TABLE, because unlike an audience or a
/// class these are not campaign data - each one is a rule the engine
/// would have to implement to mean anything by it, and a DM inventing a
/// seventh would get a word on a sheet and nothing else. When they are
/// implemented they will want their own rows; until then saying so here
/// is more honest than a table nobody may safely add to.
///
/// NONE OF THEM DO ANYTHING YET. The choice is recorded and shown; no
/// attack roll reads it. Archery's +2 and Defense's +1 are the first
/// two that should, and both want the modifier pipeline 080 started.
const FIGHTING_STYLES: &[(&str, &str)] = &[
    ("archery", "Archery - +2 to hit with ranged weapons"),
    ("defense", "Defense - +1 AC while wearing armour"),
    ("duelling", "Duelling - +2 damage with a one-handed weapon and no second one"),
    ("great_weapon", "Great Weapon Fighting - reroll 1s and 2s on two-handed damage"),
    ("protection", "Protection - impose disadvantage on an attack against someone beside you"),
    ("two_weapon", "Two-Weapon Fighting - add your modifier to the off-hand damage"),
];

/// Which fighting styles a class offers.
///
/// The Fighter gets all six; the Paladin has no ranged option and the
/// Ranger no two-handed one. That is 5e, and it is here rather than in
/// the catalogue because the styles themselves are.
fn styles_for(class_key: &str) -> Vec<&'static (&'static str, &'static str)> {
    FIGHTING_STYLES
        .iter()
        .filter(|(k, _)| match class_key {
            "paladin" => *k != "archery" && *k != "two_weapon",
            "ranger" => *k != "great_weapon" && *k != "protection",
            _ => true,
        })
        .collect()
}

/// Everything this character's classes have given them, and what is
/// still waiting on a decision.
#[tauri::command]
pub fn list_features(state: State<AppState>, character_id: String) -> Result<Value, String> {
    let token = state.token()?;
    let sheet = crate::character::load_sheet(&token, &character_id)?;

    let catalogue = load_catalogue(&token, &sheet.game_id, &sheet.classes)?;
    let choices = load_choices(&token, &character_id)?;
    // `sheet.classes` arrives lead-first, and features::held keeps that
    // order so this list reads in the same order as the headline.
    let held = crate::features::held(&sheet.classes, &catalogue, &choices);
    // 092. WHAT HAS BEEN SPENT, in one read for the whole sheet.
    let spent = load_spent(&token, &character_id)?;

    let out: Vec<Value> = held
        .iter()
        .map(|h| {
            // THE CLASS'S LEVEL, which is the whole reason this is not
            // one number: a Fighter 4 / Bard 1 gets one Action Surge.
            let level = sheet
                .classes
                .iter()
                .find(|t| t.key == h.feature.class_key)
                .map(|t| t.level)
                .unwrap_or(1);
            let ctx = crate::commands::time::context_for(&sheet, level);
            // A MALFORMED EXPRESSION READS AS NO LIMIT HERE rather than
            // emptying the list. uses::count refuses it loudly and the
            // spend path still will - a typo in the catalogue should
            // not take every other feature off the screen with it.
            let max = crate::uses::count(h.feature.uses.as_deref(), &ctx).unwrap_or(None);
            let used = spent
                .get(&(h.feature.class_key.clone(), h.feature.key.clone()))
                .copied()
                .unwrap_or(0);
            json!({
                "class_key": h.feature.class_key,
                "level": h.feature.level,
                "key": h.feature.key,
                "name": h.feature.name,
                "text": h.feature.text,
                "choose_from": h.feature.choose_from,
                "picks": h.feature.picks,
                "chosen": h.chosen,
                "owed": h.owed,
                // 092. WHAT IS LEFT, worked out per class level - a
                // Fighter 4 / Bard 1 gets one Action Surge because
                // they are a Fighter 4, never the two a level 5 might
                // suggest. None means no limit worth tracking, which
                // must not read the same as none left.
                "uses": max,
                "spent": used,
                "left": crate::uses::left(max, used),
                "recharge": h.feature.recharge,
                // ONLY FOR WHAT IS STILL OWED. Building the option list
                // for a decided feature would be work nobody reads.
                "options": if h.owed > 0 { options_for(h, &sheet) } else { json!([]) },
            })
        })
        .collect();

    // HOW MANY ARE WAITING, counted in Rust. The screen could filter
    // the list itself, and that would be a second place that knows
    // what "waiting" means - which is how the Karma preview came to
    // agree with the engine and be wrong alongside it.
    Ok(json!({
        "features": out,
        "waiting": crate::features::outstanding(&held).len(),
    }))
}

/// What a particular feature can be answered with, for this character.
fn options_for(h: &crate::features::Held, sheet: &crate::character::Sheet) -> Value {
    match h.feature.choose_from.as_deref() {
        Some("ability") => json!(["str", "dex", "con", "int", "wis", "cha"]
            .iter()
            .map(|c| json!({ "key": c, "name": c.to_uppercase() }))
            .collect::<Vec<_>>()),

        // EXPERTISE DOUBLES A PROFICIENCY, so the only skills worth
        // offering are the ones this character already has. Offering
        // the other eleven would let somebody spend a Rogue's best
        // feature on nothing.
        Some("skill") => json!(sheet
            .skills
            .iter()
            .filter(|s| sheet.profs.get(&s.key).copied().unwrap_or(0.0) >= 1.0)
            .map(|s| json!({ "key": s.key, "name": s.name }))
            .collect::<Vec<_>>()),

        Some("fighting_style") => json!(styles_for(&h.feature.class_key)
            .iter()
            .map(|(k, label)| json!({ "key": k, "name": label }))
            .collect::<Vec<_>>()),

        // 088 seeds the subclass choice POINTS with choose_from NULL
        // until there are archetypes to offer, so this arm is
        // unreachable today and is here for the pass that adds them.
        _ => json!([]),
    }
}

/// Record a decision.
///
/// THE LOCK IS THE POLICY, NOT THIS. `character_choices` lets an owner
/// INSERT and only a DM UPDATE or DELETE, so a player answering twice
/// is refused by the database rather than by a check here that a second
/// code path could forget. What this adds is the readable refusal -
/// "already decided, a DM can change it" rather than a constraint
/// violation.
#[tauri::command]
pub fn choose_feature(
    state: State<AppState>,
    character_id: String,
    class_key: String,
    feature_key: String,
    pick: i64,
    choice: String,
) -> Result<Value, String> {
    let token = state.token()?;
    let sheet = crate::character::load_sheet(&token, &character_id)?;
    let catalogue = load_catalogue(&token, &sheet.game_id, &sheet.classes)?;
    let choices = load_choices(&token, &character_id)?;
    let held = crate::features::held(&sheet.classes, &catalogue, &choices);

    // THEIRS, AND STILL OWED. Both are the same lookup: a feature they
    // do not have is not in this list at all, which is the same refusal
    // as one they have already answered.
    let Some(h) = held
        .iter()
        .find(|h| h.feature.class_key == class_key && h.feature.key == feature_key)
    else {
        return Err("that is not one of their features".to_string());
    };
    crate::features::may_choose(h)?;

    let want = choice.trim();
    if want.is_empty() {
        return Err("choose something".to_string());
    }
    if pick < 1 || pick > h.feature.picks {
        return Err(format!("{} has {} to choose", h.feature.name, h.feature.picks));
    }

    let written = supabase::rest_insert(
        &token,
        "character_choices",
        &json!({
            "character_id": character_id,
            "class_key": class_key,
            "feature_key": feature_key,
            "pick": pick,
            "choice": want,
        }),
    )?;

    // 091. AND THE HIT POINTS, when the choice was Constitution. An
    // Ability Score Improvement into CON is worth a point per level and
    // `hp_max` is stored, so it has to be told. Only for an ability
    // choice - a fighting style moves no number this knows about.
    //
    // The same ripple `set_level` and `set_class_level` already do, and
    // for the reason 029 gave: a maximum nothing recomputes is a
    // maximum that goes stale the first time anything under it moves.
    if h.feature.choose_from.as_deref() == Some("ability") {
        crate::commands::characters::rederive_hp_max(&token, &character_id)?;
    }
    Ok(written)
}

/// The catalogue rows for the classes this character holds.
///
/// SKIPPED ENTIRELY FOR THE CLASSLESS, which is every monster - the
/// same early return `class::load_map` makes, for the same reason.
fn load_catalogue(
    token: &str,
    game_id: &str,
    classes: &[crate::multiclass::Taken],
) -> Result<Vec<Feature>, String> {
    if classes.is_empty() {
        return Ok(Vec::new());
    }
    let keys: Vec<String> = classes.iter().map(|t| t.key.clone()).collect();
    let rows = supabase::rest_get(
        token,
        "class_features",
        &[
            ("select", "class_key,level,key,name,text,choose_from,picks,uses,recharge"),
            ("class_key", &format!("in.({})", keys.join(","))),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
        ],
    )?;
    Ok(rows
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        // ONE READER, in features.rs, so the two places that load a
        // catalogue cannot come to disagree about what a row means.
        .map(crate::features::feature_from_row)
        .collect())
}

fn load_choices(token: &str, character_id: &str) -> Result<Vec<Choice>, String> {
    let rows = supabase::rest_get(
        token,
        "character_choices",
        &[
            ("select", "class_key,feature_key,pick,choice"),
            ("character_id", &format!("eq.{}", character_id)),
        ],
    )?;
    Ok(rows
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|r| Choice {
            class_key: as_str(r, "class_key"),
            feature_key: as_str(r, "feature_key"),
            pick: r.get("pick").and_then(|v| v.as_i64()).unwrap_or(1),
            choice: as_str(r, "choice"),
        })
        .collect())
}

/// What this character has spent, keyed by (class, feature).
fn load_spent(
    token: &str,
    character_id: &str,
) -> Result<std::collections::HashMap<(String, String), i64>, String> {
    let rows = supabase::rest_get(
        token,
        "character_uses",
        &[
            ("select", "class_key,feature_key,spent"),
            ("character_id", &format!("eq.{}", character_id)),
        ],
    )?;
    Ok(rows
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|r| {
            (
                (as_str(r, "class_key"), as_str(r, "feature_key")),
                r.get("spent").and_then(|v| v.as_i64()).unwrap_or(0),
            )
        })
        .collect())
}

fn as_str(r: &Value, key: &str) -> String {
    r.get(key).and_then(|v| v.as_str()).unwrap_or_default().to_string()
}
