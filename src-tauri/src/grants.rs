//! What a magic thing gives whoever is wearing it.
//!
//! THE RULES ONLY. Reading the rows is equipment.rs and character.rs;
//! this parses a grant, decides which ones are live, and resolves a
//! pile of them into one number. The same division initiative.rs and
//! commands/initiative.rs have.
//!
//! 049 GAVE AN OBJECT OVERRIDES AND THEY ALL ANSWER "WHAT IS THIS".
//! A greatsword that is unusually large; a mace that rolls 1d8. None
//! of them touches the person holding it, and that is the whole of
//! what magic does: a +1 longsword is not a longsword with a bigger
//! die, it is one that makes its wielder better.
//!
//! ONE VOCABULARY. A column per effect - `attack_bonus`, `ac_bonus`,
//! `str_bonus` - is a migration every time somebody invents an item
//! and six readers that each know a different subset. A grant says
//! what it touches, how, and by how much, and every consumer asks this
//! module the same question.
//!
//! ADD AND SET ARE DIFFERENT RULES:
//!
//!   add   signed and cumulative. Two +1 cloaks are +2, and a cursed
//!         -2 is an add - which is why AbilitySource has said
//!         "negative is as real as positive" since 056.
//!   set   a FLOOR. 5e words every item that uses it the same way:
//!         "your Strength is 19 unless it is already 19 or higher".
//!         Two sets do not stack, the higher wins, and a set below
//!         what somebody already has does nothing.
//!
//! A SET THAT LOWERS IS NOT EXPRESSIBLE, deliberately. That is a
//! curse, a curse is an `add` with a negative value, and keeping them
//! apart means a Belt of Giant Strength can never nerf somebody who
//! was already stronger than it.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/* ============================ TYPES ============================ */

/// How a grant changes the number it touches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Add,
    Set,
}

impl Mode {
    /// Anything unrecognised is an ADD, and that is the safe way to be
    /// wrong: an add of 1 is a small mistake, where a set of 1 would
    /// silently floor somebody's Strength at 1.
    fn parse(s: &str) -> Mode {
        match s {
            "set" => Mode::Set,
            _ => Mode::Add,
        }
    }
}

/// One thing a magic item does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grant {
    /// What it touches, in the vocabulary 100 wrote down: `ac`,
    /// `attack`, `damage`, `save`, `save.<code>`, `skill.<key>`, or
    /// one of the six ability codes.
    pub target: String,
    pub mode: Mode,
    pub value: i64,
    /// 114. A DIE RATHER THAN A NUMBER, where the thing granting it
    /// rolls. Bless is +1d4 to attacks and saves, not +2 - and a
    /// vocabulary that could only say "+2" would have forced either a
    /// second mechanism for spells or an average nobody agreed to.
    ///
    /// `value` AND `dice` ARE BOTH ALLOWED and both apply: a grant of
    /// +1 and 1d4 is a real thing to write. A grant with neither is
    /// not a grant and `parse` drops it.
    ///
    /// NEVER ON A `set`. "Your Strength is 1d4" is not a sentence 5e
    /// ever writes, and a floor that moved every time you looked at it
    /// would be a different rule.
    pub dice: Option<String>,
    /// Whether this waits for attunement. Default false: plenty of
    /// magic needs none, and a default that quietly switched items off
    /// would read as the grants not working at all.
    pub needs_attunement: bool,
    /// What to call it on a sheet. The item's name when the row does
    /// not say otherwise - filled in by whoever collects these, since
    /// a grant does not know what it is attached to.
    pub source: String,
}

/* ============================ READING ============================ */

