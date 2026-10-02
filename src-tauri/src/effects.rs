//! Something that is true for a while.
//!
//! 094. The last piece the clock was built for. "Inspires compatriots
//! for 1 hour, max does not stack" is two rules - a duration and a
//! stacking rule - and before 092 there was nowhere to measure the
//! first and nowhere to enforce the second.
//!
//! ---------------------------------------------------------------------
//! AN EFFECT IS A ROW WITH A DEADLINE
//! ---------------------------------------------------------------------
//!
//! It knows who it is on, what it does, who put it there, and the tick
//! it stops being true at. Expiry is `clock::expired`, which is one
//! integer comparison - so an effect behaves identically in a fight and
//! on the road, and nothing has to sweep a table to notice that an hour
//! has gone by.
//!
//! NOTHING DELETES AN EXPIRED EFFECT. It is simply no longer active,
//! and the row stays as a record of what was true at the time - the
//! same contract as a roll. A DM asking "what was on him when he fell"
//! has an answer.
//!
//! ---------------------------------------------------------------------
//! STACKING IS THE INTERESTING HALF
//! ---------------------------------------------------------------------
//!
//! 5e's usual rule is that the same effect from the same source does
//! not add up, and the books say it in a dozen different ways. Four
//! behaviours cover everything asked for so far, and WHICH ONE IS A
//! PROPERTY OF THE EFFECT rather than of this module - a bard's song
//! replaces, a poison might stack, a blessing might refuse a second.
//!
//! The one that is not obvious is `Highest`: a weaker version of
//! something already running is not an error and is not an upgrade, it
//! is simply nothing. Refusing it would make a bard who sings badly
//! undo their own good song.

use crate::clock;
use serde::{Deserialize, Serialize};

/// What happens when a second one arrives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stacking {
    /// The newcomer wins and the old one ends. A bard's song: a fresh
    /// performance restarts the hour.
    Replace,
    /// The better of the two survives, by `magnitude`. A ties goes to
    /// what is already there - a new one has to BEAT it, not match it,
    /// so re-applying the same thing does not quietly reset its clock.
    Highest,
    /// Both run. Two different poisons, two different wounds.
    Stack,
    /// A second is turned away while the first is running.
    Refuse,
}

impl Stacking {
    pub fn parse(s: &str) -> Stacking {
        match s {
            "highest" => Stacking::Highest,
            "stack" => Stacking::Stack,
            "refuse" => Stacking::Refuse,
            // REPLACE IS THE DEFAULT, including for a word nobody has
            // heard of. It is the least surprising of the four: the
            // newest thing is true and there is only ever one.
            _ => Stacking::Replace,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Stacking::Replace => "replace",
            Stacking::Highest => "highest",
            Stacking::Stack => "stack",
            Stacking::Refuse => "refuse",
        }
    }
}

/// One thing that is true for a while.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Effect {
    pub id: String,
    /// What KIND this is - `inspired`, `bless`, `poisoned`. Two
    /// effects with the same key are the same thing, which is what
    /// stacking is decided across.
    pub key: String,
    pub name: String,
    /// Whose it is.
    pub character_id: String,
    /// How strong, where strength means anything. The bard's song
    /// carries the die size; a bonus carries the bonus. None where the
    /// effect is simply on or off.
    pub magnitude: Option<i64>,
    /// The tick it began.
    pub started_at: i64,
    /// The tick it stops being true at. None runs until something ends
    /// it - a curse, a condition a DM is tracking by hand.
    pub expires_at: Option<i64>,
    pub stacks: Stacking,
}

impl Effect {
    /// Whether this is still true at `now`.
    ///
    /// AN EFFECT WITH NO DEADLINE NEVER EXPIRES, which is not the same
    /// as lasting forever - something still has to end it, and that is
    /// a DM's job rather than arithmetic's.
    pub fn active(&self, now: i64) -> bool {
        match self.expires_at {
            None => true,
            Some(at) => !clock::expired(now, at),
        }
    }

    /// How long is left, in ticks. None for no deadline or already
    /// gone - the caller knows which from `active`.
    pub fn left(&self, now: i64) -> Option<i64> {
        self.expires_at.and_then(|at| clock::remaining(now, at))
    }

