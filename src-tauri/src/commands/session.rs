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
