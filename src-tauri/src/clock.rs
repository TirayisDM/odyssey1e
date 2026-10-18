//! Game time, in six-second ticks.
//!
//! 092. The engine counted rounds inside a fight and had no idea what
//! time it was outside one. "Inspires for 1 hour, does not stack" had
//! nowhere to be measured, which is what blocked every timed effect:
//! the bard's song, concentration, exhaustion, and the rest cycle that
//! brings Action Surge back.
//!
//! ---------------------------------------------------------------------
//! ONE UNIT, AND IT IS THE ROUND
//! ---------------------------------------------------------------------
//!
//! A round IS six seconds, so every duration in 5e is a whole number of
//! them and there is no second time system to keep in step:
//!
//! ```text
//!     1 round                              1
//!     1 minute                            10
//!     10 minutes  (Bardic Inspiration)   100
//!     1 hour      (short rest, attune)   600
//!     8 hours     (long rest)          4,800
//!     24 hours    (the rest limit)    14,400
//! ```
//!
//! An effect is not "an hour" - it is `expires_at = now + HOUR`, and
//! expiry is one integer comparison that reads the same in a fight and
//! on the road. That is the whole reason for this: COMBAT TIME AND
//! TRAVEL TIME STOP BEING DIFFERENT SYSTEMS.
//!
//! GAME TIME, NEVER WALL TIME. Nothing here reads a real clock. A
//! session that breaks for an hour has not aged anybody, and a duration
//! keyed to `now()` would say otherwise.
//!
//! ONE CLOCK PER GAME, on `games.tick`. A party shares a timeline; a
//! clock each would mean reconciling them the moment anybody scouted
//! ahead. The cost is that a rogue off on their own shares the camp's
//! time, which is what every table does at the table anyway.
//!
//! IT ONLY EVER GOES FORWARD. There is no rewind - a roll is a record
//! and so is the hour it happened in.

/// Seconds in a tick. A round, which is the point.
///
/// NOT CALLED YET and kept because it is the definition the whole
/// module rests on - a test asserts MINUTE * ROUND_SECONDS is 60, which
/// is what pins every other constant here to real time.
#[allow(dead_code)]
pub const ROUND_SECONDS: i64 = 6;

/// Ticks in each unit anything actually asks for.
#[allow(dead_code)]
pub const ROUND: i64 = 1;
pub const MINUTE: i64 = 10;
pub const HOUR: i64 = 60 * MINUTE;
pub const DAY: i64 = 24 * HOUR;

/// A short rest is an hour; a long rest is eight.
pub const SHORT_REST: i64 = HOUR;
pub const LONG_REST: i64 = 8 * HOUR;

/// The jumps a DM is offered. Anything else is a number of rounds.
pub const STEPS: &[(&str, i64)] = &[
    ("10 minutes", 10 * MINUTE),
    ("1 hour", HOUR),
    ("4 hours", 4 * HOUR),
    ("8 hours", 8 * HOUR),
    ("24 hours", DAY),
];

/// How far the clock may be moved in one go.
///
/// A YEAR, which is not a rule - it is a guard against a typo in a box
/// that takes a number. Somebody meaning 60 and typing 6000000 should
/// get a refusal rather than a campaign that is suddenly eleven years
/// older with every effect expired.
pub const MAX_STEP: i64 = 365 * DAY;

/// Move the clock forward.
///
/// FORWARD ONLY, and zero is not a move. An effect that expired while
/// time passed stays expired; nothing here un-expires anything, because
/// there is no going back.
pub fn advance(now: i64, by: i64) -> Result<i64, String> {
    if by <= 0 {
        return Err("time only goes forward".to_string());
    }
    if by > MAX_STEP {
        return Err(format!(
            "that is more than a year - {} at most in one go",
            said(MAX_STEP)
        ));
    }
    Ok(now + by)
}

/// Whether something that expires at `at` has expired by `now`.
///
/// AT THE TICK IT NAMES, IT IS GONE. An hour-long song cast at tick 0
/// expires at 600, and at 600 it is over - the six seconds from 599 to
/// 600 are the last of it. Treating `>=` as still-running would give
/// every duration one free round.
/// NOT CALLED YET. This and `remaining` are what the effects table
/// will ask - "has the bard's song run out" is this function and
/// nothing else. Written and tested with the clock rather than with the
/// first thing that needs them, because the off-by-one they settle is
/// the kind that is easier to get right once than to find later.
#[allow(dead_code)]
pub fn expired(now: i64, at: i64) -> bool {
    now >= at
}