    /// How long is left, in words: "42 minutes".
    pub fn said(&self, now: i64) -> String {
        match self.expires_at {
            None => "until it ends".to_string(),
            Some(at) => match clock::remaining(now, at) {
                Some(n) => clock::said(n),
                None => "over".to_string(),
            },
        }
    }
}

/// What applying one would do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Add it; nothing is in the way.
    Add,
    /// Add it, and end these first.
    Supersede(Vec<String>),
    /// Do nothing - what is already there is as good or better.
    Keep(String),
    /// Refuse, and say why.
    Refuse(String),
}

/// What happens if `incoming` is applied over what is already there.
///
/// ONLY THE SAME KEY ON THE SAME CHARACTER IS IN THE WAY. Two different
/// effects never interact here however similar they look; deciding that
/// a blessing and a bard's song are "the same bonus" is a rule about
/// those two things rather than about effects, and it is not one 5e
/// makes.
///
/// EXPIRED ONES ARE NOT IN THE WAY EITHER. A song that ran out an hour
/// ago is a record, not an obstacle - which is what lets rows stay
/// instead of being swept.
pub fn admit(existing: &[Effect], incoming: &Effect, now: i64) -> Outcome {
    let live: Vec<&Effect> = existing
        .iter()
        .filter(|e| {
            e.character_id == incoming.character_id && e.key == incoming.key && e.active(now)
        })
        .collect();

    if live.is_empty() {
        return Outcome::Add;
    }

    match incoming.stacks {
        Stacking::Stack => Outcome::Add,
        Stacking::Refuse => Outcome::Refuse(format!(
            "{} already has {} - {} left",
            incoming.character_id,
            live[0].name,
            live[0].said(now)
        )),
        Stacking::Replace => Outcome::Supersede(live.iter().map(|e| e.id.clone()).collect()),
        Stacking::Highest => {
            // A TIE GOES TO WHAT IS ALREADY RUNNING. The newcomer has
            // to BEAT it, so singing the same song twice does not
            // quietly restart the clock on it.
            let best = live.iter().map(|e| e.magnitude.unwrap_or(0)).max().unwrap_or(0);
            if incoming.magnitude.unwrap_or(0) > best {
                Outcome::Supersede(live.iter().map(|e| e.id.clone()).collect())
            } else {
                // NOT AN ERROR. A weaker version of something already
                // running is nothing - refusing would let a bad
                // performance undo a good one.
                Outcome::Keep(live[0].id.clone())
            }
        }
    }
}

/// Everything still true, soonest to end first.
///
/// THE ORDER IS A DECISION. What runs out next is what somebody needs
/// to know about, and an effect with no deadline sorts last because it
/// is not going anywhere.
pub fn active(effects: &[Effect], now: i64) -> Vec<&Effect> {
    let mut out: Vec<&Effect> = effects.iter().filter(|e| e.active(now)).collect();
    out.sort_by_key(|e| (e.expires_at.is_none(), e.left(now).unwrap_or(i64::MAX), e.name.clone()));
    out
}

