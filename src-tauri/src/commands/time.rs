//! Moving the clock, and resting against it.
//!
//! 092. Plumbing. Every rule is clock.rs and uses.rs; this reads the
//! rows, applies them and writes back.
//!
//! THE DM MOVES TIME, NOT THE ENGINE. Outside a fight nothing can know
//! how long anything took, and an engine that inferred it would be
//! confidently wrong. Inside one, a round boundary is one tick and
//! `encounters.round` already marks it.
//!
//! A REST IS A DM ACTION ON WHOEVER IS PRESENT. One player resting
//! while the party does not would recharge them alone and leave the
//! clock behind, which is not a thing that happens at a table.

use serde_json::{json, Value};
use tauri::State;

use crate::clock::{self, Rest};
use crate::supabase::{self, AppState};

/// What time it is.
#[tauri::command]
pub fn game_clock(state: State<AppState>, game_id: String) -> Result<Value, String> {
    let token = state.token()?;
    let tick = read_tick(&token, &game_id)?;
    Ok(json!({
        "tick": tick,
        "reading": clock::reading(tick),
        "steps": clock::STEPS
            .iter()
            .map(|(label, ticks)| json!({ "label": label, "ticks": ticks }))
            .collect::<Vec<_>>(),
    }))
}

/// Move the clock forward.
///
/// FORWARD ONLY, and clock::advance says so rather than this - the
/// refusal is a rule and belongs where it can be tested.
#[tauri::command]
pub fn advance_time(state: State<AppState>, game_id: String, ticks: i64) -> Result<Value, String> {
    let token = state.token()?;
    let now = read_tick(&token, &game_id)?;
    let then = clock::advance(now, ticks)?;
    write_tick(&token, &game_id, then)?;
    Ok(json!({
        "tick": then,
        "reading": clock::reading(then),
        "passed": clock::said(ticks),
    }))
}

/// Everybody a rest applies to, and what it would do for them.
///
/// ASKED BEFORE IT HAPPENS so the DM can see who is going to be
/// refused - somebody who rested four hours ago is not a surprise worth
/// finding out about afterwards.
#[tauri::command]
pub fn rest_preview(
    state: State<AppState>,
    game_id: String,
    long: bool,
) -> Result<Value, String> {
    let token = state.token()?;
    let now = read_tick(&token, &game_id)?;
    let kind = if long { Rest::Long } else { Rest::Short };
    let folk = load_resters(&token, &game_id)?;

    let rows: Vec<Value> = folk
        .iter()
        .map(|p| {
            let refused = if long {
                clock::may_long_rest(now, p.last_long_rest, p.hp_now).err()
            } else {
                None
            };
            json!({
                "character_id": p.id,
                "name": p.name,
                "hp_now": p.hp_now,
                "hp_max": p.hp_max,
                "dice_spent": p.dice_spent,
                "dice_total": p.dice_total,
                "refused": refused,
            })
        })
        .collect();

    Ok(json!({
        "kind": kind.as_str(),
        "takes": clock::said(kind.ticks()),
        "who": rows,
    }))
}

/// Take a rest, for everybody in the game.
///
/// THE CLOCK MOVES EITHER WAY. An hour passes whether or not anybody
/// gets anything back from it - a party resting next to a character who
/// slept four hours ago still spent the hour.
#[tauri::command]
pub fn take_rest(state: State<AppState>, game_id: String, long: bool) -> Result<Value, String> {
    let token = state.token()?;
    let started = read_tick(&token, &game_id)?;
    let kind = if long { Rest::Long } else { Rest::Short };

    let folk = load_resters(&token, &game_id)?;
    let mut rested: Vec<String> = Vec::new();
    let mut refused: Vec<Value> = Vec::new();

    for p in &folk {
        if long {
            if let Err(why) = clock::may_long_rest(started, p.last_long_rest, p.hp_now) {
                refused.push(json!({ "name": p.name, "why": why }));
                continue;
            }
        }

        // WHAT COMES BACK IS WHAT THE TAG SAYS. `Rest::restores` holds
        // the asymmetry - a long rest gives back everything a short one
        // would, and not the reverse.
        recharge(&token, &p.id, kind)?;

        // 107. AND THE SPELL SLOTS, if the class gets them back from
        // this kind of rest. A cleric's return on a long rest and not
        // a short one; a warlock's on either, which is why
        // prayers::slots_restored takes the class rather than it being
        // a property of the slot.
        restore_slots(&token, &p.id, kind)?;

        if long {
            // ALL HIT POINTS, BY RECORDING THE HEALING RATHER THAN
            // ERASING THE WOUNDS. Current hit points are the maximum
            // plus the sum of the events, so wiping the table would
            // also reach full - and would throw away the record of
            // everything that ever hit them. 001's rule covers this:
            // events are never rewritten. A night's sleep is one more
            // event, and the log can still say what the night undid.
            let short_by = p.hp_max - p.hp_now;
            if short_by > 0 {
                supabase::rest_insert(
                    &token,
                    "hp_events",
                    &json!({
                        "game_id": game_id,
                        "character_id": p.id,
                        "delta": short_by,
                        "note": "long rest",
                    }),
                )?;
            }

            // AND HALF THE HIT DICE, minimum one, spread across the
            // classes that have spent any.
            return_dice(&token, &p.id, clock::dice_back(p.dice_total))?;

            supabase::rest_update(
                &token,
                "characters",
                &[("id", &format!("eq.{}", p.id))],
                &json!({ "last_long_rest": started }),
            )?;
        }
        rested.push(p.name.clone());
    }

    let then = clock::advance(started, kind.ticks())?;
    write_tick(&token, &game_id, then)?;

    Ok(json!({
        "kind": kind.as_str(),
        "tick": then,
        "reading": clock::reading(then),
        "rested": rested,
        "refused": refused,
    }))
}