/// How long until `at`, or None when it has already gone.
#[allow(dead_code)]
pub fn remaining(now: i64, at: i64) -> Option<i64> {
    if expired(now, at) {
        None
    } else {
        Some(at - now)
    }
}

/// A span of ticks in words: "2 days, 3 hours".
///
/// THE TWO LARGEST UNITS THAT APPLY, because "2 days, 3 hours, 14
/// minutes and 2 rounds" is not what anybody wanted to know. Rounds
/// show only when rounds are all there is, which is exactly when
/// somebody is in a fight and counting them.
pub fn said(ticks: i64) -> String {
    if ticks <= 0 {
        return "no time".to_string();
    }
    let bigger = [
        ("day", ticks / DAY),
        ("hour", (ticks % DAY) / HOUR),
        ("minute", (ticks % HOUR) / MINUTE),
    ];
    let mut words: Vec<String> = bigger
        .iter()
        .filter(|(_, n)| *n > 0)
        .take(2)
        .map(|(unit, n)| plural(*n, unit))
        .collect();

    // ROUNDS ONLY WHEN ROUNDS ARE ALL THERE IS. "1 hour, 3 rounds" is
    // accurate and nobody asked - three rounds beside an hour is noise.
    // When they are the whole span, they are the whole answer, which is
    // exactly the case somebody in a fight is counting.
    if words.is_empty() {
        words.push(plural(ticks % MINUTE, "round"));
    }
    words.join(", ")
}

fn plural(n: i64, unit: &str) -> String {
    format!("{} {}{}", n, unit, if n == 1 { "" } else { "s" })
}

/* ===================== THE CALENDAR OF TIRAYIS (214) ===================== */
//
// A CALENDAR IS A WAY OF READING THE TICK, NOT A SECOND CLOCK. Every
// rule in this file above - durations, expiry, the rest cycle, the
// twenty-four hours between long rests - is arithmetic on one integer
// and none of it changes. All that happens here is that the integer
// learns to say its name.
//
// THE YEAR IS EARTH'S, FLAT. 365 days, no leap - Dave's call, and the
// right one: a leap rule puts a branch in every date calculation to
// correct a drift no campaign is long enough to notice. The twelve
// months carry Earth's lengths, which is what makes them sum to 365
// with Faebrach at 28 and nothing left over.
//
// A SEASON IS A PROPERTY OF THE MONTH, which is the Gaelic division
// rather than the astronomical one: Ehrrach is Faebrach to Aiberhan
// entire, not equinox to solstice. That makes a season a lookup and
// not a calculation, and it means winter wraps the year end - Samhkarn,
// Nollagg, Ennahr - which is the one case worth a test.
//
// THE WEEK DOES NOT DIVIDE THE YEAR. 365 is 52 sevens and one day
// over, so every year starts one weekday later than the last, exactly
// as Earth's does. Nothing corrects it, because there is nothing to
// correct.

/// One of the four, with what the clans call it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Season {
    pub name: &'static str,
    /// The working name of the season - what the year is FOR.
    pub epithet: &'static str,
    pub about: &'static str,
}

pub const EHRRACH: Season = Season {
    name: "Ehrrach",
    epithet: "Thaw-time",
    about: "the passes open and the surface crews go out to scout new veins",
};
pub const SAMMRADT: Season = Season {
    name: "Sammradt",
    epithet: "High-forge",
    about: "the long season of full production and trade caravans",
};
pub const FOHRM: Season = Season {
    name: "Fohrm",
    epithet: "Forge-harvest",
    about: "the year's work is tallied, stores laid in and ore stockpiled",
};
pub const GAIMRADT: Season = Season {
    name: "Gaimradt",
    epithet: "Deep-hold",
    about: "the gates are sealed and the clans withdraw underground to craft, teach and tell stories",
};

#[derive(Debug, Clone, Copy)]
pub struct Month {
    pub name: &'static str,
    /// The Earth month it answers to, kept because the lengths come
    /// from it and somebody reading this will want to know which is
    /// which.
    pub earth: &'static str,
    pub days: i64,
    pub season: Season,
}

