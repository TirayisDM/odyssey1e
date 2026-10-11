//! Who is signed in.
//!
//! THE EXEMPLAR. `me` moved here on its own, ahead of the rest of its
//! group, to prove the one thing about this split that could have been
//! wrong: that `generate_handler!` takes a path into a submodule, and
//! that the command keeps its bare name on the wire so the frontend's
//! `invoke("me")` never learns anything happened.
//!
//! `sign_up`, `sign_in` and `sign_out` are still in lib.rs. They write
//! to `AppState` rather than only reading it, and there is no reason to
//! move them until somebody is editing them anyway.

use crate::supabase::{AppState, Session};
use tauri::State;

/// Who am I, or null. Lets the frontend render without guessing.
///
/// Deliberately NOT built on `Ctx`: `Ctx::of` fails when nobody is
/// signed in, and "nobody is signed in" is a perfectly good answer to
/// this particular question rather than an error.
#[tauri::command]
pub fn me(state: State<AppState>) -> Result<Option<Session>, String> {
    state.current()
}

/* ======================== FAST LOGIN BY PIN ======================== */

use crate::pin;
use crate::supabase;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

/// Where the fast-login file lives: the OS application-data directory,
/// not beside the executable. A dev build and an installed one then
/// share nothing by accident, and the path is the one the platform
/// already expects an app to write to.
/// 200. THE SAME PATH, reachable from `run()` so the session state can
/// be told where to write a rotated refresh token.
pub fn pin_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    pin_file(app)
}

fn pin_file(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("no application data directory: {}", e))?;
    fs::create_dir_all(&dir).map_err(|e| format!("could not make {}: {}", dir.display(), e))?;
    Ok(dir.join("fast-login.json"))
}

fn read_stored(app: &tauri::AppHandle) -> Option<pin::Stored> {
    let path = pin_file(app).ok()?;
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Is a PIN set on this device, and for whom.
///
/// The email is returned so the unlock screen can SAY who it is about to
/// sign in as. A PIN that quietly signs you in as the wrong account is
/// how a DM rolls as a player.
#[tauri::command]
pub fn pin_status(app: tauri::AppHandle) -> Value {
    match read_stored(&app) {
        Some(s) => json!({ "set": true, "email": s.email }),
        None => json!({ "set": false, "email": null }),
    }
}

/// Set a PIN for whoever is signed in now.
///
/// Requires a live session, because what gets stored is that session's
/// refresh token. There is no way to set a PIN for an account you are
/// not currently holding, which is the property that keeps this from
/// being an account-stealing feature.
#[tauri::command]
pub fn set_pin(
    app: tauri::AppHandle,
    state: State<AppState>,
    pin_code: String,
) -> Result<(), String> {
    pin::validate(&pin_code)?;
    let s: Session = state
        .current()?
        .ok_or_else(|| "sign in first — a PIN stores the session you already have".to_string())?;

    let salt = pin::new_salt();
    let stored = pin::Stored {
        email: s.email.clone(),
        refresh_token: s.refresh_token.clone(),
        hash: pin::hash(&pin_code, &salt),
        salt,
    };

    let path = pin_file(&app)?;
    fs::write(
        &path,
        serde_json::to_string_pretty(&stored).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("could not write {}: {}", path.display(), e))
}

/// Unlock with the PIN.
///
/// A WRONG PIN NEVER REACHES THE NETWORK. The check is local and the
/// refusal is local, so a bad guess costs nothing and tells Supabase
/// nothing.
///
/// A right one spends the stored refresh token for a live session — and
/// Supabase issues a NEW refresh token in the same breath, invalidating
/// the old one. So the file is rewritten immediately. Forgetting that
/// would make the PIN work exactly once.
#[tauri::command]
pub fn unlock(
    app: tauri::AppHandle,
    state: State<AppState>,
    pin_code: String,
) -> Result<Session, String> {
    let stored = read_stored(&app).ok_or_else(|| "no PIN is set on this device".to_string())?;
    if !pin::verify(&pin_code, &stored) {
        return Err("that is not the PIN".to_string());
    }

    let session = supabase::refresh(&stored.refresh_token).map_err(|e| {
        // The token has expired or been revoked. Say what to do about
        // it rather than leaving a PIN that will never work again.
        format!("{} — sign in with your password and set the PIN again", e)
    })?;

    let refreshed = pin::Stored {
        email: session.email.clone(),
        refresh_token: session.refresh_token.clone(),
        ..stored
    };
    if let Ok(path) = pin_file(&app) {
        if let Ok(text) = serde_json::to_string_pretty(&refreshed) {
            let _ = fs::write(path, text);
        }
    }

    state.set(Some(session.clone()))?;
    Ok(session)
}

/// Forget the PIN and the token with it.
///
/// Deletes the file rather than blanking it: a file that exists with
/// nothing in it is a thing someone has to reason about later.
#[tauri::command]
pub fn forget_pin(app: tauri::AppHandle) -> Result<(), String> {
    let path = pin_file(&app)?;
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        // Already gone is the outcome that was asked for.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("could not remove {}: {}", path.display(), e)),
    }
}