/// Parse one list off a `grants` column.
///
/// A ROW THAT MAKES NO SENSE IS SKIPPED RATHER THAN GUESSED AT. No
/// target or no value and there is nothing to apply; inventing a
/// default would put a number on a sheet nobody wrote. The skip is
/// silent because this is reference data a DM types, and a malformed
/// row is already visible as an item that does nothing.
pub fn parse(list: &Value, source: &str) -> Vec<Grant> {
    let Some(rows) = list.as_array() else {
        return Vec::new();
    };
    rows.iter()
        .filter_map(|g| {
            let target = g.get("target")?.as_str()?.trim().to_lowercase();
            if target.is_empty() {
                return None;
            }
            // 114. EITHER A VALUE OR A DIE, and a grant with neither
            // is not a grant. `value` defaults to 0 so a pure die
            // grant can leave it out rather than writing a zero that
            // reads like a deliberate nothing.
            let value = g.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
            let dice = g
                .get("dice")
                .and_then(|d| d.as_str())
                .map(str::trim)
                .filter(|d| !d.is_empty())
                .map(str::to_string);
            // 116. A RESISTANCE IS A PRESENCE RATHER THAN A NUMBER.
            // `resist.fire` has nothing to add and nothing to floor -
            // being resistant is the whole of what it says - so it is
            // admitted with no value, rather than carrying a `1` that
            // means nothing and that somebody would eventually try to
            // add to something.
            if value == 0 && dice.is_none() && !crate::resist::is_resist_target(&target) {
                return None;
            }
            let mode = Mode::parse(g.get("mode").and_then(|m| m.as_str()).unwrap_or("add"));
            Some(Grant {
                target,
                // A die on a `set` is dropped rather than honoured -
                // see the note on `dice`.
                dice: dice.filter(|_| mode == Mode::Add),
                mode,
                value,
                needs_attunement: g
                    .get("needs_attunement")
                    .and_then(|a| a.as_bool())
                    .unwrap_or(false),
                source: g
                    .get("source")
                    .and_then(|s| s.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or(source)
                    .to_string(),
            })
        })
        .collect()
}

/* ============================ RULES ============================ */

/// Every die added to one target, as formula terms.
///
/// 114. SEPARATE FROM `total_add` BECAUSE A DIE IS NOT A NUMBER. The
/// flat adds collapse into one integer and the dice cannot - "+1d4"
/// has to reach the formula as itself so the roll shows what it rolled.
pub fn dice_for(grants: &[Grant], target: &str) -> Vec<String> {
    grants
        .iter()
        .filter(|g| g.mode == Mode::Add && g.target == target)
        .filter_map(|g| g.dice.clone())
        .collect()
}

/// Everything added to one target, summed.
pub fn total_add(grants: &[Grant], target: &str) -> i64 {
    grants
        .iter()
        .filter(|g| g.mode == Mode::Add && g.target == target)
        .map(|g| g.value)
        .sum()
}

/// The highest floor set on one target, if anything sets one.
pub fn best_set(grants: &[Grant], target: &str) -> Option<i64> {
    grants
        .iter()
        .filter(|g| g.mode == Mode::Set && g.target == target)
        .map(|g| g.value)
        .max()
}

/// A natural value, with every live grant on it applied.
///
/// THE ORDER IS THE RULE. The floor is taken first against what
/// somebody already has, and the adds land on top of whichever won -
/// so Gauntlets of Ogre Power (set 19) and a +2 belt (add 2) make 21
/// for a Strength 8 wizard and 21 for a Strength 19 barbarian, which
/// is what both items say on their own.
pub fn apply(natural: i64, grants: &[Grant], target: &str) -> i64 {
    let floored = match best_set(grants, target) {
        Some(set) => natural.max(set),
        None => natural,
    };
    floored + total_add(grants, target)
}

/// The named steps between a natural value and what `apply` returns.
///
/// FOR A SCREEN THAT SHOWS ITS WORKING. "Performance +2" is a number
/// to take on faith; "Performance +1 + Pipes +1" is an account of
/// itself, and this app has spent its whole life refusing the first
/// one - the attack preview spells out STR and the proficiency bonus
/// for exactly this reason.
///
/// THE STEPS SUM TO THE DIFFERENCE, always. A floor contributes the
/// distance it lifted the value, so a set from 10 to 19 reads as +9
/// against the thing that did it - and a set that lifted nothing
/// contributes nothing and is not listed, because an item that
/// changed no number should not appear in the arithmetic.
pub fn breakdown(natural: i64, grants: &[Grant], target: &str) -> Vec<(String, i64)> {
    let mut steps = Vec::new();
    let running = natural;

    if let Some(set) = best_set(grants, target) {
        if set > running {
            let who = grants
                .iter()
                .filter(|g| g.mode == Mode::Set && g.target == target && g.value == set)
                .map(|g| g.source.clone())
                .next()
                .unwrap_or_default();
            steps.push((who, set - running));
        }
    }
    for g in grants.iter().filter(|g| g.mode == Mode::Add && g.target == target) {
        if g.value != 0 {
            steps.push((g.source.clone(), g.value));
        }
    }
    steps
}

/// Which grants are live on a creature right now.
///
/// WORN OR HELD, which 084 made `slot` the one answer for: a +1 sword
/// in a backpack makes nobody better. Attunement is the second gate
/// and only for the grants that ask for it - already limited to three
/// by carry.rs, so this reads a state rather than inventing a rule.
pub fn live(grants: Vec<Grant>, in_a_slot: bool, attuned: bool) -> Vec<Grant> {
    if !in_a_slot {
        return Vec::new();
    }
    grants
        .into_iter()
        .filter(|g| !g.needs_attunement || attuned)
        .collect()
}

/// Everything live on one target, for a screen that wants to show its
/// working rather than a total.
pub fn sources_for<'a>(grants: &'a [Grant], target: &str) -> Vec<&'a Grant> {
    grants.iter().filter(|g| g.target == target).collect()
}

