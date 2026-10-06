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
    /// What it spent - 061. `attack`, `action`, and the three the app
    /// cannot yet write: `bonus`, `reaction`, `free`.
    ///
    /// ABSENT FALLS BACK TO THE KEY, which is the same rule 061's
    /// trigger applies. The column is stamped on insert and backfilled,
    /// so nothing in the database is missing one - but a caller
    /// building an Act by hand should not have to know that, and a
    /// silent None would quietly stop counting somebody's swings.
    pub cost: Option<String>,
    /// 149. WHOSE TURN IT WAS WHEN THIS HAPPENED - 060's stamp, read
    /// here so the count can tell a creature's own turn from everything
    /// it did outside one. `out_of_turn` is the comparison and it has
    /// been in this module since 054; what is new is that something
    /// now reads it while counting rather than only while painting.
    #[serde(default)]
    pub turn_actor_id: Option<String>,
}

impl Act {
    /// What this cost, stated or derived.
    pub fn cost(&self) -> &str {
        match self.cost.as_deref() {
            Some(c) if !c.is_empty() => c,
            _ if self.key == "attack" => "attack",
            // A COST WORD USED AS A KEY IS THAT COST - the same rule
            // 062's trigger applies, stated here so a fixture built by
            // hand and a row read from the database agree.
            _ if matches!(self.key.as_str(), "bonus" | "reaction" | "free" | "action") => {
                &self.key
            }
            _ => "action",
        }
    }
}

/// WHAT A CREATURE IS OWED IN ONE TURN. 061.
///
/// 054 counted what was spent and called a second action "beyond one
/// turn", and said plainly that Extra Attack would make that flag fire
/// on correct play. It did: Garn is a level 5 Barbarian, he is owed
/// two swings, and every second one he has ever taken has been marked
/// irregular by an app with no way to know better.
///
/// A warning that fires on correct play is worse than no warning,
/// because a DM learns to ignore it and then misses the one that
/// mattered.
///
/// STILL NOT A GATE. 051 decided the order informs and refuses
/// nothing, and a budget is the same kind of thing: it says what is
/// owed so a screen can show two swings as two of two rather than as
/// one too many. Haste, action surge and a legendary action are all
/// still outside what the engine knows, and all still legitimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Budget {
    /// From the class's Extra Attack progression - see
    /// `class::attacks_at`. One for anybody without a class, which is
    /// every monster: a statblock's multiattack is its own thing and
    /// nothing reads it yet.
    pub attacks: i64,
    /// One. The Attack action is an action, and the attacks above are
    /// what that one action buys.
    pub actions: i64,
    /// One each. 062 writes them: the fight card has a tick per slot
    /// and spending one is an action with no dice.
    ///
    /// A REACTION IS ONCE PER ROUND AND THESE ARE COUNTED PER ROUND,
    /// which agrees for every creature the app can currently produce -
    /// each takes one turn per round. It will need revisiting the day
    /// something takes two.
    pub bonus: i64,
    pub reactions: i64,
    /// The free object interaction. One per turn in 5e, and the one a
    /// DM is most likely to wave through - it is here to be SEEN
    /// rather than to be policed.
    pub free: i64,
    /// 149. HOW MANY LEGENDARY ACTIONS A ROUND, which is zero for
    /// almost everything. Thirty creatures in the bestiary have them
    /// and all thirty have three; the number is on the statblock rather
    /// than derived, because there is no rule behind it to derive from.
    ///
    /// ZERO IS THE INTERESTING VALUE, not three. A creature with none
    /// that acts out of turn is doing something a DM should look at,
    /// and that warning is what 054's `beyond_one_turn` was reaching
    /// for before it could tell the two cases apart.
    pub legendary: i64,
}

impl Default for Budget {
    /// What somebody with no class gets: one swing, one action.
    fn default() -> Self {
        Self { attacks: 1, actions: 1, bonus: 1, reactions: 1, free: 1, legendary: 0 }
    }
}

impl Budget {
    pub fn with_attacks(attacks: i64) -> Self {
        Self { attacks: attacks.max(1), ..Self::default() }
    }