/// The twelve, in order, with Earth's lengths.
pub const MONTHS: [Month; 12] = [
    Month { name: "Ennahr",     earth: "January",   days: 31, season: GAIMRADT },
    Month { name: "Faebrach",   earth: "February",  days: 28, season: EHRRACH },
    Month { name: "Märtak",     earth: "March",     days: 31, season: EHRRACH },
    Month { name: "Aiberhan",   earth: "April",     days: 30, season: EHRRACH },
    Month { name: "Baldtann",   earth: "May",       days: 31, season: SAMMRADT },
    Month { name: "Meithkamm",  earth: "June",      days: 30, season: SAMMRADT },
    Month { name: "Jüll",       earth: "July",      days: 31, season: SAMMRADT },
    Month { name: "Lunnatz",    earth: "August",    days: 31, season: FOHRM },
    Month { name: "Mehnfohrm",  earth: "September", days: 30, season: FOHRM },
    Month { name: "Dreidfohrm", earth: "October",   days: 31, season: FOHRM },
    Month { name: "Samhkarn",   earth: "November",  days: 30, season: GAIMRADT },
    Month { name: "Nollagg",    earth: "December",  days: 31, season: GAIMRADT },
];

#[derive(Debug, Clone, Copy)]
pub struct Weekday {
    pub name: &'static str,
    pub earth: &'static str,
    pub means: &'static str,
}

/// The seven, starting at Luhnstag because tick zero does.
pub const WEEKDAYS: [Weekday; 7] = [
    Weekday { name: "Luhnstag",    earth: "Monday",    means: "day of the moon" },
    Weekday { name: "Märtstag",    earth: "Tuesday",   means: "day of Mars" },
    Weekday { name: "Kedaunstag",  earth: "Wednesday", means: "day of the first fast" },
    Weekday { name: "Dardauntag",  earth: "Thursday",  means: "day between the fasts" },
    Weekday { name: "Hainetag",    earth: "Friday",    means: "day of the fast" },
    Weekday { name: "Satharnstag", earth: "Saturday",  means: "day of Saturn" },
    Weekday { name: "Domnaktag",   earth: "Sunday",    means: "the Lord's day" },
];

pub const DAYS_IN_YEAR: i64 = 365;

/// The age the campaign is in. A LABEL AND NOT ARITHMETIC: the year
/// counts from the cataclysm that followed the Leviathan's arrival, so
/// SA 2240 is the 2240th year of this age and the roughly ten thousand
/// before it belong to the First Age and to prose. Nothing here spans
/// an age boundary, because nothing has been asked to.
pub const AGE: &str = "Second Age";
pub const AGE_SHORT: &str = "SA";

/// WHERE TICK ZERO LANDS: the first of Aiberhan, SA 2240 - the first
/// week of spring, when Ehrrach and the planting are celebrated.
///
/// The weekday is a choice rather than a fact Dave gave, and this is
/// the tidy one: the first day of the month is also the first day of
/// the week. One constant to change if the world says otherwise.
pub const EPOCH_YEAR: i64 = 2240;
const EPOCH_MONTH: usize = 3; // Aiberhan, zero-based
const EPOCH_DAY: i64 = 1;
const EPOCH_WEEKDAY: usize = 0; // Luhnstag

/// A moment, read off the tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date {
    pub year: i64,
    /// Zero-based index into [`MONTHS`].
    pub month: usize,
    /// One-based, the way anybody writes a date.
    pub day: i64,
    /// Zero-based index into [`WEEKDAYS`].
    pub weekday: usize,
    pub hour: i64,
    pub minute: i64,
}

impl Date {
    pub fn month_name(&self) -> &'static str {
        MONTHS[self.month].name
    }
    pub fn weekday_name(&self) -> &'static str {
        WEEKDAYS[self.weekday].name
    }
    pub fn season(&self) -> Season {
        MONTHS[self.month].season
    }
}

/// Days from the start of the year to the first of this month.
fn days_before(month: usize) -> i64 {
    MONTHS[..month].iter().map(|m| m.days).sum()
}

