//! What a character's PEOPLE are — 056.
//!
//! 055 gave a character a class, and with it a hit die. This gives them
//! the other half of what a sheet is made of: the scores they started
//! from, how big they are, and what their body does that nobody else's
//! does.
//!
//! THIS CAMPAIGN IS ALL CUSTOM SPECIES. There is no SRD layer under
//! this catalogue for a game to override - the Unt'garoth are the
//! first of a roster that is entirely Dave's. The tenancy is here
//! anyway because every other catalogue has it and a second campaign
//! will want its own.
//!
//! BASE AND EFFECTIVE ARE DIFFERENT FACTS, and keeping them apart is
//! the whole design. A species bonus is never written into
//! `character_abilities.score`; the stored number stays what somebody
//! rolled or bought, and `effective_score` adds the bonus on the way to
//! the sheet. Writing +2 into the row would destroy the base, so
//! changing species later would double-count, and a Strength of 18
//! would be indistinguishable from a 16 with a species behind it. Same
//! argument as 049's overrides and 001's snapshots: a stored value
//! records a decision, a derived one is a consequence.
//!
//! WHAT IS APPLIED AND WHAT IS ONLY WRITTEN DOWN. Half of what a
//! species sheet says has somewhere real to land - bonuses, maxima,
//! size, carrying, skills, unarmoured AC. The other half needs systems
//! that do not exist: there is no movement system for a speed, no
//! resistance system for "half damage from fire", and the roll
//! screen's Adv/Dis is a human choice with nothing to fire it. Those
//! are carried with `applied: false` and SHOWN AS SUCH, because a DM
//! adjudicating a trait is fine and a screen that hides which ones
//! need adjudicating is not. 054 refused to guess at 193 action costs
//! for the same reason.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/* ============================ TYPES ============================ */

/// One named thing a people can do.
///
/// `applied` is the honest column: false means the engine does not act
/// on this and a DM must.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trait {
    pub name: String,
    pub text: String,
    pub applied: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Species {
    pub key: String,
    pub name: String,

    // --- applied ---
    /// Ability code -> bonus. Added by `effective_score`, never stored.
    pub ability_bonuses: Vec<(String, i64)>,
    /// Ability code -> natural ceiling. Absent means `DEFAULT_MAXIMUM`.
    pub ability_maxima: Vec<(String, i64)>,
    pub size: String,
    /// How many sizes UP this people CARRIES. Powerful Build is 1.
    pub carry_size_steps: i64,
    pub skill_profs: Vec<String>,
    pub unarmored_ac_base: Option<i64>,
    pub unarmored_ac_ability: Option<String>,

    // --- written down only ---
    pub speed: Option<i64>,
    pub damage_resistances: Vec<String>,
    pub languages: Vec<String>,

    // --- prose ---
    pub summary: Option<String>,
    pub appearance: Option<String>,
    pub culture: Option<String>,
    pub history: Option<String>,
    pub roleplaying: Option<String>,
    pub age_note: Option<String>,
    pub alignment_note: Option<String>,
    pub traits: Vec<Trait>,
}

/// 5e's ceiling for anybody without a reason to exceed it.
pub const DEFAULT_MAXIMUM: i64 = 20;

impl Species {
    pub fn bonus_for(&self, ability: &str) -> i64 {
        self.ability_bonuses
            .iter()
            .find(|(k, _)| k == ability)
            .map(|(_, v)| *v)
            .unwrap_or(0)
    }

    pub fn maximum_for(&self, ability: &str) -> i64 {
        self.ability_maxima
            .iter()
            .find(|(k, _)| k == ability)
            .map(|(_, v)| *v)
            .unwrap_or(DEFAULT_MAXIMUM)
    }

    /// The size this people CARRIES as, which is not the size they are.
    ///
    /// Powerful Build moves carrying and nothing else - not reach, not
    /// cover, not the hit die, not what a container will admit. Those
    /// all keep reading `size`, which is why this is a separate
    /// function rather than a second size stored on the character.
    pub fn carry_size(&self) -> String {
        bump_size(&self.size, self.carry_size_steps)
    }
}

/* ============================ RULES ============================ */

/// A score with its species bonus, held to the ceiling.
///
/// THE CAP IS PART OF THE RULE, not decoration. An Unt'garoth reaches
/// Strength 21 naturally and everybody else stops at 20, so applying
/// the bonus without the ceiling would let a 20 become a 22 and quietly
/// hand out a modifier nobody is entitled to.
///
/// A BASE ALREADY OVER THE CEILING IS LEFT ALONE. A DM who types 24
/// meant 24 - probably a giant, possibly a test - and silently pulling
/// it down to 20 would be this app overruling the person using it. The
/// cap binds what the SPECIES adds, which is the thing the species has
/// an opinion about.
pub fn effective_score(base: i64, bonus: i64, maximum: i64) -> i64 {
    if base >= maximum {
        return base;
    }
    (base + bonus).min(maximum)
}

