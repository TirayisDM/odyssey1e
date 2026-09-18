//! Dice engine — a faithful port of rollFormula() from diceroller.js.
//!
//! PARITY IS THE POINT. This is not a redesign. The output strings, the
//! two-space join, the U+2212 minus on negative terms, the markdown
//! struck-through drops and bolded natural 20s/1s are all reproduced
//! exactly, because roll cards built on the old system must keep reading
//! the same and because "it looks about right" is not a test.
//!
//! Where the original is odd, the oddity is preserved and commented
//! rather than tidied. Two in particular:
//!   * a positive DICE term gets no '+' prefix, but a positive NUMERIC
//!     term does — so "1d20+4" renders as "1d20 [15]  +4".
//!   * `natural` is only meaningful for a single d20 term. With two d20
//!     terms it is discarded at the end, even though each term set it.
//!     That distinction is what crit and fumble detection rests on.
//!
//! SYNTAX: NdM with optional khK / klK, plus signed integer terms.
//!   "2d8+5"  "4d6kh3"  "2d20kl1+3"  "d100"  "d20-1"

/// Where dice values come from. Injected so the engine is testable:
/// parity tests feed known rolls and assert exact output, which is
/// impossible against a hardwired RNG.
pub trait Roller {
    /// A single die, 1..=faces.
    fn roll(&mut self, faces: u32) -> i64;
}

/// Real dice.
pub struct RandomRoller;

impl Roller for RandomRoller {
    fn roll(&mut self, faces: u32) -> i64 {
        if faces == 0 {
            return 0;
        }
        1 + (rand::random::<u32>() % faces) as i64
    }
}

/// Test dice: hands back a fixed sequence, in order.
///
/// #[cfg(test)] rather than #[allow(dead_code)] — this is genuinely
/// test-only scaffolding, so it should not be compiled into the shipped
/// library at all. Silencing the warning would have left it in the
/// binary for no reason.
#[cfg(test)]
pub struct SequenceRoller {
    values: Vec<i64>,
    next: usize,
}

#[cfg(test)]
impl SequenceRoller {
    pub fn new(values: &[i64]) -> Self {
        Self { values: values.to_vec(), next: 0 }
    }
    /// True when every supplied value was consumed — a test that leaves
    /// dice on the table is usually a test that is wrong about the
    /// formula.
    pub fn exhausted(&self) -> bool {
        self.next == self.values.len()
    }
}

#[cfg(test)]
impl Roller for SequenceRoller {
    fn roll(&mut self, _faces: u32) -> i64 {
        let v = self.values.get(self.next).copied().unwrap_or(1);
        self.next += 1;
        v
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RollResult {
    /// Human-readable breakdown, in the original's markdown.
    pub detail: String,
    pub total: i64,
    /// The raw d20 face — Some ONLY when exactly one d20 term was
    /// rolled. Crit and fumble detection depend on that.
    pub natural: Option<i64>,
    /// The verdict on `natural` under the thresholds this roll used.
    ///
    /// Some exactly when `natural` is Some. A damage roll has no d20 and
    /// therefore no verdict, which is NOT the same as a normal one —
    /// hence Option rather than defaulting to Normal.
    pub outcome: Option<Outcome>,
}

/// What a raw d20 face came to mean.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Crit,
    Fumble,
    Normal,
}

/// A crit and fumble range — the house rule system, not a 5e constant.
///
/// Standard play is 20 and 1, and that is the default here. The
/// techniques table has never agreed: Deepsong Echo crits on 18, Crystal
/// Resonance fumbles on 1-3, Heavy Smash on 1-2, and a `fumble_max` of 0
/// means a technique cannot fumble at all. Those are per-technique
/// columns in 006 with check constraints on them, and this struct is the
/// engine-side counterpart of those constraints. Expect more of them,
/// not fewer — this is a system the campaign keeps extending.
///
/// CONSTRUCTED, NOT ASSEMBLED. The fields are public to read but `new`
/// is the only way to build a non-standard pair, and it enforces the two
/// bounds the database enforces plus the one the database cannot
/// express: the ranges must not meet. A check constraint can say
/// `crit_min between 2 and 20` per column, but it cannot say that this
/// column must exceed that one. If `fumble_max` ever reached `crit_min`,
/// a single face would be both a crit and a fumble and whichever test
/// ran first would win, silently, forever.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct Thresholds {
    /// A natural at or above this is a crit. 20 is standard; 18 and 19
    /// widen the range.
    pub crit_min: i64,
    /// A natural at or below this is a fumble. 1 is standard; 0 means
    /// cannot fumble; 2 and 3 widen the range.
    pub fumble_max: i64,
}