/// What day it is.
///
/// FORWARD FROM THE EPOCH, and the tick is floored at zero because the
/// clock has never gone backwards - a negative would be a bug
/// elsewhere, and rendering it as a date in 2239 would hide it.
pub fn date(tick: i64) -> Date {
    let t = tick.max(0);
    let days = t / DAY;
    let rest = t % DAY;

    // Day of the year the campaign opened on, zero-based.
    let from_year_start = days_before(EPOCH_MONTH) + (EPOCH_DAY - 1) + days;

    let year = EPOCH_YEAR + from_year_start / DAYS_IN_YEAR;
    let mut left = from_year_start % DAYS_IN_YEAR;

    let mut month = 0usize;
    while left >= MONTHS[month].days {
        left -= MONTHS[month].days;
        month += 1;
    }

    Date {
        year,
        month,
        day: left + 1,
        // 365 is not a multiple of seven, so this walks on by one day a
        // year of its own accord. That is the behaviour, not a bug.
        weekday: ((EPOCH_WEEKDAY as i64 + days).rem_euclid(7)) as usize,
        hour: (rest / MINUTE) / 60,
        minute: (rest / MINUTE) % 60,
    }
}

/// Where the clock stands, in words.
///
/// "Luhnstag, 1 Aiberhan SA 2240 · 00:00"
///
/// THE WEEKDAY LEADS because at a table the question is usually "what
/// day is it" before "what date", and the season is NOT in here - it
/// is one more thing on a line people read at a glance, and
/// `season_of` gives it to whatever has room.
pub fn reading(tick: i64) -> String {
    let d = date(tick);
    format!(
        "{}, {} {} {} {} · {:02}:{:02}",
        d.weekday_name(),
        d.day,
        d.month_name(),
        AGE_SHORT,
        d.year,
        d.hour,
        d.minute
    )
}

/// The season this tick falls in.
pub fn season_of(tick: i64) -> Season {
    date(tick).season()
}

/// Which kind of rest, and what it costs in time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rest {
    Short,
    Long,
}

impl Rest {
    pub fn ticks(self) -> i64 {
        match self {
            Rest::Short => SHORT_REST,
            Rest::Long => LONG_REST,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Rest::Short => "short",
            Rest::Long => "long",
        }
    }

    /// Whether a feature recharging on `tag` comes back from this rest.
    ///
    /// A SHORT REST IS A KIND OF LONG ONE, for recharging. 5e says
    /// "short or long" on Action Surge and means it: eight hours of
    /// sleep brings back everything an hour of sitting would. The
    /// reverse is not true, which is the whole asymmetry.
    pub fn restores(self, tag: &str) -> bool {
        match tag {
            "short" => true,
            "long" | "day" | "dawn" => self == Rest::Long,
            _ => false,
        }
    }
}

/// Whether a long rest may be taken yet.
///
/// ONE IN TWENTY-FOUR HOURS, which is 5e as written: "a character can't
/// benefit from more than one long rest in a 24-hour period". Measured
/// from the START of the last one, because that is the period the rule
/// names.
///
/// THE HIT POINT TEST IS 5e'S TOO - "a character must have at least 1
/// hit point at the start of the rest". Somebody at zero is dying, not
/// sleeping, and the rule exists so a long rest cannot be used as a
/// revival.
pub fn may_long_rest(now: i64, last_long_rest: Option<i64>, hp: i64) -> Result<(), String> {
    if hp < 1 {
        return Err("they are down - a long rest needs at least 1 hit point".to_string());
    }
    if let Some(last) = last_long_rest {
        let next = last + DAY;
        if now < next {
            return Err(format!(
                "they rested less than a day ago - {} to go",
                said(next - now)
            ));
        }
    }
    Ok(())
}

