//! Turn order: who goes when, and who is next.
//!
//! THE RULES ONLY. Rolling for it and writing the round are
//! `commands/initiative.rs`; this decides the order and the walk, the
//! same division `containers.rs` and `commands/containers.rs` have.
//!
//! 011 CREATED THE COLUMN AND SAID NOTHING READ IT. Three migrations
//! then deferred to it - 012 built actions as the turn-sized unit, 015
//! left death saves on demand "until initiative arrives". This is the
//! arriving.
//!
//! WHAT IS STORED AND WHAT IS WORKED OUT. 051 stores the round and the
//! current actor, because neither is implied by anything. Everything
//! else is derived here on the way to the screen:
//!
//!   the ORDER      initiative desc, then DEX, then name
//!   WHO IS NEXT    the next creature in that order that can take a turn
//!   THE ROUND      rolls over when the walk wraps
//!
//! A stored ordering is one enrolment away from lying, which is the
//! same argument 033 made about depth and 015 made about the dying
//! condition.
//!
//! NOTHING HERE GATES ANYTHING. A roll made out of turn is allowed, and
//! deliberately: a DM fudges initiative constantly - a surprise round, a
//! held action, somebody who stepped away - and a rig that refuses is a
//! rig they fight. The order is information and the table is the
//! authority. If that ever changes it changes here, where it can be
//! tested.

use serde::{Deserialize, Serialize};

/* ============================ TYPES ============================ */

/// WHERE A HELD CREATURE HAS DECLARED THEY WILL ACT. 063.
///
/// Not 5e's Ready. There is no trigger and no condition text: a hold
/// here is a statement about the ORDER - go after the next one, go
/// after that character, go at the end of the round - which is a thing
/// this app already has, rather than about an event, which is a thing
/// it does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HoldMode {
    /// Let the next creature go, then act. One place later.
    AfterNext,
    /// Act immediately after a named creature.
    AfterActor,
    /// Act last.
    EndOfRound,
}

impl HoldMode {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "after_next" => Some(Self::AfterNext),
            "after_actor" => Some(Self::AfterActor),
            "end_of_round" => Some(Self::EndOfRound),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hold {
    pub mode: HoldMode,
    /// Who they are waiting for. Present only for `AfterActor`, which
    /// 063 enforces with a check constraint rather than leaving to
    /// whoever writes the row.
    pub after: Option<String>,
}

/// One creature in the running, as the order needs it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contender {
    pub id: String,
    pub name: String,
    /// NULL is NOT ZERO - 011 is explicit, and the distinction is the
    /// whole reason the column is nullable. Not yet rolled goes to the
    /// bottom and cannot take a turn; a rolled 0 is a real result and
    /// takes its place above them.
    pub initiative: Option<i64>,
    /// The tiebreak 5e uses. Whatever the DM decides after that is a
    /// conversation, not a column - see 051.
    pub dex: i64,
    /// False for a creature that has fled or been captured. 011 keeps
    /// them enrolled so the DM can bring them back.
    pub active: bool,
    /// Dead is out. DYING IS NOT - a creature at zero still takes its
    /// turn, and spends it on a death save. That is the whole payoff 015
    /// was waiting for.
    pub dead: bool,
    /// 063. None is the common case: they act on their rolled number.
    pub hold: Option<Hold>,
}

impl Contender {
    /// Whether this one gets a turn when the walk reaches it.
    ///
    /// Three ways to be skipped and they are different: not rolled
    /// (not in the order yet), withdrawn, or dead. Dying is none of
    /// them.
    pub fn takes_turns(&self) -> bool {
        self.initiative.is_some() && self.active && !self.dead
    }
}

/// Where the walk landed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Turn {
    pub actor_id: String,
    /// True when the walk wrapped past the end of the order, which is
    /// what makes a round a round.
    pub new_round: bool,
}

/* ============================ RULES ============================ */

/// The order, highest first.
///
/// NOT YET ROLLED SORTS TO THE BOTTOM rather than being dropped. A
/// half-rolled encounter is the normal state for the thirty seconds
/// after enrolment, and a roster that hides the people still waiting to
/// roll is a roster nobody can chase.
///
/// Ties break on DEX and then on NAME - not because the book says name,
/// but because two goblins with the same dexterity have to come out in
/// SOME order and it had better be the same order every time. An
/// unstable sort here would shuffle the roster on every repaint.
pub fn order(contenders: &[Contender]) -> Vec<Contender> {
    place_holds(&rolled_order(contenders))
}

