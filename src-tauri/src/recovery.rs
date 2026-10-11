//! Getting back in when the password is gone.
//!
//! 202. THE RULES HALF OF PASSWORD RESET, which is almost entirely one
//! question: WHAT DID THE USER JUST PASTE?
//!
//! ---------------------------------------------------------------------
//! WHY THIS IS A PROBLEM AT ALL
//! ---------------------------------------------------------------------
//!
//! Supabase's recovery flow is built for a website. `/auth/v1/recover`
//! emails a LINK; the link lands on your site; your JavaScript reads the
//! session out of the URL fragment and calls `updateUser`. **This app is
//! not a website.** There is nowhere for that link to land, and the
//! `SITE_URL` the email points at is not ours to serve.
//!
//! GoTrue offers the other half of the same door: `/auth/v1/verify` with
//! `type: "recovery"` takes a SIX-DIGIT CODE and hands back a real
//! session. That is the desktop-shaped path - the user reads a number
//! off their phone and types it in, the way every other desktop
//! application in the world does it.
//!
//! ---------------------------------------------------------------------
//! EXCEPT THE DEFAULT EMAIL DOES NOT CONTAIN THE CODE
//! ---------------------------------------------------------------------
//!
//! `{{ .Token }}` is the six digits and the stock "Reset password"
//! template does not use it - it uses `{{ .ConfirmationURL }}` and
//! nothing else. The code EXISTS for every recovery email ever sent;
//! the template simply does not print it.
//!
//! So a code-only flow would be a flow that does not work until
//! somebody edits a template in the Supabase console - and console
//! settings are not in git, which 199 already learned the hard way.
//! Shipping a reset button that cannot work until an unrecorded
//! console change is made is a fault wearing the costume of a feature.
//!
//! **SO THIS TAKES EITHER.** The confirmation URL carries the token
//! hash as a query parameter, and `/auth/v1/verify` accepts a
//! `token_hash` just as happily as a `token`. Paste the whole link from
//! the stock email and it works TODAY. Change the template to print
//! `{{ .Token }}` and six digits work too, which is nicer. Neither
//! choice is load-bearing, and the user is never asked which kind of
//! thing they have.
//!
//! ---------------------------------------------------------------------
//! THE PARAMETER IS CALLED `token` AND HOLDS A HASH
//! ---------------------------------------------------------------------
//!
//! This is the trap in the middle of the whole module. The documented
//! shape of a confirmation URL is
//!
//! ```text
//! .../auth/v1/verify?token={{ .TokenHash }}&type=recovery&redirect_to=...
//! ```
//!
//! A parameter spelled `token` whose value is a `TokenHash`. Reading it
//! and posting it back as `token` would be the obvious, wrong thing -
//! GoTrue would compare a hash against a six-digit code and refuse it,
//! and the message would be about an invalid token rather than about the
//! field being wrong. **Anything that comes out of a URL is a HASH here,
//! whatever the parameter calls itself.**

/// What the user has in their hand, out of a recovery email.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Proof {
    /// Six digits from `{{ .Token }}`. Verified together with the email
    /// address, because a six-digit number is not unique on its own.
    Code(String),
    /// A `TokenHash`, from the confirmation link or pasted bare. Stands
    /// alone - the hash identifies the request, so no email is needed
    /// and none is sent.
    Hash(String),
}

impl Proof {
    /// What to call this in a message to the user.
    pub fn said(&self) -> &'static str {
        match self {
            Proof::Code(_) => "code",
            Proof::Hash(_) => "link",
        }
    }
}