/// Use one of something.
#[tauri::command]
pub fn spend_use(
    state: State<AppState>,
    character_id: String,
    class_key: String,
    feature_key: String,
) -> Result<Value, String> {
    let token = state.token()?;
    let (max, spent) = one_feature(&token, &character_id, &class_key, &feature_key)?;
    crate::uses::may_spend(max, spent)?;

    move_use(&token, &character_id, &class_key, &feature_key, spent, spent + 1)?;
    Ok(json!({ "left": crate::uses::left(max, spent + 1) }))
}

/// Give one back, for a misclick or a DM's say-so.
#[tauri::command]
pub fn restore_use(
    state: State<AppState>,
    character_id: String,
    class_key: String,
    feature_key: String,
) -> Result<Value, String> {
    let token = state.token()?;
    let (max, spent) = one_feature(&token, &character_id, &class_key, &feature_key)?;
    if spent <= 0 {
        return Err("nothing to give back".to_string());
    }
    move_use(&token, &character_id, &class_key, &feature_key, spent, spent - 1)?;
    Ok(json!({ "left": crate::uses::left(max, spent - 1) }))
}

/* ======================== THE WORKING PARTS ======================== */

/// Move one feature's spent count from `was` to `now`, and only if it
/// is still `was`.
///
/// 120. THE SAME FAULT THE SPELL SLOTS HAD, one function over. Reading
/// a count, checking it and then writing an absolute number is correct
/// exactly once: two presses that both read 2 both write 3, so Action
/// Surge is used twice and counted once. The value that was read goes
/// into the filter, so the second write matches no row.
///
/// NO ROW YET IS NOT A COLLISION - the first use of a feature has
/// nothing to update and falls through to an insert, where a second one
/// racing it loses on the primary key instead.
fn move_use(
    token: &str,
    character_id: &str,
    class_key: &str,
    feature_key: &str,
    was: i64,
    now: i64,
) -> Result<(), String> {
    const STALE: &str =
        "that count moved while this was in flight - look at the sheet and try again";

    let landed = supabase::rest_update_if(
        token,
        "character_uses",
        &[
            ("character_id", &format!("eq.{}", character_id)),
            ("class_key", &format!("eq.{}", class_key)),
            ("feature_key", &format!("eq.{}", feature_key)),
            ("spent", &format!("eq.{}", was)),
        ],
        &json!({ "spent": now.max(0) }),
    )?;
    if landed {
        return Ok(());
    }

    if was == 0 {
        return supabase::rest_insert(
            token,
            "character_uses",
            &json!({
                "character_id": character_id,
                "class_key": class_key,
                "feature_key": feature_key,
                "spent": now.max(0),
            }),
        )
        .map(|_| ())
        .map_err(|_| STALE.to_string());
    }

    Err(STALE.to_string())
}

struct Rester {
    id: String,
    name: String,
    hp_now: i64,
    hp_max: i64,
    dice_spent: i64,
    dice_total: i64,
    last_long_rest: Option<i64>,
}