/* ===================== GETTING BACK IN, 202 ======================= */

use crate::recovery::{self, Proof};

/// Ask for a reset email.
///
/// 202. SAYS NOTHING ABOUT WHETHER THAT ADDRESS HAS AN ACCOUNT, because
/// `send_recovery` cannot and should not. See its comment: an endpoint
/// that distinguished "sent" from "no such user" would let anyone work
/// out who plays in this campaign.
#[tauri::command]
pub fn begin_reset(email: String) -> Result<(), String> {
    let email = email.trim();
    if email.is_empty() {
        return Err("type the email address you signed up with".to_string());
    }
    if !recovery::looks_like_an_email(email) {
        return Err(format!("\"{}\" is not an email address", email));
    }
    supabase::send_recovery(email)
}

/// Spend the code or link, set the new password, and sign in.
///
/// ---------------------------------------------------------------------
/// THE LOCAL CHECKS COME FIRST, AND THAT IS THE WHOLE ORDERING
/// ---------------------------------------------------------------------
///
/// **A RECOVERY TOKEN IS SINGLE-USE.** Verifying before checking that
/// the two password boxes match would spend the token on a TYPO - the
/// user would be told the passwords differ, try again, and find the code
/// dead. They would then need a second email to recover from a
/// mistyped character.
///
/// So everything that can be known without the network is known first.
/// `check_new_password` and `read_proof` are both pure, both tested, and
/// both run before a single request goes out.
///
/// ---------------------------------------------------------------------
/// AND IF THE SECOND HALF FAILS, THE TOKEN IS ALREADY GONE
/// ---------------------------------------------------------------------
///
/// Verify and update are two requests and the first one is irreversible.
/// If GoTrue accepts the code and then refuses the password - too short,
/// too simple, 199's rules - the code cannot be used again. That is not
/// avoidable here, so it is SAID instead: the error names it and tells
/// the user to ask for a new email.
///
/// NO SESSION IS SEATED IN THAT CASE even though a valid one now exists.
/// Being silently signed in with the old password still set, after being
/// told the reset failed, is the kind of half-outcome nobody can reason
/// about later.
#[tauri::command]
pub fn finish_reset(
    app: tauri::AppHandle,
    state: State<AppState>,
    email: String,
    pasted: String,
    new_password: String,
    confirm: String,
) -> Result<Session, String> {
    recovery::check_new_password(&new_password, &confirm)?;
    let proof = recovery::read_proof(&pasted)?;

    // A code is not unique on its own and needs the address beside it.
    // A link carries its own identity, so a wrong address cannot break
    // it - and demanding one would be a question with no purpose.
    if matches!(proof, Proof::Code(_)) && email.trim().is_empty() {
        return Err("type your email address as well as the code".to_string());
    }

    let session = supabase::verify_recovery(&proof, email.trim())
        .map_err(|e| format!("that {} was refused: {}", proof.said(), e))?;

    // THE RECOVERY SESSION IS THE AUTHORITY FOR THIS WRITE, and no
    // current password is sent: the user is here precisely because they
    // do not have it. GoTrue exempts a recovery session from the
    // "require current password" guard for that reason.
    supabase::set_password(&session.access_token, &new_password, None).map_err(|e| {
        format!(
            "{} — the {} has now been used up, so ask for a new email before trying again",
            e,
            proof.said()
        )
    })?;

    // ---------------------------------------------------------------
    // THE PIN IS FORGOTTEN, AND THIS IS A DECISION RATHER THAN TIDYING
    // ---------------------------------------------------------------
    //
    // The stored refresh token belongs to the session that existed
    // before the password moved, so it is dead or dying either way and
    // reseating it from the new session would be easy.
    //
    // It is not done because A RESET IS A SECURITY EVENT. Somebody
    // either forgot their password or is recovering an account they
    // think was reached by someone else, and `pin.rs` is honest that
    // the file is "anyone who can read the file can mint a session".
    // Leaving a four-digit door standing on this machine immediately
    // after a reset is the wrong trade. Setting a new PIN is two taps
    // and the panel offers it the moment this returns.
    if let Ok(path) = pin_file(&app) {
        let _ = fs::remove_file(path);
    }

    state.set(Some(session.clone()))?;
    Ok(session)
}