    /// 149. And what the statblock says it may do out of turn.
    pub fn with_legendary(self, legendary: i64) -> Self {
        Self { legendary: legendary.max(0), ..self }
    }
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
    /// Everything that spent the ACTION: checks, death saves. The pair
    /// matters because the Attack action buys N attacks, so two swings
    /// are ONE action and two checks are two.
    ///
    /// NOT the other three slots - those have their own, because a
    /// bonus action that counted against the action would make every
    /// Rogue on the screen look over budget.
    pub other: i64,
    /// 062. Spent from the ticks on the fight card.
    pub bonus: i64,
    pub reactions: i64,
    pub free: i64,
    /// 149. WHAT IT DID OUTSIDE ITS OWN TURN, which for the thirty
    /// creatures that have them is legendary actions and for everybody
    /// else is a question.
    ///
    /// A REACTION IS ALSO OUT OF TURN and is not counted here - it has
    /// its own slot and its own allowance, and the cost column is the
    /// only thing that can tell the two apart. Anything else taken on
    /// somebody else's turn lands here.
    pub legendary: i64,
    /// What this creature was owed, so a screen can print "2 of 2"
    /// rather than counting to two and worrying.
    pub budget: Budget,
    /// Past what they are owed - 061 replaced 054's `beyond_one_turn`,
    /// which measured against a flat one and therefore fired on every
    /// Extra Attack ever taken.
    ///
    /// STILL A FLAG, NOT A VERDICT, and the list of things that make
    /// it legitimately true is shorter now but not empty: haste, action
    /// surge, a legendary action, and a DM simply allowing it. It means
    /// "look at this".
    ///
    /// DECIDED HERE, NOT ON THE SCREEN. It is two comparisons, and a
    /// comparison written on a screen is how the frontend came to
    /// disagree with the engine about whether a goblin may heal itself.
    pub over_budget: bool,
}

/* ============================ RULES ============================ */

/// Whether this was taken by somebody other than the creature holding
/// the turn.
///
/// FALSE WHERE THERE WAS NO TURN TO BE OUT OF, and 060 lists the three
/// ways that happens: an action outside an encounter, one taken before
/// the order started, and every action written before the column
/// existed. None of those is a creature jumping the queue, and saying
/// so would put a warning on most of the log the first time a DM
/// opened it.
///
/// It reads two STAMPED ids and compares them. Comparing against
/// whoever holds the turn NOW would answer a different question -
/// the pointer moves, and an action taken properly in round 1 would
/// start reading as out of turn the moment the turn passed.
pub fn out_of_turn(actor_id: Option<&str>, turn_actor_id: Option<&str>) -> bool {
    match (actor_id, turn_actor_id) {
        (Some(who), Some(whose)) => who != whose,
        _ => false,
    }
}