impl Thresholds {
    /// Natural 20 crits, natural 1 fumbles. What everything that is not
    /// a technique uses, and what keeps this engine's rendered output
    /// byte-identical to the port it came from.
    pub const STANDARD: Thresholds = Thresholds { crit_min: 20, fumble_max: 1 };

    /// Bounds match the check constraints on `techniques.crit_min` and
    /// `techniques.fumble_max`, so a row the database accepts is a pair
    /// this accepts, and vice versa. Keep them in step.
    ///
    /// Not called outside tests yet: the caller is the attack resolver,
    /// which reads a technique's two columns and builds a pair from
    /// them. Annotated rather than deleted for the same reason
    /// `d20_formula` is - it is finished and tested, and the alternative
    /// is writing it again in a fortnight.
    #[allow(dead_code)]
    pub fn new(crit_min: i64, fumble_max: i64) -> Result<Self, String> {
        if !(2..=20).contains(&crit_min) {
            return Err(format!("crit_min must be 2..=20, got {}", crit_min));
        }
        if !(0..=19).contains(&fumble_max) {
            return Err(format!("fumble_max must be 0..=19, got {}", fumble_max));
        }
        if fumble_max >= crit_min {
            return Err(format!(
                "fumble and crit ranges overlap: fumble_max {} >= crit_min {}",
                fumble_max, crit_min
            ));
        }
        Ok(Thresholds { crit_min, fumble_max })
    }

    /// What a raw d20 face means here. Crit is tested first, but `new`
    /// has already guaranteed the ranges cannot both match.
    pub fn verdict(&self, natural: i64) -> Outcome {
        if natural >= self.crit_min {
            Outcome::Crit
        } else if natural <= self.fumble_max {
            Outcome::Fumble
        } else {
            Outcome::Normal
        }
    }

    /// Whether a face is bolded in the detail string. Marking is the
    /// display half of the same rule, so it reads these thresholds
    /// rather than carrying a second hardcoded copy of 20 and 1.
    fn is_marked(&self, face: i64) -> bool {
        !matches!(self.verdict(face), Outcome::Normal)
    }
}

impl Default for Thresholds {
    fn default() -> Self {
        Thresholds::STANDARD
    }
}

/// One parsed dice term: NdM with an optional keep.
struct DiceTerm {
    count: u32,
    faces: u32,
    keep: Option<(KeepKind, u32)>,
}

#[derive(PartialEq, Clone, Copy)]
enum KeepKind {
    High,
    Low,
}

/// Parse "2d20kh1" / "d6" / "4d6kl2". Returns None when the term is not
/// dice-shaped at all, which the caller treats as "maybe a number".
fn parse_dice_term(term: &str) -> Option<DiceTerm> {
    let b = term.as_bytes();
    let mut i = 0;

    let start = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    let count_str = &term[start..i];

    if i >= b.len() || b[i] != b'd' {
        return None;
    }
    i += 1;

    let fstart = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if i == fstart {
        return None; // "d" with no faces
    }
    let faces: u32 = term[fstart..i].parse().ok()?;

    let mut keep = None;
    if i < b.len() {
        let kind = if term[i..].starts_with("kh") {
            KeepKind::High
        } else if term[i..].starts_with("kl") {
            KeepKind::Low
        } else {
            return None; // trailing junk
        };
        i += 2;
        let kstart = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        // The original's regex makes the count optional and defaults it
        // to 1: /(kh|kl)(\d+)/ with `Number(m[4] || 1)`.
        let k: u32 = if i == kstart { 1 } else { term[kstart..i].parse().ok()? };
        keep = Some((kind, k));
    }

    if i != b.len() {
        return None; // anything left over is not a dice term
    }

    // `Math.max(1, Number(m[1] || 1))` — "d20" means one d20.
    let count = if count_str.is_empty() {
        1
    } else {
        count_str.parse::<u32>().ok()?.max(1)
    };

    Some(DiceTerm { count, faces, keep })
}

