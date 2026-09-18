//! Narrative lines: the canned prose a roll card carries.
//!
//! This is the port of the NarrativePacks half of characternarrative.js.
//! It is split the same way character.rs is, and for the same reason:
//!
//!   load_lines()    talks to the network. Untestable without one.
//!   resolve_lines() is pure. Given rows it is precedence and grouping.
//!   pick()          is pure. Given lines and a Roller it is a choice.
//!
//! WHY THE LINES ARE CACHED ON THE SHEET
//!   A pack is small and it does not change while someone is playing.
//!   Reading it once with the sheet means choosing a line costs nothing
//!   at roll time — no second round trip, no patch, no window where the
//!   card is on screen without its prose. The line goes into the insert
//!   body with the dice, so the row is complete the first time anyone
//!   sees it. rolls.narrative stays patchable for the AI-generated path,
//!   which really is slow and really does have to come later.
//!
//! WHY IT USES THE DICE ROLLER
//!   Picking a line is a die roll — 1d10 over ten lines. Going through
//!   the same Roller the dice engine uses means the choice is injectable
//!   and the tests assert exact output instead of "something non-empty".

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::dice::Roller;
use crate::supabase;

/// The pack every other pack falls back to, line by line. Named in the
/// COMMENT on characters.narrative_pack and in migration 006.
pub const BASE_PACK: &str = "base";

/// One row of narrative_lines, flattened. `game_scoped` replaces the
/// nullable game_id: by the time a row is here the only thing that
/// matters about game_id is whether it was set, because the query
/// already restricted it to this game or null.
#[derive(Debug, Clone)]
pub struct LineRow {
    pub pack: String,
    pub key: String,
    pub seq: i64,
    pub game_scoped: bool,
    pub line: String,
}

/* ============================ RESOLUTION ============================ */

/// Collapse raw rows into key -> lines, ready to choose from.
///
/// Two precedences apply, and they are not the same one:
///
///   Within a (pack, key), a game-scoped row SHADOWS the global row
///   with the same seq. That is what the partial unique indexes in 006
///   are shaped for — a DM rewrites line 4 of a pack without forking
///   the other nine.
///
///   Across packs, the character's pack wins WHOLE. If it has any line
///   for a key, base is not consulted for that key at all; mixing a
///   character-voiced line with a neutral one inside the same key would
///   read as two different narrators.
///
/// A key with no lines anywhere is simply absent — pick() returns None
/// and the card ships without prose, which is what the old system did
/// for an unwritten key.
pub fn resolve_lines(rows: &[LineRow], pack: &str) -> HashMap<String, Vec<String>> {
    // (pack, key) -> seq -> (line, was_game_scoped). BTreeMap so the
    // lines come out in seq order, which is the order they were written
    // in and the order a DM editing them expects to see.
    let mut grouped: HashMap<(&str, &str), BTreeMap<i64, (&str, bool)>> = HashMap::new();

    for r in rows {
        let slot = grouped
            .entry((r.pack.as_str(), r.key.as_str()))
            .or_default();
        // A game-scoped row beats a global one regardless of which
        // arrived first — row order out of PostgREST is not a contract.
        let takes_the_slot = match slot.get(&r.seq) {
            Some((_, existing_is_scoped)) => r.game_scoped && !existing_is_scoped,
            None => true,
        };
        if takes_the_slot {
            slot.insert(r.seq, (r.line.as_str(), r.game_scoped));
        }
    }

    let keys: HashSet<&str> = rows.iter().map(|r| r.key.as_str()).collect();
    let mut out = HashMap::new();

    for key in keys {
        let chosen = grouped
            .get(&(pack, key))
            .filter(|m| !m.is_empty())
            .or_else(|| grouped.get(&(BASE_PACK, key)));

        if let Some(m) = chosen {
            let lines: Vec<String> = m.values().map(|(l, _)| (*l).to_string()).collect();
            if !lines.is_empty() {
                out.insert(key.to_string(), lines);
            }
        }
    }

    out
}

