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

pub fn roll_formula(formula: &str) -> Result<RollResult, String> {
    roll_formula_with(formula, &mut RandomRoller)
}

pub fn roll_formula_with<R: Roller>(formula: &str, rng: &mut R) -> Result<RollResult, String> {
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
                    } else if d.faces == 20 && (v == 20 || v == 1) {
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

    Ok(RollResult {
        detail: parts.join("  "), // two spaces, as in the original
        total,
        natural,
    })
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
        let mut r = SequenceRoller::new(dice);
        let out = roll_formula_with(formula, &mut r).expect("should parse");
        assert!(r.exhausted(), "test supplied more dice than the formula rolled");
        out
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
        assert!(roll_formula_with("", &mut SequenceRoller::new(&[])).is_err());
        assert!(roll_formula_with("   ", &mut SequenceRoller::new(&[])).is_err());
    }

    #[test]
    fn oversized_rolls_are_refused() {
        let e = roll_formula_with("101d6", &mut SequenceRoller::new(&[])).unwrap_err();
        assert!(e.contains("too large"), "got: {}", e);
        let e = roll_formula_with("1d1001", &mut SequenceRoller::new(&[])).unwrap_err();
        assert!(e.contains("too large"), "got: {}", e);
    }

    #[test]
    fn garbage_terms_are_refused() {
        for bad in ["2x5", "d", "1d20kx2", "abc", "2d6kh3x"] {
            assert!(
                roll_formula_with(bad, &mut SequenceRoller::new(&[1, 1, 1, 1])).is_err(),
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
                roll_formula_with(&f, &mut r)
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
}
