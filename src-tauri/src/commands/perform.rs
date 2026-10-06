//! Playing something, and seeing whether it landed.
//!
//! 076. The plumbing between a bard, a room and the HOPPER percentile
//! chart. Everything that decides anything is in karma.rs; this reads
//! the rows, rolls the die and reports.
//!
//! WHAT THIS DOES NOT DO YET, said plainly because the gap is the
//! point: nothing is kept. A performance is rolled, shown and
//! forgotten. Stage 2 gives a drafted song a row of its own and makes
//! it the record this roll should have been - see "a roll is a record"
//! in 001 - and Stage 3 gives the inspiration somewhere to land.
//! Building the record first would have meant guessing the shape of a
//! song before anybody had rolled for one.

use serde_json::{json, Value};
use tauri::State;

use crate::dice::{RandomRoller, Roller};
use crate::supabase::{self, AppState};

/// The rooms a performance can be given to, best first.
///
/// GLOBAL ROWS AND THIS GAME'S, collapsed the way every tenanted
/// catalogue in this schema is: a game that writes its own `hostile`
/// shadows the global one rather than colliding with it.
#[tauri::command]
pub fn list_audiences(state: State<AppState>, game_id: String) -> Result<Value, String> {
    let token = state.token()?;
    let rows = supabase::rest_get(
        &token,
        "audiences",
        &[
            ("select", "key,game_id,name,rating,sort"),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
            ("order", "sort.asc"),
        ],
    )?;

    // The game's own row wins over the global one of the same key.
    let all = rows.as_array().cloned().unwrap_or_default();
    let mut out: Vec<Value> = Vec::new();
    for r in &all {
        let Some(key) = r.get("key").and_then(|v| v.as_str()) else {
            continue;
        };
        let mine = r.get("game_id").and_then(|v| v.as_str()).is_some();
        let already = out
            .iter()
            .position(|o| o.get("key").and_then(|v| v.as_str()) == Some(key));
        match already {
            Some(i) if mine => out[i] = r.clone(),
            Some(_) => {}
            None => out.push(r.clone()),
        }
    }
    Ok(json!(out))
}