/// Rewrite the PIN file's token, keeping the hash and salt.
///
/// 203. The same three lines `unlock` has always had, lifted out now
/// that a second caller needs them. Best effort and silent, for 200's
/// reason: a PIN that cannot be rewritten is a recoverable nuisance,
/// and failing the operation the user actually asked for because of it
/// is the wrong trade.
fn reseat_stored(app: &tauri::AppHandle, session: &Session) {
    let Some(stored) = read_stored(app) else { return };
    let refreshed = pin::Stored {
        email: session.email.clone(),
        refresh_token: session.refresh_token.clone(),
        ..stored
    };
    if let Ok(path) = pin_file(app) {
        if let Ok(text) = serde_json::to_string_pretty(&refreshed) {
            let _ = fs::write(path, text);
        }
    }
}

/// Change the password while signed in and knowing the old one.
///
/// 202. A DIFFERENT FLOW FROM `finish_reset`, not a special case of it.
/// This one knows the current password and has no email round trip;
/// that one knows neither and is gated on reaching the inbox. Folding
/// them together would mean one function whose rules depend on which
/// arguments are null.
///
/// ---------------------------------------------------------------------
/// 203. IT SIGNS IN AGAIN FIRST, AND THAT IS THE WHOLE TRICK
/// ---------------------------------------------------------------------
///
/// 202 shipped this sending only `current_password`, and every single
/// attempt came back `Password update requires reauthentication (400)`.
/// 199's OTHER guard - "secure password change" - wants the SESSION to
/// have been created recently, and 202 guessed that would be a rare
/// edge case after 24 hours. **It is the normal case and always was.**
///
/// A PIN unlock spends a refresh token, and a refresh EXTENDS a session
/// rather than creating one. Everybody who uses the PIN - which is the
/// whole point of the PIN - is carrying a session whose creation date
/// never moves. 200 then made those sessions renew indefinitely. So the
/// session is permanently live and permanently old, and this command
/// could never once have worked for its most likely user.
///
/// 202's advice was wrong as well as its code: "sign out and sign back
/// in" lands on the unlock screen, where signing back in means the PIN,
/// which produces exactly the same stale session again.
///
/// THE FIX NEEDS NO NONCE AND NO EMAIL. This command already demands
/// the current password, so it spends it on a fresh `sign_in`. That one
/// call does three jobs at once:
///
///   - it PROVES the current password, with a far better error than
///     GoTrue gives for a refused `current_password`
///   - it creates a session made SECONDS ago, which is what the guard
///     is actually asking for
///   - it needs nothing from the inbox, so it works whether or not this
///     project can send mail - which 202 noted is still unverified
///
/// `current_password` still rides along on the update. Both guards are
/// on and both are now satisfied honestly rather than worked around.
///
/// ---------------------------------------------------------------------
/// AND IT SIGNS IN ONCE MORE AFTERWARDS
/// ---------------------------------------------------------------------
///
/// Changing a password can revoke the sessions that existed under the
/// old one, including the one that just did the changing. Rather than
/// reason about which survive, this takes the certain route: sign in
/// with the NEW password and seat that.
///
/// IF THAT LAST STEP FAILS THE PASSWORD HAS STILL CHANGED, and saying
/// otherwise would be the worst available lie - the user would try the
/// old password for ever. So it reports success and says what to do.
#[tauri::command]
pub fn change_password(
    app: tauri::AppHandle,
    state: State<AppState>,
    current_password: String,
    new_password: String,
    confirm: String,
) -> Result<String, String> {
    if current_password.is_empty() {
        return Err("type your current password".to_string());
    }
    recovery::check_new_password(&new_password, &confirm)?;
    if new_password == current_password {
        return Err("that is the password you already have".to_string());
    }

    let who = state
        .current()?
        .ok_or_else(|| "sign in before changing your password".to_string())?;

    let proven = supabase::sign_in(&who.email, &current_password)
        .map_err(|e| format!("could not confirm your current password: {}", e))?;

    supabase::set_password(&proven.access_token, &new_password, Some(&current_password))?;

    // From here the password HAS changed and nothing below may report
    // failure, only what to do next.
    match supabase::sign_in(&who.email, &new_password) {
        Ok(fresh) => {
            reseat_stored(&app, &fresh);
            let _ = state.set(Some(fresh));
            Ok("Password changed.".to_string())
        }
        Err(e) => Ok(format!(
            "Password changed - but signing back in with it failed ({}). Sign out and in again.",
            e
        )),
    }
}
