//! Did it succeed?
//!
//! One module because the rule combining the two verdicts has to live in
//! one place. A roll carries a FACE verdict - crit, fumble or normal,
//! decided by the raw d20 against whatever thresholds were in force -
//! and, when it has a target, a MARGIN verdict, decided by the total
//! against that number. They are different axes and they can disagree.
//!
//! Getting that wrong is the classic bug: a natural 20 that gets
//! reported as a miss because the total came up short, or a natural 1
//! recorded as a hit because the modifier was large. Both are correct
//! arithmetic and wrong rules, and both are invisible in the output -
//! the number looks right.
//!
//! AN ATTACK AND A CHECK ARE NOT THE SAME HERE, and this is the only
//! place they differ. In 5e a natural 20 on an ATTACK hits whatever the
//! AC is, and a natural 1 misses whatever it is. Neither rule applies to
//! an ability check or a save: a natural 20 on Stealth against DC 25 is
//! a 20, and it fails. That is rules-as-written and it is what this
//! implements. If the campaign wants naturals to decide checks too, this
//! is the one function to change - see `auto_decides`.

use crate::dice::Outcome;

/// What a roll was trying to beat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// The number to meet or beat. Meeting it succeeds: AC 15 on a total
    /// of 15 is a hit, which is 5e and is also what every table expects.
    pub value: i64,
    pub kind: TargetKind,
    /// What it was, in words, for the record and for the narrator.
    pub label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    /// An opponent's armour class. Naturals decide it outright.
    Ac,
    /// A difficulty class - a check or a save. Naturals do not.
    Dc,
}

impl TargetKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            TargetKind::Ac => "ac",
            TargetKind::Dc => "dc",
        }
    }

    pub fn parse(s: &str) -> Option<TargetKind> {
        match s.trim().to_lowercase().as_str() {
            "ac" => Some(TargetKind::Ac),
            "dc" => Some(TargetKind::Dc),
            _ => None,
        }
    }

    /// Whether a crit or fumble on the face settles this kind of target
    /// without reference to the total.
    ///
    /// The single line this whole module exists to isolate.
    fn auto_decides(&self) -> bool {
        matches!(self, TargetKind::Ac)
    }
}

/// The verdict, and why it came out that way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    pub success: bool,
    /// `total - target`. Kept even when the face decided it, because
    /// "hit on a natural 20, three under the AC" is a fact worth having.
    /// Negative on a successful auto-hit is correct, not a bug.
    pub margin: i64,
    pub reason: Reason,
}

/// Why the verdict is what it is. Exists so a card can say "natural 20"
/// instead of leaving a player to wonder how 14 beat an 18, and so a
/// narrator is told what actually happened rather than inferring it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// A crit on an attack. Hits regardless of the total.
    AutoHit,
    /// A fumble on an attack. Misses regardless of the total.
    AutoMiss,
    /// The total met the target.
    Met,
    /// The total did not.
    Missed,
}

impl Reason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Reason::AutoHit => "auto_hit",
            Reason::AutoMiss => "auto_miss",
            Reason::Met => "met",
            Reason::Missed => "missed",
        }
    }
}

