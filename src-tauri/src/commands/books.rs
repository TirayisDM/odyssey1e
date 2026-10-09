//! Spellbooks: what is written in one, and writing something new.
//!
//! 172. PLUMBING ONLY. Every rule here comes from `scribe.rs` - what a
//! copy costs, how much room it takes, who may make it - and this reads
//! rows, calls one of those, and writes rows. The architecture line in
//! `mod.rs` is the whole brief.

use serde_json::{json, Value};
use tauri::State;

use crate::casting;
use crate::scribe;
use crate::supabase::{self, AppState};

/// One book and everything written in it.
///
/// READS THE BOOK, NOT THE READER. A book found on a shelf answers this
/// as readily as the one in a wizard's hands - that is the whole point
/// of 172 keying the writing to the object.
#[tauri::command]
pub fn list_book(state: State<AppState>, object_id: String) -> Result<Value, String> {
    let token = state.token()?;
    let (name, capacity, game_id, scroll) = book_of(&token, &object_id)?;
    let written = written_in(&token, &object_id)?;

    // The catalogue rows for what is in it, so the screen can show a
    // level and a school rather than a key.
    let mut out: Vec<Value> = Vec::new();
    let mut used = 0;
    if !written.is_empty() {
        let keys: Vec<String> = written.iter().map(|k| crate::narrative::quoted(k)).collect();
        let rows = supabase::rest_get(
            &token,
            "spells",
            &[
                ("select", "key,name,level,school,category"),
                ("key", &format!("in.({})", keys.join(","))),
                ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
                ("order", "level.asc,name.asc"),
            ],
        )?;
        let mut seen = std::collections::HashSet::new();
        for r in rows.as_array().unwrap_or(&Vec::new()) {
            let Some(key) = r.get("key").and_then(|v| v.as_str()) else {
                continue;
            };
            if !seen.insert(key.to_string()) {
                continue;
            }
            let level = r.get("level").and_then(|v| v.as_i64()).unwrap_or(0);
            let pages = scribe::pages_on(level, scroll);
            used += pages;
            let mut row = r.clone();
            row["pages"] = json!(pages);
            out.push(row);
        }
    }

    Ok(json!({
        "object_id": object_id,
        "name": name,
        "capacity": capacity,
        "used": used,
        "left": (capacity - used).max(0),
        // 175. So the screen can say "scroll" and offer the right
        // things - a scroll is read and copied FROM, not prepared from.
        "scroll": scroll,
        "spells": out,
    }))
}