/// The six ability codes, which three of the target forms are built
/// from.
pub const ABILITIES: [&str; 6] = ["str", "dex", "con", "int", "wis", "cha"];

/// Whether anything will ever read this target.
///
/// THE SILENT NO-OP IS THE FAULT THIS PREVENTS. A grant written
/// against `armour` parses perfectly, stores perfectly, and does
/// nothing for ever - the item simply is not magic and nobody can see
/// why. That is this codebase's named defect class, and the cheapest
/// place to stop it is the write.
///
/// A SKILL KEY IS CHECKED FOR SHAPE AND NOT FOR EXISTENCE. The
/// catalogue is a table and this module has no database; the command
/// that writes a grant checks the key against it, which is where the
/// read already is.
pub fn known_target(target: &str) -> bool {
    if ["ac", "attack", "damage", "save"].contains(&target) {
        return true;
    }
    // 116. `resist.fire`, `immune.poison`, `vulnerable.cold`, and the
    // several-type form Protection from Energy needs. resist.rs owns
    // the spelling and the list of damage types; this asks it rather
    // than keeping a second copy of thirteen words.
    if crate::resist::is_resist_target(target) {
        return true;
    }
    if ABILITIES.contains(&target) {
        return true;
    }
    if let Some(code) = target.strip_prefix("save.") {
        return ABILITIES.contains(&code);
    }
    if let Some(key) = target.strip_prefix("skill.") {
        return !key.is_empty();
    }
    false
}