/// Settle a roll against its target.
///
/// `face` is None when no single d20 was rolled - a damage roll, or
/// anything with two d20 terms. A roll with no face is settled on the
/// total alone, which is right: the auto rules need a face to apply.
pub fn resolve(total: i64, face: Option<Outcome>, target: &Target) -> Resolution {
    let margin = total - target.value;

    if target.kind.auto_decides() {
        match face {
            Some(Outcome::Crit) => {
                return Resolution { success: true, margin, reason: Reason::AutoHit }
            }
            Some(Outcome::Fumble) => {
                return Resolution { success: false, margin, reason: Reason::AutoMiss }
            }
            _ => {}
        }
    }

    if margin >= 0 {
        Resolution { success: true, margin, reason: Reason::Met }
    } else {
        Resolution { success: false, margin, reason: Reason::Missed }
    }
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    fn ac(value: i64) -> Target {
        Target { value, kind: TargetKind::Ac, label: Some("Goblin 1".to_string()) }
    }
    fn dc(value: i64) -> Target {
        Target { value, kind: TargetKind::Dc, label: None }
    }

    /* ---------------- the ordinary case ----------------------------- */

    #[test]
    fn meeting_the_target_exactly_succeeds() {
        // AC 15 on a total of 15 is a hit. Off-by-one here is a bug
        // nobody notices for months.
        let r = resolve(15, Some(Outcome::Normal), &ac(15));
        assert!(r.success);
        assert_eq!(r.margin, 0);
        assert_eq!(r.reason, Reason::Met);
    }

    #[test]
    fn one_under_fails() {
        let r = resolve(14, Some(Outcome::Normal), &ac(15));
        assert!(!r.success);
        assert_eq!(r.margin, -1);
        assert_eq!(r.reason, Reason::Missed);
    }

    #[test]
    fn margin_records_how_comfortably() {
        assert_eq!(resolve(27, Some(Outcome::Normal), &ac(15)).margin, 12);
        assert_eq!(resolve(3, Some(Outcome::Normal), &ac(15)).margin, -12);
    }

    /* ---------------- the two verdicts disagreeing ------------------ */

    #[test]
    fn a_crit_hits_an_ac_it_did_not_reach() {
        // Natural 20, total still short. It hits, and the margin keeps
        // the fact that it should not have.
        let r = resolve(12, Some(Outcome::Crit), &ac(18));
        assert!(r.success);
        assert_eq!(r.reason, Reason::AutoHit);
        assert_eq!(r.margin, -6);
    }

    #[test]
    fn a_fumble_misses_an_ac_it_cleared() {
        let r = resolve(24, Some(Outcome::Fumble), &ac(10));
        assert!(!r.success);
        assert_eq!(r.reason, Reason::AutoMiss);
        assert_eq!(r.margin, 14);
    }

    #[test]
    fn a_widened_crit_auto_hits_the_same_way() {
        // The face verdict already accounts for a technique's range;
        // this module only asks what the verdict was.
        let r = resolve(11, Some(Outcome::Crit), &ac(19));
        assert!(r.success);
        assert_eq!(r.reason, Reason::AutoHit);
    }

    /* ---------------- checks and saves are different ---------------- */

    #[test]
    fn a_natural_twenty_does_not_carry_a_check() {
        // RAW: a 20 on an ability check is just a 20.
        let r = resolve(22, Some(Outcome::Crit), &dc(25));
        assert!(!r.success);
        assert_eq!(r.reason, Reason::Missed);
    }

    #[test]
    fn a_natural_one_does_not_sink_a_check_that_cleared_it() {
        let r = resolve(11, Some(Outcome::Fumble), &dc(10));
        assert!(r.success);
        assert_eq!(r.reason, Reason::Met);
    }

    #[test]
    fn the_only_difference_between_the_kinds_is_the_auto_rule() {
        // Same numbers, same face, different kind, different answer.
        // If this ever passes with both equal, auto_decides is broken.
        let attack = resolve(12, Some(Outcome::Crit), &ac(18));
        let check = resolve(12, Some(Outcome::Crit), &dc(18));
        assert!(attack.success);
        assert!(!check.success);
    }

    /* ---------------- no face at all -------------------------------- */

    #[test]
    fn a_roll_with_no_face_is_settled_on_the_total() {
        // Two d20 terms, or no d20 at all. The auto rules need a face.
        let r = resolve(20, None, &ac(15));
        assert!(r.success);
        assert_eq!(r.reason, Reason::Met);
    }

    #[test]
    fn a_faceless_roll_that_falls_short_still_fails_plainly() {
        let r = resolve(9, None, &ac(15));
        assert!(!r.success);
        assert_eq!(r.reason, Reason::Missed);
    }

    /* ---------------- vocabulary matches the database --------------- */

    #[test]
    fn kind_strings_match_the_check_constraint() {
        // rolls_target_kind_check admits exactly these.
        assert_eq!(TargetKind::Ac.as_str(), "ac");
        assert_eq!(TargetKind::Dc.as_str(), "dc");
        assert_eq!(TargetKind::parse("AC"), Some(TargetKind::Ac));
        assert_eq!(TargetKind::parse(" dc "), Some(TargetKind::Dc));
        assert_eq!(TargetKind::parse("armour"), None);
    }

    #[test]
    fn every_reason_has_a_distinct_string() {
        let all = [Reason::AutoHit, Reason::AutoMiss, Reason::Met, Reason::Missed];
        let mut seen: Vec<&str> = all.iter().map(|r| r.as_str()).collect();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), all.len());
    }

    #[test]
    fn a_reason_always_agrees_with_its_success_flag() {
        // The pairing is the contract: a card reading the reason must
        // never be able to contradict the flag beside it.
        for (total, face, target) in [
            (12, Some(Outcome::Crit), ac(18)),
            (24, Some(Outcome::Fumble), ac(10)),
            (15, Some(Outcome::Normal), ac(15)),
            (14, Some(Outcome::Normal), ac(15)),
            (22, Some(Outcome::Crit), dc(25)),
            (20, None, ac(15)),
        ] {
            let r = resolve(total, face, &target);
            let implied = matches!(r.reason, Reason::AutoHit | Reason::Met);
            assert_eq!(r.success, implied, "reason and success disagree: {:?}", r);
        }
    }
}