/// One line for this key, at random, or None if the key has none.
///
/// The index comes off the Roller as a 1..=n die, so a ten-line key is
/// literally a d10. Nothing here can panic: an empty key leaves early,
/// and the roll is clamped in case a Roller hands back something outside
/// its own range.
pub fn pick<R: Roller>(
    lines: &HashMap<String, Vec<String>>,
    key: &str,
    rng: &mut R,
) -> Option<String> {
    let choices = lines.get(key)?;
    if choices.is_empty() {
        return None;
    }
    let face = rng.roll(choices.len() as u32);
    let idx = (face - 1).clamp(0, choices.len() as i64 - 1) as usize;
    Some(choices[idx].clone())
}

/* ============================ LOADING ============================ */

/// Quote a value for a PostgREST `in.(...)` list.
///
/// Pack names are free text — the pack shipped with 006 is called
/// 'Rodnar Shieldcrest', with a space in it — so nothing may be
/// interpolated raw. A quoted value escapes a literal quote or
/// backslash with a backslash, which is what PostgREST's parser expects.
///
/// Shared, not private: STATUS.md names this as the helper to use for
/// every reference table filtered by a name, and equipment.rs filters
/// items by key with it. An escaping rule copied into a second module
/// is an escaping rule that will diverge.
pub fn quoted(v: &str) -> String {
    let mut s = String::with_capacity(v.len() + 2);
    s.push('"');
    for c in v.chars() {
        if c == '"' || c == '\\' {
            s.push('\\');
        }
        s.push(c);
    }
    s.push('"');
    s
}

