//! What one person at the table may see of another creature.
//!
//! 203. A PLAYER'S SCREEN IS NOT THE DM'S SCREEN WITH BUTTONS REMOVED.
//! Dave's call: a player sees a monster's NAME, its place in the turn
//! order, its conditions, and a BAND - hurt, bloodied, down. No hit
//! points and no armour class. Their own character shows real numbers,
//! because those are theirs.
//!
//! WHY THIS IS A RULE AND NOT A `case` IN THE FRONTEND. "Nothing on
//! screen computes a game number" is the line this project holds, and
//! "at or below half" is a game number. Putting the threshold in
//! JavaScript would also put it in a second place the day the DM's card
//! wants to show the same word.
//!
//! ---------------------------------------------------------------------
//! IT IS A COURTESY, NOT A GUARD, AND SAYING SO IS THE POINT
//! ---------------------------------------------------------------------
//!
//! **A player who is a member of the game can already read every
//! monster's `hp_max` and `ac_override` straight out of `characters`,
//! and every row of `hp_events`.** Verified by probe against the live
//! database before this module was written: as a player in Dave's game,
//! 18 character rows, 14 of them NPCs with their maxima, and 25 hp
//! events.
//!
//! So this decides WHAT THE APP SHOWS, exactly as `amDM()` decides what
//! the app OFFERS, and neither is a security boundary. The boundary is
//! RLS, and RLS currently lets a member read those columns.
//!
//! **Making it a real guard is a separate piece of work** and a bigger
//! one: it means a player never RECEIVES those columns, which is a
//! narrowed `characters` SELECT policy plus a read path that returns the
//! band from SQL. That is worth doing before anyone plays with a
//! stranger. It is not done, and this module must not be read as having
//! done it.

/// How hurt something looks to somebody who is not allowed to count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Band {
    /// No visible wound. Renders as nothing at all - see [`said`].
    Untouched,
    /// Below its maximum, above half.
    Hurt,
    /// At or below half. 5e's own word for it.
    Bloodied,
    /// At zero. Dying or dead; which of those is not a player's business
    /// and the band deliberately does not distinguish them.
    Down,
}

/// The band, or `None` when the maximum is unknown and no honest band
/// can be named.
///
/// THE ORDER OF THESE TESTS IS THE RULE. `Down` is decided on the
/// current value alone, so a creature at zero reads as down even when
/// nothing knows its maximum - which is the one thing that is certain
/// without it.
///
/// `hp_current` is expected FLOORED, the way `death::hp_floor` floors
/// it: 013 lets the event log run negative and clamps the reading, so a
/// goblin fourteen past zero is at zero here. Passing a raw sum still
/// gives `Down`, because the first test is `<= 0`.
///
/// AT OR BELOW HALF, not below half, and `* 2` rather than `/ 2` so an
/// odd maximum does not round its way out of the band: 26 of 52 is
/// bloodied, and so is 13 of 27.
pub fn band(hp_current: i64, hp_max: Option<i64>) -> Option<Band> {
    if hp_current <= 0 {
        return Some(Band::Down);
    }
    match hp_max {
        // A maximum of zero or less is not a maximum. Rather than
        // invent a band, say nothing - the DM's screen has the real
        // numbers and a wrong word is worse than no word.
        None => None,
        Some(max) if max <= 0 => None,
        Some(max) if hp_current >= max => Some(Band::Untouched),
        Some(max) if hp_current * 2 <= max => Some(Band::Bloodied),
        Some(_) => Some(Band::Hurt),
    }
}

/// The word a screen prints, or `None` when it should print nothing.
///
/// UNTOUCHED IS SILENT. A strip that labelled every unhurt creature
/// "untouched" would put a word against every line in the first round
/// and teach a player to stop reading them.
pub fn said(b: Band) -> Option<&'static str> {
    match b {
        Band::Untouched => None,
        Band::Hurt => Some("hurt"),
        Band::Bloodied => Some("bloodied"),
        Band::Down => Some("down"),
    }
}

