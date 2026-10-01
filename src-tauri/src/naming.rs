//! What a character is called, and what follows from changing it.
//!
//! A character carries two names. `name` is the whole of it - "Rodnar
//! Shieldcrest" - and `token_name` is the short one that roll cards and
//! the roster print, "Rodnar". 005 made the second nullable because for
//! most people there is no difference worth storing, and NULL means
//! "the long one will do".
//!
//! THAT SECOND NAME IS WHY RENAMING IS NOT ONE UPDATE. Every screen in
//! the app prints `token_name || name`, so a rename that moves only
//! `name` leaves the roster, the initiative strip and every future roll
//! card still saying the old one. The sheet heading would change and
//! nothing else would, which is the same shape as `edit_object`
//! repainting one tab: a write that lands, and a screen that does not
//! show it.
//!
//! NOT SOLVED BY OVERWRITING BOTH. A short name that genuinely differs
//! from the long one is information somebody typed on purpose, and
//! renaming "Rodnar Shieldcrest" to "Rodnar Oathbreaker" must not throw
//! "Rodnar" away. So the rule is narrow and it is written here, once,
//! where it can be tested - see `renamed_token`.

/// What `token_name` should become when `name` changes.
///
/// THREE CASES, AND THE MIDDLE ONE IS THE POINT:
///
/// ```text
///   old name        old token    new name            result
///   Test PC 1       NULL         Garn                NULL     unchanged
///   Snot            Snot         Grisk               Grisk    follows
///   Rodnar Shield.  Rodnar       Rodnar Oathbreaker  Rodnar   kept
/// ```
///
/// A token name that was a COPY of the long one was never a separate
/// fact, so it follows the rename. One that differs was a decision, and
/// a rename is not permission to undo it.
///
/// Compared on trimmed text and ignoring case, because "Snot" and
/// "snot " were the same decision however they were typed.
pub fn renamed_token(old_name: &str, old_token: Option<&str>, new_name: &str) -> Option<String> {
    let token = old_token?;
    if token.trim().eq_ignore_ascii_case(old_name.trim()) {
        Some(new_name.trim().to_string())
    } else {
        Some(token.to_string())
    }
}

/// A name somebody can be called, or why this one will not do.
///
/// REFUSED RATHER THAN CORRECTED. `characters.name` is NOT NULL and an
/// empty string would satisfy that while naming nobody - a row that
/// prints as a blank line on every screen and cannot be picked out of a
/// list. An accidental clear is far more likely than a deliberate one,
/// so it is an error rather than a silent no-op.
pub fn clean(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("a character needs a name".to_string());
    }
    Ok(trimmed.to_string())
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_short_name_stays_no_short_name() {
        // Every character made by the creation form is this case -
        // `token_name` is sent as null and nothing has ever set one.
        assert_eq!(renamed_token("Test PC 1", None, "Garn"), None);
    }

    #[test]
    fn a_short_name_that_was_a_copy_follows_the_rename() {
        // The NPC case. `instantiate_npc` writes the same text into
        // both, so a goblin renamed Grisk must not keep answering to
        // Snot on the roster.
        assert_eq!(
            renamed_token("Snot", Some("Snot"), "Grisk"),
            Some("Grisk".to_string())
        );
    }

    #[test]
    fn a_short_name_that_differs_is_a_decision_and_survives() {
        assert_eq!(
            renamed_token("Rodnar Shieldcrest", Some("Rodnar"), "Rodnar Oathbreaker"),
            Some("Rodnar".to_string())
        );
    }

    #[test]
    fn case_and_padding_do_not_make_it_a_different_decision() {
        // "snot " in the short field was not somebody choosing a
        // nickname, it was the same word typed carelessly.
        assert_eq!(
            renamed_token("Snot", Some("snot "), "Grisk"),
            Some("Grisk".to_string())
        );
    }

    #[test]
    fn the_new_short_name_is_trimmed_like_the_long_one() {
        assert_eq!(
            renamed_token("Snot", Some("Snot"), "  Grisk  "),
            Some("Grisk".to_string())
        );
    }

    #[test]
    fn renaming_to_the_same_thing_changes_nothing() {
        assert_eq!(
            renamed_token("Snot", Some("Snot"), "Snot"),
            Some("Snot".to_string())
        );
    }

    #[test]
    fn a_name_is_trimmed() {
        assert_eq!(clean("  Garn  "), Ok("Garn".to_string()));
    }

    #[test]
    fn nobody_is_called_nothing() {
        assert!(clean("").is_err());
        assert!(clean("   ").is_err());
        assert!(clean("\t\n").is_err());
    }

    #[test]
    fn inner_spacing_is_left_alone() {
        // Only the ends are the accident. A double space in the middle
        // is not this function's business to judge.
        assert_eq!(clean("Rodnar  Shieldcrest"), Ok("Rodnar  Shieldcrest".to_string()));
    }
}