/// Write a spell into a book.
///
/// 172. THE WHOLE ACT, IN ORDER, AND IT REFUSES BEFORE IT SPENDS. Every
/// check runs before the first write, because a scribing that takes the
/// ink and then discovers the book is full has cost somebody money for
/// nothing. 116 made the same choice about a spell cast without naming
/// its damage type.
///
/// THE CLOCK IS NOT ADVANCED. `scribe::to_copy` says how many hours it
/// takes and this reports them, but moving the clock expires effects
/// and ends concentration - consequences that belong to a DM saying
/// "you spend the afternoon", not to a button. The hours are in the
/// answer and the DM spends them.
#[tauri::command]
pub fn scribe_spell(
    state: State<AppState>,
    character_id: String,
    object_id: String,
    spell_key: String,
) -> Result<Value, String> {
    let token = state.token()?;
    let sheet = crate::character::load_sheet(&token, &character_id)?;
    let Some(caster) = sheet.caster.clone() else {
        return Err(format!("{} does not cast spells", sheet.name));
    };

    let (book_name, capacity, game_id, scroll) = book_of(&token, &object_id)?;
    let spell = one_spell(&token, &game_id, &spell_key)?;
    let level = spell.get("level").and_then(|v| v.as_i64()).unwrap_or(0);
    let school = spell.get("school").and_then(|v| v.as_str()).unwrap_or("");
    let name = spell
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(&spell_key)
        .to_string();
    let classes: Vec<String> = spell
        .get("classes")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|c| c.as_str()).map(String::from).collect())
        .unwrap_or_default();

    // THE HIGHEST SLOT THEY HAVE, which is what the book's own rule
    // means by "a spell you can prepare".
    let top_slot = casting::slots_at(caster.level)
        .iter()
        .enumerate()
        .filter(|(_, n)| **n > 0)
        .map(|(i, _)| i as i64 + 1)
        .next_back()
        .unwrap_or(0);

    let already = written_in(&token, &object_id)?;
    scribe::may_copy(
        caster.source,
        &classes,
        &caster.class_key,
        level,
        top_slot,
        already.contains(&spell_key),
    )?;

    // How full it is, in the unit 171 chose.
    let used = used_levels(&token, &game_id, &already, scroll)?;
    // 175. ON A SCROLL THE SPELL TAKES THE SCROLL, whatever its level,
    // so a one-page scroll holds exactly one of anything.
    scribe::fits_on(capacity, used, level, scroll)?;

    let pct = school_pct(&token, school)?;
    // WRITING A SCROLL IS TWICE WRITING IT DOWN. Copying into a book is
    // transcription - the spell is understood and the book is a
    // reference. A scroll carries the whole working on its own, for
    // somebody who may not understand it.
    let cost = if scroll {
        scribe::to_scroll(level, pct)
    } else {
        scribe::to_copy(level, pct)
    };

    // THE TOOL, WHICH IS NOT CONSUMED. 170: nothing is written without
    // one, and it is not used up by the work.
    let entity = entity_of(&token, &character_id)?;
    if count_held(&token, &entity, "quill")? < 1 {
        return Err("they have no quill - nothing is written without one".to_string());
    }
    let ink = count_held(&token, &entity, "ink_vial")?;
    if ink < cost.ink {
        return Err(format!(
            "copying {} takes {} vials of ink and they have {}",
            name, cost.ink, ink
        ));
    }

    // ---- past every refusal; now it spends ----
    spend_held(&token, &entity, "ink_vial", cost.ink)?;
    supabase::rest_insert(
        &token,
        "scribed_spells",
        &json!({ "object_id": object_id, "spell_key": spell_key }),
    )?;

    Ok(json!({
        "spell": name,
        "book": book_name,
        "hours": cost.hours,
        "ink": cost.ink,
        "pages": scribe::pages_on(level, scroll),
        "left": (capacity - used - scribe::pages_on(level, scroll)).max(0),
        "said": format!(
            "{} copied into {} - {} hours and {} vials, {} levels of room left",
            name,
            book_name,
            cost.hours,
            cost.ink,
            (capacity - used - scribe::pages_on(level, scroll)).max(0)
        ),
    }))
}

/// Everything this character could write into this thing, priced.
///
/// 192. THE ENGINE PRICES IT, NOT THE SCREEN. 191 put the cost on the
/// picker by doing the arithmetic again in JavaScript, which worked and
/// was one fact in two places - the per-school percentage was the half
/// that did not make the journey, and the label quietly under-reported
/// the moment anybody tuned a school.
///
/// So the list arrives priced. `scribe.rs` is still the only thing that
/// knows what a copy costs, the screen reads `ink` and `hours` off each
/// row, and a school tuned in `scribe_schools` shows up on the label the
/// same second it starts being charged.
///
/// ONE QUERY FOR THE SCHOOLS, not one per spell. Eight rows, read once
/// and looked up in memory - the alternative is a round trip per
/// candidate and there are two hundred of them.
///
/// EMPTY FOR ANYBODY WHO DOES NOT WRITE. A cleric gets no list rather
/// than an error: the screen asks this to decide whether to offer
/// anything at all, and "nothing to offer" is the answer, not a fault.
#[tauri::command]
pub fn scribe_options(
    state: State<AppState>,
    character_id: String,
    object_id: String,
) -> Result<Value, String> {
    let token = state.token()?;
    let sheet = crate::character::load_sheet(&token, &character_id)?;
    let Some(caster) = sheet.caster.clone() else {
        return Ok(json!({ "ink": 0, "options": [] }));
    };
    if caster.source != casting::Source::Book {
        return Ok(json!({ "ink": 0, "options": [] }));
    }

    let (_, _, game_id, scroll) = book_of(&token, &object_id)?;
    let written: std::collections::HashSet<String> =
        written_in(&token, &object_id)?.into_iter().collect();
    let rates = school_rates(&token)?;
    let ink = count_held(&token, &entity_of(&token, &character_id)?, "ink_vial")?;

    let rows = supabase::rest_get(
        &token,
        "spells",
        &[
            ("select", "key,name,level,school,game_id"),
            ("classes", &format!("cs.{{{}}}", caster.class_key)),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
            ("order", "level.asc,name.asc"),
        ],
    )?;

    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<Value> = Vec::new();
    for r in rows.as_array().unwrap_or(&Vec::new()) {
        let Some(key) = r.get("key").and_then(|v| v.as_str()) else {
            continue;
        };
        if written.contains(key) || !seen.insert(key.to_string()) {
            continue;
        }
        let level = r.get("level").and_then(|v| v.as_i64()).unwrap_or(0);
        let school = r.get("school").and_then(|v| v.as_str()).unwrap_or("");
        let pct = rates.get(school).copied().unwrap_or(100);
        let cost = if scroll {
            scribe::to_scroll(level, pct)
        } else {
            scribe::to_copy(level, pct)
        };
        out.push(json!({
            "key": key,
            "name": r.get("name").and_then(|v| v.as_str()).unwrap_or(key),
            "level": level,
            "school": school,
            "ink": cost.ink,
            "hours": cost.hours,
            "pages": scribe::pages_on(level, scroll),
            "affordable": cost.ink <= ink,
        }));
    }

    Ok(json!({ "ink": ink, "scroll": scroll, "options": out }))
}

