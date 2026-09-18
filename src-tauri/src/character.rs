//! Character sheet: loading it, and turning a named request into dice.
//!
//! This is the port of getRollerContext() + resolveRequest() from
//! diceroller.js, split in two on purpose:
//!
//!   load_sheet()      talks to the network. Untestable without one.
//!   resolve_request() is pure. Given a Sheet it is arithmetic and
//!                     string matching, so it gets real unit tests.
//!
//! That split is the whole reason the rules are worth having in Rust.
//! In the old system resolution was tangled up with reading spreadsheet
//! ranges, so the only way to check "does Insight give +6" was to roll
//! and squint at the card.
//!
//! WHAT IS NOT HERE YET: techniques, spells, weapon attacks, death
//! saves, rests. Those were the back half of resolveRequest and each
//! needs its own table. A request that matches none of the cases below
//! falls through to "treat it as a raw formula", exactly as the original
//! did, so `2d6+3` still works and nothing is lost by the gap.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::dice::d20_formula;
use crate::equipment::{self, Equipped};
use crate::narrative;
use crate::supabase;

/* ============================ THE SHEET ============================ */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ability {
    pub score: i64,
    pub save_prof: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDef {
    pub key: String,
    pub name: String,
    /// Ability code this skill keys off: str/dex/con/int/wis/cha.
    pub ability: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sheet {
    pub character_id: String,
    pub game_id: String,
    pub name: String,
    pub level: i64,
    /// Ability code -> score and save proficiency. Always six entries;
    /// the database seeds them on character creation.
    pub abilities: HashMap<String, Ability>,
    /// The skill catalogue in effect for this character's game, with
    /// game-scoped overrides already resolved over the global rows.
    pub skills: Vec<SkillDef>,
    /// Skill key -> proficiency multiplier. Absent means untrained.
    pub profs: HashMap<String, f64>,
    /// Which narrative_lines pack voices this character's roll cards.
    pub narrative_pack: String,
    /// Roll key -> the lines available for it, pack precedence and game
    /// overrides already resolved. Read once with the sheet so choosing
    /// a line at roll time costs nothing — see narrative.rs.
    ///
    /// NOT SENT TO THE FRONTEND. A full pack is 180 lines, and the
    /// webview has no use for any of them — roll_named picks the line on
    /// this side and the chosen one travels on the roll row. Serializing
    /// it would put ~20KB of prose through the IPC boundary on every
    /// get_sheet. `default` keeps Deserialize working, since skipping a
    /// field on the way out says nothing about the way in.
    #[serde(skip_serializing, default)]
    pub narratives: HashMap<String, Vec<String>>,
    /// Foundry `traits.weaponProf.value`: `sim`, `mar`, or a bare
    /// baseItem granting one weapon. Empty by default, because a wrong
    /// proficiency is worse than an absent one — it moves to-hit and
    /// says nothing.
    pub weapon_profs: Vec<String>,
    /// Foundry `traits.armorProf.value`: lgt med hvy shl, the same
    /// spelling 008 normalizes `items.armor_category` to.
    pub armor_profs: Vec<String>,
    /// What is equipped right now, with proficiency and attack modes
    /// already derived. Unlike `narratives` this one IS sent out — a
    /// loadout is a handful of rows and a sheet screen wants it.
    pub loadout: Vec<Equipped>,
}

impl Sheet {
    /// floor((level-1)/4)+2. Computed here rather than stored, so the
    /// rule lives in exactly one place.
    pub fn proficiency_bonus(&self) -> i64 {
        ((self.level - 1) / 4) + 2
    }

    /// floor((score-10)/2). div_euclid, not plain division: Rust
    /// truncates toward zero and JS's Math.floor does not, so a score of
    /// 7 must give -2 and not -1.
    pub fn ability_mod(&self, code: &str) -> i64 {
        match self.abilities.get(code) {
            Some(a) => (a.score - 10).div_euclid(2),
            None => 0,
        }
    }

    pub fn save_prof(&self, code: &str) -> bool {
        self.abilities.get(code).map(|a| a.save_prof).unwrap_or(false)
    }

    /// The proficiency contribution for a skill. floor(multiplier * PB),
    /// matching Math.floor(s.prof * ctx.pb) — so half-proficiency at PB 3
    /// is +1, not +1.5.
    pub fn skill_prof_bonus(&self, key: &str) -> i64 {
        let mult = self.profs.get(key).copied().unwrap_or(0.0);
        (mult * self.proficiency_bonus() as f64).floor() as i64
    }

    fn find_skill(&self, needle: &str) -> Option<&SkillDef> {
        let n = needle.trim().to_lowercase();
        self.skills
            .iter()
            .find(|s| s.key.to_lowercase() == n || s.name.to_lowercase() == n)
    }
}

/* ============================ RESOLUTION ============================ */

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Resolved {
    /// What the card says: "Insight (WIS)", "WIS Save".
    pub label: String,
    /// What the dice engine is handed.
    pub formula: String,
    /// The modifier that went into it, so a UI can explain the number
    /// instead of just showing it.
    pub modifier: i64,
    /// THE ROLL KIND, in engine vocabulary: a skill key ("ins"),
    /// "wis_save", "wis_check", or "custom" for a raw formula. Not the
    /// request the player typed — "Insight", "insight" and "ins" all
    /// resolve to the same key.
    ///
    /// This is the vocabulary narrative_lines.key and skill_prompts.key
    /// are written in, so it is what a narrative lookup and, later, an
    /// AI prompt seed are found by. The full vocabulary is emitted here
    /// even where no lines are seeded for it yet: a pack that grows a
    /// "wis_save" key then works without a Rust change.
    pub key: String,
}

/// Long and short ability names, both accepted. Ported from ABIL_NAMES.
fn ability_code(s: &str) -> Option<&'static str> {
    match s.trim().to_lowercase().as_str() {
        "str" | "strength" => Some("str"),
        "dex" | "dexterity" => Some("dex"),
        "con" | "constitution" => Some("con"),
        "int" | "intelligence" => Some("int"),
        "wis" | "wisdom" => Some("wis"),
        "cha" | "charisma" => Some("cha"),
        _ => None,
    }
}

/// Turn a request into a label and a formula.
///
/// Order matters and mirrors the original: saves, then skills, then
/// ability checks, then fall through to a raw formula. A skill named
/// "Athletics" must not be shadowed by anything, and an unknown request
/// must stay usable as a formula.
pub fn resolve_request(sheet: &Sheet, request: &str, mode: &str) -> Resolved {
    let t = request.trim().to_lowercase();

    // "<ability> save" / "wis save" / "wisdom save"
    if let Some(stem) = t.strip_suffix(" save") {
        if let Some(code) = ability_code(stem) {
            let pb = sheet.proficiency_bonus();
            let m = sheet.ability_mod(code)
                + if sheet.save_prof(code) { pb } else { 0 };
            return Resolved {
                label: format!("{} Save", code.to_uppercase()),
                formula: d20_formula(m, mode),
                modifier: m,
                key: format!("{}_save", code),
            };
        }
    }

    // Skill, by key ("ins") or by name ("insight")
    if let Some(s) = sheet.find_skill(&t) {
        let m = sheet.ability_mod(&s.ability) + sheet.skill_prof_bonus(&s.key);
        return Resolved {
            label: format!("{} ({})", s.name, s.ability.to_uppercase()),
            formula: d20_formula(m, mode),
            modifier: m,
            key: s.key.clone(),
        };
    }

    // Ability check: "wis" or "wisdom check"
    let stem = t.strip_suffix(" check").unwrap_or(&t);
    if let Some(code) = ability_code(stem) {
        let m = sheet.ability_mod(code);
        return Resolved {
            label: format!("{} Check", code.to_uppercase()),
            formula: d20_formula(m, mode),
            modifier: m,
            key: format!("{}_check", code),
        };
    }

    // Anything else is a raw formula. The dice engine will reject it if
    // it is nonsense, which is the right place for that to happen.
    Resolved {
        label: request.trim().to_string(),
        formula: t,
        modifier: 0,
        key: "custom".to_string(),
    }
}

/* ============================ LOADING ============================ */

fn as_i64(v: &Value, key: &str, default: i64) -> i64 {
    v.get(key).and_then(|x| x.as_i64()).unwrap_or(default)
}

fn as_str(v: &Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

/// A Postgres text[] arrives as a JSON array. Absent reads as empty,
/// which is the right default for a proficiency list: claiming none is
/// safe, claiming one that was not granted is not.
fn as_strings(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(|x| x.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str()).map(str::to_string).collect())
        .unwrap_or_default()
}

/// Read everything resolve_request needs, plus the narrative pack and
/// the equipped loadout, in seven queries.
///
/// Seven is more than this wants to be. It runs on every roll, and the
/// two equipment reads sit ahead of the dice with the pack read. None of
/// them is slow individually and none has been worth splitting yet — but
/// this is the count to watch, and `preview_request` pays all of it for
/// a hover.
///
/// Could be one query with PostgREST embedding, but four explicit reads
/// are easier to debug when a policy denies one of them — an embedded
/// join that silently returns fewer rows looks like missing data rather
/// than a permission problem.
pub fn load_sheet(token: &str, character_id: &str) -> Result<Sheet, String> {
    let chars = supabase::rest_get(
        token,
        "characters",
        &[
            (
                "select",
                "id,game_id,name,level,narrative_pack,weapon_profs,armor_profs",
            ),
            ("id", &format!("eq.{}", character_id)),
        ],
    )?;
    let c = chars
        .as_array()
        .and_then(|a| a.first())
        .ok_or_else(|| "character not found, or not visible to you".to_string())?;

    let game_id = as_str(c, "game_id");

    let abil_rows = supabase::rest_get(
        token,
        "character_abilities",
        &[
            ("select", "ability,score,save_prof"),
            ("character_id", &format!("eq.{}", character_id)),
        ],
    )?;
    let mut abilities = HashMap::new();
    if let Some(rows) = abil_rows.as_array() {
        for r in rows {
            abilities.insert(
                as_str(r, "ability"),
                Ability {
                    score: as_i64(r, "score", 10),
                    save_prof: r.get("save_prof").and_then(|x| x.as_bool()).unwrap_or(false),
                },
            );
        }
    }

    // Global rows plus this game's overrides, in one request. The
    // override wins because it is inserted second below.
    let skill_rows = supabase::rest_get(
        token,
        "skills",
        &[
            ("select", "key,name,ability,game_id,sort_order"),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
            ("order", "sort_order.asc"),
        ],
    )?;

    let mut by_key: HashMap<String, SkillDef> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    if let Some(rows) = skill_rows.as_array() {
        // Globals first, then overrides, so an override replaces the
        // global entry it shadows rather than appearing beside it.
        for pass in [true, false] {
            for r in rows {
                let is_global = r.get("game_id").map(|g| g.is_null()).unwrap_or(true);
                if is_global != pass {
                    continue;
                }
                let key = as_str(r, "key");
                if !by_key.contains_key(&key) {
                    order.push(key.clone());
                }
                by_key.insert(
                    key.clone(),
                    SkillDef {
                        key,
                        name: as_str(r, "name"),
                        ability: as_str(r, "ability"),
                    },
                );
            }
        }
    }
    let skills: Vec<SkillDef> = order
        .iter()
        .filter_map(|k| by_key.get(k).cloned())
        .collect();

    let prof_rows = supabase::rest_get(
        token,
        "character_skills",
        &[
            ("select", "skill_key,prof"),
            ("character_id", &format!("eq.{}", character_id)),
        ],
    )?;
    let mut profs = HashMap::new();
    if let Some(rows) = prof_rows.as_array() {
        for r in rows {
            // PostgREST returns numeric as a JSON string to preserve
            // precision, so as_f64 alone is not enough.
            let p = r
                .get("prof")
                .and_then(|x| x.as_f64().or_else(|| x.as_str().and_then(|s| s.parse().ok())))
                .unwrap_or(0.0);
            profs.insert(as_str(r, "skill_key"), p);
        }
    }

    // The pack, and base underneath it. An empty column reads as base
    // rather than as a pack with no lines, so a row written before 006
    // added the default still narrates.
    let mut narrative_pack = as_str(c, "narrative_pack");
    if narrative_pack.trim().is_empty() {
        narrative_pack = narrative::BASE_PACK.to_string();
    }
    let line_rows = narrative::load_lines(token, &game_id, &narrative_pack)?;
    let narratives = narrative::resolve_lines(&line_rows, &narrative_pack);

    let weapon_profs = as_strings(c, "weapon_profs");
    let armor_profs = as_strings(c, "armor_profs");
    let loadout = equipment::load_loadout(
        token,
        character_id,
        &game_id,
        &weapon_profs,
        &armor_profs,
    )?;

    Ok(Sheet {
        character_id: as_str(c, "id"),
        game_id,
        name: as_str(c, "name"),
        level: as_i64(c, "level", 1),
        abilities,
        skills,
        profs,
        narrative_pack,
        narratives,
        weapon_profs,
        armor_profs,
        loadout,
    })
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    /// Rodnar at level 5: PB +3. WIS 16 (+3), STR 8 (-1), DEX 14 (+2).
    /// Proficient in Insight, expertise in Perception, half in Stealth,
    /// untrained in Athletics. WIS saves proficient, STR saves not.
    fn rodnar() -> Sheet {
        let mut abilities = HashMap::new();
        abilities.insert("str".into(), Ability { score: 8,  save_prof: false });
        abilities.insert("dex".into(), Ability { score: 14, save_prof: false });
        abilities.insert("con".into(), Ability { score: 15, save_prof: false });
        abilities.insert("int".into(), Ability { score: 10, save_prof: false });
        abilities.insert("wis".into(), Ability { score: 16, save_prof: true  });
        abilities.insert("cha".into(), Ability { score: 12, save_prof: false });

        let skills = vec![
            SkillDef { key: "ins".into(), name: "Insight".into(),    ability: "wis".into() },
            SkillDef { key: "prc".into(), name: "Perception".into(), ability: "wis".into() },
            SkillDef { key: "ste".into(), name: "Stealth".into(),    ability: "dex".into() },
            SkillDef { key: "ath".into(), name: "Athletics".into(),  ability: "str".into() },
        ];

        let mut profs = HashMap::new();
        profs.insert("ins".into(), 1.0);
        profs.insert("prc".into(), 2.0);
        profs.insert("ste".into(), 0.5);

        Sheet {
            character_id: "c1".into(),
            game_id: "g1".into(),
            name: "Rodnar Shieldcrest".into(),
            level: 5,
            abilities,
            skills,
            profs,
            // Resolution is arithmetic and string matching; the prose
            // rides along on the sheet but has no part in it. The
            // lines themselves are tested in narrative.rs.
            narrative_pack: narrative::BASE_PACK.into(),
            narratives: HashMap::new(),
            // Rodnar's, from the 008 backfill. Resolution does not read
            // them yet - the attack key is the next job - but the sheet
            // carries them and the fixture should not lie about it. The
            // rules that DO read them are tested in equipment.rs.
            weapon_profs: vec!["sim".into()],
            armor_profs: vec!["lgt".into(), "med".into(), "shl".into()],
            loadout: Vec::new(),
        }
    }

    #[test]
    fn proficiency_bonus_by_level() {
        let mut s = rodnar();
        for (level, pb) in [(1, 2), (4, 2), (5, 3), (8, 3), (9, 4), (13, 5), (17, 6), (20, 6)] {
            s.level = level;
            assert_eq!(s.proficiency_bonus(), pb, "level {}", level);
        }
    }

    #[test]
    fn ability_modifiers_floor_toward_negative() {
        let s = rodnar();
        assert_eq!(s.ability_mod("wis"), 3);   // 16
        assert_eq!(s.ability_mod("dex"), 2);   // 14
        assert_eq!(s.ability_mod("con"), 2);   // 15 -> floor(2.5)
        assert_eq!(s.ability_mod("int"), 0);   // 10
        assert_eq!(s.ability_mod("str"), -1);  // 8  -> floor(-1.0)
    }

    #[test]
    fn odd_low_scores_round_the_right_way() {
        // The case plain integer division gets wrong: 7 must be -2, not
        // -1. Rust truncates toward zero; Math.floor does not.
        let mut s = rodnar();
        s.abilities.insert("str".into(), Ability { score: 7, save_prof: false });
        assert_eq!(s.ability_mod("str"), -2);
        s.abilities.insert("str".into(), Ability { score: 3, save_prof: false });
        assert_eq!(s.ability_mod("str"), -4);
        s.abilities.insert("str".into(), Ability { score: 1, save_prof: false });
        assert_eq!(s.ability_mod("str"), -5);
    }

    #[test]
    fn proficient_skill_adds_full_bonus() {
        let r = resolve_request(&rodnar(), "insight", "normal");
        assert_eq!(r.label, "Insight (WIS)");
        assert_eq!(r.modifier, 6);            // WIS +3, PB +3
        assert_eq!(r.formula, "1d20+6");
    }

    #[test]
    fn skill_resolves_by_key_as_well_as_name() {
        let by_key = resolve_request(&rodnar(), "ins", "normal");
        let by_name = resolve_request(&rodnar(), "Insight", "normal");
        assert_eq!(by_key, by_name);
    }

    #[test]
    fn expertise_doubles_the_bonus() {
        let r = resolve_request(&rodnar(), "perception", "normal");
        assert_eq!(r.modifier, 9);            // WIS +3, PB +3 doubled
        assert_eq!(r.formula, "1d20+9");
    }

    #[test]
    fn half_proficiency_floors() {
        // 0.5 * 3 = 1.5 -> 1, not 2 and not 1.5.
        let r = resolve_request(&rodnar(), "stealth", "normal");
        assert_eq!(r.modifier, 3);            // DEX +2, half PB +1
        assert_eq!(r.formula, "1d20+3");
    }

    #[test]
    fn untrained_skill_is_the_bare_ability() {
        let r = resolve_request(&rodnar(), "athletics", "normal");
        assert_eq!(r.modifier, -1);           // STR -1, no proficiency
        assert_eq!(r.formula, "1d20-1");
    }

    #[test]
    fn advantage_and_disadvantage_change_only_the_dice() {
        let adv = resolve_request(&rodnar(), "insight", "adv");
        let dis = resolve_request(&rodnar(), "insight", "dis");
        assert_eq!(adv.formula, "2d20kh1+6");
        assert_eq!(dis.formula, "2d20kl1+6");
        assert_eq!(adv.modifier, dis.modifier);
        assert_eq!(adv.label, dis.label);
    }

    #[test]
    fn proficient_save_adds_the_bonus() {
        let r = resolve_request(&rodnar(), "wis save", "normal");
        assert_eq!(r.label, "WIS Save");
        assert_eq!(r.modifier, 6);            // WIS +3, proficient
    }

    #[test]
    fn unproficient_save_does_not() {
        let r = resolve_request(&rodnar(), "str save", "normal");
        assert_eq!(r.label, "STR Save");
        assert_eq!(r.modifier, -1);
    }

    #[test]
    fn long_ability_names_work_too() {
        assert_eq!(
            resolve_request(&rodnar(), "wisdom save", "normal"),
            resolve_request(&rodnar(), "wis save", "normal")
        );
        assert_eq!(
            resolve_request(&rodnar(), "strength check", "normal").label,
            "STR Check"
        );
    }

    #[test]
    fn ability_check_never_adds_proficiency() {
        let r = resolve_request(&rodnar(), "wis", "normal");
        assert_eq!(r.label, "WIS Check");
        assert_eq!(r.modifier, 3);            // not 6 — a check is not a save
    }

    #[test]
    fn a_skill_is_not_shadowed_by_an_ability_check() {
        // "ins" is a skill key; it must not be mistaken for anything
        // else. Ordering in resolve_request is what guarantees this.
        let r = resolve_request(&rodnar(), "ins", "normal");
        assert_eq!(r.label, "Insight (WIS)");
    }

    #[test]
    fn unknown_requests_pass_through_as_formulas() {
        let r = resolve_request(&rodnar(), "2d6+3", "normal");
        assert_eq!(r.formula, "2d6+3");
        assert_eq!(r.label, "2d6+3");
        assert_eq!(r.modifier, 0);
    }

    #[test]
    fn case_and_whitespace_do_not_matter() {
        let r = resolve_request(&rodnar(), "  INSIGHT  ", "normal");
        assert_eq!(r.label, "Insight (WIS)");
        assert_eq!(r.formula, "1d20+6");
    }

    #[test]
    fn the_key_is_engine_vocabulary_not_what_was_typed() {
        // narrative_lines.key and skill_prompts.key are written in this
        // vocabulary. Every spelling of a request must land on one key
        // or a pack would have to carry a line per synonym.
        for req in ["insight", "Insight", "ins", "  INSIGHT  "] {
            assert_eq!(resolve_request(&rodnar(), req, "normal").key, "ins", "{}", req);
        }
        assert_eq!(resolve_request(&rodnar(), "wis save", "normal").key, "wis_save");
        assert_eq!(resolve_request(&rodnar(), "wisdom save", "normal").key, "wis_save");
        assert_eq!(resolve_request(&rodnar(), "str check", "normal").key, "str_check");
        assert_eq!(resolve_request(&rodnar(), "wis", "normal").key, "wis_check");
        assert_eq!(resolve_request(&rodnar(), "2d6+3", "normal").key, "custom");
    }

    #[test]
    fn a_level_one_character_has_pb_two() {
        let mut s = rodnar();
        s.level = 1;
        let r = resolve_request(&s, "insight", "normal");
        assert_eq!(r.modifier, 5);            // WIS +3, PB +2
    }

    #[test]
    fn resolved_formulas_are_always_rollable() {
        // The contract between this module and dice.rs: anything
        // resolve_request emits, roll_formula must accept.
        let s = rodnar();
        for req in ["insight", "perception", "stealth", "athletics", "wis save", "str", "2d6+3"] {
            for mode in ["normal", "adv", "dis"] {
                let r = resolve_request(&s, req, mode);
                crate::dice::roll_formula(&r.formula).unwrap_or_else(|e| {
                    panic!("{} / {} produced unrollable {}: {}", req, mode, r.formula, e)
                });
            }
        }
    }
}