/// The order the dice made, before anybody declared anything.
///
/// SPLIT OUT so `place_holds` has something stable to reason about and
/// so a test can assert the two halves separately - the sort is the
/// part that must never shuffle, and the placement is the part with
/// the interesting edges.
fn rolled_order(contenders: &[Contender]) -> Vec<Contender> {
    let mut out = contenders.to_vec();
    out.sort_by(|a, b| {
        // None last, whatever it is beside.
        match (a.initiative, b.initiative) {
            (Some(x), Some(y)) => y.cmp(&x),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
        .then_with(|| b.dex.cmp(&a.dex))
        .then_with(|| a.name.cmp(&b.name))
        // Two goblins can share a name. The id is the last resort and
        // the only one guaranteed to differ.
        .then_with(|| a.id.cmp(&b.id))
    });
    out
}

/// Put the held creatures where they said they would be.
///
/// THE ORDER OF OPERATIONS IS THE RULE. Everybody who is not holding
/// keeps their rolled place; the end-of-round holders go last; then
/// the ones naming a creature or a slot are inserted, repeatedly,
/// until a pass places nobody new.
///
/// IT CANNOT LOOP AND IT CANNOT LOSE ANYBODY, which are the two things
/// worth guaranteeing. A holds after B while B holds after A is a
/// declaration with no answer - each pass places neither, the loop
/// stops, and both fall to the end in their rolled order. Same for a
/// hold naming somebody who has left the fight: 063's foreign key
/// nulls the target, and a target that resolves to nothing is treated
/// exactly like one that cannot be placed.
///
/// NOTHING IS REFUSED, which is 051's decision about the order applied
/// to the second way of sitting in it. A hold that makes no sense is
/// shown somewhere defensible rather than rejected at the point a DM
/// was trying to describe what happened at their table.
fn place_holds(rolled: &[Contender]) -> Vec<Contender> {
    if rolled.iter().all(|c| c.hold.is_none()) {
        return rolled.to_vec();
    }

    let mut line: Vec<Contender> = Vec::with_capacity(rolled.len());
    let mut waiting: Vec<Contender> = Vec::new();

    for c in rolled {
        match &c.hold {
            None => line.push(c.clone()),
            Some(_) => waiting.push(c.clone()),
        }
    }

    // END OF ROUND FIRST, because "last" has to mean last among the
    // creatures who are not themselves waiting to be placed - and
    // somebody holding AFTER an end-of-round creature should land
    // behind them, which only works if they are already down.
    let (last, mut rest): (Vec<Contender>, Vec<Contender>) = waiting
        .drain(..)
        .partition(|c| matches!(c.hold.as_ref().map(|h| h.mode), Some(HoldMode::EndOfRound)));
    line.extend(last);

    // Then the ones that name a place, until a pass changes nothing.
    // Bounded by the number waiting: each pass either places at least
    // one or ends the loop.
    loop {
        let before = rest.len();
        let mut still: Vec<Contender> = Vec::new();

        for c in rest.drain(..) {
            let placed = match c.hold.as_ref() {
                Some(h) if h.mode == HoldMode::AfterActor => {
                    match h.after.as_deref().and_then(|t| index_of(&line, t)) {
                        Some(i) => {
                            line.insert(i + 1, c.clone());
                            true
                        }
                        None => false,
                    }
                }
                Some(h) if h.mode == HoldMode::AfterNext => {
                    // ONE PLACE LATER. Their rolled neighbour is
                    // whoever they would have gone before; letting that
                    // creature past and going next is exactly what "go
                    // after the next one" says.
                    match neighbour_after(rolled, &c.id)
                        .and_then(|n| index_of(&line, &n))
                    {
                        Some(i) => {
                            line.insert(i + 1, c.clone());
                            true
                        }
                        // Nobody after them to wait for: they were
                        // already last, so holding changes nothing.
                        None => {
                            line.push(c.clone());
                            true
                        }
                    }
                }
                _ => false,
            };
            if !placed {
                still.push(c);
            }
        }

        rest = still;
        if rest.len() == before {
            break;
        }
    }

    // Whatever is left could not be placed - a cycle, or a target that
    // is gone. They go last, in the order the dice gave them.
    line.extend(rest);
    line
}

fn index_of(line: &[Contender], id: &str) -> Option<usize> {
    line.iter().position(|c| c.id == id)
}

/// Who comes after this creature on the ROLLED order, skipping anybody
/// else who is holding - waiting for somebody who is themselves
/// waiting is not what "go after the next one" means.
fn neighbour_after(rolled: &[Contender], id: &str) -> Option<String> {
    let at = rolled.iter().position(|c| c.id == id)?;
    rolled
        .iter()
        .skip(at + 1)
        .find(|c| c.hold.is_none())
        .map(|c| c.id.clone())
}

/// Who acts next, and whether that starts a new round.
///
/// `current` is whoever is acting now, or None when the order has not
/// started - or when the creature whose turn it was has been REMOVED,
/// which 051 turns into a NULL rather than deleting the fight. Both
/// mean the same thing here: start from the top.
///
/// SKIPS ARE PART OF THE WALK, not a filter applied first. A creature
/// that has not rolled, has withdrawn or is dead does not get a turn,
/// and the walk steps over it - but a creature that is DYING does get
/// one, and spends it on a death save. That is 015's payoff and the
/// reason `takes_turns` is not simply `active`.
///
/// NEW ROUND MEANS THE WALK WRAPPED. Starting an order from nobody is
/// therefore also a new round, which is what makes `begin` and
/// `advance` the same function.
pub fn next_turn(order: &[Contender], current: Option<&str>) -> Option<Turn> {
    let eligible: Vec<&Contender> = order.iter().filter(|c| c.takes_turns()).collect();
    if eligible.is_empty() {
        return None;
    }

    // Where the current actor sits among those who take turns. A
    // current actor that has died, withdrawn or left the roster
    // entirely is not in this list, and reads as None - which starts
    // the order again rather than stalling on a creature that is gone.
    let at = current.and_then(|id| eligible.iter().position(|c| c.id == id));

    match at {
        Some(i) if i + 1 < eligible.len() => Some(Turn {
            actor_id: eligible[i + 1].id.clone(),
            new_round: false,
        }),
        // Past the end, or starting fresh. Either way the top of the
        // order, and either way a round begins.
        _ => Some(Turn {
            actor_id: eligible[0].id.clone(),
            new_round: true,
        }),
    }
}

/// The initiative a d20 and a DEX modifier come to.
///
/// Here rather than at the call site so that the one place it is
/// written down is the place with a test under it. No proficiency: 5e
/// gives initiative no proficiency bonus, and the day a house rule does
/// it changes here.
pub fn score(d20: i64, dex_mod: i64) -> i64 {
    d20 + dex_mod
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    fn c(id: &str, init: Option<i64>, dex: i64) -> Contender {
        Contender {
            id: id.into(),
            name: id.into(),
            initiative: init,
            dex,
            active: true,
            dead: false,
            hold: None,
        }
    }

    /// The same, holding. `after` is the creature named, for
    /// `AfterActor`; the other two modes ignore it.
    fn held(id: &str, init: i64, mode: HoldMode, after: Option<&str>) -> Contender {
        Contender {
            hold: Some(Hold { mode, after: after.map(String::from) }),
            ..c(id, Some(init), 0)
        }
    }

    /// Just the ids, which is what every placement test is about.
    fn ids(v: &[Contender]) -> Vec<&str> {
        v.iter().map(|x| x.id.as_str()).collect()
    }

    /* ---------- the order ---------- */

    #[test]
    fn highest_goes_first() {
        let out = order(&[c("a", Some(12), 0), c("b", Some(20), 0), c("c", Some(3), 0)]);
        assert_eq!(ids(&out), ["b", "a", "c"]);
    }

    #[test]
    fn dexterity_breaks_a_tie() {
        let out = order(&[c("slow", Some(15), 1), c("quick", Some(15), 4)]);
        assert_eq!(ids(&out), ["quick", "slow"]);
    }

    // NOT YET ROLLED IS NOT ZERO, which 011 says outright. A rolled 0 is
    // a real and terrible result and outranks everybody still waiting.
    #[test]
    fn unrolled_sorts_below_a_rolled_zero() {
        let out = order(&[c("waiting", None, 20), c("rolled_badly", Some(0), 0)]);
        assert_eq!(ids(&out), ["rolled_badly", "waiting"]);
    }

    #[test]
    fn the_unrolled_are_shown_not_dropped() {
        let out = order(&[c("a", Some(10), 0), c("b", None, 0)]);
        assert_eq!(out.len(), 2, "a roster that hides them cannot chase them");
    }

    // AN UNSTABLE ORDER WOULD SHUFFLE ON EVERY REPAINT. Two goblins with
    // one name and one dexterity still have to come out the same way
    // twice.
    #[test]
    fn identical_creatures_come_out_in_a_stable_order() {
        let mut one = c("id-1", Some(10), 2);
        one.name = "Goblin".into();
        let mut two = c("id-2", Some(10), 2);
        two.name = "Goblin".into();

        let a = order(&[one.clone(), two.clone()]);
        let b = order(&[two, one]);
        assert_eq!(ids(&a), ids(&b));
    }

    /* ---------- the walk ---------- */

    fn three() -> Vec<Contender> {
        order(&[c("a", Some(20), 0), c("b", Some(15), 0), c("c", Some(10), 0)])
    }

    #[test]
    fn starting_from_nobody_takes_the_top_and_opens_a_round() {
        let t = next_turn(&three(), None).unwrap();
        assert_eq!(t.actor_id, "a");
        assert!(t.new_round);
    }

    #[test]
    fn the_walk_goes_down_the_order() {
        let o = three();
        assert_eq!(next_turn(&o, Some("a")).unwrap().actor_id, "b");
        assert!(!next_turn(&o, Some("a")).unwrap().new_round);
    }

    #[test]
    fn past_the_end_wraps_and_that_is_what_a_round_is() {
        let t = next_turn(&three(), Some("c")).unwrap();
        assert_eq!(t.actor_id, "a");
        assert!(t.new_round);
    }

    // 015'S PAYOFF. A creature at zero still takes its turn - that is
    // what it spends on a death save - so dying must not be skipped.
    #[test]
    fn a_dying_creature_still_gets_its_turn() {
        // Nothing here knows about hit points; dying is simply not dead,
        // and `dead` is the only thing that removes a creature.
        let o = three();
        assert_eq!(next_turn(&o, Some("a")).unwrap().actor_id, "b");
    }

    #[test]
    fn the_dead_are_stepped_over() {
        let mut b = c("b", Some(15), 0);
        b.dead = true;
        let o = order(&[c("a", Some(20), 0), b, c("c", Some(10), 0)]);
        assert_eq!(next_turn(&o, Some("a")).unwrap().actor_id, "c");
    }

    #[test]
    fn the_withdrawn_are_stepped_over() {
        let mut b = c("b", Some(15), 0);
        b.active = false;
        let o = order(&[c("a", Some(20), 0), b, c("c", Some(10), 0)]);
        assert_eq!(next_turn(&o, Some("a")).unwrap().actor_id, "c");
    }

    // Somebody enrolled mid-fight who has not rolled is not in the order
    // yet, and must not be handed a turn.
    #[test]
    fn the_unrolled_do_not_get_a_turn() {
        let o = order(&[c("a", Some(20), 0), c("late", None, 0)]);
        let t = next_turn(&o, Some("a")).unwrap();
        assert_eq!(t.actor_id, "a", "wrapped to the only one who can act");
        assert!(t.new_round);
    }

    // THE CURRENT ACTOR CAN VANISH. 051 nulls the pointer when a
    // creature is removed, and a goblin can also die on its own turn.
    // Either way the walk restarts rather than stalling.
    #[test]
    fn a_current_actor_who_is_gone_restarts_the_order() {
        let t = next_turn(&three(), Some("never-enrolled")).unwrap();
        assert_eq!(t.actor_id, "a");
        assert!(t.new_round);
    }

    #[test]
    fn a_current_actor_who_just_died_does_not_stall_the_fight() {
        let mut a = c("a", Some(20), 0);
        a.dead = true;
        let o = order(&[a, c("b", Some(15), 0)]);
        let t = next_turn(&o, Some("a")).unwrap();
        assert_eq!(t.actor_id, "b");
    }

    #[test]
    fn nobody_able_to_act_is_no_turn_at_all() {
        let mut a = c("a", Some(20), 0);
        a.dead = true;
        assert_eq!(next_turn(&order(&[a]), None), None);
        assert_eq!(next_turn(&[], None), None);
    }

    // One creature alone still advances, and every advance is a new
    // round - which is correct: it acts once per round.
    #[test]
    fn a_lone_creature_rounds_over_every_turn() {
        let o = order(&[c("a", Some(20), 0)]);
        let t = next_turn(&o, Some("a")).unwrap();
        assert_eq!(t.actor_id, "a");
        assert!(t.new_round);
    }

    /* ---------- the score ---------- */

    #[test]
    fn initiative_is_the_die_and_the_modifier() {
        assert_eq!(score(14, 3), 17);
        assert_eq!(score(1, -1), 0, "a real zero, which is not the same as unrolled");
        assert_eq!(score(1, -3), -2, "and it can go below zero");
    }

    /* ===================== HOLDING A PLACE (063) ===================== */

    // Nobody holding: the rolled order, untouched. The cheap path, and
    // the one every existing test depends on.
    #[test]
    fn with_no_holds_the_order_is_the_roll() {
        let all = vec![c("a", Some(20), 0), c("b", Some(10), 0), c("c", Some(5), 0)];
        assert_eq!(ids(&order(&all)), ["a", "b", "c"]);
    }

    #[test]
    fn end_of_round_goes_last() {
        let all = vec![
            held("a", 20, HoldMode::EndOfRound, None),
            c("b", Some(10), 0),
            c("c", Some(5), 0),
        ];
        assert_eq!(ids(&order(&all)), ["b", "c", "a"]);
    }

    // Two of them keep their rolled order relative to each other.
    #[test]
    fn two_holding_to_the_end_stay_in_rolled_order() {
        let all = vec![
            held("a", 20, HoldMode::EndOfRound, None),
            c("b", Some(15), 0),
            held("c", 10, HoldMode::EndOfRound, None),
        ];
        assert_eq!(ids(&order(&all)), ["b", "a", "c"]);
    }

    #[test]
    fn after_actor_lands_immediately_behind_them() {
        let all = vec![
            held("a", 20, HoldMode::AfterActor, Some("c")),
            c("b", Some(15), 0),
            c("c", Some(10), 0),
            c("d", Some(5), 0),
        ];
        assert_eq!(ids(&order(&all)), ["b", "c", "a", "d"]);
    }

    // Waiting for somebody SLOWER is the ordinary case; naming somebody
    // faster is legal and simply means acting sooner than the dice
    // said, which a DM may well want.
    #[test]
    fn a_hold_may_name_somebody_who_already_went() {
        let all = vec![
            c("a", Some(20), 0),
            c("b", Some(15), 0),
            held("c", 10, HoldMode::AfterActor, Some("a")),
        ];
        assert_eq!(ids(&order(&all)), ["a", "c", "b"]);
    }

    // ONE PLACE LATER, which is what "go after the next one" says.
    #[test]
    fn after_next_drops_one_place() {
        let all = vec![
            held("a", 20, HoldMode::AfterNext, None),
            c("b", Some(15), 0),
            c("c", Some(10), 0),
        ];
        assert_eq!(ids(&order(&all)), ["b", "a", "c"]);
    }

    // Already last, so letting the next one past changes nothing -
    // there is no next one.
    #[test]
    fn after_next_at_the_bottom_stays_at_the_bottom() {
        let all = vec![
            c("a", Some(20), 0),
            c("b", Some(15), 0),
            held("c", 10, HoldMode::AfterNext, None),
        ];
        assert_eq!(ids(&order(&all)), ["a", "b", "c"]);
    }

    // The neighbour it waits for is somebody who is NOT also waiting -
    // holding for a creature who is themselves holding is not what the
    // words mean.
    #[test]
    fn after_next_skips_another_holder_to_find_its_neighbour() {
        let all = vec![
            held("a", 20, HoldMode::AfterNext, None),
            held("b", 15, HoldMode::EndOfRound, None),
            c("d", Some(10), 0),
        ];
        assert_eq!(ids(&order(&all)), ["d", "a", "b"]);
    }

    /* ---------------- the ones that must not break ---------------- */

    // THE CYCLE. Two creatures each waiting for the other is a
    // declaration with no answer. It must not loop and must not lose
    // them - they fall to the end in rolled order.
    #[test]
    fn two_holding_for_each_other_do_not_loop() {
        let all = vec![
            c("a", Some(20), 0),
            held("b", 15, HoldMode::AfterActor, Some("c")),
            held("c", 10, HoldMode::AfterActor, Some("b")),
        ];
        let placed = order(&all);
        let got = ids(&placed);
        assert_eq!(got.len(), 3, "nobody is lost");
        assert_eq!(got[0], "a");
        assert!(got.contains(&"b") && got.contains(&"c"));
    }

    // A chain resolves: c waits for b, b waits for a, a is not holding.
    #[test]
    fn a_chain_of_holds_resolves() {
        let all = vec![
            c("a", Some(20), 0),
            held("b", 15, HoldMode::AfterActor, Some("a")),
            held("c", 10, HoldMode::AfterActor, Some("b")),
            c("d", Some(1), 0),
        ];
        assert_eq!(ids(&order(&all)), ["a", "b", "c", "d"]);
    }

    // 063 nulls the target when the creature leaves the fight. A hold
    // with nobody to follow goes last rather than vanishing.
    #[test]
    fn a_hold_naming_nobody_falls_to_the_end() {
        let all = vec![
            c("a", Some(20), 0),
            held("b", 15, HoldMode::AfterActor, None),
            c("c", Some(10), 0),
        ];
        assert_eq!(ids(&order(&all)), ["a", "c", "b"]);
    }

    #[test]
    fn a_hold_naming_somebody_who_is_not_here_falls_to_the_end() {
        let all = vec![
            c("a", Some(20), 0),
            held("b", 15, HoldMode::AfterActor, Some("ghost")),
            c("c", Some(10), 0),
        ];
        assert_eq!(ids(&order(&all)), ["a", "c", "b"]);
    }

    // EVERYBODY HOLDING. Degenerate and worth pinning: nobody is lost
    // and the answer is the same twice.
    #[test]
    fn a_fight_where_everybody_holds_still_returns_everybody() {
        let all = vec![
            held("a", 20, HoldMode::EndOfRound, None),
            held("b", 15, HoldMode::AfterNext, None),
            held("c", 10, HoldMode::AfterActor, Some("a")),
        ];
        let placed = order(&all);
        let again = order(&all);
        assert_eq!(ids(&placed).len(), 3);
        assert_eq!(ids(&placed), ids(&again), "and the same answer twice");
    }

    // A held creature is in the order like anybody else. Holding is
    // not a reason to be passed over.
    #[test]
    fn next_turn_walks_the_held_order() {
        let all = order(&[
            held("a", 20, HoldMode::EndOfRound, None),
            c("b", Some(15), 0),
            c("c", Some(10), 0),
        ]);
        assert_eq!(ids(&all), ["b", "c", "a"]);
        let t = next_turn(&all, Some("c")).unwrap();
        assert_eq!(t.actor_id, "a", "the holder acts last, not never");
        assert!(!t.new_round);
    }

    #[test]
    fn the_placement_is_stable_across_repaints() {
        let all = vec![
            held("a", 20, HoldMode::AfterActor, Some("c")),
            c("b", Some(15), 0),
            c("c", Some(10), 0),
            held("d", 5, HoldMode::EndOfRound, None),
        ];
        let first = order(&all);
        let once = ids(&first);
        for _ in 0..5 {
            let again = order(&all);
            assert_eq!(ids(&again), once);
        }
    }

    #[test]
    fn a_mode_parses_from_the_column_or_not_at_all() {
        assert_eq!(HoldMode::parse("after_next"), Some(HoldMode::AfterNext));
        assert_eq!(HoldMode::parse("after_actor"), Some(HoldMode::AfterActor));
        assert_eq!(HoldMode::parse("end_of_round"), Some(HoldMode::EndOfRound));
        assert_eq!(HoldMode::parse("whenever"), None);
        assert_eq!(HoldMode::parse(""), None);
    }
}
