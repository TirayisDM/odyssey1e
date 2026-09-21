//! Fast login: a four-digit PIN instead of an email and a password.
//!
//! WHAT THIS IS HONESTLY FOR, AND WHAT IT IS NOT
//!
//! It is convenience. The session has lived in memory since the start,
//! so every restart signs you out, and a day of switching between the
//! DM and a player is a day of retyping a password. A PIN turns that
//! into four taps.
//!
//! IT IS NOT SECURITY, and the file says so out loud rather than
//! implying otherwise:
//!
//!   The file holds a REFRESH TOKEN. That token is the credential -
//!   anyone who can read the file can mint a session from it without
//!   ever knowing the PIN. The PIN gates the app, not the token.
//!
//!   Four digits is ten thousand guesses. Salted or not, that is
//!   milliseconds for anyone who has the file and wants the PIN. The
//!   hash here keeps the PIN from being written down in plain sight;
//!   it does not make it hard to find.
//!
//! So this defends against someone picking up an unlocked machine and
//! opening the app. It defends against nothing else, and pretending
//! otherwise would be worse than not having it.
//!
//! THE REAL ANSWER is the OS keychain - Credential Manager on Windows,
//! Keychain on macOS, the Keystore on Android - which is what item 7 in
//! STATUS.md has said since before this existed. Then the token is held
//! by the operating system, the PIN becomes a genuine second factor
//! rather than the only one, and this module changes from "where the
//! token lives" to "what unlocks it".
//!
//! Split the way the rest of the codebase is: everything here is pure
//! and tested, and the commands in commands/session.rs do the file and
//! the network.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// What is written to disk. One account - this is a fast path onto the
/// device's usual user, not an account switcher.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stored {
    /// Shown on the unlock screen so it is obvious WHO is about to be
    /// signed in. A PIN that silently logs you in as the wrong account
    /// is how a DM posts as a player.
    pub email: String,
    /// THE CREDENTIAL. See the module header: the PIN does not protect
    /// this, it only gates the app that uses it.
    pub refresh_token: String,
    pub salt: String,
    pub hash: String,
}

/// Four digits, and nothing else.
///
/// Refused rather than coerced: trimming "12 34" into something that
/// works would mean the PIN a person thinks they set is not the one
/// that was set.
pub fn validate(pin: &str) -> Result<(), String> {
    if pin.len() != 4 || !pin.chars().all(|c| c.is_ascii_digit()) {
        return Err("a PIN is exactly four digits".to_string());
    }
    Ok(())
}

/// Salted SHA-256, hex.
///
/// The salt makes two people with the same PIN store different hashes
/// and stops a precomputed table being useful. It does NOT make four
/// digits hard to guess - see the module header. It is here so the PIN
/// is not sitting in the file in plain text, which is a lower bar than
/// it sounds and the only one being cleared.
pub fn hash(pin: &str, salt: &str) -> String {
    let mut h = Sha256::new();
    h.update(salt.as_bytes());
    h.update(pin.as_bytes());
    h.finalize().iter().map(|b| format!("{:02x}", b)).collect()
}

/// Constant-time-ish comparison is not the point here and pretending it
/// were would be theatre: the attack on four digits is guessing, not
/// timing. A plain comparison, honestly.
pub fn verify(pin: &str, stored: &Stored) -> bool {
    validate(pin).is_ok() && hash(pin, &stored.salt) == stored.hash
}

/// A fresh salt. Sixteen random bytes as hex, from the same source the
/// dice use.
pub fn new_salt() -> String {
    (0..16)
        .map(|_| format!("{:02x}", rand::random::<u8>()))
        .collect()
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    fn stored_with(pin: &str) -> Stored {
        let salt = new_salt();
        Stored {
            email: "tirayis.dm+p1@gmail.com".into(),
            refresh_token: "r".into(),
            hash: hash(pin, &salt),
            salt,
        }
    }

    #[test]
    fn four_digits_and_nothing_else() {
        assert!(validate("0000").is_ok());
        assert!(validate("9182").is_ok());
        for bad in ["123", "12345", "", "12 4", "abcd", "12a4", "１２３４"] {
            assert!(validate(bad).is_err(), "accepted {:?}", bad);
        }
    }

    #[test]
    fn a_leading_zero_is_a_digit_like_any_other() {
        // "0042" must not become 42 anywhere along the way.
        let s = stored_with("0042");
        assert!(verify("0042", &s));
        assert!(!verify("42", &s));
    }

    #[test]
    fn the_right_pin_opens_it_and_a_wrong_one_does_not() {
        let s = stored_with("1234");
        assert!(verify("1234", &s));
        assert!(!verify("1235", &s));
        assert!(!verify("4321", &s));
    }

    #[test]
    fn the_same_pin_stores_differently_for_two_people() {
        // What the salt is for. Identical PINs must not produce
        // identical hashes, or one leak reads across every account.
        let a = stored_with("1111");
        let b = stored_with("1111");
        assert_ne!(a.salt, b.salt);
        assert_ne!(a.hash, b.hash);
        assert!(verify("1111", &a) && verify("1111", &b));
    }

    #[test]
    fn a_malformed_pin_is_refused_before_it_is_hashed() {
        // verify() must not accept something validate() would reject,
        // whatever the stored hash happens to be.
        let mut s = stored_with("1234");
        s.hash = hash("12345", &s.salt);
        assert!(!verify("12345", &s));
    }

    #[test]
    fn the_hash_is_hex_and_the_full_width() {
        let s = stored_with("5555");
        assert_eq!(s.hash.len(), 64, "sha-256 is 32 bytes");
        assert!(s.hash.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(s.salt.len(), 32, "16 bytes of salt");
    }
}
