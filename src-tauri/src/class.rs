//! What a player character IS — the twelve callings, from 055.
//!
//! THE POINT OF THIS FILE IS THE HIT DIE. Everything else a class
//! carries is convenience for a creation screen; the die is the fact
//! something derives from, and it is the one `vitality.rs` has been
//! waiting for since 029:
//!
//! > WHAT THIS IS NOT. A player character's hit points are not this.
//! > 5e maxes a PC's first hit die and rolls the rest, and the die
//! > comes from class rather than size.
//!
//! So a monster's die comes from its SIZE and a character's comes from
//! their CLASS, and those are two different rules rather than one rule
//! with an exception. `vitality::average_hp` is the monster one and
//! `vitality::pc_hp` is this one. Both live there, together, because
//! they are the same arithmetic over a different die.
//!
//! WHY THE CATALOGUE IS NOT AN ENUM. Twelve classes are in the book and
//! a thirteenth is in somebody's homebrew folder. 055 gives classes the
//! nullable tenancy `items` and `skills` have had since 004, so a table
//! can write its own Fighter with a d12 and see it instead of the SRD
//! one. An enum would make that a code change.
//!
//! THE PROFICIENCY VOCABULARY IS NOT THIS FILE'S OPINION. A class
//! carries `sim`/`mar` and `lgt`/`med`/`hvy`/`shl` because that is what
//! `equipment::is_proficient` reads. Writing "simple" would give a
//! Fighter proficiency with nothing and say so nowhere - see 055's
//! header, and the numeric trap in supabase.rs for the same shape of
//! mistake caught three times.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/* ============================ TYPES ============================ */

/// One class, as the catalogue holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Class {
    pub key: String,
    pub name: String,
    /// The die hit points come from. The whole reason this table exists.
    pub hit_die: i64,
    /// Advisory: what the class runs on. Nothing derives from it yet -
    /// it is here so a creation screen can say which scores matter
    /// before anybody rolls them.
    pub primary_abilities: Vec<String>,
    /// The two saves this class is proficient in. Ability codes.
    pub saving_throws: Vec<String>,
    /// `lgt`/`med`/`hvy`/`shl` - the vocabulary is_proficient checks.
    pub armor_profs: Vec<String>,
    /// `sim`/`mar`, or exact item keys for the restricted classes.
    pub weapon_profs: Vec<String>,
    pub skill_choices: i64,
    /// Three-letter keys from the skills catalogue. EMPTY MEANS ANY,
    /// which is how the Bard is written and is a real answer rather
    /// than a gap.
    pub skill_options: Vec<String>,
    pub description: Option<String>,
}

impl Class {
    /// Whether this class may pick that skill. An empty option list is
    /// the Bard: any skill at all.
    ///
    /// NOT CALLED YET, and kept rather than deleted because the rule it
    /// states is the non-obvious half of `skill_options` - that empty
    /// means ANY rather than NONE. The creation screen picks a class
    /// today and will pick skills next; this is the function that
    /// screen needs, and re-deriving it later is how the empty list
    /// gets read as "no skills" by somebody reasonable.
    #[allow(dead_code)]
    pub fn may_take(&self, skill_key: &str) -> bool {
        self.skill_options.is_empty() || self.skill_options.iter().any(|s| s == skill_key)
    }
}

/// The columns a class read asks for. One place, so a select and a
/// parser cannot drift apart.
pub const CLASS_COLUMNS: &str = "key,game_id,name,hit_die,primary_abilities,saving_throws,\
armor_profs,weapon_profs,skill_choices,skill_options,description";

/* ============================ READING ============================ */