/// Copy the spell off a scroll and into a book, consuming the scroll.
///
/// 175. WHAT A SCROLL IS FOR. A wizard cannot prepare from one - that
/// is `in_books` and the tag it filters on - so the only thing to do
/// with somebody else's scroll is to take the hour and the ink and put
/// it in the book where it can be prepared.
///
/// IT COSTS WHAT COPYING COSTS, not what writing the scroll cost. This
/// is transcription from a source that is already worked out, which is
/// the cheap direction; `to_scroll` is the expensive one and is what
/// made the scroll in the first place.
///
/// THE SCROLL IS DESTROYED, which is the rule everywhere this exists
/// and is why a scroll is worth what it is. 172's cascade does the
/// writing for us: delete the object and its one row goes with it.
#[tauri::command]
pub fn copy_from_scroll(
    state: State<AppState>,
    character_id: String,
    scroll_id: String,
    book_id: String,
) -> Result<Value, String> {
    let token = state.token()?;
    let sheet = crate::character::load_sheet(&token, &character_id)?;
    let Some(caster) = sheet.caster.clone() else {
        return Err(format!("{} does not cast spells", sheet.name));
    };

    let (scroll_name, _, game_id, is_scroll) = book_of(&token, &scroll_id)?;
    if !is_scroll {
        return Err(format!("{} is not a scroll", scroll_name));
    }
    let on_it = written_in(&token, &scroll_id)?;
    let Some(spell_key) = on_it.first().cloned() else {
        return Err(format!("{} is blank", scroll_name));
    };

    let (book_name, capacity, _, book_is_scroll) = book_of(&token, &book_id)?;
    if book_is_scroll {
        return Err("a scroll is not a book to copy into".to_string());
    }

    let spell = one_spell(&token, &game_id, &spell_key)?;
    let level = spell.get("level").and_then(|v| v.as_i64()).unwrap_or(0);
    let school = spell.get("school").and_then(|v| v.as_str()).unwrap_or("");
    let name = spell
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(&spell_key)
        .to_string();
    let classes: Vec<String> = spell
        .get("classes")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|c| c.as_str()).map(String::from).collect())
        .unwrap_or_default();

    let top_slot = casting::slots_at(caster.level)
        .iter()
        .enumerate()
        .filter(|(_, n)| **n > 0)
        .map(|(i, _)| i as i64 + 1)
        .next_back()
        .unwrap_or(0);

    let already = written_in(&token, &book_id)?;
    scribe::may_copy(
        caster.source,
        &classes,
        &caster.class_key,
        level,
        top_slot,
        already.contains(&spell_key),
    )?;
    let used = used_levels(&token, &game_id, &already, false)?;
    scribe::fits_on(capacity, used, level, false)?;

    let cost = scribe::to_copy(level, school_pct(&token, school)?);
    let entity = entity_of(&token, &character_id)?;
    if count_held(&token, &entity, "quill")? < 1 {
        return Err("they have no quill - nothing is written without one".to_string());
    }
    let ink = count_held(&token, &entity, "ink_vial")?;
    if ink < cost.ink {
        return Err(format!(
            "copying {} takes {} vials of ink and they have {}",
            name, cost.ink, ink
        ));
    }

    // ---- past every refusal ----
    spend_held(&token, &entity, "ink_vial", cost.ink)?;
    supabase::rest_insert(
        &token,
        "scribed_spells",
        &json!({ "object_id": book_id, "spell_key": spell_key }),
    )?;
    // AND THE SCROLL IS GONE. The writing on it goes too, by 172's
    // cascade - one delete, not two.
    supabase::rest_delete(&token, "objects", &[("id", &format!("eq.{}", scroll_id))])?;

    Ok(json!({
        "spell": name,
        "book": book_name,
        "hours": cost.hours,
        "ink": cost.ink,
        "said": format!(
            "{} copied from {} into {} - {} hours and {} vials, and the scroll is spent",
            name, scroll_name, book_name, cost.hours, cost.ink
        ),
    }))
}