/// The character's pack and base, global rows plus this game's
/// overrides, enabled only — everything resolve_lines needs, in one
/// request.
pub fn load_lines(token: &str, game_id: &str, pack: &str) -> Result<Vec<LineRow>, String> {
    let packs = if pack == BASE_PACK {
        quoted(BASE_PACK)
    } else {
        format!("{},{}", quoted(pack), quoted(BASE_PACK))
    };

    let rows = supabase::rest_get(
        token,
        "narrative_lines",
        &[
            ("select", "pack,key,seq,game_id,line"),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
            ("enabled", "is.true"),
            ("pack", &format!("in.({})", packs)),
            ("order", "key.asc,seq.asc"),
        ],
    )?;

    let mut out = Vec::new();
    if let Some(rows) = rows.as_array() {
        for r in rows {
            let line = r.get("line").and_then(|x| x.as_str()).unwrap_or("");
            if line.is_empty() {
                continue;
            }
            out.push(LineRow {
                pack: r.get("pack").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                key: r.get("key").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                seq: r.get("seq").and_then(|x| x.as_i64()).unwrap_or(0),
                game_scoped: !r.get("game_id").map(|g| g.is_null()).unwrap_or(true),
                line: line.to_string(),
            });
        }
    }
    Ok(out)
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dice::SequenceRoller;

    fn row(pack: &str, key: &str, seq: i64, game_scoped: bool, line: &str) -> LineRow {
        LineRow {
            pack: pack.into(),
            key: key.into(),
            seq,
            game_scoped,
            line: line.into(),
        }
    }

    /// Two packs: base covers 'ins' and 'ath', the voiced pack only 'ins'.
    fn rows() -> Vec<LineRow> {
        vec![
            row("base", "ins", 1, false, "base ins 1"),
            row("base", "ins", 2, false, "base ins 2"),
            row("base", "ath", 1, false, "base ath 1"),
            row("Rodnar Shieldcrest", "ins", 1, false, "rodnar ins 1"),
            row("Rodnar Shieldcrest", "ins", 2, false, "rodnar ins 2"),
        ]
    }

    #[test]
    fn the_characters_pack_wins_where_it_has_lines() {
        let r = resolve_lines(&rows(), "Rodnar Shieldcrest");
        assert_eq!(r["ins"], vec!["rodnar ins 1", "rodnar ins 2"]);
    }

    #[test]
    fn a_key_the_pack_does_not_cover_falls_back_to_base() {
        let r = resolve_lines(&rows(), "Rodnar Shieldcrest");
        assert_eq!(r["ath"], vec!["base ath 1"]);
    }

    #[test]
    fn falling_back_takes_the_whole_key_not_the_missing_lines() {
        // The voiced pack keeps only line 1 of 'ins'. Base's line 2 must
        // NOT be spliced in beside it — one narrator per key.
        let mut rs = rows();
        rs.retain(|r| !(r.pack == "Rodnar Shieldcrest" && r.seq == 2));
        let r = resolve_lines(&rs, "Rodnar Shieldcrest");
        assert_eq!(r["ins"], vec!["rodnar ins 1"]);
    }

    #[test]
    fn an_unknown_pack_reads_as_base() {
        let r = resolve_lines(&rows(), "no such pack");
        assert_eq!(r["ins"], vec!["base ins 1", "base ins 2"]);
    }

    #[test]
    fn a_game_override_shadows_the_global_line_with_the_same_seq() {
        let mut rs = rows();
        rs.push(row("base", "ins", 2, true, "this table only"));
        let r = resolve_lines(&rs, "base");
        assert_eq!(r["ins"], vec!["base ins 1", "this table only"]);
    }

    #[test]
    fn the_override_wins_whichever_order_the_rows_arrive_in() {
        let rs = vec![
            row("base", "ins", 1, true, "override"),
            row("base", "ins", 1, false, "global"),
        ];
        assert_eq!(resolve_lines(&rs, "base")["ins"], vec!["override"]);

        let rs = vec![
            row("base", "ins", 1, false, "global"),
            row("base", "ins", 1, true, "override"),
        ];
        assert_eq!(resolve_lines(&rs, "base")["ins"], vec!["override"]);
    }

    #[test]
    fn lines_come_out_in_seq_order_not_row_order() {
        let rs = vec![
            row("base", "ins", 3, false, "third"),
            row("base", "ins", 1, false, "first"),
            row("base", "ins", 2, false, "second"),
        ];
        assert_eq!(
            resolve_lines(&rs, "base")["ins"],
            vec!["first", "second", "third"]
        );
    }

    #[test]
    fn a_key_with_no_lines_anywhere_is_absent() {
        let r = resolve_lines(&rows(), "base");
        assert!(r.get("prc").is_none());
    }

    #[test]
    fn pick_uses_the_die_face_as_a_one_based_index() {
        let lines = resolve_lines(&rows(), "base");
        let mut rng = SequenceRoller::new(&[2]);
        assert_eq!(pick(&lines, "ins", &mut rng).as_deref(), Some("base ins 2"));
        assert!(rng.exhausted());
    }

    #[test]
    fn pick_returns_none_for_a_key_with_no_lines() {
        let lines = resolve_lines(&rows(), "base");
        let mut rng = SequenceRoller::new(&[1]);
        assert!(pick(&lines, "prc", &mut rng).is_none());
    }

    #[test]
    fn pick_survives_a_face_outside_the_range() {
        // Defensive: a Roller is injected, so nothing here may assume it
        // honours its own bounds.
        let lines = resolve_lines(&rows(), "base");
        let mut rng = SequenceRoller::new(&[99]);
        assert_eq!(pick(&lines, "ins", &mut rng).as_deref(), Some("base ins 2"));
    }

    #[test]
    fn pack_names_are_quoted_for_the_in_list() {
        assert_eq!(quoted("base"), "\"base\"");
        assert_eq!(quoted("Rodnar Shieldcrest"), "\"Rodnar Shieldcrest\"");
        // The two characters that would otherwise end the value early.
        assert_eq!(quoted("say \"hi\""), "\"say \\\"hi\\\"\"");
        assert_eq!(quoted("back\\slash"), "\"back\\\\slash\"");
    }
}