/// What everybody has spent in the given round.
///
/// Sorted by actor so the same input always gives the same output -
/// the screen indexes into it rather than reading it in order, but a
/// function that returns rows in hash order is one that cannot be
/// tested honestly.
pub fn this_round(acts: &[Act], round: i64, budgets: &[(String, Budget)]) -> Vec<Spent> {
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
                // A creature with no budget given gets the default:
                // one swing, one action. That is every monster today -
                // a statblock's multiattack is its own thing and
                // nothing reads it yet - and it is also the honest
                // answer for anybody the caller failed to look up.
                let budget = budgets
                    .iter()
                    .find(|(k, _)| k == id)
                    .map(|(_, b)| *b)
                    .unwrap_or_default();
                out.push(Spent {
                    actor_id: id.to_string(),
                    actions: 0,
                    attacks: 0,
                    other: 0,
                    bonus: 0,
                    reactions: 0,
                    free: 0,
                    legendary: 0,
                    budget,
                    over_budget: false,
                });
                out.last_mut().expect("just pushed")
            }
        };
        slot.actions += 1;

        // 149. OUTSIDE ITS OWN TURN IS ITS OWN TALLY, and this is the
        // part worth reading twice.
        //
        // An action taken on somebody ELSE'S turn does not spend this
        // creature's turn. Counting it against the ordinary budget is
        // what made a dragon taking three legendary actions and then
        // its own turn read as four actions and light up as over
        // budget - a warning firing on correct play, which 061 already
        // had to fix once for Extra Attack.
        //
        // A REACTION IS ALSO OUT OF TURN and keeps its own slot: it is
        // the one out-of-turn thing the cost column can name, so it is
        // the one that can be told apart. Everything else lands in
        // `legendary`, including for a creature that has none - a goblin
        // acting on the wizard's turn is exactly the thing a DM wants
        // flagged, and `budget.legendary` of zero flags it.
        if out_of_turn(Some(id), a.turn_actor_id.as_deref()) {
            match a.cost() {
                "reaction" => slot.reactions += 1,
                _ => slot.legendary += 1,
            }
        } else {
            // COUNTED BY COST, NOT BY KEY, so the day a technique costs
            // a bonus action the count follows the column rather than
            // needing this line edited. `Act::cost` falls back to the
            // key, which is the same rule 061's trigger applies.
            // EACH SLOT ON ITS OWN TALLY. Before 062 everything that
            // was not a swing landed in `other`, which was right while
            // nothing could write the other three and would have made
            // one bonus action read as a spent turn the moment
            // something could.
            match a.cost() {
                "attack" => slot.attacks += 1,
                "bonus" => slot.bonus += 1,
                "reaction" => slot.reactions += 1,
                "free" => slot.free += 1,
                _ => slot.other += 1,
            }
        }
        // THE ATTACK ACTION IS AN ACTION. That is the part worth
        // stating: swinging N times costs ONE action however large N
        // is, so a Fighter's four swings are one action - but swinging
        // at all and then making a check is TWO, because the check
        // needs the action the Attack already spent.
        //
        // A pre-existing test caught this. The first version of the
        // rule gave attacks and checks separate pools, which let a
        // level 1 character swing and then make a check inside one
        // turn. `a_check_is_an_action_too` has asserted otherwise
        // since 054 and was right.
        let slots = i64::from(slot.attacks > 0) + slot.other;
        slot.over_budget = slot.attacks > slot.budget.attacks
            || slots > slot.budget.actions
            || slot.bonus > slot.budget.bonus
            || slot.reactions > slot.budget.reactions
            || slot.free > slot.budget.free
            // 149. And spending more out of turn than the statblock
            // allows - which for almost everything is any at all.
            || slot.legendary > slot.budget.legendary;
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
        one_with(acts, round, actor_id, &[])
    }

    /// The same, for a creature that is owed more than the default.
    fn one_with(acts: &[Act], round: i64, actor_id: &str, budgets: &[(String, Budget)]) -> Spent {
        this_round(acts, round, budgets)
            .into_iter()
            .find(|s| s.actor_id == actor_id)
            .unwrap_or(Spent {
                actor_id: actor_id.to_string(),
                actions: 0,
                attacks: 0,
                other: 0,
                bonus: 0,
                reactions: 0,
                free: 0,
                legendary: 0,
                budget: Budget::default(),
                over_budget: false,
            })
    }

    /// The same act, taken while somebody else held the turn.
    fn out(actor: &str, key: &str, round: i64, whose_turn: &str) -> Act {
        Act { turn_actor_id: Some(whose_turn.to_string()), ..act(Some(actor), key, Some(round)) }
    }

    /* ------------- legendary actions (149) -------------------------- */

    #[test]
    fn a_dragons_three_legendary_actions_are_three_of_three() {
        let b = vec![("drag".to_string(), Budget::default().with_legendary(3))];
        let acts = vec![
            out("drag", "attack", 1, "falon"),
            out("drag", "attack", 1, "luci"),
            out("drag", "attack", 1, "webbys"),
        ];
        let s = one_with(&acts, 1, "drag", &b);
        assert_eq!(s.legendary, 3);
        assert_eq!(s.attacks, 0, "none of them spent its own turn");
        assert!(!s.over_budget, "three of three is correct play");
    }

    #[test]
    fn its_own_turn_is_still_its_own_turn() {
        // The whole reason out-of-turn gets its own tally: a dragon
        // that spends three legendary actions AND takes its turn is
        // doing exactly what the book says, and used to read as four
        // actions and light up.
        let b = vec![("drag".to_string(), Budget::default().with_legendary(3))];
        let acts = vec![
            out("drag", "attack", 1, "falon"),
            out("drag", "attack", 1, "luci"),
            out("drag", "attack", 1, "webbys"),
            act(Some("drag"), "attack", Some(1)),
        ];
        let s = one_with(&acts, 1, "drag", &b);
        assert_eq!(s.legendary, 3);
        assert_eq!(s.attacks, 1, "the swing on its own turn");
        assert_eq!(s.actions, 4, "four things happened and the log says so");
        assert!(!s.over_budget);
    }

    #[test]
    fn a_fourth_legendary_action_is_over_budget() {
        let b = vec![("drag".to_string(), Budget::default().with_legendary(3))];
        let acts: Vec<Act> = (0..4).map(|i| out("drag", "attack", 1, &format!("pc{}", i))).collect();
        let s = one_with(&acts, 1, "drag", &b);
        assert_eq!(s.legendary, 4);
        assert!(s.over_budget);
    }

    #[test]
    fn a_goblin_acting_on_somebody_elses_turn_is_flagged() {
        // Zero is the interesting value. Almost nothing has legendary
        // actions, so any out-of-turn action by almost anything is the
        // thing a DM wants to see.
        let s = one(&[out("gob", "attack", 1, "falon")], 1, "gob");
        assert_eq!(s.legendary, 1);
        assert_eq!(s.budget.legendary, 0);
        assert!(s.over_budget);
    }

    #[test]
    fn a_reaction_out_of_turn_is_a_reaction_and_not_legendary() {
        // The one out-of-turn thing the cost column can name. An
        // opportunity attack must not read as a legendary action.
        let mut a = out("gob", "attack", 1, "falon");
        a.cost = Some("reaction".to_string());
        let s = one(&[a], 1, "gob");
        assert_eq!(s.reactions, 1);
        assert_eq!(s.legendary, 0);
        assert!(!s.over_budget, "one reaction is what everybody is owed");
    }

    #[test]
    fn an_unstamped_turn_is_not_out_of_turn() {
        // 060's three ways to have no stamp - outside an encounter,
        // before the order started, written before the column existed.
        // None of them is a creature jumping the queue.
        let mut a = act(Some("gob"), "attack", Some(1));
        a.turn_actor_id = None;
        let s = one(&[a], 1, "gob");
        assert_eq!(s.legendary, 0);
        assert_eq!(s.attacks, 1);
        assert!(!s.over_budget);
    }

    #[test]
    fn the_allowance_cannot_be_negative() {
        assert_eq!(Budget::default().with_legendary(-3).legendary, 0);
    }

    fn act(actor: Option<&str>, key: &str, round: Option<i64>) -> Act {
        Act {
            actor_id: actor.map(String::from),
            key: key.to_string(),
            round,
            // Left unstated on purpose: `Act::cost` falls back to the
            // key, which is the same rule 061's trigger applies, and
            // these fixtures exercise that fallback rather than
            // restating the column.
            cost: None,
            // IN TURN BY DEFAULT. Every fixture written before 149 was
            // about a creature on its own turn, and saying so keeps
            // them testing what they were written to test.
            turn_actor_id: actor.map(String::from),
        }
    }

    /// A creature owed more than the default, for the tests that are
    /// about Extra Attack.
    fn owed(actor: &str, attacks: i64) -> Vec<(String, Budget)> {
        vec![(actor.to_string(), Budget::with_attacks(attacks))]
    }

    #[test]
    fn nothing_spent_is_zero_not_absent() {
        let s = one(&[], 1, "a");
        assert_eq!(s.actions, 0);
        assert_eq!(s.attacks, 0);
        assert!(!s.over_budget);
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
        // Owed one, took two - worth a look.
        assert!(s.over_budget);
    }

    /* ------------------- what the budget changes ------------------- */

    // GARN. A level 5 Barbarian is owed two swings, and before 061
    // every second one he took was flagged as irregular by an app with
    // no way to know better.
    #[test]
    fn a_second_swing_is_owed_to_somebody_with_extra_attack() {
        let acts = vec![
            act(Some("a"), "attack", Some(1)),
            act(Some("a"), "attack", Some(1)),
        ];
        let s = one_with(&acts, 1, "a", &owed("a", 2));
        assert_eq!(s.attacks, 2);
        assert_eq!(s.budget.attacks, 2);
        assert!(!s.over_budget, "two of two is not over");
    }

    #[test]
    fn and_a_third_is_not() {
        let acts = vec![
            act(Some("a"), "attack", Some(1)),
            act(Some("a"), "attack", Some(1)),
            act(Some("a"), "attack", Some(1)),
        ];
        let s = one_with(&acts, 1, "a", &owed("a", 2));
        assert_eq!(s.attacks, 3);
        assert!(s.over_budget);
    }

    // THE PAIR IS THE POINT. The Attack action buys N swings, so two
    // swings are one action - but two CHECKS are two actions, and the
    // budget for those is one however many attacks you get.
    #[test]
    fn two_checks_are_over_even_for_a_fighter() {
        let acts = vec![
            act(Some("a"), "prc", Some(1)),
            act(Some("a"), "ins", Some(1)),
        ];
        let s = one_with(&acts, 1, "a", &owed("a", 4));
        assert_eq!(s.other, 2);
        assert_eq!(s.attacks, 0);
        assert!(s.over_budget, "four attacks buys no extra checks");
    }

    #[test]
    fn attacks_and_a_check_are_counted_apart() {
        let acts = vec![
            act(Some("a"), "attack", Some(1)),
            act(Some("a"), "attack", Some(1)),
            act(Some("a"), "prc", Some(1)),
        ];
        let s = one_with(&acts, 1, "a", &owed("a", 2));
        assert_eq!(s.attacks, 2);
        assert_eq!(s.other, 1);
        assert_eq!(s.actions, 3, "the total is still every row");
        // AND IT IS OVER, because the Attack action already spent the
        // action the check wants. Both swings were owed; the check is
        // the one that does not fit.
        assert!(s.over_budget);
    }

    // The whole of Extra Attack inside one action, which is the case
    // the separate-pools version got wrong in the other direction.
    #[test]
    fn four_swings_are_still_one_action() {
        let acts: Vec<Act> = (0..4)
            .map(|_| act(Some("a"), "attack", Some(1)))
            .collect();
        let s = one_with(&acts, 1, "a", &owed("a", 4));
        assert_eq!(s.attacks, 4);
        assert_eq!(s.other, 0);
        assert!(!s.over_budget, "a level 20 Fighter, doing exactly their job");
    }

    // A death save IS the turn of a creature that is dying - 015 - so
    // it costs the action rather than being free.
    #[test]
    fn a_death_save_spends_the_action() {
        let acts = vec![act(Some("a"), "death", Some(1))];
        let s = one(&acts, 1, "a");
        assert_eq!(s.other, 1);
        assert_eq!(s.attacks, 0);
        assert!(!s.over_budget);
    }

    // The column is what counts; the key is only the fallback. A
    // technique that one day costs a bonus action must not be counted
    // as a swing because its key still says attack.
    #[test]
    fn a_stated_cost_beats_the_key() {
        let mut a = act(Some("a"), "attack", Some(1));
        a.cost = Some("bonus".into());
        let s = one(&[a], 1, "a");
        assert_eq!(s.attacks, 0, "not a swing - the column said so");
        assert_eq!(s.bonus, 1, "and it lands in its own slot, not in other");
        assert_eq!(s.other, 0);
    }

    /* --------------------- the other three (062) --------------------- */

    // A cost word used as a KEY is that cost, which is how 062's
    // markers are written and what its trigger does.
    #[test]
    fn a_marker_keyed_by_its_cost_needs_no_stated_cost() {
        for word in ["bonus", "reaction", "free"] {
            let s = one(&[act(Some("a"), word, Some(1))], 1, "a");
            let got = match word {
                "bonus" => s.bonus,
                "reaction" => s.reactions,
                _ => s.free,
            };
            assert_eq!(got, 1, "{} did not land in its slot", word);
            assert_eq!(s.other, 0, "{} leaked into the action slot", word);
            assert!(!s.over_budget, "one {} is exactly one", word);
        }
    }

    // THE POINT OF SEPARATE TALLIES. A Rogue who attacks and then
    // Cunning Actions has spent an action and a bonus action, which is
    // one turn played correctly - and would have read as two actions
    // and a warning before 062.
    #[test]
    fn an_attack_and_a_bonus_action_is_one_ordinary_turn() {
        let acts = vec![
            act(Some("a"), "attack", Some(1)),
            act(Some("a"), "bonus", Some(1)),
        ];
        let s = one(&acts, 1, "a");
        assert_eq!(s.attacks, 1);
        assert_eq!(s.bonus, 1);
        assert_eq!(s.other, 0);
        assert!(!s.over_budget);
    }

    #[test]
    fn a_second_bonus_action_is_over() {
        let acts = vec![
            act(Some("a"), "bonus", Some(1)),
            act(Some("a"), "bonus", Some(1)),
        ];
        let s = one(&acts, 1, "a");
        assert_eq!(s.bonus, 2);
        assert!(s.over_budget);
    }

    // Each slot is checked against its OWN budget - a Fighter owed four
    // swings is owed one reaction like everybody else.
    #[test]
    fn extra_attacks_buy_no_extra_reactions() {
        let acts = vec![
            act(Some("a"), "reaction", Some(1)),
            act(Some("a"), "reaction", Some(1)),
        ];
        let s = one_with(&acts, 1, "a", &owed("a", 4));
        assert_eq!(s.budget.attacks, 4);
        assert_eq!(s.reactions, 2);
        assert!(s.over_budget);
    }

    // A whole legal turn for somebody with everything: two swings, a
    // bonus action, a reaction and a free interaction.
    #[test]
    fn a_full_legal_turn_is_not_over_budget() {
        let acts = vec![
            act(Some("a"), "attack", Some(1)),
            act(Some("a"), "attack", Some(1)),
            act(Some("a"), "bonus", Some(1)),
            act(Some("a"), "reaction", Some(1)),
            act(Some("a"), "free", Some(1)),
        ];
        let s = one_with(&acts, 1, "a", &owed("a", 2));
        assert_eq!(s.actions, 5, "five rows");
        assert!(!s.over_budget, "and every one of them owed");
    }

    #[test]
    fn an_unbudgeted_creature_gets_one_swing() {
        let acts = vec![act(Some("goblin"), "attack", Some(1))];
        let s = one(&acts, 1, "goblin");
        assert_eq!(s.budget.attacks, 1);
        assert!(!s.over_budget);
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
        assert!(this_round(&acts, 1, &[]).is_empty());
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
        assert!(s.over_budget);
    }

    /// Somebody not enrolled can still roll, and the log keeps it. It
    /// just cannot be shown against a roster row that does not exist.
    #[test]
    fn an_action_by_nobody_in_the_fight_counts_against_nobody() {
        let acts = vec![
            act(None, "attack", Some(1)),
            act(Some("a"), "attack", Some(1)),
        ];
        let all = this_round(&acts, 1, &[]);
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
        let all = this_round(&acts, 1, &[]);
        assert_eq!(all.len(), 2);
        // Sorted, so this is stable rather than incidental.
        assert_eq!(all[0].actor_id, "a");
        assert_eq!(all[0].actions, 1);
        assert_eq!(all[1].actor_id, "b");
        assert_eq!(all[1].actions, 2);
    }

    /* ---------------------- out of turn ---------------------- */

    #[test]
    fn the_creature_whose_turn_it_is_is_in_turn() {
        assert!(!out_of_turn(Some("a"), Some("a")));
    }

    #[test]
    fn anybody_else_is_out_of_turn() {
        assert!(out_of_turn(Some("b"), Some("a")));
    }

    /// 060: three ways to have no turn, and none of them is jumping
    /// the queue. Every action written before that column existed
    /// lands here, so getting this wrong would put a warning on most
    /// of the log the first time a DM opened it.
    #[test]
    fn no_turn_is_not_a_turn_somebody_took_early() {
        assert!(!out_of_turn(Some("a"), None), "before the order started");
        assert!(!out_of_turn(None, Some("a")), "nobody in the fight swung it");
        assert!(!out_of_turn(None, None), "outside an encounter entirely");
    }

    #[test]
    fn one_action_is_not_beyond_a_turn() {
        let acts = vec![act(Some("a"), "attack", Some(1))];
        assert!(!one(&acts, 1, "a").over_budget);
    }
}