/// When something starting now and lasting `ticks` would end.
///
/// NONE FOR NO DURATION, which is how "until something ends it" is
/// said. Zero and negative are the same answer: an effect that lasts no
/// time is not an effect.
pub fn ends_at(now: i64, ticks: Option<i64>) -> Option<i64> {
    ticks.filter(|t| *t > 0).map(|t| now + t)
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    fn eff(id: &str, key: &str, who: &str, stacks: Stacking) -> Effect {
        Effect {
            id: id.into(),
            key: key.into(),
            name: key.into(),
            character_id: who.into(),
            magnitude: None,
            started_at: 0,
            expires_at: Some(clock::HOUR),
            stacks,
        }
    }

    fn with_magnitude(mut e: Effect, n: i64) -> Effect {
        e.magnitude = Some(n);
        e
    }

    /* ---------- being true for a while ---------- */

    #[test]
    fn an_hour_long_song_is_true_for_an_hour() {
        let e = eff("1", "inspired", "falon", Stacking::Replace);
        assert!(e.active(0));
        assert!(e.active(clock::HOUR - 1));
        assert!(!e.active(clock::HOUR), "over at the hour");
    }

    #[test]
    fn an_effect_with_no_deadline_is_always_true() {
        let mut e = eff("1", "cursed", "falon", Stacking::Replace);
        e.expires_at = None;
        assert!(e.active(0));
        assert!(e.active(clock::DAY * 400));
        assert_eq!(e.said(0), "until it ends");
    }

    #[test]
    fn what_is_left_counts_down_and_then_says_so() {
        let e = eff("1", "inspired", "falon", Stacking::Replace);
        assert_eq!(e.left(0), Some(clock::HOUR));
        assert_eq!(e.said(0), "1 hour");
        assert_eq!(e.said(clock::HOUR - 10 * clock::MINUTE), "10 minutes");
        assert_eq!(e.said(clock::HOUR), "over");
    }

    /* ---------- the first one ---------- */

    #[test]
    fn nothing_in_the_way_is_simply_added() {
        let incoming = eff("new", "inspired", "falon", Stacking::Replace);
        assert_eq!(admit(&[], &incoming, 0), Outcome::Add);
    }

    #[test]
    fn somebody_elses_effect_is_not_in_the_way() {
        let theirs = eff("a", "inspired", "mira", Stacking::Refuse);
        let incoming = eff("new", "inspired", "falon", Stacking::Refuse);
        assert_eq!(admit(&[theirs], &incoming, 0), Outcome::Add);
    }

    #[test]
    fn a_different_effect_is_not_in_the_way_however_similar() {
        // Deciding a blessing and a bard's song are "the same bonus" is
        // a rule about those two things and not one 5e makes.
        let bless = eff("a", "bless", "falon", Stacking::Refuse);
        let incoming = eff("new", "inspired", "falon", Stacking::Refuse);
        assert_eq!(admit(&[bless], &incoming, 0), Outcome::Add);
    }

    #[test]
    fn an_expired_one_is_a_record_rather_than_an_obstacle() {
        // Which is what lets rows stay instead of being swept.
        let old = eff("a", "inspired", "falon", Stacking::Refuse);
        let incoming = eff("new", "inspired", "falon", Stacking::Refuse);
        assert_eq!(admit(&[old], &incoming, clock::HOUR), Outcome::Add);
    }

    /* ---------- the four behaviours ---------- */

    #[test]
    fn replace_ends_what_was_there() {
        let old = eff("a", "inspired", "falon", Stacking::Replace);
        let incoming = eff("new", "inspired", "falon", Stacking::Replace);
        assert_eq!(
            admit(&[old], &incoming, 0),
            Outcome::Supersede(vec!["a".to_string()])
        );
    }

    #[test]
    fn replace_ends_every_one_of_them() {
        // Several can only exist if something was applied under a
        // different rule earlier, which is a real state - the catalogue
        // can be edited between castings.
        let a = eff("a", "inspired", "falon", Stacking::Stack);
        let b = eff("b", "inspired", "falon", Stacking::Stack);
        let incoming = eff("new", "inspired", "falon", Stacking::Replace);
        assert_eq!(
            admit(&[a, b], &incoming, 0),
            Outcome::Supersede(vec!["a".to_string(), "b".to_string()])
        );
    }

    #[test]
    fn stack_simply_adds() {
        let old = eff("a", "poisoned", "falon", Stacking::Stack);
        let incoming = eff("new", "poisoned", "falon", Stacking::Stack);
        assert_eq!(admit(&[old], &incoming, 0), Outcome::Add);
    }

    #[test]
    fn refuse_turns_a_second_away_and_says_how_long() {
        let old = eff("a", "blessed", "falon", Stacking::Refuse);
        let incoming = eff("new", "blessed", "falon", Stacking::Refuse);
        match admit(&[old], &incoming, 0) {
            Outcome::Refuse(why) => assert!(why.contains("1 hour"), "unhelpful: {}", why),
            other => panic!("expected a refusal, got {:?}", other),
        }
    }

    /* ---------- highest, which is the subtle one ---------- */

    #[test]
    fn a_better_one_supersedes() {
        let old = with_magnitude(eff("a", "inspired", "falon", Stacking::Highest), 6);
        let incoming = with_magnitude(eff("new", "inspired", "falon", Stacking::Highest), 8);
        assert_eq!(
            admit(&[old], &incoming, 0),
            Outcome::Supersede(vec!["a".to_string()])
        );
    }

    #[test]
    fn a_weaker_one_is_nothing_rather_than_an_error() {
        // Refusing would let a bard who sings badly undo their own
        // good song.
        let old = with_magnitude(eff("a", "inspired", "falon", Stacking::Highest), 8);
        let incoming = with_magnitude(eff("new", "inspired", "falon", Stacking::Highest), 6);
        assert_eq!(admit(&[old], &incoming, 0), Outcome::Keep("a".to_string()));
    }

    #[test]
    fn a_tie_goes_to_what_is_already_running() {
        // The newcomer has to BEAT it, so singing the same song twice
        // does not quietly restart the clock on it.
        let old = with_magnitude(eff("a", "inspired", "falon", Stacking::Highest), 6);
        let incoming = with_magnitude(eff("new", "inspired", "falon", Stacking::Highest), 6);
        assert_eq!(admit(&[old], &incoming, 0), Outcome::Keep("a".to_string()));
    }

    #[test]
    fn the_best_of_several_is_what_must_be_beaten() {
        let a = with_magnitude(eff("a", "inspired", "falon", Stacking::Stack), 4);
        let b = with_magnitude(eff("b", "inspired", "falon", Stacking::Stack), 10);
        let incoming = with_magnitude(eff("new", "inspired", "falon", Stacking::Highest), 6);
        assert_eq!(admit(&[a, b], &incoming, 0), Outcome::Keep("a".to_string()));
    }

    /* ---------- reading the words ---------- */

    #[test]
    fn stacking_words_parse_and_an_unknown_one_replaces() {
        assert_eq!(Stacking::parse("replace"), Stacking::Replace);
        assert_eq!(Stacking::parse("highest"), Stacking::Highest);
        assert_eq!(Stacking::parse("stack"), Stacking::Stack);
        assert_eq!(Stacking::parse("refuse"), Stacking::Refuse);
        // The least surprising of the four.
        assert_eq!(Stacking::parse("fortnightly"), Stacking::Replace);
        assert_eq!(Stacking::parse(""), Stacking::Replace);
    }

    #[test]
    fn every_word_survives_a_round_trip() {
        for s in ["replace", "highest", "stack", "refuse"] {
            assert_eq!(Stacking::parse(s).as_str(), s);
        }
    }

    /* ---------- what is running ---------- */

    #[test]
    fn active_leaves_out_what_has_run_out() {
        let mut gone = eff("a", "inspired", "falon", Stacking::Replace);
        gone.expires_at = Some(10);
        let running = eff("b", "bless", "falon", Stacking::Replace);
        let all = [gone, running];
        let got = active(&all, 20);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].id, "b");
    }

    #[test]
    fn what_runs_out_soonest_comes_first() {
        let mut soon = eff("soon", "a", "falon", Stacking::Replace);
        soon.expires_at = Some(100);
        let mut later = eff("later", "b", "falon", Stacking::Replace);
        later.expires_at = Some(5000);
        let all = [later, soon];
        let got = active(&all, 0);
        assert_eq!(got.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), vec!["soon", "later"]);
    }

    #[test]
    fn something_with_no_deadline_sorts_last() {
        // It is not going anywhere, so it is not what anybody needs to
        // be told about.
        let mut forever = eff("forever", "cursed", "falon", Stacking::Replace);
        forever.expires_at = None;
        let soon = eff("soon", "inspired", "falon", Stacking::Replace);
        let all = [forever, soon];
        let got = active(&all, 0);
        assert_eq!(got.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), vec!["soon", "forever"]);
    }

    /* ---------- when it ends ---------- */

    #[test]
    fn an_hour_from_now_is_an_hour_from_now() {
        assert_eq!(ends_at(1000, Some(clock::HOUR)), Some(1600));
    }

    #[test]
    fn no_duration_is_until_something_ends_it() {
        assert_eq!(ends_at(1000, None), None);
    }

    #[test]
    fn an_effect_that_lasts_no_time_is_not_an_effect() {
        assert_eq!(ends_at(1000, Some(0)), None);
        assert_eq!(ends_at(1000, Some(-5)), None);
    }
}
