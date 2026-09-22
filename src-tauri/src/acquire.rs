//! Taking something that is not yours.
//!
//! FIVE NAMES, TWO MECHANISMS. Take, Pick Pocket, Buy, Sell and Give
//! are all one event - an object changes holder - and they differ only
//! in what has to be true first. Two of them settle it with a roll and
//! three of them settle it with consent or a price, so this file is the
//! contested half. `commands/inventory.rs` already holds the consented
//! half: `give_item` and `take_object` are the transfer, and nothing
//! here does the moving.
//!
//! THE SHAPE IS `swing`, NOT AN INVENTORY EDIT. A take is a roll, a
//! resolution and a consequence, and it belongs in the action log the
//! same way an attack does. That is why this file is rules and holds no
//! network: it answers what happened, and something else writes it down.
//!
//! ---------------------------------------------------------------------
//! THE RULE, as the table plays it
//!
//!   A dead or unconscious holder gets nothing. No notice, no struggle,
//!   the object changes hands. Looting a body is not a contest.
//!
//!   Otherwise the holder rolls WISDOM TO NOTICE. That save does not
//!   stop the take - it only decides whether they see it happening. Miss
//!   it and the object is simply gone.
//!
//!   Notice it and it becomes DEXTERITY AGAINST DEXTERITY: one hand on
//!   the thing and one hand keeping it. A tie leaves the situation as it
//!   was, which is 5e's rule for every contest and means the holder
//!   keeps it.
//!
//! WHY NOTICE IS NOT THE SAME QUESTION AS SUCCESS. It is the difference
//! between a rule about stealth and a rule about strength, and folding
//! them into one roll would lose the case that makes the whole thing
//! worth having: taking something cleanly from someone who never knew.
//!
//! PICK POCKET IS THE THINNER READING and is marked as such. One roll
//! decides both - lift it against the holder's Wisdom, and failing means
//! they felt it and you did not get it. That is 5e's Sleight of Hand and
//! it is one line of the spec rather than four, so it is built to the
//! line and no further.

// RULES AHEAD OF THEIR PLUMBING, and deliberately. Nothing calls this
// yet: a player-initiated take cannot be a REST write, because the
// `objects` policies reach a campaign through the holder's owner_uid
// and a player has no claim on someone else's gear. It needs a
// SECURITY DEFINER function that runs the contest and the transfer
// together, the same shape `write_action` uses.
//
// The rule is settled and tested first anyway, because the rule is the
// part that was specified and the part worth getting right before a
// migration hardens a shape around it.
#![allow(dead_code)]

use crate::death::Condition;

/* ============================ TYPES ============================ */

/// How someone is going about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// Overt. Notice and success are separate questions.
    Take,
    /// Covert. One roll answers both.
    PickPocket,
}

/// How it ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Nobody was in a position to object. The holder is dead or
    /// unconscious and this is looting, not stealing.
    Unopposed,
    /// They never noticed. The object changes hands and the holder does
    /// not know it until they reach for it.
    Unnoticed,
    /// They noticed and lost the scuffle anyway.
    Wrested,
    /// They noticed and kept hold of it.
    Kept,
}

impl Outcome {
    /// Did the object actually move?
    ///
    /// The one question every caller has, and the reason it is a method
    /// rather than a match at each call site: three of four outcomes
    /// succeed, and an inverted check would be easy to write and hard
    /// to see.
    pub fn took_it(self) -> bool {
        !matches!(self, Outcome::Kept)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Unopposed => "unopposed",
            Outcome::Unnoticed => "unnoticed",
            Outcome::Wrested => "wrested",
            Outcome::Kept => "kept",
        }
    }
}