/// Roll under standard 20/1 thresholds. What every caller that is not
/// resolving a technique wants.
pub fn roll_formula(formula: &str) -> Result<RollResult, String> {
    roll_formula_with(formula, &mut RandomRoller, Thresholds::STANDARD)
}

/// Roll under a technique's own crit and fumble range.
///
/// Not called yet - the attack key is what calls it. See
/// `Thresholds::new`.
#[allow(dead_code)]
pub fn roll_formula_as(formula: &str, thresholds: Thresholds) -> Result<RollResult, String> {
    roll_formula_with(formula, &mut RandomRoller, thresholds)
}

pub fn roll_formula_with<R: Roller>(
    formula: &str,
    rng: &mut R,
    thresholds: Thresholds,
) -> Result<RollResult, String> {
    let clean: String = formula
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_lowercase();

    if clean.is_empty() {
        return Err("Empty formula".to_string());
    }

    // The original's trick for signed terms: turn every '-' into '+-'
    // then split on '+'. Leading signs and empty fragments fall out.
    let spaced = clean.replace('-', "+-");
    let terms: Vec<&str> = spaced.split('+').filter(|t| !t.is_empty()).collect();

    let mut total: i64 = 0;
    let mut parts: Vec<String> = Vec::new();
    let mut natural: Option<i64> = None;
    let mut d20_terms = 0usize;

    for raw in terms {
        let (sign, term) = if let Some(rest) = raw.strip_prefix('-') {
            (-1i64, rest)
        } else {
            (1i64, raw)
        };

        if term.is_empty() {
            return Err(format!("Bad term: \"{}\"", raw));
        }

        if let Some(d) = parse_dice_term(term) {
            if d.count > 100 || d.faces > 1000 {
                return Err(format!("Roll too large: {}", term));
            }

            let rolls: Vec<i64> = (0..d.count).map(|_| rng.roll(d.faces)).collect();

            // Keep highest/lowest: sort a copy descending, then take
            // from the front (kh) or the back (kl).
            let kept: Vec<i64> = match d.keep {
                None => rolls.clone(),
                Some((kind, k)) => {
                    let k = k.min(d.count) as usize;
                    let mut sorted = rolls.clone();
                    sorted.sort_unstable_by(|a, b| b.cmp(a));
                    match kind {
                        KeepKind::High => sorted[..k].to_vec(),
                        KeepKind::Low => sorted[d.count as usize - k..].to_vec(),
                    }
                }
            };

            let sum: i64 = kept.iter().sum();
            total += sign * sum;

            if d.faces == 20 {
                d20_terms += 1;
                natural = Some(if kept.len() == 1 {
                    kept[0]
                } else {
                    kept.iter().copied().max().unwrap_or(0)
                });
            }

            // Render each die. Dropped dice are struck through; a KEPT
            // natural 20 or 1 on a d20 is bolded. The pool walk is how
            // the original handles duplicates — two 6s where only one is
            // kept must strike exactly one of them.
            let mut pool = kept.clone();
            let shown: Vec<String> = rolls
                .iter()
                .map(|&v| {
                    let kept_here = if d.keep.is_some() {
                        match pool.iter().position(|&p| p == v) {
                            Some(idx) => {
                                pool.remove(idx);
                                true
                            }
                            None => false,
                        }
                    } else {
                        true
                    };

                    if !kept_here {
                        format!("~~{}~~", v)
                    } else if d.faces == 20 && thresholds.is_marked(v) {
                        format!("**{}**", v)
                    } else {
                        v.to_string()
                    }
                })
                .collect();

            // A positive dice term carries no '+'. That asymmetry with
            // numeric terms below is in the original; keep it.
            parts.push(format!(
                "{}{} [{}]",
                if sign < 0 { "\u{2212}" } else { "" },
                term,
                shown.join(", ")
            ));
        } else if term.bytes().all(|c| c.is_ascii_digit()) {
            let n: i64 = term.parse().map_err(|_| format!("Bad term: \"{}\"", term))?;
            total += sign * n;
            parts.push(format!(
                "{}{}",
                if sign < 0 { "\u{2212}" } else { "+" },
                term
            ));
        } else {
            return Err(format!("Bad term: \"{}\"", term));
        }
    }

    // Only a single d20 term yields a natural. Two d20 terms that are
    // not a keep pair (e.g. "1d20+1d20") are not a crit candidate.
    if d20_terms != 1 {
        natural = None;
    }

    // The verdict rides with the face it was reached from, so a caller
    // can never pair one roll's natural with another roll's thresholds.
    let outcome = natural.map(|n| thresholds.verdict(n));

    Ok(RollResult {
        detail: parts.join("  "), // two spaces, as in the original
        total,
        natural,
        outcome,
    })
}