/// Scrape a spell out of a book.
///
/// NOTHING COMES BACK. The ink is spent and the page is ruined - this
/// is for a DM correcting a mistake and for a wizard making room, not
/// for a refund.
#[tauri::command]
pub fn erase_spell(
    state: State<AppState>,
    object_id: String,
    spell_key: String,
) -> Result<Value, String> {
    let token = state.token()?;
    supabase::rest_delete(
        &token,
        "scribed_spells",
        &[
            ("object_id", &format!("eq.{}", object_id)),
            ("spell_key", &format!("eq.{}", spell_key)),
        ],
    )?;
    Ok(json!({ "erased": spell_key }))
}

/* ======================== the reads ======================== */

/// The object as a book: its name, how many spell levels it holds, and
/// whose game it is in. Refuses anything that is not a book, which is
/// `scribe.rs`'s rule about writing in a sword made concrete.
fn book_of(token: &str, object_id: &str) -> Result<(String, i64, String, bool), String> {
    let rows = supabase::rest_get(
        token,
        "objects",
        &[
            ("select", "id,name,item_key,game_id"),
            ("id", &format!("eq.{}", object_id)),
        ],
    )?;
    let obj = rows
        .as_array()
        .and_then(|a| a.first())
        .cloned()
        .ok_or_else(|| "no such thing to write in".to_string())?;
    let item_key = obj.get("item_key").and_then(|v| v.as_str()).unwrap_or("");
    let game_id = obj
        .get("game_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let items = supabase::rest_get(
        token,
        "items",
        &[
            ("select", "key,name,spell_levels,content_tags,game_id"),
            ("key", &format!("eq.{}", item_key)),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
            // THE GAME'S OWN ROW WINS. `game_id.desc` would not do it:
            // Postgres puts NULLS FIRST on a descending sort, so the
            // global row would come back ahead of the override. This is
            // the spelling casting.rs uses.
            ("order", "game_id.asc.nullslast"),
        ],
    )?;
    let item = items
        .as_array()
        .and_then(|a| a.first())
        .cloned()
        .ok_or_else(|| format!("no item with key '{}'", item_key))?;
    let capacity = item
        .get("spell_levels")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| {
            format!(
                "{} is not a spellbook - nothing can be written in it",
                item.get("name").and_then(|v| v.as_str()).unwrap_or(item_key)
            )
        })?;

    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .or_else(|| item.get("name").and_then(|v| v.as_str()))
        .unwrap_or(item_key)
        .to_string();
    // 175. A SCROLL IS A ONE-PAGE BOOK, and the tag is what says so.
    // `spell_levels` cannot: a scroll has one and so could a very small
    // book, and only one of the two may be prepared from.
    let scroll = item
        .get("content_tags")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().any(|t| t.as_str() == Some("scroll")))
        .unwrap_or(false);
    Ok((name, capacity, game_id, scroll))
}

fn written_in(token: &str, object_id: &str) -> Result<Vec<String>, String> {
    let rows = supabase::rest_get(
        token,
        "scribed_spells",
        &[
            ("select", "spell_key"),
            ("object_id", &format!("eq.{}", object_id)),
        ],
    )?;
    Ok(rows
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|r| r.get("spell_key")?.as_str().map(String::from))
        .collect())
}

/// How many levels of room a set of written spells takes up.
fn used_levels(token: &str, game_id: &str, keys: &[String], scroll: bool) -> Result<i64, String> {
    if keys.is_empty() {
        return Ok(0);
    }
    let quoted: Vec<String> = keys.iter().map(|k| crate::narrative::quoted(k)).collect();
    let rows = supabase::rest_get(
        token,
        "spells",
        &[
            ("select", "key,level"),
            ("key", &format!("in.({})", quoted.join(","))),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
        ],
    )?;
    let mut seen = std::collections::HashSet::new();
    let mut used = 0;
    for r in rows.as_array().unwrap_or(&Vec::new()) {
        let Some(key) = r.get("key").and_then(|v| v.as_str()) else {
            continue;
        };
        if !seen.insert(key.to_string()) {
            continue;
        }
        used += scribe::pages_on(r.get("level").and_then(|v| v.as_i64()).unwrap_or(0), scroll);
    }
    Ok(used)
}

