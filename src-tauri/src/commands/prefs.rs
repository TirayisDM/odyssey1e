//! What your die looks like.
//!
//! 292. The first preference in this application that is about a
//! PERSON rather than a machine. The other two - whether the feed is
//! open, which of its tabs you left showing - are in `localStorage`
//! on purpose, because they are facts about the laptop in front of
//! you. The colour of your die is not: sit down somewhere else and it
//! should still be yours.
//!
//! NOTHING IN HERE DECIDES ANYTHING, which is what this directory is
//! for. Which colours exist is stated by the CHECK constraints in
//! 292; what any of them looks like is stated by the stylesheet. This
//! file carries a name between them.

use crate::commands::Ctx;
use crate::supabase::{self, AppState};
use serde_json::{json, Value};
use tauri::State;

/// One row out of what PostgREST hands back, or the plain refusal
/// rather than a silent `null` the caller would have to interpret.
fn one(rows: Value) -> Result<Value, String> {
    rows.as_array()
        .and_then(|a| a.first())
        .cloned()
        .ok_or_else(|| "the database accepted that and returned nothing".to_string())
}

/// The colours this person chose.
///
/// AN UPSERT, FOR A READ, AND THE REASON IS THE DEFAULTS. Somebody who
/// has never chosen still has a die, and what it looks like is stated
/// once - in 292's column defaults. A plain read of a missing row
/// would mean restating `white` and `black` here, and then again in
/// the `<select>`s, and 288 is the entry about what happens to a rule
/// written twice. Asking for the row MAKES it, with the database
/// filling in both columns, so the answer always comes from the one
/// place that names them.
///
/// It is idempotent: the conflict path writes the key back over
/// itself and leaves the colours alone.
#[tauri::command]
pub fn dice_prefs(state: State<AppState>) -> Result<Value, String> {
    let ctx = Ctx::of(&state)?;
    one(supabase::rest_upsert(
        &ctx.token,
        "dice_prefs",
        &json!({ "profile_id": ctx.profile_id }),
        "profile_id",
    )?)
}

/// Choose them.
///
/// NOT VALIDATED HERE. 292's CHECK constraints are the list of what a
/// colour may be, and a copy of that list in Rust would be a second
/// place for it to be wrong - the fault 288 exists to describe. A
/// value the `<select>`s cannot produce is refused by Postgres, which
/// is the right answer to a bug rather than a polite one.
///
/// `updated_at` is not sent: 293 attached `touch_updated_at()`, so the
/// database stamps it and no client has to remember to.
#[tauri::command]
pub fn set_dice_prefs(
    state: State<AppState>,
    die_colour: String,
    pip_colour: String,
) -> Result<Value, String> {
    let ctx = Ctx::of(&state)?;
    one(supabase::rest_upsert(
        &ctx.token,
        "dice_prefs",
        &json!({
            "profile_id": ctx.profile_id,
            "die_colour": die_colour,
            "pip_colour": pip_colour,
        }),
        "profile_id",
    )?)
}