/// The skill key a target names, for the one caller that can check it
/// against the catalogue.
pub fn skill_key(target: &str) -> Option<&str> {
    target.strip_prefix("skill.")
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    /* ---------------- a grant that rolls (114) ---------------- */

    #[test]
    fn a_grant_can_carry_a_die_instead_of_a_number() {
        // Bless is +1d4, not +2 - and a vocabulary that could only say
        // "+2" would have forced an average nobody agreed to.
        let list = serde_json::json!([
            { "target": "attack", "mode": "add", "dice": "1d4" }
        ]);
        let got = parse(&list, "Bless");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].dice.as_deref(), Some("1d4"));
        assert_eq!(got[0].value, 0, "a pure die grant leaves value out");
    }

    #[test]
    fn a_grant_with_neither_a_value_nor_a_die_is_not_a_grant() {
        let list = serde_json::json!([{ "target": "attack", "mode": "add" }]);
        assert!(parse(&list, "x").is_empty());
    }

    #[test]
    fn a_value_and_a_die_both_apply() {
        let list = serde_json::json!([
            { "target": "save", "mode": "add", "value": 1, "dice": "1d4" }
        ]);
        let got = parse(&list, "x");
        assert_eq!(got[0].value, 1);
        assert_eq!(got[0].dice.as_deref(), Some("1d4"));
    }

    #[test]
    fn a_die_on_a_set_is_dropped() {
        // "Your Strength is 1d4" is not a sentence 5e ever writes, and
        // a floor that moved every time you looked at it would be a
        // different rule.
        let list = serde_json::json!([
            { "target": "str", "mode": "set", "value": 19, "dice": "1d4" }
        ]);
        let got = parse(&list, "Belt");
        assert_eq!(got[0].value, 19);
        assert_eq!(got[0].dice, None);
    }

    #[test]
    fn dice_for_collects_only_the_matching_target() {
        let list = serde_json::json!([
            { "target": "attack", "mode": "add", "dice": "1d4" },
            { "target": "save",   "mode": "add", "dice": "1d4" }
        ]);
        let got = parse(&list, "Bless");
        assert_eq!(dice_for(&got, "attack"), vec!["1d4"]);
        assert_eq!(dice_for(&got, "save"), vec!["1d4"]);
        assert!(dice_for(&got, "damage").is_empty());
    }

    #[test]
    fn bane_is_bless_with_a_minus_on_it() {
        // The clearest argument for one shape: a curse is an add with
        // a sign, exactly as 056 said of AbilitySource.
        let list = serde_json::json!([
            { "target": "attack", "mode": "add", "dice": "-1d4" }
        ]);
        assert_eq!(dice_for(&parse(&list, "Bane"), "attack"), vec!["-1d4"]);
    }

    use serde_json::json;

    fn g(target: &str, mode: Mode, value: i64) -> Grant {
        Grant {
            target: target.to_string(),
            mode,
            value,
            dice: None,
            needs_attunement: false,
            source: "a test".to_string(),
        }
    }

    /* ---------------------- parsing ---------------------- */

    #[test]
    fn a_plain_bonus_reads_off_the_row() {
        let got = parse(&json!([{"target": "ac", "mode": "add", "value": 1}]), "Cloak");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].target, "ac");
        assert_eq!(got[0].mode, Mode::Add);
        assert_eq!(got[0].value, 1);
        assert_eq!(got[0].source, "Cloak", "the item names itself");
        assert!(!got[0].needs_attunement);
    }

    #[test]
    fn a_missing_mode_is_an_add() {
        let got = parse(&json!([{"target": "attack", "value": 1}]), "x");
        assert_eq!(got[0].mode, Mode::Add);
    }

    /// An unknown mode is an ADD, which is the safe way to be wrong: a
    /// stray add of 1 is a small mistake, and a stray SET of 1 would
    /// floor somebody's Strength at 1.
    #[test]
    fn an_unknown_mode_is_an_add_rather_than_a_set() {
        let got = parse(&json!([{"target": "str", "mode": "replace", "value": 1}]), "x");
        assert_eq!(got[0].mode, Mode::Add);
    }

    #[test]
    fn a_row_with_nothing_to_apply_is_skipped() {
        let got = parse(
            &json!([
                {"mode": "add", "value": 1},
                {"target": "ac"},
                {"target": "", "value": 2},
                {"target": "ac", "value": 1}
            ]),
            "x",
        );
        assert_eq!(got.len(), 1, "only the last one says what to do");
    }

    #[test]
    fn a_target_is_matched_without_case_or_padding() {
        let got = parse(&json!([{"target": "  AC ", "value": 1}]), "x");
        assert_eq!(got[0].target, "ac");
    }

    #[test]
    fn nothing_at_all_is_no_grants() {
        assert!(parse(&json!([]), "x").is_empty());
        assert!(parse(&json!(null), "x").is_empty());
    }

    /* ---------------------- adding ---------------------- */

    #[test]
    fn two_cloaks_are_worth_two() {
        let gs = vec![g("ac", Mode::Add, 1), g("ac", Mode::Add, 1)];
        assert_eq!(total_add(&gs, "ac"), 2);
    }

    #[test]
    fn a_curse_is_an_add_that_is_negative() {
        let gs = vec![g("str", Mode::Add, 2), g("str", Mode::Add, -4)];
        assert_eq!(apply(14, &gs, "str"), 12);
    }

    #[test]
    fn a_grant_on_something_else_is_not_counted() {
        let gs = vec![g("ac", Mode::Add, 1)];
        assert_eq!(total_add(&gs, "attack"), 0);
        assert_eq!(apply(10, &gs, "attack"), 10);
    }

    /* ---------------------- setting ---------------------- */

    /// The sentence every 5e item that sets a score uses: "unless it is
    /// already that high".
    #[test]
    fn a_set_is_a_floor_and_never_a_demotion() {
        let gauntlets = vec![g("str", Mode::Set, 19)];
        assert_eq!(apply(8, &gauntlets, "str"), 19, "the weak are raised");
        assert_eq!(apply(20, &gauntlets, "str"), 20, "the strong are left alone");
    }

    #[test]
    fn two_sets_do_not_stack_and_the_higher_wins() {
        let both = vec![g("str", Mode::Set, 19), g("str", Mode::Set, 21)];
        assert_eq!(apply(10, &both, "str"), 21);
        assert_eq!(best_set(&both, "str"), Some(21));
    }

    /// THE ORDER IS THE RULE. The floor settles first, the adds land on
    /// top, so a belt and a pair of gauntlets give the same final
    /// number to a wizard and a barbarian - which is what each item
    /// claims on its own.
    #[test]
    fn a_floor_settles_before_the_bonuses_land() {
        let kit = vec![g("str", Mode::Set, 19), g("str", Mode::Add, 2)];
        assert_eq!(apply(8, &kit, "str"), 21);
        assert_eq!(apply(19, &kit, "str"), 21);
    }

    /* ---------------------- when it counts ---------------------- */

    #[test]
    fn a_sword_in_a_backpack_grants_nothing() {
        let gs = vec![g("attack", Mode::Add, 1)];
        assert!(live(gs, false, false).is_empty());
    }

    #[test]
    fn attunement_gates_only_what_asks_for_it() {
        let mut needs = g("str", Mode::Set, 19);
        needs.needs_attunement = true;
        let gs = vec![needs, g("ac", Mode::Add, 1)];

        let unattuned = live(gs.clone(), true, false);
        assert_eq!(unattuned.len(), 1);
        assert_eq!(unattuned[0].target, "ac", "the cloak still works");

        assert_eq!(live(gs, true, true).len(), 2);
    }

    /* ---------------- showing the working ---------------- */

    #[test]
    fn a_bonus_is_listed_against_whatever_gave_it() {
        let gs = vec![Grant { source: "Pipes".into(), ..g("skill.prf", Mode::Add, 1) }];
        assert_eq!(breakdown(1, &gs, "skill.prf"), vec![("Pipes".to_string(), 1)]);
    }

    /// THE STEPS SUM TO THE DIFFERENCE. Whatever is listed has to
    /// account for the whole of the change, or the working is a
    /// decoration rather than an explanation.
    #[test]
    fn the_steps_add_up_to_what_apply_returns() {
        let gs = vec![
            Grant { source: "Gauntlets".into(), ..g("str", Mode::Set, 19) },
            Grant { source: "Belt".into(), ..g("str", Mode::Add, 2) },
        ];
        for natural in [8, 13, 19, 20, 22] {
            let shown: i64 = breakdown(natural, &gs, "str").iter().map(|(_, v)| v).sum();
            assert_eq!(natural + shown, apply(natural, &gs, "str"), "from {}", natural);
        }
    }

    #[test]
    fn a_floor_reads_as_the_distance_it_lifted_you() {
        let gs = vec![Grant { source: "Gauntlets".into(), ..g("str", Mode::Set, 19) }];
        assert_eq!(breakdown(10, &gs, "str"), vec![("Gauntlets".to_string(), 9)]);
    }

    /// An item that changed nothing must not appear in the sum. A
    /// barbarian at 20 wearing gauntlets that set 19 is not "+0 from
    /// the gauntlets" - they are wearing jewellery.
    #[test]
    fn a_floor_that_lifted_nothing_is_not_mentioned() {
        let gs = vec![Grant { source: "Gauntlets".into(), ..g("str", Mode::Set, 19) }];
        assert!(breakdown(20, &gs, "str").is_empty());
    }

    #[test]
    fn nothing_granted_shows_nothing() {
        assert!(breakdown(5, &[], "skill.prf").is_empty());
    }

    /* ---------------- the vocabulary ---------------- */

    #[test]
    fn every_target_a_consumer_reads_is_known() {
        for t in ["ac", "attack", "damage", "save", "str", "dex", "cha"] {
            assert!(known_target(t), "{}", t);
        }
        assert!(known_target("save.dex"));
        assert!(known_target("skill.ste"));
    }

    /// THE SILENT NO-OP THIS EXISTS TO STOP. Every one of these parses
    /// and stores perfectly and would never be read by anything - an
    /// item that simply is not magic, with nobody able to see why.
    #[test]
    fn a_target_nothing_reads_is_refused() {
        for t in [
            "armour",      // the other spelling
            "hp",          // a reasonable guess at a thing we do not do
            "save.luck",   // not an ability
            "skill.",      // a prefix with nothing after it
            "attack.melee" // finer than anything resolves
        ] {
            assert!(!known_target(t), "{} should not be accepted", t);
        }
    }

    #[test]
    fn a_skill_target_hands_back_its_key_to_be_checked() {
        assert_eq!(skill_key("skill.ste"), Some("ste"));
        assert_eq!(skill_key("ac"), None);
    }

    #[test]
    fn the_working_can_be_shown_rather_than_the_total() {
        let gs = vec![
            Grant { source: "Ring of Protection".into(), ..g("ac", Mode::Add, 1) },
            Grant { source: "+1 Shield".into(), ..g("ac", Mode::Add, 1) },
            g("attack", Mode::Add, 1),
        ];
        let shown = sources_for(&gs, "ac");
        assert_eq!(shown.len(), 2);
        assert_eq!(shown[0].source, "Ring of Protection");
    }
}
