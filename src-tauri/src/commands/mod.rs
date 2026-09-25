//! The command surface, branching out of lib.rs.
//!
//! WHY THIS EXISTS
//!   `lib.rs` opens by saying every command in it is a thin wrapper and
//!   that rules logic does not belong there. Both halves are still true,
//!   and the file is 944 lines anyway - because "thin" is a statement
//!   about each command, not about how many of them one file should
//!   hold. Twenty-six commands across eight subjects is a directory, not
//!   a file, and every subsystem added since 009 has made it longer.
//!
//! THE LINE THIS DIRECTORY DRAWS
//!   `src/*.rs`          RULES. Pure where it can be, tested, no Tauri.
//!                       dice, character, equipment, attack, resolution,
//!                       death, encounter, narrative.
//!   `src/commands/*.rs` PLUMBING. Pull the session out of state, call
//!                       one thing, hand the result back. No rule ever
//!                       gets decided here, and nothing in here has
//!                       tests, because there should be nothing in here
//!                       worth testing. A command that wants a test is a
//!                       rule wearing a wrapper and belongs one level up.
//!
//! HOW IT IS BEING DONE
//!   Foundation and new material first; nothing is being moved for the
//!   sake of moving. A group migrates when it is being worked on anyway,
//!   so the diff that moves it is a diff somebody is already reading.
//!   `me` moved on its own as the exemplar - see `session.rs` - because
//!   the one thing worth proving up front was that `generate_handler!`
//!   takes a path into a submodule and the frontend never notices.
//!
//! THE MAP
//!
//!   file                 takes                                     ~lines  state
//!   ------------------   ---------------------------------------   ------  -----
//!   session.rs           sign_up sign_in sign_out me                   35  me moved
//!   games.rs             list_games create_game join_game
//!                        list_members                                  50  in lib.rs
//!   characters.rs        list_characters create_character              40  in lib.rs
//!   sheet.rs             get_sheet set_level set_ability
//!                        set_skill_prof                                85  in lib.rs
//!   equipment.rs         list_inventory set_item_equipped
//!                        check_item_keys                               90  in lib.rs
//!   rolls.rs             list_rolls roll_dice create_roll
//!                        patch_roll_narrative                         100  in lib.rs
//!   encounters.rs        list_encounters list_targets                  40  in lib.rs
//!   actions.rs           preview_request roll_named death_save
//!                        + roll_row parse_target vitals_payload       450  in lib.rs
//!
//!   `actions.rs` is half the file on its own and is the one group that
//!   is genuinely hard to move: `roll_named` takes fourteen arguments
//!   and three private helpers travel with it. It should go LAST, and
//!   probably not until initiative forces a change to it anyway.
//!
//!   NEW MATERIAL LANDS HERE FROM NOW ON, no migration required:
//!   dm.rs                create encounter, enrol actor, add
//!                        challenge, write a statblock     (item 2) DONE
//!   inventory.rs         list the catalogue, give, drop, take,
//!                        name and destroy an object                DONE
//!   containers.rs        what is inside, put in, take out          DONE
//!   store.rs             what it costs here, and buying it         DONE
//!   locations.rs         build the world, and put things down in it DONE
//!   initiative.rs        rolling for it, reading turn order   (item 1) DONE
//!   challenges.rs        whether the iron lock has been picked     (item 3)
//!
//!   dm.rs was the first real tenant and it never touched lib.rs, which
//!   is the whole point: eleven commands of new surface and the crate
//!   root grew by eleven lines of registration. The other two are next
//!   on the list in STATUS.md and land the same way.
//!
//! WHAT DOES NOT LIVE HERE
//!   `run()` and the `generate_handler!` list stay in `lib.rs`. One
//!   registry, in the crate root, is the thing you read to find out what
//!   the app can do - splitting that across files would hide exactly the
//!   overview the split is supposed to give back.

use crate::supabase::{AppState, Session};
use tauri::State;

pub mod containers;
pub mod dm;
pub mod initiative;
pub mod inventory;
pub mod locations;
pub mod session;
pub mod store;

/// Everything a command needs from the session, resolved once.
///
/// `state.token()?` appears sixteen times in lib.rs and the longer
/// three-line dance for the user id - `current()?` then `ok_or_else`
/// into "not signed in" - five more. They are the same question asked
/// two ways, and the second spelling exists only because a few commands
/// need the id as well as the token.
///
/// A command takes what it needs off this and stops repeating the
/// question. `token` is `String` rather than `&str` because every call
/// into `supabase` wants an owned one anyway.
#[allow(dead_code)]
pub struct Ctx {
    pub token: String,
    pub user_id: String,
}

#[allow(dead_code)]
impl Ctx {
    /// The signed-in caller, or the same "not signed in" every other
    /// path already returns. Stated once here rather than rediscovered
    /// as a 401 at each call site - the reasoning `AppState::token`
    /// already gives, extended to the id.
    pub fn of(state: &State<AppState>) -> Result<Self, String> {
        let s: Session = state.current()?.ok_or("not signed in")?;
        Ok(Ctx {
            token: s.access_token,
            user_id: s.user_id,
        })
    }
}