/// Play to a room and see what comes of it.
///
/// THE CHART IS THE WHOLE RESOLUTION. Karma down one axis, the
/// audience along the other, and a d100 rolled OVER the number where
/// they cross - see karma.rs, which is where every one of those words
/// is tested, and which ran the other way until the first live
/// performance proved it backwards.
///
/// AN INSTRUMENT IS REQUIRED AND ITS PROFICIENCY IS ONLY REPORTED.
/// Whether playing something you were never taught should cost you is
/// a rule Dave has not made yet, and inventing one would bake a guess
/// into the first performance anybody rolls. So `proficient` comes
/// back on the result and changes no number. The screen says so
/// loudly, the same way an unproficient weapon already does.
#[tauri::command]
pub fn perform(
    state: State<AppState>,
    character_id: String,
    audience_key: String,
    object_id: String,
) -> Result<Value, String> {
    let token = state.token()?;
    let sheet = crate::character::load_sheet(&token, &character_id)?;

    // NO KARMA IS A REFUSAL, NOT A ZERO. A fighter has no Karma
    // expression at all, and rolling them against the chart at 0 would
    // report a 36% chance of drafting a song rather than saying they
    // cannot draft one.
    let Some(karma) = &sheet.karma else {
        return Err(format!(
            "{} has no class that expresses Karma - only a bard can draft a song",
            sheet.name
        ));
    };

    // THE INSTRUMENT MUST BE ONE THEY ARE ACTUALLY HOLDING. Read off
    // the loadout rather than off the catalogue, so a lute somebody
    // sold is a lute they cannot play.
    let Some(held) = sheet.loadout.iter().find(|o| o.id == object_id) else {
        return Err("that is not something this character is carrying".to_string());
    };
    if held.item.kind != "instrument" {
        return Err(format!("{} is not an instrument", held.item.name));
    }

    let room = load_audience(&token, &sheet.game_id, &audience_key)?;
    let rating = room.get("rating").and_then(|v| v.as_i64()).unwrap_or(0);

    let target = crate::karma::target(karma.rating, rating);
    let roll = RandomRoller.roll(100);
    let made = crate::karma::made_it(roll, target);

    // 094. AND IT LANDS ON THE PARTY. This is what the chart was for:
    // a drafted song that inspires compatriots for an hour, and does
    // not stack with itself.
    //
    // ON EVERY PLAYER CHARACTER IN THE GAME, the bard included - an
    // hour is long enough that working out who was in earshot is a
    // DM's call rather than a radius this engine could know. A DM who
    // disagrees ends one.
    //
    // `highest` RATHER THAN `replace`, so a second, worse performance
    // cannot undo a good one - which is the rule effects.rs calls the
    // subtle one, and the reason a tie goes to what is already running.
    let mut inspired: Vec<String> = Vec::new();
    if made {
        let now = crate::commands::effects::read_tick(&token, &sheet.game_id)?;
        let band = inspiration_die(karma.rating);
        for who in party(&token, &sheet.game_id)? {
            let (outcome, _) = crate::commands::effects::apply_inner(
                &token,
                &sheet.game_id,
                &who.0,
                "inspired",
                &format!("Inspired by {}", sheet.name),
                Some(crate::clock::HOUR),
                Some(band),
                "highest",
                Some(&character_id),
                Some("perform"),
                None,
                // The song is tracked and read by a person, not by the
                // dice - its die is on the chip rather than in a roll.
                &serde_json::json!([]),
                now,
            )?;
            // KEPT MEANS THEY ALREADY HAD BETTER, which is not a
            // failure and not a reach - so they are not named.
            if !matches!(outcome, crate::effects::Outcome::Keep(_)) {
                inspired.push(who.1);
            }
        }
    }

    Ok(json!({
        "character": sheet.name,
        "instrument": held.item.name,
        "object_id": held.id,
        // REPORTED, NOT APPLIED - see the note on this function.
        "proficient": held.proficient,
        "audience": room.get("name"),
        "audience_rating": rating,
        "karma": karma.rating,
        "karma_raw": karma.raw,
        "karma_class": karma.class_key,
        "karma_parts": karma.parts,
        "target": target,
        "printed": crate::karma::printed(karma.rating, rating),
        "roll": roll,
        "made_it": made,
        "margin": crate::karma::margin(roll, target),
        // WHO IT REACHED, and it is not everybody when somebody is
        // already carrying a better song - see effects::admit.
        "inspired": inspired,
        "die": if made { Some(inspiration_die(karma.rating)) } else { None },
    }))
}

/// How big the inspiration die is, off the performer's Karma.
///
/// DAVE'S SCALE, NOT 5e's. Bardic Inspiration climbs with BARD LEVEL -
/// d6, d8 at 5, d10 at 10, d12 at 15 - and this is a different feature
/// on a different axis: a song drafted against the HOPPER chart, where
/// Karma is what the chart reads. Tying it to Karma keeps one number
/// deciding both how likely the song was and how much it is worth.
fn inspiration_die(karma: i64) -> i64 {
    match karma {
        k if k >= 20 => 12,
        k if k >= 14 => 10,
        k if k >= 8 => 8,
        _ => 6,
    }
}

/// Every player character in the game: (id, name).
///
/// PLAYER CHARACTERS ONLY. Inspiring the goblins would be a funny bug
/// to find in the middle of a fight.
fn party(token: &str, game_id: &str) -> Result<Vec<(String, String)>, String> {
    let rows = supabase::rest_get(
        token,
        "characters",
        &[
            ("select", "id,name"),
            ("game_id", &format!("eq.{}", game_id)),
            ("is_npc", "is.false"),
            // 123. A TEMPLATE IS NOT A CREATURE IN THE WORLD.
            ("is_template", "is.false"),
            ("is_active", "is.true"),
        ],
    )?;
    Ok(rows
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|r| {
            Some((
                r.get("id")?.as_str()?.to_string(),
                r.get("name")?.as_str()?.to_string(),
            ))
        })
        .collect())
}

/// One audience row, this game's before the global one.
fn load_audience(token: &str, game_id: &str, key: &str) -> Result<Value, String> {
    let rows = supabase::rest_get(
        token,
        "audiences",
        &[
            ("select", "key,game_id,name,rating"),
            ("key", &format!("eq.{}", key)),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
            // A game's own row sorts first - NULLs last - so the first
            // result is the one that should win.
            ("order", "game_id.asc.nullslast"),
        ],
    )?;
    rows.as_array()
        .and_then(|a| a.first())
        .cloned()
        .ok_or_else(|| format!("no such audience: {}", key))
}