/// What a reader gathered about one creature in the fight.
///
/// Plain data on purpose: this module does no network and no Tauri, so
/// the caller does the reading and this does the deciding.
#[derive(Debug, Clone)]
pub struct Facts {
    pub actor_id: String,
    /// The instance's name - "Goblin 0001" - which is what 018 and 001
    /// exist to make sure is never 'Someone'.
    pub name: String,
    pub character_id: Option<String>,
    pub hp_current: i64,
    pub hp_max: Option<i64>,
    /// Effect keys AS STORED, with the `cond.` prefix on them.
    ///
    /// NOT STRIPPED, and that is not an accident: `conditions::of_key`
    /// does `strip_prefix(PREFIX)?`, so it REQUIRES the prefix and
    /// quietly answers `None` for a bare key. Handing it `paralysed`
    /// instead of `cond.paralysed` makes every condition question
    /// answer "no condition" - which is a passing-looking answer, and
    /// is exactly how the first version of this module's own test
    /// passed while asserting nothing. Read the row in, decide here,
    /// strip on the way out.
    pub conditions: Vec<String>,
    pub is_current: bool,
    pub takes_turns: bool,
    pub dead: bool,
    /// What they rolled for initiative, or `None` if nobody has.
    pub initiative: Option<i64>,
}

/// One line of a player's fight strip, with nothing in it they may not
/// see.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub actor_id: String,
    pub name: String,
    /// Whether this is one of the viewer's own characters.
    pub yours: bool,
    /// Real numbers, and ONLY for your own. `None` is not "unknown", it
    /// is "not yours to count".
    pub hp: Option<(i64, i64)>,
    /// The word for everybody else. `None` when they are unhurt, or
    /// when no honest band could be named.
    pub band: Option<&'static str>,
    /// Condition keys in their CANONICAL stored spelling - prefixed,
    /// and the project's spelling whichever was written.
    ///
    /// PREFIXED BECAUSE THAT IS WHAT THE CATALOGUE USES.
    /// `list_conditions` sends `key_for(c)`, which carries the `cond.`
    /// prefix, and the frontend looks a chip's name, glyph and tooltip
    /// up by that key rather than being sent a second copy of them.
    /// Returning the bare key here would have matched nothing in that
    /// catalogue and rendered as NO CHIPS AT ALL - a creature that
    /// looks unafflicted rather than an error anybody would notice.
    pub conditions: Vec<String>,
    /// The condition that stops a turn, named, when one applies - 190's
    /// rule, asked rather than re-listed here.
    pub cannot_act: Option<String>,
    pub is_current: bool,
    pub takes_turns: bool,
    /// The character behind this line, and ONLY when it is yours.
    ///
    /// 205. A strip that can offer an attack has to be able to say who
    /// is swinging, and the frontend's `state.characterId` is set by
    /// clicking your name in the SIDEBAR - a selection that has nothing
    /// to do with this panel and is empty until you make it. Carrying
    /// it here means the strip depends on itself. `None` for everybody
    /// else, because nothing on their line is actionable.
    pub your_character_id: Option<String>,
    /// Whether to OFFER this viewer the initiative roll for this line.
    ///
    /// YOURS, AND ONLY WHILE IT IS UNROLLED. Offering it again mid-fight
    /// would put a button on screen whose honest label is "reorder the
    /// turn", which is the DM's call and not a player's. Nothing here
    /// REFUSES a re-roll - `roll_initiative` and the member UPDATE
    /// policy on `encounter_actors` both still allow it - this decides
    /// what is on offer, the way `amDM` does.
    pub needs_initiative: bool,
}

