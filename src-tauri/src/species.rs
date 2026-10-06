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

/// One named thing a people can do, or cannot.
///
/// TWO HONEST COLUMNS, and they answer different questions. `applied`
/// says whether the ENGINE acts on it or a DM must. `kind` says whether
/// it is an upside or a cost. A trait can be any combination: Dense
/// Mass is a drawback nothing enforces, Powerful Build is a feature the
/// engine applies, and both belong on the same list looking different.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trait {
    pub name: String,
    pub text: String,
    pub applied: bool,
    /// "feature" or "drawback". PRESENTATIONAL AND DELIBERATELY SO -
    /// nothing computes differently, and pretending otherwise would be
    /// inventing a mechanic. What it buys is a screen that does not
    /// show "cannot swim" in the same colour as "resistance to fire".
    pub kind: String,
}

impl Trait {
    /// NOT CALLED FROM RUST - the screen splits the list itself, since
    /// it is the thing that has to show the two apart. Kept because the
    /// vocabulary is the non-obvious part: "drawback" rather than a
    /// bool, so a third kind can arrive without a migration rewriting
    /// every row.
    #[allow(dead_code)]
    pub fn is_drawback(&self) -> bool {
        self.kind == "drawback"
    }
}

/// A language, and the two separate facts about it.
///
/// SPOKEN AND WRITTEN ARE NOT ONE FACT. Most tongues are both and 5e
/// writes them as one phrase, which is why this was a list of names
/// until 058. The interesting cases are the others: a tongue with no
/// script, a dead language read and never pronounced, a character who
/// speaks four and reads none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tongue {
    pub name: String,
    pub spoken: bool,
    pub written: bool,
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
    /// A flat bonus on a saving throw AGAINST A SPELL - 098, the
    /// Ny'ook's +2. None for a people without the trait, which is
    /// almost all of them; Some(0) would be a people that grants
    /// nothing, and that is a different claim.
    pub spell_save_bonus: Option<i64>,

    /// How tall, in feet. THE FACT the category is a consequence of -
    /// see size.rs. Stored because a campaign running from a 2-foot
    /// rodent people to a 25-foot Imiear loses too much to six rungs.
    pub height_min_ft: Option<f64>,
    pub height_max_ft: Option<f64>,
    /// Whether this people can be chosen at creation. The Imiear are
    /// semi-intelligent and are not PCs, which is a fact about them
    /// rather than a permission check.
    pub playable: bool,
    /// DERIVED AND SENT OUT: the rung the midpoint of the height band
    /// falls on, what this people reaches, and how much floor they
    /// stand on. Computed once here so no screen has to hold its own
    /// copy of the ladder.
    pub derived: Option<String>,
    pub reach_ft: Option<i64>,
    pub space_ft: Option<f64>,

    // --- written down only ---
    /// 122. 5e's fourteen-way classification. NORMALLY WHAT ANSWERS
    /// FOR EVERY CHARACTER OF THIS PEOPLE, since every Ny'ook is the
    /// same thing - a character states its own only to contradict
    /// this. None means nobody has said yet, which is not humanoid.
    pub creature_type: Option<String>,
    pub speed: Option<i64>,
    pub damage_resistances: Vec<String>,
    /// 058. Replaces a bare list of names, which could not say that a
    /// tongue has no script.
    pub tongues: Vec<Tongue>,

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

    /// The rung their stated height lands on, or None when no height
    /// is recorded. SERIALISED rather than computed on the screen -
    /// main.js held its own copy of the ladder for one commit and that
    /// is how the four copies size.rs replaced got there.
    ///
    /// SEPARATE FROM `size`, WHICH IS STATED, so the two can be
    /// compared. A document that says Large and a height that says
    /// Medium is a contradiction worth surfacing rather than one of
    /// them silently winning - `size_matches_height` is that check.
    ///
    /// OFF THE MIDPOINT, NOT THE MINIMUM, and the Unt'garoth are why.
    /// They run 7 to 10 feet and their document calls them Large; 5e's
    /// Large starts at 8, so their SHORTEST adult is Medium and their
    /// tallest is Large. A species is typed by its typical adult rather
    /// than by whoever in it is smallest, so the band's middle decides
    /// - 8.5 feet, which is Large, which is what the document says.
    /// Categorising off the minimum would report a contradiction for
    /// every people whose range crosses a line, which is most of them.
    fn derive_size(min_ft: Option<f64>, max_ft: Option<f64>) -> Option<&'static str> {
        let lo = min_ft?;
        let mid = match max_ft {
            Some(hi) if hi >= lo => (lo + hi) / 2.0,
            _ => lo,
        };
        Some(crate::size::for_height(mid).key)
    }

    /// Whether the stated category agrees with the stated height.
    ///
    /// True when there is no height to check against: an unstated fact
    /// cannot contradict anything.
    ///
    /// NOT CALLED FROM RUST - the screen compares `derived` against
    /// `size` itself, because it is the thing that has to SAY so. Kept
    /// because the rule it states is not obvious: a disagreement is
    /// reported rather than resolved, and neither value silently wins.
    /// A species editor will want exactly this when one lands.
    #[allow(dead_code)]
    pub fn size_matches_height(&self) -> bool {
        match self.derived {
            Some(ref d) => d == &self.size,
            None => true,
        }
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

/// Whether a NATURAL score may be written at all.
///
/// `effective_score` deliberately lets a base above the ceiling
/// through, because it is answering "what does the species ADD" and a
/// DM who typed 24 meant 24. That left the Ny'ook's cap half built:
/// no bonus and no Ability Score Improvement could push Strength past
/// 13, and nothing stopped a player assigning a rolled 16 to it.
///
/// THIS IS THE OTHER HALF, and it belongs at the point of WRITING
/// rather than inside the arithmetic. A cap on what somebody may have
/// is a refusal, and a refusal has to be able to say no out loud -
/// quietly lowering 16 to 13 would be the expensive kind of helpful
/// and the player would never learn why their rolls did not land.
///
/// ONLY A STATED MAXIMUM REFUSES. No species, or a species with no
/// opinion about this ability, means no limit here - and that is not
/// laziness, it is the Tarrasque. A statblock is a character too and
/// monsters run to Strength 30; imposing 5e's default 20 on everybody
/// would make the dragon unwritable to enforce a rule about the
/// Ny'ook.
///
/// NATURAL IS THE WORD THAT MATTERS. A spell or a magical item may
/// carry somebody over their cap and that is the rule as Dave states
/// it. Nothing does that yet - 094's effects do not reach ability
/// scores - and when something does it must add on TOP of the stored
/// score at read time rather than route through `apply_bumps`, which
/// clamps to the ceiling because training is exactly what the ceiling
/// is about.
pub fn within_natural_cap(
    species: Option<&Species>,
    ability: &str,
    score: i64,
) -> Result<(), String> {
    let Some(sp) = species else {
        return Ok(());
    };
    // `maximum_for` falls back to 20 for anything unstated, which is
    // the right answer for arithmetic and the wrong one for a refusal
    // - so this asks the map directly.
    let Some((_, cap)) = sp.ability_maxima.iter().find(|(k, _)| k == ability) else {
        return Ok(());
    };
    if score > *cap {
        return Err(format!(
            "a {}'s {} cannot naturally exceed {} - {} was asked for",
            sp.name,
            ability.to_uppercase(),
            cap,
            score
        ));
    }
    Ok(())
}

/// One step up the size ladder, per step.
///
/// THIS FUNCTION USED TO CARRY ITS OWN ARRAY OF THE SIX WORDS - the
/// third copy of the ladder in the crate, written while vitality.rs's
/// own comment warned that a second one would eventually disagree. It
/// asks size.rs now, and an unrecognised size still comes back
/// untouched rather than defaulted, because a typo should not silently
/// become Medium halfway through a capacity.
pub fn bump_size(size: &str, steps: i64) -> String {
    match crate::size::of(size) {
        Some(_) => crate::size::bump(size, steps).to_string(),
        None => size.to_string(),
    }
}

/* ============================ READING ============================ */

pub const SPECIES_COLUMNS: &str = "key,game_id,name,ability_bonuses,ability_maxima,size,\
carry_size_steps,skill_profs,unarmored_ac_base,unarmored_ac_ability,speed,\
spell_save_bonus,damage_resistances,tongues,height_min_ft,height_max_ft,playable,creature_type,\
summary,appearance,culture,history,roleplaying,\
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

/// A jsonb list of {name, spoken, written}.
///
/// A MISSING BOOLEAN READS AS TRUE, which is the opposite call to
/// `Trait::applied` and right for the opposite reason: an entry that
/// names a language is claiming the character HAS it, and the common
/// case by far is both. Defaulting to false would silently mute
/// somebody for a field nobody filled in.
pub fn tongues_of(v: &Value, key: &str) -> Vec<Tongue> {
    v.get(key)
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|t| {
                    Some(Tongue {
                        name: t.get("name")?.as_str()?.to_string(),
                        spoken: t.get("spoken").and_then(|x| x.as_bool()).unwrap_or(true),
                        written: t.get("written").and_then(|x| x.as_bool()).unwrap_or(true),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
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
                        // ABSENT MEANS FEATURE. Every trait written
                        // before 058 was one, and a people whose
                        // author did not think about it has no cost -
                        // which is the safe way round for a label that
                        // decides how something is coloured.
                        kind: t
                            .get("kind")
                            .and_then(|x| x.as_str())
                            .unwrap_or("feature")
                            .to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn from_row(r: &Value) -> Species {
    let min_ft = crate::supabase::numeric_at(r, "height_min_ft");
    let max_ft = crate::supabase::numeric_at(r, "height_max_ft");
    let stated = r.get("size").and_then(|x| x.as_str()).unwrap_or("med");
    let rung = crate::size::of(stated);
    Species {
        key: r.get("key").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        name: r.get("name").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        ability_bonuses: ability_map(r, "ability_bonuses"),
        ability_maxima: ability_map(r, "ability_maxima"),
        size: stated.to_string(),
        carry_size_steps: r.get("carry_size_steps").and_then(|x| x.as_i64()).unwrap_or(0),
        skill_profs: strs(r, "skill_profs"),
        unarmored_ac_base: r.get("unarmored_ac_base").and_then(|x| x.as_i64()),
        unarmored_ac_ability: opt_text(r, "unarmored_ac_ability"),
        spell_save_bonus: r.get("spell_save_bonus").and_then(|x| x.as_i64()),
        height_min_ft: min_ft,
        height_max_ft: max_ft,
        derived: Species::derive_size(min_ft, max_ft).map(str::to_string),
        reach_ft: rung.map(|s| s.reach_ft),
        space_ft: rung.map(|s| s.space_ft),
        // ABSENT MEANS PLAYABLE. The column defaults true and every
        // species seeded so far is; the Imiear will be the first false
        // and they do not exist yet.
        playable: r.get("playable").and_then(|x| x.as_bool()).unwrap_or(true),
        speed: r.get("speed").and_then(|x| x.as_i64()),
        creature_type: opt_text(r, "creature_type"),
        damage_resistances: strs(r, "damage_resistances"),
        tongues: tongues_of(r, "tongues"),
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
            "height_min_ft": 7, "height_max_ft": 10, "playable": true,
            "tongues": [
                { "name": "Common", "spoken": true, "written": true },
                { "name": "Unt'garoth Dialect", "spoken": true, "written": true }
            ],
            "summary": "Towering.", "traits": [
                { "name": "Powerful Build", "text": "One size larger for carrying.", "applied": true },
                { "name": "Elemental Resilience", "text": "Fire and cold.", "applied": false }
            ]
        })
    }

    /* ---------------- the natural ceiling ---------------- */

    /// The Ny'ook: Strength cannot naturally pass 13.
    fn nyook() -> Species {
        from_row(&json!({
            "key": "nyook", "name": "Ny'ook",
            "ability_bonuses": {"dex": 2, "cha": 2},
            "ability_maxima": {"str": 13},
            "size": "sm", "playable": true
        }))
    }

    #[test]
    fn a_stated_ceiling_refuses_a_score_above_it() {
        let e = within_natural_cap(Some(&nyook()), "str", 16).unwrap_err();
        assert!(e.contains("Ny'ook"), "{}", e);
        assert!(e.contains("13"), "{}", e);
        assert!(e.contains("16"), "the number asked for, so it reads back: {}", e);
    }

    #[test]
    fn the_ceiling_itself_is_allowed() {
        assert!(within_natural_cap(Some(&nyook()), "str", 13).is_ok());
        assert!(within_natural_cap(Some(&nyook()), "str", 8).is_ok());
    }

    #[test]
    fn a_ceiling_binds_only_the_ability_it_names() {
        assert!(within_natural_cap(Some(&nyook()), "dex", 18).is_ok());
        assert!(within_natural_cap(Some(&nyook()), "cha", 20).is_ok());
    }

    /// THE TARRASQUE TEST. A statblock is a character and monsters run
    /// to Strength 30. Imposing 5e's default 20 on everything with no
    /// species would make the dragon unwritable in order to enforce a
    /// rule about the Ny'ook - so an unstated ceiling is no ceiling.
    #[test]
    fn no_species_means_no_refusal() {
        assert!(within_natural_cap(None, "str", 30).is_ok());
    }

    #[test]
    fn a_species_with_no_opinion_about_an_ability_does_not_refuse() {
        // The Unt'garoth state a Strength ceiling and nothing else.
        let s = from_row(&untgaroth());
        assert!(within_natural_cap(Some(&s), "con", 25).is_ok());
    }

    /// A RAISED CEILING IS STILL A CEILING. 21 is the whole point of
    /// the Unt'garoth, and 22 is still too far.
    #[test]
    fn a_raised_ceiling_refuses_above_itself() {
        let s = from_row(&untgaroth());
        assert!(within_natural_cap(Some(&s), "str", 21).is_ok());
        assert!(within_natural_cap(Some(&s), "str", 22).is_err());
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

    /* ---------------------- height and size ---------------------- */

    #[test]
    fn a_height_lands_on_a_rung_and_is_checked_against_the_stated_one() {
        let s = from_row(&untgaroth());
        assert_eq!(s.height_min_ft, Some(7.0));
        assert_eq!(s.height_max_ft, Some(10.0));
        // 7 to 10 straddles the Medium/Large line at 8. The midpoint
        // is 8.5, which is Large, which is what their document says.
        assert_eq!(s.derived.as_deref(), Some("lg"));
        assert!(s.size_matches_height(), "the document and the feet agree");
    }

    // The check has to be able to FAIL, or it is decoration.
    #[test]
    fn a_stated_size_that_the_height_contradicts_is_reported() {
        let wrong = from_row(&json!({
            "key": "x", "name": "X", "size": "huge",
            "height_min_ft": 4, "height_max_ft": 5
        }));
        assert_eq!(wrong.derived.as_deref(), Some("med"));
        assert!(!wrong.size_matches_height(), "five feet is not Huge");
    }

    // The Unt'gar, whose band sits entirely inside one rung.
    #[test]
    fn a_band_inside_one_rung_agrees_with_itself() {
        let untgar = from_row(&json!({
            "key": "untgar", "name": "Unt'gar", "size": "med",
            "height_min_ft": 4, "height_max_ft": 5
        }));
        assert_eq!(untgar.derived.as_deref(), Some("med"));
        assert!(untgar.size_matches_height());
    }

    #[test]
    fn a_people_with_no_height_contradicts_nothing() {
        let s = from_row(&json!({ "key": "x", "name": "X", "size": "med" }));
        assert_eq!(s.derived.as_deref(), None);
        assert!(s.size_matches_height());
    }

    #[test]
    fn reach_and_space_come_off_the_ladder() {
        let s = from_row(&untgaroth());
        assert_eq!(s.reach_ft, Some(10), "Large reaches ten feet");
        assert_eq!(s.space_ft, Some(10.0));
    }

    // Absent means playable; the column defaults true.
    #[test]
    fn a_people_is_playable_unless_it_says_otherwise() {
        assert!(from_row(&untgaroth()).playable);
        let imiear = from_row(&json!({ "key": "i", "name": "I", "playable": false }));
        assert!(!imiear.playable);
    }

    /* ------------------- drawbacks and tongues ------------------- */

    // A trait written before 058 has no `kind`, and every one of them
    // was a feature. The safe default is the one that does not accuse
    // a people of a cost its author never wrote.
    #[test]
    fn a_trait_with_no_kind_is_a_feature() {
        let s = from_row(&json!({
            "key": "x", "name": "X",
            "traits": [{ "name": "Old", "text": "Written before 058." }]
        }));
        assert_eq!(s.traits[0].kind, "feature");
        assert!(!s.traits[0].is_drawback());
    }

    #[test]
    fn a_drawback_says_so() {
        let s = from_row(&json!({
            "key": "x", "name": "X",
            "traits": [
                { "name": "Powerful Build", "text": "...", "kind": "feature", "applied": true },
                { "name": "Dense Mass", "text": "Cannot swim.", "kind": "drawback" }
            ]
        }));
        assert!(!s.traits[0].is_drawback());
        assert!(s.traits[1].is_drawback());
        // The two columns are INDEPENDENT: a drawback the engine does
        // not enforce is the commonest kind there is.
        assert!(!s.traits[1].applied);
    }

    #[test]
    fn a_tongue_carries_two_separate_facts() {
        let s = from_row(&json!({
            "key": "x", "name": "X",
            "tongues": [
                { "name": "Common", "spoken": true, "written": true },
                { "name": "Old Jotun", "spoken": false, "written": true },
                { "name": "Cant", "spoken": true, "written": false }
            ]
        }));
        assert_eq!(s.tongues.len(), 3);
        assert!(s.tongues[1].written && !s.tongues[1].spoken, "read, never pronounced");
        assert!(s.tongues[2].spoken && !s.tongues[2].written, "no script");
    }

    // The opposite default to `applied`, and for the opposite reason:
    // naming a language is claiming it, and both is the common case.
    #[test]
    fn a_tongue_that_does_not_say_is_both() {
        let s = from_row(&json!({
            "key": "x", "name": "X", "tongues": [{ "name": "Common" }]
        }));
        assert!(s.tongues[0].spoken && s.tongues[0].written);
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