/// Double the DICE in a damage formula, leaving flat modifiers alone.
///
/// "1d6+2" becomes "2d6+2", never "2d6+4" — a crit doubles dice, and the
/// ability modifier is still added once. Ported from the CritDamage
/// column of the Attacks tab, which built `(n * 2) + 'd' + den + flat`.
///
/// Lives here because it is formula parsing and rendering, which is this
/// module's job, and because the attack resolver should not be writing a
/// second dice parser to do it.
///
/// REFUSES A KEEP TERM. Damage formulas do not have them, and doubling
/// the pool of "4d6kh3" would quietly change what the formula means
/// rather than doubling its dice.
///
/// Not called yet - the attack resolver applies it when `Outcome::Crit`
/// comes back. See `Thresholds::new`.
#[allow(dead_code)]
pub fn double_dice(formula: &str) -> Result<String, String> {
    let clean: String = formula
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_lowercase();

    if clean.is_empty() {
        return Err("Empty formula".to_string());
    }

    let spaced = clean.replace('-', "+-");
    let terms: Vec<&str> = spaced.split('+').filter(|t| !t.is_empty()).collect();

    let mut out = String::new();
    for raw in terms {
        let (neg, term) = match raw.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, raw),
        };

        if term.is_empty() {
            return Err(format!("Bad term: \"{}\"", raw));
        }

        let rendered = if let Some(d) = parse_dice_term(term) {
            if d.keep.is_some() {
                return Err(format!("Cannot double a keep term: \"{}\"", term));
            }
            format!("{}d{}", d.count * 2, d.faces)
        } else if term.bytes().all(|c| c.is_ascii_digit()) {
            term.to_string()
        } else {
            return Err(format!("Bad term: \"{}\"", term));
        };

        if out.is_empty() {
            if neg {
                out.push('-');
            }
        } else {
            out.push(if neg { '-' } else { '+' });
        }
        out.push_str(&rendered);
    }

    Ok(out)
}