/// What the attempt needs next, or how it ended.
///
/// A STATE MACHINE BECAUSE THE DICE ARE ELSEWHERE. Each roll happens on
/// the device and comes back, so this cannot be one function that asks
/// for everything up front - half the inputs do not exist until an
/// earlier step has answered. Returning the next question is what lets
/// the rule stay pure while the rolls happen outside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// A trap on the body or the container fires before anything else.
    ///
    /// NOTHING SETS THIS YET. There is no trap subsystem - no table, no
    /// column, nothing - so `trapped` is always false at every live call
    /// site today. The step is here because the rule was specified and
    /// the ORDER is the part worth not losing: the trap answers first,
    /// before anyone notices anything.
    SpringsTrap,
    /// The holder rolls Wisdom against this, to notice.
    NeedsNotice { dc: i64 },
    /// They noticed. Dexterity against Dexterity.
    NeedsContest,
    /// Nothing left to roll.
    Settled(Outcome),
}

/* ============================ RULES ============================ */

/// Whether a holder is in any condition to object.
///
/// Conscious or nothing. A creature at zero hit points is not making a
/// Wisdom save and is not keeping hold of anything, and 5e does not ask
/// it to - which is also the case that makes looting work at all.
pub fn may_resist(holder: Condition) -> bool {
    holder.is_conscious()
}

/// What the holder's Wisdom save is against.
///
/// 10 PLUS THE TAKER'S DEXTERITY, which is the passive shape 5e uses
/// everywhere a roll is opposed by something that is not rolling. The
/// taker makes no roll to take - that was the spec, and it is what
/// makes a Take different from a Pick Pocket - so the difficulty has to
/// come from somewhere, and their Dexterity is the fact in play.
///
/// A DM SETTING ONE OVERRIDES THIS. Every other difficulty in this app
/// is the DM's to state - see `add_challenge` - and this is the default
/// for when nobody has.
pub fn notice_dc(taker_dex_mod: i64) -> i64 {
    10 + taker_dex_mod
}

/// Who wins a contest.
///
/// A TIE LEAVES THINGS AS THEY WERE, so the holder keeps it. That is
/// 5e's rule for every contest and it is the one people reach for a
/// coin over: the roll decides a CHANGE, and a tie is the absence of
/// one.
pub fn taker_wins(taker_total: i64, holder_total: i64) -> bool {
    taker_total > holder_total
}

/// Begin an attempt.
///
/// `trapped` is the body or the container, not the creature - and it is
/// always false today, because nothing in this schema can say otherwise.
pub fn begin(
    method: Method,
    holder: Condition,
    taker_dex_mod: i64,
    trapped: bool,
) -> Step {
    if trapped {
        return Step::SpringsTrap;
    }
    if !may_resist(holder) {
        // Looting. Neither method asks a corpse for a saving throw.
        return Step::Settled(Outcome::Unopposed);
    }
    // BOTH METHODS START THE SAME WAY, and the method is not consulted
    // here on purpose. Either way the first question is whether the
    // holder notices; they part at `after_notice`, over what a yes
    // MEANS. A match with two identical arms would suggest the fork is
    // here and it is not.
    let _ = method;
    Step::NeedsNotice {
        dc: notice_dc(taker_dex_mod),
    }
}

/// Continue after the trap has been dealt with.
///
/// Separate from `begin` so a trap cannot be skipped by passing false a
/// second time: the caller that sprang one comes back HERE, and the
/// attempt carries on from where it was.
pub fn after_trap(method: Method, holder: Condition, taker_dex_mod: i64) -> Step {
    begin(method, holder, taker_dex_mod, false)
}

/// Continue once the holder has rolled to notice.
///
/// THE TWO METHODS PART HERE, and this is the whole difference between
/// them. A Take that goes unnoticed has already succeeded and a Take
/// that is noticed becomes a struggle. A Pick Pocket that is noticed is
/// simply over - there is no second chance to wrestle a purse out of
/// somebody's hand once they have felt you reach for it.
pub fn after_notice(method: Method, noticed: bool) -> Step {
    match (method, noticed) {
        (_, false) => Step::Settled(Outcome::Unnoticed),
        (Method::Take, true) => Step::NeedsContest,
        (Method::PickPocket, true) => Step::Settled(Outcome::Kept),
    }
}