/// Everybody in the game who can rest.
///
/// PLAYER CHARACTERS ONLY. A goblin's hit points are a DM's business
/// and `rederive_hp_max` has refused NPCs since 064 for the same
/// reason - healing the opposition to full on the party's rest is not a
/// thing anybody wants.
fn load_resters(token: &str, game_id: &str) -> Result<Vec<Rester>, String> {
    let rows = supabase::rest_get(
        token,
        "characters",
        &[
            ("select", "id,name,hp_max,last_long_rest"),
            ("game_id", &format!("eq.{}", game_id)),
            ("is_npc", "is.false"),
            // 123. A TEMPLATE IS NOT A CREATURE IN THE WORLD.
            ("is_template", "is.false"),
            ("is_active", "is.true"),
        ],
    )?;
    let people = rows.as_array().cloned().unwrap_or_default();
    if people.is_empty() {
        return Ok(Vec::new());
    }
    let ids: Vec<String> = people
        .iter()
        .filter_map(|r| r.get("id").and_then(|v| v.as_str()))
        .map(str::to_string)
        .collect();

    // Damage so far, and the dice they have spent - two reads for the
    // whole party rather than two per person.
    // A SIGNED DELTA, which 013 says in as many words: -7 is seven
    // damage and +5 is five healing, one column rather than an amount
    // plus a kind that could disagree with it. So current hit points
    // are the maximum PLUS the sum, not minus it.
    let events = supabase::rest_get(
        token,
        "hp_events",
        &[
            ("select", "character_id,delta"),
            ("character_id", &format!("in.({})", ids.join(","))),
        ],
    )?;
    let mut moved: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for r in events.as_array().unwrap_or(&Vec::new()) {
        let Some(cid) = r.get("character_id").and_then(|v| v.as_str()) else {
            continue;
        };
        *moved.entry(cid.to_string()).or_insert(0) +=
            r.get("delta").and_then(|v| v.as_i64()).unwrap_or(0);
    }

    let classes = supabase::rest_get(
        token,
        "character_classes",
        &[
            ("select", "character_id,level,hit_dice_spent"),
            ("character_id", &format!("in.({})", ids.join(","))),
        ],
    )?;
    let mut dice: std::collections::HashMap<String, (i64, i64)> = std::collections::HashMap::new();
    for r in classes.as_array().unwrap_or(&Vec::new()) {
        let Some(cid) = r.get("character_id").and_then(|v| v.as_str()) else {
            continue;
        };
        let e = dice.entry(cid.to_string()).or_insert((0, 0));
        e.0 += r.get("level").and_then(|v| v.as_i64()).unwrap_or(0);
        e.1 += r.get("hit_dice_spent").and_then(|v| v.as_i64()).unwrap_or(0);
    }

    Ok(people
        .iter()
        .filter_map(|r| {
            let id = r.get("id")?.as_str()?.to_string();
            let hp_max = r.get("hp_max").and_then(|v| v.as_i64()).unwrap_or(0);
            let (total, spent) = dice.get(&id).copied().unwrap_or((0, 0));
            Some(Rester {
                hp_now: hp_max + moved.get(&id).copied().unwrap_or(0),
                hp_max,
                dice_total: total,
                dice_spent: spent,
                last_long_rest: r.get("last_long_rest").and_then(|v| v.as_i64()),
                name: r.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                id,
            })
        })
        .collect())
}

/// Hand back spell slots, for whichever classes get them from this
/// rest.
///
/// PER CLASS, because the rule is. A Cleric 5 / Warlock 2 on a short
/// rest gets their pact slots and not their cleric ones - which falls
/// out of asking the question once per class rather than once per
/// character.
fn restore_slots(token: &str, character_id: &str, kind: Rest) -> Result<(), String> {
    let rows = supabase::rest_get(
        token,
        "character_classes",
        &[
            ("select", "class_key"),
            ("character_id", &format!("eq.{}", character_id)),
        ],
    )?;
    let gives_back = rows
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|r| r.get("class_key").and_then(|v| v.as_str()))
        .any(|k| crate::prayers::slots_restored(k, kind == Rest::Long));

    if gives_back {
        crate::commands::prayers::clear_slots(token, character_id)?;
    }
    Ok(())
}

/// Clear what this rest brings back.
///
/// ONLY THE FEATURES WHOSE TAG SAYS SO, which is why this reads the
/// catalogue rather than wiping the table: a short rest that cleared
/// everything would hand back a barbarian's Rage.
fn recharge(token: &str, character_id: &str, kind: Rest) -> Result<(), String> {
    let spent = supabase::rest_get(
        token,
        "character_uses",
        &[
            ("select", "class_key,feature_key,spent"),
            ("character_id", &format!("eq.{}", character_id)),
            ("spent", "gt.0"),
        ],
    )?;
    let spent = spent.as_array().cloned().unwrap_or_default();
    if spent.is_empty() {
        return Ok(());
    }

    let keys: Vec<String> = spent
        .iter()
        .filter_map(|r| r.get("class_key").and_then(|v| v.as_str()))
        .map(str::to_string)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    let cat = supabase::rest_get(
        token,
        "class_features",
        &[
            ("select", "class_key,key,recharge"),
            ("class_key", &format!("in.({})", keys.join(","))),
            ("recharge", "not.is.null"),
        ],
    )?;

    for row in &spent {
        let (Some(ck), Some(fk)) = (
            row.get("class_key").and_then(|v| v.as_str()),
            row.get("feature_key").and_then(|v| v.as_str()),
        ) else {
            continue;
        };
        let tag = cat.as_array().unwrap_or(&Vec::new()).iter().find_map(|c| {
            (c.get("class_key").and_then(|v| v.as_str()) == Some(ck)
                && c.get("key").and_then(|v| v.as_str()) == Some(fk))
            .then(|| c.get("recharge").and_then(|v| v.as_str()).unwrap_or("").to_string())
        });
        let Some(tag) = tag else { continue };
        if !kind.restores(&tag) {
            continue;
        }
        supabase::rest_update(
            token,
            "character_uses",
            &[
                ("character_id", &format!("eq.{}", character_id)),
                ("class_key", &format!("eq.{}", ck)),
                ("feature_key", &format!("eq.{}", fk)),
            ],
            &json!({ "spent": 0 }),
        )?;
    }
    Ok(())
}