/// 170's per-school tuning. A school nobody has a row for is the
/// book's own rate, which is what 100 means.
fn school_pct(token: &str, school: &str) -> Result<i64, String> {
    if school.is_empty() {
        return Ok(100);
    }
    let rows = supabase::rest_get(
        token,
        "scribe_schools",
        &[("select", "pct"), ("school", &format!("eq.{}", school))],
    )?;
    Ok(rows
        .as_array()
        .and_then(|a| a.first())
        .and_then(|r| r.get("pct"))
        .and_then(|v| v.as_i64())
        .unwrap_or(100))
}

/// 192. EVERY SCHOOL'S RATE IN ONE READ. `school_pct` asks about one
/// and is right for one scribing; pricing two hundred candidates with
/// it would be two hundred round trips.
fn school_rates(token: &str) -> Result<std::collections::HashMap<String, i64>, String> {
    let rows = supabase::rest_get(token, "scribe_schools", &[("select", "school,pct")])?;
    let mut out = std::collections::HashMap::new();
    for r in rows.as_array().unwrap_or(&Vec::new()) {
        if let (Some(s), Some(p)) = (
            r.get("school").and_then(|v| v.as_str()),
            r.get("pct").and_then(|v| v.as_i64()),
        ) {
            out.insert(s.to_string(), p);
        }
    }
    Ok(out)
}

fn entity_of(token: &str, character_id: &str) -> Result<String, String> {
    let rows = supabase::rest_get(
        token,
        "characters",
        &[
            ("select", "entity_id"),
            ("id", &format!("eq.{}", character_id)),
        ],
    )?;
    rows.as_array()
        .and_then(|a| a.first())
        .and_then(|r| r.get("entity_id"))
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| "that character holds nothing".to_string())
}

/// How many of one thing they are carrying, counting stacks.
fn count_held(token: &str, entity: &str, item_key: &str) -> Result<i64, String> {
    let rows = supabase::rest_get(
        token,
        "objects",
        &[
            ("select", "quantity"),
            ("holder_id", &format!("eq.{}", entity)),
            ("item_key", &format!("eq.{}", item_key)),
        ],
    )?;
    Ok(rows
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|r| r.get("quantity").and_then(|v| v.as_i64()).unwrap_or(1))
        .sum())
}

/// Take `want` of something off them, emptying stacks in turn.
///
/// THE CALLER HAS ALREADY COUNTED. This spends what it is told to and
/// does not second-guess it, because the refusal belongs before any of
/// the spending starts - see `scribe_spell`.
fn spend_held(token: &str, entity: &str, item_key: &str, want: i64) -> Result<(), String> {
    let rows = supabase::rest_get(
        token,
        "objects",
        &[
            ("select", "id,quantity"),
            ("holder_id", &format!("eq.{}", entity)),
            ("item_key", &format!("eq.{}", item_key)),
            ("order", "quantity.asc"),
        ],
    )?;
    let mut left = want;
    for r in rows.as_array().unwrap_or(&Vec::new()) {
        if left <= 0 {
            break;
        }
        let Some(id) = r.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        let have = r.get("quantity").and_then(|v| v.as_i64()).unwrap_or(1);
        if have <= left {
            supabase::rest_delete(token, "objects", &[("id", &format!("eq.{}", id))])?;
            left -= have;
        } else {
            supabase::rest_update(
                token,
                "objects",
                &[("id", &format!("eq.{}", id))],
                &json!({ "quantity": have - left }),
            )?;
            left = 0;
        }
    }
    Ok(())
}

fn one_spell(token: &str, game_id: &str, key: &str) -> Result<Value, String> {
    let rows = supabase::rest_get(
        token,
        "spells",
        &[
            ("select", "key,name,level,school,classes"),
            ("key", &format!("eq.{}", key)),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
            // THE GAME'S OWN ROW WINS. `game_id.desc` would not do it:
            // Postgres puts NULLS FIRST on a descending sort, so the
            // global row would come back ahead of the override. This is
            // the spelling casting.rs uses.
            ("order", "game_id.asc.nullslast"),
        ],
    )?;
    rows.as_array()
        .and_then(|a| a.first())
        .cloned()
        .ok_or_else(|| format!("no spell with key '{}'", key))
}