/// Continue once Dexterity has been rolled against Dexterity.
pub fn after_contest(taker_total: i64, holder_total: i64) -> Step {
    Step::Settled(if taker_wins(taker_total, holder_total) {
        Outcome::Wrested
    } else {
        Outcome::Kept
    })
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    fn settled(s: Step) -> Outcome {
        match s {
            Step::Settled(o) => o,
            other => panic!("expected a settled attempt, got {:?}", other),
        }
    }

    #[test]
    fn looting_a_body_asks_nobody_anything() {
        for holder in [Condition::Dead, Condition::Down, Condition::Stable] {
            for method in [Method::Take, Method::PickPocket] {
                assert_eq!(
                    settled(begin(method, holder, 0, false)),
                    Outcome::Unopposed,
                    "{:?} against {:?}",
                    method,
                    holder
                );
            }
        }
    }

    #[test]
    fn a_conscious_holder_gets_a_chance_to_notice() {
        assert_eq!(
            begin(Method::Take, Condition::Conscious, 3, false),
            Step::NeedsNotice { dc: 13 }
        );
    }

    #[test]
    fn the_notice_save_does_not_stop_the_take() {
        // The spec's own distinction: failing to notice does not mean
        // failing to resist, it means the object is already gone.
        let out = settled(after_notice(Method::Take, false));
        assert_eq!(out, Outcome::Unnoticed);
        assert!(out.took_it());
    }

    #[test]
    fn noticing_a_take_turns_it_into_a_struggle() {
        assert_eq!(after_notice(Method::Take, true), Step::NeedsContest);
    }

    #[test]
    fn noticing_a_pick_pocket_ends_it() {
        // No second chance once they have felt you. This is where the
        // two methods differ and the only place they do.
        let out = settled(after_notice(Method::PickPocket, true));
        assert_eq!(out, Outcome::Kept);
        assert!(!out.took_it());
    }

    #[test]
    fn a_contest_goes_to_the_higher_roll() {
        assert_eq!(settled(after_contest(18, 12)), Outcome::Wrested);
        assert_eq!(settled(after_contest(12, 18)), Outcome::Kept);
    }

    #[test]
    fn a_tie_leaves_the_situation_as_it_was() {
        // 5e's rule for every contest, and the one people reach for a
        // coin over. The holder keeps it.
        assert_eq!(settled(after_contest(14, 14)), Outcome::Kept);
        assert!(!taker_wins(14, 14));
    }

    #[test]
    fn a_trap_answers_before_anything_else() {
        // Even against a corpse: the body is trapped whether or not it
        // can object to being searched, and the order is the part of
        // this that is worth not losing.
        assert_eq!(
            begin(Method::Take, Condition::Dead, 0, true),
            Step::SpringsTrap
        );
        assert_eq!(
            begin(Method::Take, Condition::Conscious, 0, true),
            Step::SpringsTrap
        );
    }

    #[test]
    fn the_attempt_carries_on_after_the_trap() {
        assert_eq!(
            after_trap(Method::Take, Condition::Conscious, 2),
            Step::NeedsNotice { dc: 12 }
        );
        assert_eq!(
            settled(after_trap(Method::Take, Condition::Dead, 2)),
            Outcome::Unopposed
        );
    }

    #[test]
    fn a_clumsy_taker_is_easier_to_spot() {
        assert_eq!(notice_dc(-1), 9);
        assert_eq!(notice_dc(0), 10);
        assert_eq!(notice_dc(5), 15);
    }

    #[test]
    fn only_keeping_it_means_the_object_did_not_move() {
        assert!(Outcome::Unopposed.took_it());
        assert!(Outcome::Unnoticed.took_it());
        assert!(Outcome::Wrested.took_it());
        assert!(!Outcome::Kept.took_it());
    }
}
