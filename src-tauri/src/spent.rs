//! What a creature has already done this round.
//!
//! THE RULES ONLY. Reading the actions out of the database is
//! `commands/log.rs`; this counts them, the same division
//! `initiative.rs` and `commands/initiative.rs` have.
//!
//! 012 CALLED THE ACTION THE TURN-SIZED UNIT and it has never been
//! counted. "A miss spends an initiative slot the same as a hit" was
//! written when there were no slots; 051 built the slot and 054 stamps
//! the round onto each action, which is the fact that makes counting
//! possible at all. A timestamp could never answer it - the round
//! advances when a DM presses a button, so two swings a second apart
//! can belong to different rounds.
//!
//! NOTHING HERE REFUSES ANYTHING, and that is the same decision 051
//! took about turn order. A second action in one round is NORMAL at
//! this table: Extra Attack from level 5, a haste spell, an action
//! surge, a legendary action, or a DM simply allowing it. What was
//! missing is the screen being able to SAY a creature has gone twice,
//! not the app being able to stop it.
//!
//! WHAT IS NOT MODELLED, deliberately: 5e's action / bonus action /
//! reaction split. No technique in the catalogue says what it costs,
//! and inventing that here would mean guessing for 193 rows. One
//! honest count beats four fabricated ones - see 054.

use serde::{Deserialize, Serialize};

/* ============================ TYPES ============================ */

/// One thing that was done, as counting needs it.
///
/// A thin read of `actions`: who did it, which round, and what kind of
/// thing it was. Deliberately not the whole row - the log carries the
/// dice and the prose, and this counts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Act {
    /// WHO, as a participant in this fight. None for an action taken by
    /// somebody not enrolled - a shopkeeper rolling Insight in the
    /// middle of a brawl they are not in. Those are real actions and
    /// they belong in the log; they just do not belong to a row of the
    /// roster, so nothing can show them there.
    pub actor_id: Option<String>,
    /// Engine vocabulary: `attack`, `death`, a skill key. The same
    /// vocabulary `rolls.request` resolves to - see 012.
    pub key: String,
    /// NULL is not zero and not "the first". 054: an action outside an
    /// encounter, or one written before the column existed. Neither can
    /// be attributed to a round, so neither is counted in one.
    pub round: Option<i64>,
}

/// What one creature has spent, in one round.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spent {
    pub actor_id: String,
    /// Everything: attacks, checks, death saves. A death save IS the
    /// turn of a creature that is dying, so it counts.
    pub actions: i64,
    /// The subset that were swings. Shown separately because "acted
    /// twice" and "attacked twice" are different things to a DM, and
    /// the second is the one that was asked about.
    pub attacks: i64,
    /// More than one turn's worth, by the plain rule - one action per
    /// turn.
    ///
    /// A FLAG, NOT A VERDICT. Extra Attack, haste, action surge and a
    /// legendary action all make this true and legitimate, and the
    /// engine knows about none of them. It means "look at this", which
    /// is what was being asked for: right now a character can swing
    /// again and again and nothing anywhere says so.
    ///
    /// DECIDED HERE, NOT ON THE SCREEN. It is one comparison, and one
    /// comparison is exactly the kind of thing that ends up written
    /// twice and drifts - the frontend is already the place where a
    /// goblin was stopped from healing itself while the player's roll
    /// box allowed it.
    pub beyond_one_turn: bool,
}

/* ============================ RULES ============================ */