fn strs(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(|x| x.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

fn text(v: &Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or_default().to_string()
}

/// One catalogue row.
///
/// `hit_die` falls back to 8 ONLY when the column is absent, which the
/// schema forbids - 055 makes it NOT NULL with a check constraint. The
/// fallback exists so a malformed row degrades to a d8 character rather
/// than a zero-hit-point one; it is not a default anybody should reach.
pub fn from_row(r: &Value) -> Class {
    Class {
        key: text(r, "key"),
        name: text(r, "name"),
        hit_die: r.get("hit_die").and_then(|x| x.as_i64()).unwrap_or(8),
        primary_abilities: strs(r, "primary_abilities"),
        saving_throws: strs(r, "saving_throws"),
        armor_profs: strs(r, "armor_profs"),
        weapon_profs: strs(r, "weapon_profs"),
        skill_choices: r.get("skill_choices").and_then(|x| x.as_i64()).unwrap_or(2),
        skill_options: strs(r, "skill_options"),
        description: r.get("description").and_then(|x| x.as_str()).map(str::to_string),
    }
}

/// Globals first, then a game's own rows on top of them.
///
/// THE SAME TWO PASSES `equipment::collapse_overrides` MAKES, and for
/// the same reason: one query returns both spaces, and which row wins
/// is a rule rather than an ordering accident. A table that writes its
/// own `fighter` sees only theirs.
pub fn collapse(rows: &[Value]) -> Vec<Class> {
    let mut out: Vec<Class> = Vec::new();
    for pass in [true, false] {
        for r in rows {
            let is_global = r.get("game_id").map(|g| g.is_null()).unwrap_or(true);
            if is_global != pass {
                continue;
            }
            let c = from_row(r);
            match out.iter_mut().find(|x| x.key == c.key) {
                Some(existing) => *existing = c,
                None => out.push(c),
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fighter() -> Value {
        json!({
            "key": "fighter", "game_id": null, "name": "Fighter", "hit_die": 10,
            "primary_abilities": ["str", "dex"], "saving_throws": ["str", "con"],
            "armor_profs": ["lgt", "med", "hvy", "shl"], "weapon_profs": ["sim", "mar"],
            "skill_choices": 2, "skill_options": ["acr", "ath"],
            "description": "Every weapon, every armour."
        })
    }

    // THE TRAP 4951b29 REPAIRS AND 016 RECORDED: a wrapped select with
    // a real newline in it. Rust's line continuation eats the newline
    // AND the leading whitespace, but only if the backslash is there -
    // and the failure is a blank screen, not an error.
    #[test]
    fn the_select_is_one_unbroken_line() {
        assert!(!CLASS_COLUMNS.contains(char::is_whitespace), "{}", CLASS_COLUMNS);
        assert!(CLASS_COLUMNS.starts_with("key,game_id,name,hit_die"));
        assert!(CLASS_COLUMNS.ends_with("description"));
    }

    #[test]
    fn a_class_reads_off_its_row() {
        let c = from_row(&fighter());
        assert_eq!(c.key, "fighter");
        assert_eq!(c.hit_die, 10);
        assert_eq!(c.saving_throws, vec!["str", "con"]);
        assert_eq!(c.armor_profs, vec!["lgt", "med", "hvy", "shl"]);
        assert_eq!(c.skill_choices, 2);
    }

    // The vocabulary is the whole point - a class that said "simple"
    // would be proficient with nothing and report nothing.
    #[test]
    fn the_profs_are_in_the_vocabulary_is_proficient_reads() {
        let c = from_row(&fighter());
        for p in &c.weapon_profs {
            assert!(p == "sim" || p == "mar", "not a weapon class prefix: {}", p);
        }
        for p in &c.armor_profs {
            assert!(
                ["lgt", "med", "hvy", "shl"].contains(&p.as_str()),
                "not an armour category: {}",
                p
            );
        }
    }

    #[test]
    fn a_missing_list_is_empty_rather_than_absent() {
        let c = from_row(&json!({ "key": "homebrew", "name": "Homebrew", "hit_die": 8 }));
        assert!(c.armor_profs.is_empty());
        assert!(c.skill_options.is_empty());
        assert_eq!(c.skill_choices, 2);
    }

    // An empty option list is the Bard: any skill at all. That is a
    // real answer and not a gap, so it must not read as "none".
    #[test]
    fn an_empty_option_list_admits_everything() {
        let bard = from_row(&json!({
            "key": "bard", "name": "Bard", "hit_die": 8, "skill_options": []
        }));
        assert!(bard.may_take("arc"));
        assert!(bard.may_take("ath"));

        let fighter = from_row(&fighter());
        assert!(fighter.may_take("ath"));
        assert!(!fighter.may_take("arc"));
    }

    #[test]
    fn a_games_own_class_wins_over_the_global_one() {
        let mine = json!({
            "key": "fighter", "game_id": "abc", "name": "Fighter", "hit_die": 12,
            "primary_abilities": [], "saving_throws": [], "armor_profs": [],
            "weapon_profs": [], "skill_choices": 2, "skill_options": []
        });
        let out = collapse(&[fighter(), mine]);
        assert_eq!(out.len(), 1, "one key, one class");
        assert_eq!(out[0].hit_die, 12, "the table's own d12 Fighter");
    }

    // Order of arrival must not decide it. PostgREST makes no promise
    // about which space comes back first.
    #[test]
    fn and_wins_whichever_order_the_rows_arrive_in() {
        let mine = json!({
            "key": "fighter", "game_id": "abc", "name": "Fighter", "hit_die": 12
        });
        let out = collapse(&[mine, fighter()]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].hit_die, 12);
    }

    #[test]
    fn classes_come_back_sorted_by_name() {
        let wizard = json!({ "key": "wizard", "game_id": null, "name": "Wizard", "hit_die": 6 });
        let out = collapse(&[wizard, fighter()]);
        assert_eq!(out[0].name, "Fighter");
        assert_eq!(out[1].name, "Wizard");
    }
}