/// One step up the size ladder, per step.
///
/// USES `vitality::size_rank` RATHER THAN A SECOND LADDER, which is the
/// argument that file makes about itself: one vocabulary in one place,
/// or two that eventually disagree about whether "grg" exists.
/// Gargantuan is the top and stays there rather than wrapping.
pub fn bump_size(size: &str, steps: i64) -> String {
    const LADDER: [&str; 6] = ["tiny", "sm", "med", "lg", "huge", "grg"];
    match crate::vitality::size_rank(size) {
        Some(rank) => {
            let up = (rank + steps.max(0)).min(LADDER.len() as i64 - 1);
            LADDER[up as usize].to_string()
        }
        // An unrecognised size is returned untouched rather than
        // defaulted, the same call carry.rs makes: this feeds a display
        // number, and a typo should not silently become "med".
        None => size.to_string(),
    }
}

/* ============================ READING ============================ */

pub const SPECIES_COLUMNS: &str = "key,game_id,name,ability_bonuses,ability_maxima,size,\
carry_size_steps,skill_profs,unarmored_ac_base,unarmored_ac_ability,speed,\
damage_resistances,languages,summary,appearance,culture,history,roleplaying,\
age_note,alignment_note,traits";

fn strs(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(|x| x.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

fn opt_text(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(|x| x.as_str()).map(str::to_string)
}

/// A jsonb object of ability -> integer, as pairs.
///
/// SORTED, so a screen that prints "+2 STR, +1 CON" prints it the same
/// way twice. A JSON object has no order and Postgres does not promise
/// one back.
fn ability_map(v: &Value, key: &str) -> Vec<(String, i64)> {
    let mut out: Vec<(String, i64)> = v
        .get(key)
        .and_then(|x| x.as_object())
        .map(|o| {
            o.iter()
                .filter_map(|(k, n)| n.as_i64().map(|n| (k.clone(), n)))
                .collect()
        })
        .unwrap_or_default();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn traits_of(v: &Value) -> Vec<Trait> {
    v.get("traits")
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|t| {
                    Some(Trait {
                        name: t.get("name")?.as_str()?.to_string(),
                        text: t.get("text").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                        // ABSENT MEANS NOT APPLIED. The safe default is
                        // the one that tells a DM to check, rather than
                        // the one that claims the engine has it covered.
                        applied: t.get("applied").and_then(|x| x.as_bool()).unwrap_or(false),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn from_row(r: &Value) -> Species {
    Species {
        key: r.get("key").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        name: r.get("name").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        ability_bonuses: ability_map(r, "ability_bonuses"),
        ability_maxima: ability_map(r, "ability_maxima"),
        size: r.get("size").and_then(|x| x.as_str()).unwrap_or("med").to_string(),
        carry_size_steps: r.get("carry_size_steps").and_then(|x| x.as_i64()).unwrap_or(0),
        skill_profs: strs(r, "skill_profs"),
        unarmored_ac_base: r.get("unarmored_ac_base").and_then(|x| x.as_i64()),
        unarmored_ac_ability: opt_text(r, "unarmored_ac_ability"),
        speed: r.get("speed").and_then(|x| x.as_i64()),
        damage_resistances: strs(r, "damage_resistances"),
        languages: strs(r, "languages"),
        summary: opt_text(r, "summary"),
        appearance: opt_text(r, "appearance"),
        culture: opt_text(r, "culture"),
        history: opt_text(r, "history"),
        roleplaying: opt_text(r, "roleplaying"),
        age_note: opt_text(r, "age_note"),
        alignment_note: opt_text(r, "alignment_note"),
        traits: traits_of(r),
    }
}

/// Globals first, then a game's own rows over them — the two passes
/// `class::collapse` and `equipment::collapse_overrides` both make.
pub fn collapse(rows: &[Value]) -> Vec<Species> {
    let mut out: Vec<Species> = Vec::new();
    for pass in [true, false] {
        for r in rows {
            let is_global = r.get("game_id").map(|g| g.is_null()).unwrap_or(true);
            if is_global != pass {
                continue;
            }
            let s = from_row(r);
            match out.iter_mut().find(|x| x.key == s.key) {
                Some(existing) => *existing = s,
                None => out.push(s),
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

    fn untgaroth() -> Value {
        json!({
            "key": "untgaroth", "game_id": null, "name": "Unt'garoth",
            "ability_bonuses": { "str": 2, "con": 1 },
            "ability_maxima": { "str": 21 },
            "size": "lg", "carry_size_steps": 1, "skill_profs": ["ath"],
            "unarmored_ac_base": 12, "unarmored_ac_ability": "con",
            "speed": 40, "damage_resistances": ["cold", "fire"],
            "languages": ["Common", "Unt'garoth Dialect"],
            "summary": "Towering.", "traits": [
                { "name": "Powerful Build", "text": "One size larger for carrying.", "applied": true },
                { "name": "Elemental Resilience", "text": "Fire and cold.", "applied": false }
            ]
        })
    }

    #[test]
    fn a_species_reads_off_its_row() {
        let s = from_row(&untgaroth());
        assert_eq!(s.name, "Unt'garoth");
        assert_eq!(s.size, "lg");
        assert_eq!(s.speed, Some(40));
        assert_eq!(s.bonus_for("str"), 2);
        assert_eq!(s.bonus_for("con"), 1);
        assert_eq!(s.bonus_for("dex"), 0, "no bonus is zero, not absent");
        assert_eq!(s.skill_profs, vec!["ath"]);
        assert_eq!(s.unarmored_ac_base, Some(12));
    }

    // A JSON object has no order and Postgres promises none back, so a
    // screen printing the bonuses would print them differently run to
    // run without this.
    #[test]
    fn bonuses_come_back_in_a_stable_order() {
        let s = from_row(&untgaroth());
        assert_eq!(s.ability_bonuses, vec![("con".into(), 1), ("str".into(), 2)]);
    }

    #[test]
    fn a_maximum_defaults_to_twenty() {
        let s = from_row(&untgaroth());
        assert_eq!(s.maximum_for("str"), 21, "their whole point");
        assert_eq!(s.maximum_for("con"), 20, "everything else is ordinary");
    }

    /* ---------------------- effective_score ---------------------- */

    #[test]
    fn a_bonus_is_added_to_the_base() {
        assert_eq!(effective_score(18, 2, 21), 20);
        assert_eq!(effective_score(13, 1, 20), 14);
    }

    #[test]
    fn and_held_to_the_ceiling() {
        // 19 + 2 would be 21, and for most peoples 20 is the wall.
        assert_eq!(effective_score(19, 2, 20), 20);
        // An Unt'garoth's Strength is the exception that pays for the
        // whole ability_maxima column.
        assert_eq!(effective_score(19, 2, 21), 21);
    }

    // A DM who types 24 meant 24. The cap binds what the SPECIES adds.
    #[test]
    fn a_base_already_past_the_ceiling_is_left_alone() {
        assert_eq!(effective_score(24, 2, 20), 24);
        assert_eq!(effective_score(20, 2, 20), 20);
    }

    #[test]
    fn no_bonus_changes_nothing() {
        assert_eq!(effective_score(11, 0, 20), 11);
    }

    /* ------------------------- carry size ------------------------- */

    // Powerful Build on a Large creature: carries as Huge, which is
    // x4 rather than x2 in carry.rs.
    #[test]
    fn powerful_build_moves_one_step_up_the_ladder() {
        let s = from_row(&untgaroth());
        assert_eq!(s.size, "lg", "they ARE large");
        assert_eq!(s.carry_size(), "huge", "they CARRY as huge");
    }

    #[test]
    fn without_the_trait_carrying_size_is_just_size() {
        assert_eq!(bump_size("med", 0), "med");
        assert_eq!(bump_size("sm", 0), "sm");
    }

    #[test]
    fn gargantuan_is_the_top_and_does_not_wrap() {
        assert_eq!(bump_size("grg", 1), "grg");
        assert_eq!(bump_size("huge", 3), "grg");
    }

    #[test]
    fn a_size_nobody_recognises_comes_back_untouched() {
        assert_eq!(bump_size("enormous", 1), "enormous");
    }

    /* --------------------------- traits --------------------------- */

    #[test]
    fn traits_keep_their_order_and_their_honesty() {
        let s = from_row(&untgaroth());
        assert_eq!(s.traits.len(), 2);
        assert_eq!(s.traits[0].name, "Powerful Build");
        assert!(s.traits[0].applied);
        assert!(!s.traits[1].applied, "fire and cold is a DM's job today");
    }

    // The safe default is the one that tells a DM to check.
    #[test]
    fn a_trait_that_does_not_say_is_not_applied() {
        let s = from_row(&json!({
            "key": "x", "name": "X",
            "traits": [{ "name": "Vague", "text": "Something." }]
        }));
        assert!(!s.traits[0].applied);
    }

    #[test]
    fn a_games_own_species_wins_over_the_global_one() {
        let mine = json!({
            "key": "untgaroth", "game_id": "abc", "name": "Unt'garoth",
            "ability_bonuses": { "str": 3 }, "size": "huge"
        });
        let out = collapse(&[untgaroth(), mine]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].bonus_for("str"), 3);
        assert_eq!(out[0].size, "huge");
    }

    #[test]
    fn the_select_is_one_unbroken_line() {
        assert!(!SPECIES_COLUMNS.contains(char::is_whitespace), "{}", SPECIES_COLUMNS);
        assert!(SPECIES_COLUMNS.starts_with("key,game_id,name"));
        assert!(SPECIES_COLUMNS.ends_with("traits"));
    }
}