/// Decide one line for one viewer.
///
/// `owned` is the viewer's own character ids. A DM looking at this would
/// own most of the board, which is why the DM has a different screen and
/// this is not it.
///
/// YOUR OWN CHARACTER GETS NUMBERS AND NO BAND. Both would be two
/// spellings of one fact on the same row, and the numbers are stricter.
pub fn reveal(owned: &[String], f: &Facts) -> Line {
    let yours = f
        .character_id
        .as_deref()
        .is_some_and(|c| owned.iter().any(|o| o == c));

    // A dead actor reads as down whatever the arithmetic says. The flag
    // is the DM's explicit answer and outranks a derived one.
    let b = if f.dead {
        Some(Band::Down)
    } else {
        band(f.hp_current, f.hp_max)
    };

    Line {
        actor_id: f.actor_id.clone(),
        name: f.name.clone(),
        yours,
        hp: if yours {
            Some((f.hp_current.max(0), f.hp_max.unwrap_or(0)))
        } else {
            None
        },
        band: if yours { None } else { b.and_then(said) },
        // CONDITIONS ARE PUBLIC, which is Dave's call and also the
        // table's: a frightened ogre acts frightened where everyone can
        // see. They are the one thing a player learns about a monster
        // without being told a number.
        //
        // Asked through `of_key` rather than string-trimmed, so a key
        // the closed list does not recognise is dropped instead of
        // reaching a screen - 015's rule that an unknown condition is
        // not a condition.
        conditions: f
            .conditions
            .iter()
            .filter_map(|k| crate::conditions::of_key(k))
            .map(crate::conditions::key_for)
            .collect(),
        cannot_act: crate::conditions::cannot_act(&f.conditions)
            .map(|c| c.name.to_string()),
        is_current: f.is_current,
        takes_turns: f.takes_turns,
        your_character_id: if yours { f.character_id.clone() } else { None },
        needs_initiative: yours && f.initiative.is_none(),
    }
}

