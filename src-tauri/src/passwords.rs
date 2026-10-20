//! What this app is allowed to decide about a password.
//!
//! 223. ONE FUNCTION, AND THE INTERESTING PART IS WHAT IS NOT HERE.
//!
//! This was `recovery.rs` until the reset flow was taken out - see
//! STATUS 223 for why, and commit `158effd` for the code, which was
//! complete and tested and failed on CONFIGURATION rather than on
//! anything it did. What survived is the half password CHANGE needs.

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
