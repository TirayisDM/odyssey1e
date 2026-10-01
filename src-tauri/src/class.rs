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
    /// The levels at which this class gains ANOTHER attack in the
    /// Attack action. 061. Empty for seven of the twelve.
    pub extra_attack_levels: Vec<i64>,
    pub skill_choices: i64,
    /// 075. How many tool proficiencies this class picks freely - the
    /// bard's three instruments. NOT ENFORCED: nothing counts chosen
    /// against owed, exactly as `skill_choices` has not since 055.
    pub tool_choices: i64,
    /// Tools granted outright, with no choice. Empty for the bard.
    pub tool_grants: Vec<String>,
    /// 076. Which skills sum to this class's Karma, by `skills.key`.
    /// Bard is Insight and Performance. EMPTY IS THE COMMON CASE - the
    /// bard is the only class with a Karma expression so far, and a
    /// class without one simply has no Karma.
    pub karma_skills: Vec<String>,
    /// Three-letter keys from the skills catalogue. EMPTY MEANS ANY,
    /// which is how the Bard is written and is a real answer rather
    /// than a gap.
    pub skill_options: Vec<String>,
    pub description: Option<String>,
}

impl Class {
    /// How many attacks the Attack action gives at this level.
    ///
    /// ONE, PLUS HOW MANY OF THE THRESHOLDS YOU HAVE REACHED. A
    /// Fighter's {5,11,20} therefore reads 1, 2, 3, 4 across twenty
    /// levels, and a Barbarian's {5} reads 1 then 2.
    ///
    /// THE LIST NEED NOT BE SORTED and this does not sort it, because
    /// it counts rather than walks - a homebrew row written {11,5}
    /// gives the same answer as {5,11}. A function that quietly
    /// depended on the order would be a trap for whoever writes the
    /// class editor.
    ///
    /// FLOORED AT ONE. Everybody gets a swing; a level below 1 is a
    /// character who does not exist yet rather than one who cannot
    /// act.
    pub fn attacks_at(&self, level: i64) -> i64 {
        1 + self
            .extra_attack_levels
            .iter()
            .filter(|&&at| level >= at)
            .count() as i64
    }

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
    karma_skills,tool_choices,tool_grants,\
armor_profs,weapon_profs,extra_attack_levels,skill_choices,skill_options,description";

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
        extra_attack_levels: r
            .get("extra_attack_levels")
            .and_then(|x| x.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_i64()).collect())
            .unwrap_or_default(),
        tool_choices: r.get("tool_choices").and_then(|v| v.as_i64()).unwrap_or(0),
        tool_grants: strs(r, "tool_grants"),
        karma_skills: strs(r, "karma_skills"),
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

/* ============================ READING ============================ */

/// Which classes this character holds, in the order they took them.
///
/// 073. THE ORDER IS THE POINT and it is the database's, not a sort
/// applied here: `added_at` then `class_key`, which is exactly the
/// order `sync_character_level` breaks its own tie in. The two have to
/// agree, because the trigger decides which class `characters.class_key`
/// names and `multiclass::primary` decides which one the sheet puts
/// first, and a screen that disagreed with the row it reads would be
/// the worst kind of wrong - quietly.
///
/// TWO ROUND TRIPS, NOT ONE PER CLASS. The rows come back first and
/// their keys are resolved against the catalogue in a single `in.()`.
///
/// A KEY THE CATALOGUE CANNOT FIND KEEPS ITS ROW, with a hit die of
/// zero. Dropping it would take a class a character genuinely holds off
/// their own sheet because a DM renamed a catalogue entry - see
/// `multiclass::Taken::hit_die`, and `load_effective`, which has made
/// the same call about attacks since 061.
pub fn load_taken(
    token: &str,
    game_id: &str,
    character_id: &str,
) -> Result<Vec<crate::multiclass::Taken>, String> {
    let rows = crate::supabase::rest_get(
        token,
        "character_classes",
        &[
            ("select", "class_key,level"),
            ("character_id", &format!("eq.{}", character_id)),
            ("order", "added_at.asc,class_key.asc"),
        ],
    )?;
    let rows = rows.as_array().cloned().unwrap_or_default();
    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let keys: Vec<String> = rows
        .iter()
        .filter_map(|r| r.get("class_key").and_then(|v| v.as_str()))
        .map(str::to_string)
        .collect();
    let catalogue = load_map(token, game_id, &keys)?;

    Ok(rows
        .iter()
        .filter_map(|r| {
            let key = r.get("class_key").and_then(|v| v.as_str())?;
            Some(crate::multiclass::Taken {
                hit_die: catalogue
                    .iter()
                    .find(|c| c.key == key)
                    .map(|c| c.hit_die)
                    .unwrap_or(0),
                key: key.to_string(),
                level: r.get("level").and_then(|v| v.as_i64()).unwrap_or(1),
            })
        })
        .collect())
}