/// The value of a query parameter in a pasted URL, if it is there.
///
/// Hand-rolled rather than pulling in a URL parser, because the input is
/// not reliably a URL - it may be a bare token somebody copied out of
/// one - and a parser would reject the input this is most needed for.
fn param(text: &str, name: &str) -> Option<String> {
    let needle = format!("{}=", name);
    // `&` first so `?token=` and `&token=` both match, and so that
    // `token_hash=` cannot be found by a search for `token=`.
    for sep in ['?', '&'] {
        let pat = format!("{}{}", sep, needle);
        if let Some(at) = text.find(&pat) {
            let rest = &text[at + pat.len()..];
            let end = rest.find(['&', '#']).unwrap_or(rest.len());
            let v = rest[..end].trim();
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

/// Could this be a token hash somebody copied on its own?
///
/// DELIBERATELY LOOSE, and the length floor is what makes it safe. The
/// only other thing this function has to be told apart from is a
/// six-digit code, and 20 is far above six. Being generous here costs a
/// clear rejection from GoTrue; being mean costs a user who pasted a
/// perfectly good token being told it is not one.
fn looks_like_a_bare_token(s: &str) -> bool {
    s.len() >= 20
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Read whatever the user pasted.
///
/// ORDER MATTERS AND IS NOT ALPHABETICAL. A URL is checked before a
/// code, because a `redirect_to` can carry digits and a URL is never a
/// code. `token_hash` is checked before `token` so an email template
/// that was changed to the newer spelling still works.
pub fn read_proof(input: &str) -> Result<Proof, String> {
    let text = input.trim();
    if text.is_empty() {
        return Err("paste the code or the link from the email".to_string());
    }

    // A pasted URL that already carries a session is a different animal
    // and worth naming, because the user cannot tell by looking.
    if text.contains("access_token=") {
        return Err("that link has already been opened - ask for a new email".to_string());
    }

    if text.contains("://") || text.contains('?') || text.contains('&') {
        // See the module note: ANYTHING out of a URL is a hash.
        for name in ["token_hash", "token"] {
            if let Some(v) = param(text, name) {
                return Ok(Proof::Hash(v));
            }
        }
        return Err("that link has no token in it - paste the whole link, or the 6-digit code"
            .to_string());
    }

    // Whitespace inside a code is how people read numbers aloud.
    let tight: String = text.chars().filter(|c| !c.is_whitespace()).collect();

    if tight.len() == 6 && tight.chars().all(|c| c.is_ascii_digit()) {
        return Ok(Proof::Code(tight));
    }
    if looks_like_a_bare_token(&tight) {
        return Ok(Proof::Hash(tight));
    }
    if tight.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!(
            "a reset code is 6 digits and that is {} - check the email again",
            tight.len()
        ));
    }
    Err("that is neither a 6-digit code nor a reset link".to_string())
}

/// The two boxes agree and neither is empty.
///
/// ---------------------------------------------------------------------
/// AND IT CHECKS NOTHING ABOUT STRENGTH, ON PURPOSE
/// ---------------------------------------------------------------------
///
/// 199 set the real rules in the Supabase console - ten characters,
/// upper and lower and digits and symbols - and wrote down that **the
/// console is the only record of them.** Restating them here would make
/// this the second place one rule lives, which is the fault this project
/// keeps finding: the console could be retuned tomorrow and this file
/// would go on confidently enforcing last month's policy, rejecting
/// passwords the server would have accepted.
///
/// GoTrue's own refusal is specific - it names the length or the missing
/// character class - so the user is told what is wrong either way. What
/// is checked here is only what is OURS to know: that there are two
/// boxes and they match, which no server can tell us.
/// Is this worth sending to GoTrue at all?
///
/// 203. THE CHEAPEST POSSIBLE CHECK, and deliberately nothing like a
/// real one. Typing `t` into the address box and pressing the button
/// cost a network round trip to be told `Unable to validate email
/// address: invalid format (400)` - a true answer, from the wrong end
/// of a wire, to a question that could be settled here.
///
/// IT IS NOT A SECOND COPY OF ANYBODY'S EMAIL RULES. GoTrue remains the
/// authority and still sees every address this lets past; what this
/// catches is the empty box and the half-typed word, which are the only
/// two mistakes that actually happen. Anything resembling an address
/// goes to the server to be judged properly - no dot-counting, no
/// domain list, none of the folklore that rejects real addresses.
pub fn looks_like_an_email(s: &str) -> bool {
    let s = s.trim();
    match s.split_once('@') {
        Some((before, after)) => {
            !before.is_empty() && !after.is_empty() && !s.chars().any(char::is_whitespace)
        }
        None => false,
    }
}

pub fn check_new_password(new: &str, confirm: &str) -> Result<(), String> {
    if new.is_empty() {
        return Err("choose a new password".to_string());
    }
    if new != confirm {
        return Err("the two passwords do not match".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_digits_is_a_code() {
        assert_eq!(read_proof("123456"), Ok(Proof::Code("123456".into())));
        assert_eq!(read_proof("  123456 "), Ok(Proof::Code("123456".into())));
    }

    #[test]
    fn a_code_read_aloud_still_works() {
        // People copy "123 456" out of an email without thinking.
        assert_eq!(read_proof("123 456"), Ok(Proof::Code("123456".into())));
    }

    #[test]
    fn a_leading_zero_survives() {
        // The obvious way to get this wrong is to parse it as a number.
        assert_eq!(read_proof("000123"), Ok(Proof::Code("000123".into())));
    }

    #[test]
    fn the_stock_confirmation_url_gives_a_hash() {
        // THE SHAPE THAT SHIPS TODAY, and the parameter is called
        // `token` while holding a hash. See the module note.
        let url = "https://shraejtytdxmoxmxkwxq.supabase.co/auth/v1/verify\
                   ?token=abc123def456ghi789jkl&type=recovery\
                   &redirect_to=https://example.com";
        assert_eq!(
            read_proof(url),
            Ok(Proof::Hash("abc123def456ghi789jkl".into()))
        );
    }

    #[test]
    fn a_redirect_full_of_digits_does_not_become_a_code() {
        // The reason URLs are tested before codes.
        let url = "https://x.supabase.co/auth/v1/verify?token=zzzzzzzzzzzzzzzzzzzzzz\
                   &type=recovery&redirect_to=https://example.com/123456";
        assert_eq!(
            read_proof(url),
            Ok(Proof::Hash("zzzzzzzzzzzzzzzzzzzzzz".into()))
        );
    }

    #[test]
    fn the_newer_spelling_wins_when_both_are_there() {
        let url = "https://x/auth/v1/verify?token=oldoldoldoldoldoldold\
                   &token_hash=newnewnewnewnewnewnew&type=recovery";
        assert_eq!(
            read_proof(url),
            Ok(Proof::Hash("newnewnewnewnewnewnew".into()))
        );
    }

    #[test]
    fn token_hash_is_not_found_by_searching_for_token() {
        // `param` anchors on `?` or `&` for exactly this reason: a
        // sloppy `contains("token=")` matches inside `token_hash=`.
        let url = "https://x/auth/v1/verify?token_hash=hhhhhhhhhhhhhhhhhhhh&type=recovery";
        assert_eq!(read_proof(url), Ok(Proof::Hash("hhhhhhhhhhhhhhhhhhhh".into())));
    }

    #[test]
    fn a_bare_token_pasted_on_its_own_is_a_hash() {
        let t = "pkce_9f8e7d6c5b4a39281706";
        assert_eq!(read_proof(t), Ok(Proof::Hash(t.into())));
    }

    #[test]
    fn a_link_that_has_already_been_opened_says_so() {
        // The fragment form, which is what a browser leaves behind. The
        // user cannot tell this from a fresh link by looking at it.
        let e = read_proof("https://example.com/#access_token=eyJ...&type=recovery").unwrap_err();
        assert!(e.contains("already been opened"), "{}", e);
    }

    #[test]
    fn a_url_with_no_token_is_refused_as_a_url() {
        let e = read_proof("https://example.com/reset?type=recovery").unwrap_err();
        assert!(e.contains("no token"), "{}", e);
    }

    #[test]
    fn nothing_pasted_asks_for_something() {
        assert!(read_proof("").is_err());
        assert!(read_proof("    ").is_err());
    }

    #[test]
    fn the_wrong_number_of_digits_says_how_many() {
        // A MORE USEFUL ERROR THAN "not a code". Mistyping or dropping
        // a digit is the likeliest failure of the whole flow.
        let e = read_proof("12345").unwrap_err();
        assert!(e.contains("6 digits") && e.contains("that is 5"), "{}", e);
    }

    #[test]
    fn a_short_piece_of_nonsense_is_neither() {
        let e = read_proof("hello").unwrap_err();
        assert!(e.contains("neither"), "{}", e);
    }

    #[test]
    fn what_to_call_it_in_a_message() {
        assert_eq!(Proof::Code("123456".into()).said(), "code");
        assert_eq!(Proof::Hash("x".into()).said(), "link");
    }

    #[test]
    fn a_half_typed_address_never_reaches_the_wire() {
        // The actual failure: one character in the box.
        assert!(!looks_like_an_email("t"));
        assert!(!looks_like_an_email(""));
        assert!(!looks_like_an_email("   "));
        assert!(!looks_like_an_email("tirayis.dm"));
        assert!(!looks_like_an_email("@gmail.com"));
        assert!(!looks_like_an_email("tirayis.dm@"));
    }

    #[test]
    fn anything_resembling_an_address_is_left_to_gotrue() {
        assert!(looks_like_an_email("tirayis.dm+dm@gmail.com"));
        assert!(looks_like_an_email("  tirayis.dm@gmail.com  "));
        // NO DOT IS REQUIRED. `dave@localhost` is a real address and
        // the folklore check that rejects it is how valid addresses get
        // refused by forms. GoTrue decides, not this.
        assert!(looks_like_an_email("dave@localhost"));
    }

    #[test]
    fn two_matching_passwords_pass() {
        assert!(check_new_password("whatever", "whatever").is_ok());
    }

    #[test]
    fn a_typo_in_the_second_box_is_caught_here() {
        let e = check_new_password("Sturdy-Pass-1", "Sturdy-Pass-2").unwrap_err();
        assert!(e.contains("do not match"), "{}", e);
    }

    #[test]
    fn an_empty_new_password_is_refused_before_the_round_trip() {
        assert!(check_new_password("", "").is_err());
    }

    #[test]
    fn strength_is_not_judged_here() {
        // 199's rules live in the console and this is not a second copy
        // of them. A weak password passes THIS check and is refused by
        // GoTrue, which is the only thing that knows the real policy.
        assert!(check_new_password("a", "a").is_ok());
        assert!(check_new_password("password", "password").is_ok());
    }
}