/// Take the hit points off a target list for somebody not entitled to
/// them.
///
/// 205. THE SAME RULE, APPLIED WHERE IT WAS LEAKING. `list_targets`
/// feeds two pickers - the DM's card and the PLAYER'S OWN ROLL BOX -
/// and the player's one prints `106/135 hp` beside every creature it
/// offers. So the strip above it said "hurt" while the dropdown two
/// inches below gave the exact number, which makes 203 decoration.
///
/// Applied here rather than in the frontend because that is where the
/// band already lives, and a threshold written twice is this project's
/// most expensive habit.
///
/// WHAT IT DOES NOT TAKE: the armour class. `roll_named` is handed
/// `targetValue` BY THE CLIENT, so the number a roll is resolved
/// against has to reach the client to be sent back. Removing it means
/// `roll_named` looking the target up from `targetId` server-side,
/// which is a real change to the roll path and not this one. Written
/// down rather than quietly left - Dave's rule was "no hit points and
/// no AC", and this delivers the first half.
///
/// The death condition and the save tally stay too: "down" is a band
/// the strip already shows, and a dying creature's tally is the thing
/// the whole table is watching.
pub fn hide_other_hit_points(
    targets: &mut [crate::encounter::Target],
    am_dm: bool,
    owned: &[String],
) {
    if am_dm {
        return;
    }
    for t in targets.iter_mut() {
        let mine = t
            .character_id
            .as_deref()
            .is_some_and(|c| owned.iter().any(|o| o == c));
        if !mine {
            t.hp_current = None;
            t.hp_max = None;
        }
    }
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> Facts {
        Facts {
            actor_id: "a1".into(),
            name: "Goblin 0001".into(),
            character_id: Some("c1".into()),
            hp_current: 7,
            hp_max: Some(7),
            conditions: vec![],
            is_current: false,
            takes_turns: true,
            dead: false,
            initiative: None,
        }
    }

    #[test]
    fn full_health_is_untouched_and_silent() {
        assert_eq!(band(7, Some(7)), Some(Band::Untouched));
        assert_eq!(said(Band::Untouched), None);
    }

    #[test]
    fn one_point_down_is_hurt() {
        assert_eq!(band(6, Some(7)), Some(Band::Hurt));
    }

    /// Luci's 52: half is 26, and 26 is bloodied rather than hurt.
    #[test]
    fn at_exactly_half_is_bloodied() {
        assert_eq!(band(26, Some(52)), Some(Band::Bloodied));
        assert_eq!(band(27, Some(52)), Some(Band::Hurt));
    }

    /// The reason for `* 2` instead of `/ 2`: integer division would put
    /// 13 of 27 at "half is 13, 13 is not below 13" and call it hurt.
    #[test]
    fn an_odd_maximum_does_not_round_out_of_the_band() {
        assert_eq!(band(13, Some(27)), Some(Band::Bloodied));
        assert_eq!(band(14, Some(27)), Some(Band::Hurt));
    }

    #[test]
    fn zero_is_down() {
        assert_eq!(band(0, Some(135)), Some(Band::Down));
    }

    /// 013 lets the sum run past the floor. A raw sum still reads down,
    /// because the first test is on the current value alone.
    #[test]
    fn a_sum_past_the_floor_still_reads_down() {
        assert_eq!(band(-14, Some(7)), Some(Band::Down));
    }

    #[test]
    fn down_needs_no_maximum() {
        assert_eq!(band(0, None), Some(Band::Down));
        assert_eq!(band(-3, None), Some(Band::Down));
    }

    #[test]
    fn no_maximum_names_no_band() {
        assert_eq!(band(5, None), None);
        assert_eq!(band(5, Some(0)), None);
        assert_eq!(band(5, Some(-2)), None);
    }

    #[test]
    fn above_the_maximum_is_untouched() {
        assert_eq!(band(60, Some(52)), Some(Band::Untouched));
    }

    #[test]
    fn your_own_character_gets_numbers_and_no_band() {
        let mut f = facts();
        f.hp_current = 41;
        f.hp_max = Some(52);
        let line = reveal(&["c1".to_string()], &f);
        assert!(line.yours);
        assert_eq!(line.hp, Some((41, 52)));
        assert_eq!(line.band, None);
    }

    #[test]
    fn somebody_elses_gets_a_band_and_no_numbers() {
        let mut f = facts();
        f.hp_current = 26;
        f.hp_max = Some(52);
        let line = reveal(&["someone-else".to_string()], &f);
        assert!(!line.yours);
        assert_eq!(line.hp, None);
        assert_eq!(line.band, Some("bloodied"));
    }

    /// The case the whole module exists for: a monster has no owner, so
    /// an empty `owned` must not accidentally make it yours.
    #[test]
    fn an_unowned_monster_is_not_yours() {
        let mut f = facts();
        f.character_id = None;
        let line = reveal(&[], &f);
        assert!(!line.yours);
        assert_eq!(line.hp, None);
    }

    #[test]
    fn an_unhurt_stranger_shows_nothing_at_all() {
        let line = reveal(&[], &facts());
        assert_eq!(line.band, None);
        assert_eq!(line.hp, None);
    }

    /// `dead` is the DM's explicit answer and outranks the arithmetic -
    /// an actor marked dead while its events still read healthy is down.
    #[test]
    fn the_dead_flag_outranks_the_arithmetic() {
        let mut f = facts();
        f.dead = true;
        f.hp_current = 7;
        f.hp_max = Some(7);
        assert_eq!(reveal(&[], &f).band, Some("down"));
    }

    /// Your own numbers are floored for the reading, per 013, so a sheet
    /// never shows a negative.
    #[test]
    fn your_own_numbers_are_floored() {
        let mut f = facts();
        f.hp_current = -5;
        f.hp_max = Some(52);
        let line = reveal(&["c1".to_string()], &f);
        assert_eq!(line.hp, Some((0, 52)));
    }

    #[test]
    fn conditions_are_public_and_cannot_act_is_named() {
        let mut f = facts();
        // PREFIXED, as the effect row stores it. This test is the
        // reason the prefix is documented on `Facts`: it first went in
        // as a bare `paralysed`, `of_key` answered None to everything,
        // and the assertion below failed - which was the lucky
        // outcome. A test asserting `cannot_act == None` would have
        // passed on the same mistake.
        f.conditions = vec!["cond.paralysed".into()];
        let line = reveal(&[], &f);
        assert_eq!(line.conditions, vec!["cond.paralysed".to_string()]);
        assert!(
            line.cannot_act.is_some(),
            "paralysed is one of the five that stop a turn"
        );
    }

    fn target(id: &str, character: Option<&str>) -> crate::encounter::Target {
        crate::encounter::Target {
            id: id.into(),
            row: "actor",
            target_kind: "ac",
            label: id.into(),
            value: 17,
            source: String::new(),
            character_id: character.map(String::from),
            hp_current: Some(106),
            hp_max: Some(135),
            condition: None,
            death_successes: 0,
            death_failures: 0,
        }
    }

    #[test]
    fn a_player_is_not_told_a_monsters_hit_points() {
        let mut ts = vec![target("lich", Some("lich-char"))];
        hide_other_hit_points(&mut ts, false, &["c1".to_string()]);
        assert_eq!(ts[0].hp_current, None);
        assert_eq!(ts[0].hp_max, None);
        // and the number the roll is resolved against survives
        assert_eq!(ts[0].value, 17);
    }

    #[test]
    fn a_player_still_counts_their_own() {
        let mut ts = vec![target("bryn", Some("c1"))];
        hide_other_hit_points(&mut ts, false, &["c1".to_string()]);
        assert_eq!(ts[0].hp_current, Some(106));
    }

    #[test]
    fn the_dm_is_told_everything() {
        let mut ts = vec![target("lich", Some("lich-char"))];
        hide_other_hit_points(&mut ts, true, &[]);
        assert_eq!(ts[0].hp_current, Some(106));
    }

    /// A challenge - a lock, a door - has no character and no hit
    /// points to take, and must not be mistaken for somebody's.
    #[test]
    fn a_target_with_no_character_is_nobodys() {
        let mut ts = vec![target("iron lock", None)];
        hide_other_hit_points(&mut ts, false, &["c1".to_string()]);
        assert_eq!(ts[0].hp_current, None);
    }

    #[test]
    fn your_own_line_carries_its_character_and_others_do_not() {
        let f = facts();
        assert_eq!(
            reveal(&["c1".to_string()], &f).your_character_id,
            Some("c1".to_string())
        );
        assert_eq!(reveal(&["someone-else".to_string()], &f).your_character_id, None);
    }

    #[test]
    fn your_own_unrolled_initiative_is_offered() {
        let f = facts();
        assert!(reveal(&["c1".to_string()], &f).needs_initiative);
    }

    #[test]
    fn once_rolled_it_is_not_offered_again() {
        let mut f = facts();
        f.initiative = Some(14);
        assert!(!reveal(&["c1".to_string()], &f).needs_initiative);
    }

    /// The case that matters: a monster's initiative is never a
    /// player's to roll, rolled or not.
    #[test]
    fn somebody_elses_initiative_is_never_offered() {
        let mut f = facts();
        f.character_id = None;
        assert!(!reveal(&[], &f).needs_initiative);

        let mut g = facts();
        g.initiative = None;
        assert!(!reveal(&["not-mine".to_string()], &g).needs_initiative);
    }

    #[test]
    fn an_ordinary_condition_does_not_stop_a_turn() {
        let mut f = facts();
        f.conditions = vec!["cond.frightened".into()];
        let line = reveal(&[], &f);
        assert_eq!(line.conditions, vec!["cond.frightened".to_string()]);
        assert_eq!(line.cannot_act, None);
    }

    /// The American spelling is what `of_key` forgives, and it must
    /// still come back in this project's spelling so one key reaches
    /// the screen however it was written.
    #[test]
    fn the_american_spelling_answers_in_ours() {
        let mut f = facts();
        f.conditions = vec!["cond.paralyzed".into()];
        let line = reveal(&[], &f);
        assert_eq!(line.conditions, vec!["cond.paralysed".to_string()]);
        assert!(line.cannot_act.is_some());
    }

    /// A key outside the closed list is dropped rather than printed.
    #[test]
    fn an_unknown_condition_is_not_a_condition() {
        let mut f = facts();
        f.conditions = vec!["cond.bewildered".into(), "not.a.condition".into()];
        let line = reveal(&[], &f);
        assert!(line.conditions.is_empty());
        assert_eq!(line.cannot_act, None);
    }
}