/// What everybody has spent in the given round.
///
/// Sorted by actor so the same input always gives the same output -
/// the screen indexes into it rather than reading it in order, but a
/// function that returns rows in hash order is one that cannot be
/// tested honestly.
pub fn this_round(acts: &[Act], round: i64) -> Vec<Spent> {
    let mut out: Vec<Spent> = Vec::new();

    for a in acts {
        // Three ways not to count, and they are different reasons.
        // A different round, no round at all (054), or an action by
        // somebody who is not a participant.
        if a.round != Some(round) {
            continue;
        }
        let Some(id) = a.actor_id.as_deref() else {
            continue;
        };

        let slot = match out.iter_mut().find(|s| s.actor_id == id) {
            Some(s) => s,
            None => {
                out.push(Spent {
                    actor_id: id.to_string(),
                    actions: 0,
                    attacks: 0,
                    beyond_one_turn: false,
                });
                out.last_mut().expect("just pushed")
            }
        };
        slot.actions += 1;
        if a.key == "attack" {
            slot.attacks += 1;
        }
        slot.beyond_one_turn = slot.actions > 1;
    }

    out.sort_by(|a, b| a.actor_id.cmp(&b.actor_id));
    out
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    /// What one creature spent, for a test that is about one
    /// creature. The library offers the whole round because that is
    /// what the screen paints; picking one out is the test's business.
    fn one(acts: &[Act], round: i64, actor_id: &str) -> Spent {
        this_round(acts, round)
            .into_iter()
            .find(|s| s.actor_id == actor_id)
            .unwrap_or(Spent {
                actor_id: actor_id.to_string(),
                actions: 0,
                attacks: 0,
                beyond_one_turn: false,
            })
    }

    fn act(actor: Option<&str>, key: &str, round: Option<i64>) -> Act {
        Act {
            actor_id: actor.map(String::from),
            key: key.to_string(),
            round,
        }
    }

    #[test]
    fn nothing_spent_is_zero_not_absent() {
        let s = one(&[], 1, "a");
        assert_eq!(s.actions, 0);
        assert_eq!(s.attacks, 0);
        assert!(!s.beyond_one_turn);
    }

    #[test]
    fn two_swings_in_one_round_are_two() {
        let acts = vec![
            act(Some("a"), "attack", Some(1)),
            act(Some("a"), "attack", Some(1)),
        ];
        let s = one(&acts, 1, "a");
        assert_eq!(s.actions, 2);
        assert_eq!(s.attacks, 2);
        assert!(s.beyond_one_turn);
    }

    #[test]
    fn last_round_does_not_count_against_this_one() {
        let acts = vec![
            act(Some("a"), "attack", Some(1)),
            act(Some("a"), "attack", Some(2)),
        ];
        assert_eq!(one(&acts, 1, "a").attacks, 1);
        assert_eq!(one(&acts, 2, "a").attacks, 1);
    }

    /// 054 is explicit that NULL is not zero and not "the first round".
    /// An Insight check in a tavern has no round, and counting it in
    /// one would put a swing in a fight that had not started.
    #[test]
    fn an_action_with_no_round_belongs_to_none_of_them() {
        let acts = vec![act(Some("a"), "attack", None)];
        assert_eq!(one(&acts, 0, "a").actions, 0);
        assert_eq!(one(&acts, 1, "a").actions, 0);
        assert!(this_round(&acts, 1).is_empty());
    }

    /// Round 0 is a real value - 051 uses it for "the order has not
    /// started" - so a swing taken before anybody rolled is countable.
    /// It is the screen's job to decide whether saying so is useful.
    #[test]
    fn round_zero_is_a_round_like_any_other() {
        let acts = vec![act(Some("a"), "attack", Some(0))];
        assert_eq!(one(&acts, 0, "a").attacks, 1);
    }

    /// A death save is the whole turn of a dying creature. It is not a
    /// swing, and both facts have to survive the count.
    #[test]
    fn a_death_save_spends_the_turn_without_being_an_attack() {
        let acts = vec![act(Some("a"), "death", Some(3))];
        let s = one(&acts, 3, "a");
        assert_eq!(s.actions, 1);
        assert_eq!(s.attacks, 0);
    }

    #[test]
    fn a_check_is_an_action_too() {
        let acts = vec![
            act(Some("a"), "ins", Some(1)),
            act(Some("a"), "attack", Some(1)),
        ];
        let s = one(&acts, 1, "a");
        assert_eq!(s.actions, 2);
        assert_eq!(s.attacks, 1);
        assert!(s.beyond_one_turn);
    }

    /// Somebody not enrolled can still roll, and the log keeps it. It
    /// just cannot be shown against a roster row that does not exist.
    #[test]
    fn an_action_by_nobody_in_the_fight_counts_against_nobody() {
        let acts = vec![
            act(None, "attack", Some(1)),
            act(Some("a"), "attack", Some(1)),
        ];
        let all = this_round(&acts, 1);
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].actor_id, "a");
    }

    #[test]
    fn creatures_are_counted_apart() {
        let acts = vec![
            act(Some("b"), "attack", Some(1)),
            act(Some("a"), "attack", Some(1)),
            act(Some("b"), "attack", Some(1)),
        ];
        let all = this_round(&acts, 1);
        assert_eq!(all.len(), 2);
        // Sorted, so this is stable rather than incidental.
        assert_eq!(all[0].actor_id, "a");
        assert_eq!(all[0].actions, 1);
        assert_eq!(all[1].actor_id, "b");
        assert_eq!(all[1].actions, 2);
    }

    #[test]
    fn one_action_is_not_beyond_a_turn() {
        let acts = vec![act(Some("a"), "attack", Some(1))];
        assert!(!one(&acts, 1, "a").beyond_one_turn);
    }
}