/// Build the d20 formula for a named roll. Ported from d20Formula().
///
/// Not called yet: it is consumed the moment a named request like
/// "insight" can be resolved, which needs the character sheet in 002 to
/// supply the modifier. Annotated rather than deleted — it is finished,
/// tested, and deleting it would mean writing it again.
#[allow(dead_code)]
pub fn d20_formula(modifier: i64, mode: &str) -> String {
    let base = match mode {
        "adv" => "2d20kh1",
        "dis" => "2d20kl1",
        _ => "1d20",
    };
    if modifier >= 0 {
        format!("{}+{}", base, modifier)
    } else {
        format!("{}{}", base, modifier) // parse() re-reads the '-'
    }
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    fn roll(formula: &str, dice: &[i64]) -> RollResult {
        roll_as(formula, dice, Thresholds::STANDARD)
    }

    fn roll_as(formula: &str, dice: &[i64], thresholds: Thresholds) -> RollResult {
        let mut r = SequenceRoller::new(dice);
        let out = roll_formula_with(formula, &mut r, thresholds).expect("should parse");
        assert!(r.exhausted(), "test supplied more dice than the formula rolled");
        out
    }

    /// The techniques that actually exist, by their real numbers.
    fn deepsong_echo() -> Thresholds {
        Thresholds::new(18, 1).unwrap()
    }
    fn crystal_resonance() -> Thresholds {
        Thresholds::new(20, 3).unwrap()
    }

    #[test]
    fn simple_d20_with_modifier() {
        let r = roll("1d20+4", &[15]);
        assert_eq!(r.detail, "1d20 [15]  +4");
        assert_eq!(r.total, 19);
        assert_eq!(r.natural, Some(15));
    }

    #[test]
    fn bare_d_means_one_die() {
        let r = roll("d100", &[73]);
        assert_eq!(r.total, 73);
        assert_eq!(r.detail, "d100 [73]");
        assert_eq!(r.natural, None); // not a d20
    }

    #[test]
    fn natural_twenty_is_bolded() {
        let r = roll("1d20+4", &[20]);
        assert_eq!(r.detail, "1d20 [**20**]  +4");
        assert_eq!(r.natural, Some(20));
    }

    #[test]
    fn natural_one_is_bolded() {
        let r = roll("1d20+4", &[1]);
        assert_eq!(r.detail, "1d20 [**1**]  +4");
        assert_eq!(r.natural, Some(1));
        assert_eq!(r.total, 5);
    }

    #[test]
    fn advantage_keeps_the_higher_and_strikes_the_other() {
        let r = roll("2d20kh1+3", &[7, 18]);
        assert_eq!(r.detail, "2d20kh1 [~~7~~, 18]  +3");
        assert_eq!(r.total, 21);
        assert_eq!(r.natural, Some(18));
    }

    #[test]
    fn disadvantage_keeps_the_lower() {
        let r = roll("2d20kl1+3", &[18, 7]);
        assert_eq!(r.detail, "2d20kl1 [~~18~~, 7]  +3");
        assert_eq!(r.total, 10);
        assert_eq!(r.natural, Some(7));
    }

    #[test]
    fn dropped_natural_twenty_stays_struck_not_bolded() {
        // Disadvantage that rolls a 20 and throws it away. The 20 must
        // NOT read as a crit — this is the case the bolding logic is
        // ordered for.
        let r = roll("2d20kl1", &[20, 4]);
        assert_eq!(r.detail, "2d20kl1 [~~20~~, 4]");
        assert_eq!(r.natural, Some(4));
    }

    #[test]
    fn keep_highest_three_of_four_handles_duplicates() {
        // Two 4s, one kept and one dropped: exactly one gets struck.
        let r = roll("4d6kh3", &[4, 6, 4, 2]);
        assert_eq!(r.detail, "4d6kh3 [4, 6, 4, ~~2~~]");
        assert_eq!(r.total, 14);
        assert_eq!(r.natural, None);
    }

    #[test]
    fn duplicate_dropped_value_strikes_only_one() {
        // Three 5s, two kept. One 5 must be struck, two must not.
        let r = roll("3d6kh2", &[5, 5, 5]);
        assert_eq!(r.detail, "3d6kh2 [5, 5, ~~5~~]");
        assert_eq!(r.total, 10);
    }

    #[test]
    fn negative_numeric_term_uses_the_minus_sign() {
        let r = roll("d20-1", &[12]);
        assert_eq!(r.detail, "d20 [12]  \u{2212}1");
        assert_eq!(r.total, 11);
    }

    #[test]
    fn negative_dice_term_subtracts() {
        let r = roll("2d6-1d4", &[3, 5, 2]);
        assert_eq!(r.detail, "2d6 [3, 5]  \u{2212}1d4 [2]");
        assert_eq!(r.total, 6);
    }

    #[test]
    fn two_d20_terms_discard_natural() {
        // Each term sets natural; the final check throws it away
        // because crit detection is only meaningful for a single d20.
        let r = roll("1d20+1d20", &[20, 15]);
        assert_eq!(r.total, 35);
        assert_eq!(r.natural, None);
    }

    #[test]
    fn multiple_terms_join_with_two_spaces() {
        let r = roll("2d8+5+1d4", &[6, 3, 2]);
        assert_eq!(r.detail, "2d8 [6, 3]  +5  1d4 [2]");
        assert_eq!(r.total, 16);
    }

    #[test]
    fn whitespace_and_case_are_ignored() {
        let r = roll("  2D6 + 3 ", &[4, 4]);
        assert_eq!(r.detail, "2d6 [4, 4]  +3");
        assert_eq!(r.total, 11);
    }

    #[test]
    fn keep_count_defaults_to_one() {
        let r = roll("2d20kh", &[9, 14]);
        assert_eq!(r.total, 14);
        assert_eq!(r.natural, Some(14));
    }

    #[test]
    fn keep_larger_than_pool_keeps_everything() {
        let r = roll("2d6kh5", &[3, 4]);
        assert_eq!(r.detail, "2d6kh5 [3, 4]");
        assert_eq!(r.total, 7);
    }

    #[test]
    fn empty_formula_is_an_error() {
        assert!(roll_formula_with("", &mut SequenceRoller::new(&[]), Thresholds::STANDARD).is_err());
        assert!(roll_formula_with("   ", &mut SequenceRoller::new(&[]), Thresholds::STANDARD).is_err());
    }

    #[test]
    fn oversized_rolls_are_refused() {
        let e = roll_formula_with("101d6", &mut SequenceRoller::new(&[]), Thresholds::STANDARD).unwrap_err();
        assert!(e.contains("too large"), "got: {}", e);
        let e = roll_formula_with("1d1001", &mut SequenceRoller::new(&[]), Thresholds::STANDARD).unwrap_err();
        assert!(e.contains("too large"), "got: {}", e);
    }

    #[test]
    fn garbage_terms_are_refused() {
        for bad in ["2x5", "d", "1d20kx2", "abc", "2d6kh3x"] {
            assert!(
                roll_formula_with(bad, &mut SequenceRoller::new(&[1, 1, 1, 1]), Thresholds::STANDARD).is_err(),
                "{} should not parse",
                bad
            );
        }
    }

    #[test]
    fn d20_formula_shapes() {
        assert_eq!(d20_formula(4, "normal"), "1d20+4");
        assert_eq!(d20_formula(0, "normal"), "1d20+0");
        assert_eq!(d20_formula(-1, "normal"), "1d20-1");
        assert_eq!(d20_formula(3, "adv"), "2d20kh1+3");
        assert_eq!(d20_formula(3, "dis"), "2d20kl1+3");
    }

    #[test]
    fn d20_formula_output_is_parseable() {
        // The two halves have to agree: whatever d20_formula emits,
        // roll_formula must accept.
        for mode in ["normal", "adv", "dis"] {
            for m in [-3i64, 0, 5] {
                let f = d20_formula(m, mode);
                let mut r = SequenceRoller::new(&[10, 10]);
                roll_formula_with(&f, &mut r, Thresholds::STANDARD)
                    .unwrap_or_else(|e| panic!("{} failed to parse: {}", f, e));
            }
        }
    }

    #[test]
    fn random_roller_stays_in_range() {
        let mut r = RandomRoller;
        for _ in 0..500 {
            let v = r.roll(20);
            assert!((1..=20).contains(&v), "d20 out of range: {}", v);
        }
        for _ in 0..500 {
            let v = r.roll(6);
            assert!((1..=6).contains(&v), "d6 out of range: {}", v);
        }
    }

    /* ---------------- thresholds: the house rule system ------------- */

    #[test]
    fn standard_is_twenty_and_one() {
        assert_eq!(Thresholds::STANDARD.crit_min, 20);
        assert_eq!(Thresholds::STANDARD.fumble_max, 1);
        assert_eq!(Thresholds::default(), Thresholds::STANDARD);
    }

    #[test]
    fn a_widened_crit_range_calls_eighteen_a_crit() {
        let t = deepsong_echo();
        assert_eq!(t.verdict(18), Outcome::Crit);
        assert_eq!(t.verdict(19), Outcome::Crit);
        assert_eq!(t.verdict(20), Outcome::Crit);
        assert_eq!(t.verdict(17), Outcome::Normal);
    }

    #[test]
    fn a_widened_fumble_range_calls_three_a_fumble() {
        let t = crystal_resonance();
        assert_eq!(t.verdict(1), Outcome::Fumble);
        assert_eq!(t.verdict(3), Outcome::Fumble);
        assert_eq!(t.verdict(4), Outcome::Normal);
    }

    #[test]
    fn fumble_max_zero_means_it_cannot_fumble() {
        let t = Thresholds::new(20, 0).unwrap();
        assert_eq!(t.verdict(1), Outcome::Normal);
        assert_eq!(t.verdict(20), Outcome::Crit);
    }

    #[test]
    fn the_bounds_match_the_check_constraints() {
        // techniques.crit_min is `between 2 and 20`
        assert!(Thresholds::new(1, 0).is_err());
        assert!(Thresholds::new(21, 1).is_err());
        assert!(Thresholds::new(2, 0).is_ok());
        assert!(Thresholds::new(20, 1).is_ok());
        // techniques.fumble_max is `between 0 and 19`
        assert!(Thresholds::new(20, -1).is_err());
        assert!(Thresholds::new(20, 20).is_err());
    }

    #[test]
    fn overlapping_ranges_are_refused() {
        // The rule a per-column check constraint cannot express: a face
        // that is both a crit and a fumble.
        assert!(Thresholds::new(18, 18).is_err());
        assert!(Thresholds::new(18, 19).is_err());
        assert!(Thresholds::new(18, 17).is_ok());
    }

    #[test]
    fn a_refused_pair_says_which_bound_it_broke() {
        let e = Thresholds::new(18, 18).unwrap_err();
        assert!(e.contains("overlap"), "unhelpful error: {}", e);
        let e = Thresholds::new(1, 0).unwrap_err();
        assert!(e.contains("crit_min"), "unhelpful error: {}", e);
    }

    /* ---------------- thresholds applied to a roll ------------------ */

    #[test]
    fn standard_thresholds_render_exactly_as_before() {
        // Parity: the bold marking now reads Thresholds, and under the
        // standard pair it must produce the identical string.
        assert_eq!(roll("1d20+4", &[20]).detail, "1d20 [**20**]  +4");
        assert_eq!(roll("1d20+4", &[1]).detail, "1d20 [**1**]  +4");
        assert_eq!(roll("1d20+4", &[19]).detail, "1d20 [19]  +4");
    }

    #[test]
    fn a_widened_crit_is_bolded_too() {
        let r = roll_as("1d20+7", &[18], deepsong_echo());
        assert_eq!(r.detail, "1d20 [**18**]  +7");
        assert_eq!(r.outcome, Some(Outcome::Crit));
    }

    #[test]
    fn the_same_face_is_ordinary_under_standard_thresholds() {
        let r = roll("1d20+7", &[18]);
        assert_eq!(r.detail, "1d20 [18]  +7");
        assert_eq!(r.outcome, Some(Outcome::Normal));
    }

    #[test]
    fn a_widened_fumble_is_bolded_and_reported() {
        let r = roll_as("1d20+5", &[3], crystal_resonance());
        assert_eq!(r.detail, "1d20 [**3**]  +5");
        assert_eq!(r.outcome, Some(Outcome::Fumble));
    }

    #[test]
    fn advantage_keeps_the_higher_and_judges_only_that_one() {
        // The dropped die is struck through and takes no part in the
        // verdict, even when it would have been a fumble.
        let r = roll_as("2d20kh1+7", &[1, 18], deepsong_echo());
        assert_eq!(r.natural, Some(18));
        assert_eq!(r.outcome, Some(Outcome::Crit));
        assert!(r.detail.contains("~~1~~"), "dropped die not struck: {}", r.detail);
    }

    #[test]
    fn a_damage_roll_has_no_verdict_at_all() {
        // Not Normal - there was no d20, so nothing was judged.
        let r = roll("1d6+2", &[4]);
        assert_eq!(r.natural, None);
        assert_eq!(r.outcome, None);
    }

    #[test]
    fn two_d20_terms_yield_no_verdict() {
        let r = roll("1d20+1d20", &[20, 20]);
        assert_eq!(r.natural, None);
        assert_eq!(r.outcome, None);
    }

    #[test]
    fn house_dice_roll_and_are_never_judged() {
        // 1d7 and 1d14 exist in the techniques table. They parse, they
        // roll, and they are not d20s so they carry no verdict.
        let r = roll_as("1d7", &[7], deepsong_echo());
        assert_eq!(r.total, 7);
        assert_eq!(r.outcome, None);
        let r = roll_as("1d14", &[14], deepsong_echo());
        assert_eq!(r.total, 14);
        assert_eq!(r.outcome, None);
    }

    /* ---------------- doubling damage on a crit --------------------- */

    #[test]
    fn doubling_doubles_dice_and_leaves_the_modifier_alone() {
        assert_eq!(double_dice("1d6+2").unwrap(), "2d6+2");
        assert_eq!(double_dice("1d10+1").unwrap(), "2d10+1");
    }

    #[test]
    fn doubling_matches_the_attacks_tab_crit_column() {
        // The four rows the old seeder produced, damage -> CritDamage.
        assert_eq!(double_dice("1d6+2").unwrap(), "2d6+2");
        assert_eq!(double_dice("1d4+2").unwrap(), "2d4+2");
        assert_eq!(double_dice("1d10+1").unwrap(), "2d10+1");
    }

    #[test]
    fn doubling_handles_house_dice_and_multi_dice_techniques() {
        assert_eq!(double_dice("1d14").unwrap(), "2d14");
        assert_eq!(double_dice("1d7").unwrap(), "2d7");
        assert_eq!(double_dice("3d6").unwrap(), "6d6");
        assert_eq!(double_dice("3d10").unwrap(), "6d10");
    }

    #[test]
    fn a_bare_d_doubles_to_two() {
        assert_eq!(double_dice("d6").unwrap(), "2d6");
    }

    #[test]
    fn a_negative_modifier_survives_doubling() {
        assert_eq!(double_dice("1d6-1").unwrap(), "2d6-1");
    }

    #[test]
    fn doubling_refuses_a_keep_term_rather_than_changing_its_meaning() {
        let e = double_dice("4d6kh3").unwrap_err();
        assert!(e.contains("keep"), "unhelpful error: {}", e);
    }

    #[test]
    fn doubling_refuses_garbage_and_empty() {
        assert!(double_dice("mace of the deep song").is_err());
        assert!(double_dice("").is_err());
    }

    #[test]
    fn a_doubled_formula_is_still_rollable() {
        // The output has to feed straight back into the engine.
        let doubled = double_dice("1d6+2").unwrap();
        let r = roll(&doubled, &[3, 5]);
        assert_eq!(r.total, 10);
        assert_eq!(r.detail, "2d6 [3, 5]  +2");
    }
}
