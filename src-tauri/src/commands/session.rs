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