/// How many hit dice come back from a long rest: half the total,
/// minimum one.
///
/// 5e EXACTLY, and the minimum is what makes it worth a function. Half
/// of one is zero, so a level 1 character would get nothing back from a
/// night's sleep without it.
pub fn dice_back(total: i64) -> i64 {
    (total / 2).max(1).min(total)
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    /* ---------- the units line up ---------- */

    #[test]
    fn a_tick_is_a_round_is_six_seconds() {
        assert_eq!(ROUND, 1);
        assert_eq!(ROUND_SECONDS, 6);
        assert_eq!(MINUTE * ROUND_SECONDS, 60);
    }

    #[test]
    fn every_duration_five_e_asks_for_is_a_whole_number_of_ticks() {
        assert_eq!(MINUTE, 10);
        assert_eq!(10 * MINUTE, 100, "Bardic Inspiration, 10 minutes");
        assert_eq!(HOUR, 600, "a short rest");
        assert_eq!(LONG_REST, 4_800, "eight hours");
        assert_eq!(DAY, 14_400);
    }

    #[test]
    fn the_steps_a_dm_is_offered_are_all_real_spans() {
        for (label, ticks) in STEPS {
            assert!(*ticks > 0, "{} is not a span", label);
        }
        assert_eq!(STEPS.len(), 5);
    }

    /* ---------- moving ---------- */

    #[test]
    fn time_goes_forward() {
        assert_eq!(advance(0, HOUR), Ok(600));
        assert_eq!(advance(600, HOUR), Ok(1200));
    }

    #[test]
    fn time_does_not_go_back_and_zero_is_not_a_move() {
        assert!(advance(600, 0).is_err());
        assert!(advance(600, -HOUR).is_err());
    }

    #[test]
    fn a_typo_in_the_box_is_refused_rather_than_aged_into() {
        assert!(advance(0, MAX_STEP).is_ok());
        assert!(advance(0, MAX_STEP + 1).is_err());
    }

    /* ---------- expiry ---------- */

    #[test]
    fn an_hour_long_song_is_over_at_the_hour() {
        let cast_at = 0;
        let ends = cast_at + HOUR;
        assert!(!expired(599, ends), "still playing with a round to go");
        assert!(expired(600, ends), "over at the hour");
        assert!(expired(601, ends));
    }

    #[test]
    fn remaining_counts_down_and_then_stops_existing() {
        let ends = HOUR;
        assert_eq!(remaining(0, ends), Some(600));
        assert_eq!(remaining(599, ends), Some(1));
        assert_eq!(remaining(600, ends), None);
    }

    /* ---------- saying it ---------- */

    #[test]
    fn a_span_reads_in_the_two_units_that_matter() {
        assert_eq!(said(HOUR), "1 hour");
        assert_eq!(said(2 * HOUR), "2 hours");
        assert_eq!(said(DAY + 3 * HOUR), "1 day, 3 hours");
        assert_eq!(said(90 * MINUTE), "1 hour, 30 minutes");
    }

    #[test]
    fn rounds_show_only_when_rounds_are_all_there_is() {
        // Which is exactly when somebody is in a fight counting them.
        assert_eq!(said(3), "3 rounds");
        assert_eq!(said(1), "1 round");
        // Three rounds is not worth saying beside an hour.
        assert_eq!(said(HOUR + 3), "1 hour");
    }

    #[test]
    fn no_time_is_no_time() {
        assert_eq!(said(0), "no time");
        assert_eq!(said(-5), "no time");
    }

    /* ---------- the calendar of Tirayis (214) ---------- */

    /// The one that pins everything else: Earth's lengths with no leap
    /// day have to come to the year this calendar claims.
    #[test]
    fn the_twelve_months_are_a_year() {
        let total: i64 = MONTHS.iter().map(|m| m.days).sum();
        assert_eq!(total, DAYS_IN_YEAR);
        assert_eq!(total, 365);
    }

    #[test]
    fn the_campaign_opens_in_the_first_week_of_spring() {
        assert_eq!(reading(0), "Luhnstag, 1 Aiberhan SA 2240 · 00:00");
        assert_eq!(season_of(0).name, "Ehrrach");
        assert_eq!(season_of(0).epithet, "Thaw-time");
    }

    #[test]
    fn the_hours_still_run_inside_the_day() {
        assert_eq!(reading(HOUR), "Luhnstag, 1 Aiberhan SA 2240 · 01:00");
        assert_eq!(
            reading(14 * HOUR + 30 * MINUTE),
            "Luhnstag, 1 Aiberhan SA 2240 · 14:30"
        );
    }

    #[test]
    fn a_day_on_is_the_next_date_and_the_next_weekday() {
        let d = date(DAY);
        assert_eq!((d.day, d.month_name(), d.weekday_name()), (2, "Aiberhan", "Märtstag"));
    }

    /// Aiberhan has thirty, so the thirtieth is its last.
    #[test]
    fn a_month_ends_where_its_length_says() {
        assert_eq!(date(29 * DAY).day, 30);
        let over = date(30 * DAY);
        assert_eq!((over.day, over.month_name()), (1, "Baldtann"));
        assert_eq!(over.season().name, "Sammradt");
    }

    /// 365 is fifty-two weeks and a day, so a year on is the same date
    /// and the NEXT weekday. Earth does this too; nothing corrects it.
    #[test]
    fn a_year_on_is_one_weekday_later() {
        let d = date(DAYS_IN_YEAR * DAY);
        assert_eq!((d.day, d.month_name(), d.year), (1, "Aiberhan", 2241));
        assert_eq!(d.weekday_name(), "Märtstag");
    }

    /// The case the Gaelic division makes interesting: winter is
    /// Samhkarn, Nollagg and Ennahr, so it straddles the year end.
    #[test]
    fn winter_wraps_the_year() {
        for m in ["Samhkarn", "Nollagg", "Ennahr"] {
            let month = MONTHS.iter().find(|x| x.name == m).unwrap();
            assert_eq!(month.season.name, "Gaimradt", "{} is deep-hold", m);
        }
        // and the two it sits between are not winter
        let dreid = MONTHS.iter().find(|x| x.name == "Dreidfohrm").unwrap();
        let faeb = MONTHS.iter().find(|x| x.name == "Faebrach").unwrap();
        assert_eq!(dreid.season.name, "Fohrm");
        assert_eq!(faeb.season.name, "Ehrrach");
    }

    #[test]
    fn every_month_belongs_to_exactly_one_season_and_each_season_has_three() {
        for s in ["Ehrrach", "Sammradt", "Fohrm", "Gaimradt"] {
            let n = MONTHS.iter().filter(|m| m.season.name == s).count();
            assert_eq!(n, 3, "{} should hold three months", s);
        }
    }

    #[test]
    fn the_week_is_seven_and_comes_round() {
        assert_eq!(WEEKDAYS.len(), 7);
        assert_eq!(date(0).weekday_name(), date(7 * DAY).weekday_name());
    }

    /// Time only goes forward, so a negative is a bug somewhere else -
    /// and reading it as a date in 2239 would hide it.
    #[test]
    fn before_the_beginning_reads_as_the_beginning() {
        assert_eq!(reading(-1), reading(0));
    }

    /* ---------- rests ---------- */

    #[test]
    fn a_short_rest_is_an_hour_and_a_long_one_is_eight() {
        assert_eq!(Rest::Short.ticks(), HOUR);
        assert_eq!(Rest::Long.ticks(), 8 * HOUR);
    }

    #[test]
    fn a_long_rest_brings_back_everything_a_short_one_would() {
        // Action Surge says "short or long" and means it.
        assert!(Rest::Short.restores("short"));
        assert!(Rest::Long.restores("short"));
    }

    #[test]
    fn and_the_reverse_is_not_true() {
        // The whole asymmetry: Rage waits for the night.
        assert!(!Rest::Short.restores("long"));
        assert!(Rest::Long.restores("long"));
    }

    #[test]
    fn a_recharge_nobody_has_heard_of_restores_nothing() {
        assert!(!Rest::Short.restores("fortnightly"));
        assert!(!Rest::Long.restores(""));
    }

    #[test]
    fn the_first_long_rest_needs_no_permission() {
        assert!(may_long_rest(0, None, 10).is_ok());
    }

    #[test]
    fn a_second_long_rest_waits_a_day() {
        let first = 0;
        assert!(may_long_rest(first + LONG_REST, Some(first), 10).is_err());
        assert!(may_long_rest(first + DAY - 1, Some(first), 10).is_err());
        assert!(may_long_rest(first + DAY, Some(first), 10).is_ok());
    }

    #[test]
    fn the_refusal_says_how_long_is_left() {
        let err = may_long_rest(HOUR, Some(0), 10).unwrap_err();
        assert!(err.contains("23 hours"), "unhelpful: {}", err);
    }

    #[test]
    fn somebody_at_zero_is_dying_rather_than_sleeping() {
        let err = may_long_rest(DAY * 9, None, 0).unwrap_err();
        assert!(err.contains("1 hit point"), "unhelpful: {}", err);
    }

    /* ---------- hit dice ---------- */

    #[test]
    fn a_long_rest_returns_half_your_dice() {
        assert_eq!(dice_back(8), 4);
        assert_eq!(dice_back(20), 10);
    }

    #[test]
    fn and_never_fewer_than_one() {
        // Half of one is zero, so a level 1 character would get nothing
        // back from a night's sleep without 5e's stated minimum.
        assert_eq!(dice_back(1), 1);
        assert_eq!(dice_back(3), 1);
    }

    #[test]
    fn nor_more_than_you_have() {
        assert_eq!(dice_back(0), 0);
    }
}