/// The catalogue rows for a set of keys, global and game-scoped
/// collapsed as `collapse` does it.
///
/// SPLIT OUT BECAUSE FOUR PLACES WANT IT - the sheet, the effective
/// loader, creation and the hit-point rederivation - and the `or=`
/// tenancy filter is the kind of thing that is right in three copies
/// and wrong in the fourth.
pub fn load_map(token: &str, game_id: &str, keys: &[String]) -> Result<Vec<Class>, String> {
    let unique: Vec<String> = keys
        .iter()
        .cloned()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    if unique.is_empty() {
        return Ok(Vec::new());
    }
    let rows = crate::supabase::rest_get(
        token,
        "classes",
        &[
            ("select", CLASS_COLUMNS),
            ("key", &format!("in.({})", unique.join(","))),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
        ],
    )?;
    Ok(collapse(rows.as_array().unwrap_or(&Vec::new())))
}

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

    /* ---------------------- how many swings ---------------------- */

    fn with_levels(levels: Value) -> Class {
        from_row(&json!({
            "key": "x", "name": "X", "hit_die": 10,
            "extra_attack_levels": levels
        }))
    }

    // The Fighter is the only one in the book with three thresholds,
    // which makes it the one worth checking every step of.
    #[test]
    fn a_fighter_climbs_one_two_three_four() {
        let f = with_levels(json!([5, 11, 20]));
        assert_eq!(f.attacks_at(1), 1);
        assert_eq!(f.attacks_at(4), 1, "the level before");
        assert_eq!(f.attacks_at(5), 2, "and the level itself");
        assert_eq!(f.attacks_at(10), 2);
        assert_eq!(f.attacks_at(11), 3);
        assert_eq!(f.attacks_at(19), 3);
        assert_eq!(f.attacks_at(20), 4);
    }

    // Garn. The character this whole migration is about.
    #[test]
    fn a_level_five_barbarian_gets_two() {
        let b = with_levels(json!([5]));
        assert_eq!(b.attacks_at(4), 1);
        assert_eq!(b.attacks_at(5), 2);
        assert_eq!(b.attacks_at(20), 2, "and never a third");
    }

    #[test]
    fn seven_of_the_twelve_never_gain_one() {
        let w = with_levels(json!([]));
        for level in [1, 5, 11, 20] {
            assert_eq!(w.attacks_at(level), 1, "at level {}", level);
        }
    }

    // It counts rather than walks, so a homebrew row written out of
    // order gives the same answer.
    #[test]
    fn the_thresholds_need_not_be_sorted() {
        let a = with_levels(json!([5, 11, 20]));
        let b = with_levels(json!([20, 5, 11]));
        for level in 1..=20 {
            assert_eq!(a.attacks_at(level), b.attacks_at(level), "at {}", level);
        }
    }

    #[test]
    fn everybody_gets_at_least_one_swing() {
        let f = with_levels(json!([5, 11, 20]));
        assert_eq!(f.attacks_at(0), 1);
        assert_eq!(f.attacks_at(-3), 1);
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