/// Hand back `how_many` hit dice, taking them off whichever classes
/// have spent any.
fn return_dice(token: &str, character_id: &str, how_many: i64) -> Result<(), String> {
    if how_many <= 0 {
        return Ok(());
    }
    let rows = supabase::rest_get(
        token,
        "character_classes",
        &[
            ("select", "class_key,hit_dice_spent"),
            ("character_id", &format!("eq.{}", character_id)),
            ("hit_dice_spent", "gt.0"),
            ("order", "added_at.asc"),
        ],
    )?;
    let mut left = how_many;
    for r in rows.as_array().unwrap_or(&Vec::new()) {
        if left <= 0 {
            break;
        }
        let Some(key) = r.get("class_key").and_then(|v| v.as_str()) else {
            continue;
        };
        let spent = r.get("hit_dice_spent").and_then(|v| v.as_i64()).unwrap_or(0);
        let back = left.min(spent);
        supabase::rest_update(
            token,
            "character_classes",
            &[
                ("character_id", &format!("eq.{}", character_id)),
                ("class_key", &format!("eq.{}", key)),
            ],
            &json!({ "hit_dice_spent": spent - back }),
        )?;
        left -= back;
    }
    Ok(())
}

/// One feature's ceiling and what has been spent against it.
fn one_feature(
    token: &str,
    character_id: &str,
    class_key: &str,
    feature_key: &str,
) -> Result<(Option<i64>, i64), String> {
    let sheet = crate::character::load_sheet(token, character_id)?;
    let Some(held) = sheet.classes.iter().find(|t| t.key == class_key) else {
        return Err("they do not have that class".to_string());
    };

    let rows = supabase::rest_get(
        token,
        "class_features",
        &[
            ("select", "uses"),
            ("class_key", &format!("eq.{}", class_key)),
            ("key", &format!("eq.{}", feature_key)),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", sheet.game_id)),
        ],
    )?;
    let expr = rows
        .as_array()
        .and_then(|a| a.first())
        .and_then(|r| r.get("uses"))
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let ctx = context_for(&sheet, held.level);
    let max = crate::uses::count(expr.as_deref(), &ctx)?;

    let rows = supabase::rest_get(
        token,
        "character_uses",
        &[
            ("select", "spent"),
            ("character_id", &format!("eq.{}", character_id)),
            ("class_key", &format!("eq.{}", class_key)),
            ("feature_key", &format!("eq.{}", feature_key)),
        ],
    )?;
    let spent = rows
        .as_array()
        .and_then(|a| a.first())
        .and_then(|r| r.get("spent"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    Ok((max, spent))
}

/// THE CLASS'S LEVEL AND THE SHEET'S MODIFIERS. A Fighter 4 / Bard 1
/// gets one Action Surge because they are a Fighter 4 - reading their
/// total of 5 is the mistake features::held exists to avoid.
pub fn context_for(sheet: &crate::character::Sheet, class_level: i64) -> crate::uses::Context {
    crate::uses::Context {
        level: class_level,
        str_mod: sheet.ability_mod("str"),
        dex_mod: sheet.ability_mod("dex"),
        con_mod: sheet.ability_mod("con"),
        int_mod: sheet.ability_mod("int"),
        wis_mod: sheet.ability_mod("wis"),
        cha_mod: sheet.ability_mod("cha"),
    }
}

fn read_tick(token: &str, game_id: &str) -> Result<i64, String> {
    let rows = supabase::rest_get(
        token,
        "games",
        &[("select", "tick"), ("id", &format!("eq.{}", game_id))],
    )?;
    rows.as_array()
        .and_then(|a| a.first())
        .and_then(|r| r.get("tick"))
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "no such game".to_string())
}

fn write_tick(token: &str, game_id: &str, tick: i64) -> Result<(), String> {
    supabase::rest_update(
        token,
        "games",
        &[("id", &format!("eq.{}", game_id))],
        &json!({ "tick": tick }),
    )?;
    Ok(())
}
