//! Putting things in things, and taking them out again.
//!
//! PLUMBING. Every decision is in `containers.rs` and tested there; the
//! order of the requests is all that lives here.
//!
//! ONE MOVE, TWO DIRECTIONS. `put_in` and `take_out` are the same
//! operation with the destination swapped - an object's holder changes
//! - so they share `move_into`, and the checks that differ are the ones
//! about what a CONTAINER will accept. A character's hands accept
//! anything; that is the only asymmetry, and it is why a character is
//! not just a container with a permissive tag list.

use serde_json::{json, Value};
use tauri::State;

use crate::containers;
use crate::holders;
use crate::objects;
use crate::supabase::{self, AppState};

/// What is inside, in the same shape an inventory comes back in.
///
/// Deliberately `load_loadout`, which renders as an `Owned` row and so
/// reuses the panel the player's own inventory already uses. It derives
/// proficiency against empty lists on the way, which is meaningless for
/// a chest and harmless - nothing in a chest is being wielded.
#[tauri::command]
pub fn list_contents(
    state: State<AppState>,
    container_id: String,
) -> Result<Vec<crate::equipment::Owned>, String> {
    let token = state.token()?;
    let c = objects::load_object(&token, &container_id)?;
    let entity = c
        .entity_id
        .ok_or_else(|| "that is not a container".to_string())?;
    crate::equipment::load_loadout(&token, &entity, &c.game_id, &[], &[], false)
}

/// Whether this holder would still be a legal container afterwards.
///
/// THE ONE GATE. Every way of getting an object into a holder asks this
/// - moving one in, cloning one that is already there, and raising the
/// quantity of one that is already there - because a container over its
/// own capacity is the same broken state however it was reached, and it
/// then refuses ordinary moves with arithmetic that reads like a bug.
///
/// THE TWO THAT NEVER ASKED. `put_in_container` had these checks and
/// was the only thing that did, because it was the only command that
/// MOVED anything. Two others write into a holder without moving:
///
///   clone_object  copies a row into the SAME holder, so cloning a coin
///                 inside a full purse made a twenty-sixth coin in a
///                 purse that holds twenty-five.
///   edit_object   sets a quantity outright, so seventeen gold could
///                 become a hundred with nothing consulted.
///
/// Neither is exotic. Clone is a button on every row and quantity is a
/// box in every editor.
///
/// A HOLDER THAT IS NOT A CONTAINER IS ALWAYS FINE. A character has no
/// capacity - encumbrance is a rule about a person and does not exist
/// yet - and a location is a floor. Both answer Ok, which is what makes
/// this safe to call from anywhere without first asking what kind of
/// holder it is.
///
/// `adding` is what the total for that key will BE in this container
/// when the write lands. `ignoring` names a row already inside whose
/// quantity is being replaced rather than added to, so it is left out
/// of the existing total instead of counted twice.
pub(crate) fn guard_capacity(
    token: &str,
    holder: Option<&str>,
    game_id: &str,
    item_key: &str,
    adding: i64,
    ignoring: Option<&str>,
) -> Result<(), String> {
    // Nowhere holds anything you like.
    let Some(into) = holder else { return Ok(()) };

    // Which object IS this holder, if any. A character's entity and a
    // location's match none, and that is the common case.
    let rows = supabase::rest_get(
        token,
        "objects",
        &[
            ("select", "id,item_key,name"),
            ("entity_id", &format!("eq.{}", into)),
        ],
    )?;
    let Some(con) = rows.as_array().and_then(|a| a.first()) else {
        return Ok(());
    };
    let con_key = con.get("item_key").and_then(|v| v.as_str()).unwrap_or("");
    let name = con
        .get("name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(con_key)
        .to_string();

    let Some(profile) = containers::load_profile(token, game_id, con_key)? else {
        return Ok(());
    };

    let inside: Vec<objects::Stack> = objects::load_held(token, into)?
        .into_iter()
        .filter(|s| Some(s.id.as_str()) != ignoring)
        .collect();

    let mut keys: Vec<String> = inside.iter().map(|s| s.item_key.clone()).collect();
    keys.push(item_key.to_string());
    let bulks = containers::load_bulk(token, game_id, &keys)?;
    let find = |k: &str| {
        bulks
            .iter()
            .find(|b| b.key == k)
            .cloned()
            .ok_or_else(|| format!("no item with key '{}' in this game", k))
    };

    let incoming = find(item_key)?;
    let mut contents: Vec<(containers::Bulk, i64)> = Vec::new();
    for s in &inside {
        contents.push((find(&s.item_key)?, s.quantity));
    }

    // THREE OBJECTIONS, WORST-FIRST. They are not the same question and
    // any one of them is enough:
    //
    //   admits       a purse takes coins, and that is not a coin
    //   admits_size  nothing bigger than tiny goes in, and that is large
    //   fits         it would go in, but there is no room left
    //
    // Size before slots because it is the objection a person reaches
    // for and it gives the better sentence. Arithmetic about how much
    // room a greatsword needs in a coin purse answers a question nobody
    // asked.
    containers::admits(&profile, &incoming, &name)?;
    containers::admits_size(&profile, &incoming, &name)?;
    containers::fits(&profile, &contents, &incoming, adding, &name)?;
    Ok(())
}

/// Put something into a container.
#[tauri::command]
pub fn put_in_container(
    state: State<AppState>,
    object_id: String,
    container_id: String,
    quantity: Option<i64>,
) -> Result<Value, String> {
    let token = state.token()?;
    if object_id == container_id {
        return Err("a container cannot be put inside itself".to_string());
    }

    let obj = objects::load_object(&token, &object_id)?;
    let con = objects::load_object(&token, &container_id)?;
    let into = con
        .entity_id
        .clone()
        .ok_or_else(|| "that is not a container".to_string())?;

    // Loaded only to refuse a non-container early and so the reach
    // message has something to call it; the three capacity questions
    // belong to guard_capacity.
    containers::load_profile(&token, &con.game_id, &con.item_key)?
        .ok_or_else(|| "that is not a container".to_string())?;
    let name = con.name.clone().unwrap_or_else(|| con.item_key.clone());
    let thing = obj.name.clone().unwrap_or_else(|| obj.item_key.clone());

    // CAN ANYBODY REACH BOTH OF THESE AT ONCE. The first question and
    // the only one that is not about the container: a thing held by
    // Rodnar goes in a container held by Rodnar, and a thing lying in
    // the Frostvalley Inn goes in a chest in the Frostvalley Inn.
    // Nothing crosses between the two, however willing the chest.
    //
    // Asked before what the container accepts, how big it is and how
    // much room is left, because those three ask whether the thing
    // BELONGS in it and none of their answers matter if it is in
    // another building.
    //
    // WHO may do it is not decided here. 031's objects policy has said
    // since it was written: the DM, the owner of whoever is holding it,
    // or any member when the thing is loose. A player packing their own
    // kit and a party looting a room are both already covered, and a
    // second copy of that rule in this file is a second place for it to
    // be wrong.
    let (world, holders, _) = holders::load_world(&token, &con.game_id)?;
    holders::within_reach(
        &holders::root_of(obj.holder_id.as_deref(), &world, &holders),
        &holders::root_of(con.holder_id.as_deref(), &world, &holders),
        &thing,
        &name,
    )?;

    // HOW MANY, decided before the room is checked, because the room
    // needed is the room for what is ACTUALLY going in. Asking whether
    // eighteen coins fit when one is being moved refuses a purse with
    // plenty of space in it.
    let (_, going) = objects::split(obj.quantity, quantity)?;

    // What a container accepts, how big it takes and how much room is
    // left are all guard_capacity's now, because two other commands
    // write into a holder without moving anything and never reached
    // these three questions - see its header.
    guard_capacity(&token, Some(&into), &con.game_id, &obj.item_key, going, None)?;

    let inside = objects::load_held(&token, &into)?;
    move_into(&token, &obj, &object_id, &inside, Some(&into), quantity)
}

/// Take something out of a container and into somebody's hands.
///
/// No acceptance check: hands take anything. The one thing worth
/// refusing is taking from nowhere, which would silently create gear.
#[tauri::command]
pub fn take_from_container(
    state: State<AppState>,
    object_id: String,
    character_id: String,
    quantity: Option<i64>,
) -> Result<Value, String> {
    let token = state.token()?;
    let obj = objects::load_object(&token, &object_id)?;
    let taker = crate::character::load_profile(&token, &character_id)?;
    let held = objects::load_held(&token, &taker.entity_id)?;
    // NOTHING TO CHECK ON THE WAY OUT. A person is not a container with
    // a capacity - encumbrance is a rule about what they can CARRY and
    // it does not exist yet - so taking is only ever a split and a
    // merge.
    move_into(&token, &obj, &object_id, &held, Some(&taker.entity_id), quantity)
}

/// Move an object to a holder, merging into a stack already there.
///
/// Move a thing, or some of it, into a holder.
///
/// The merge is the whole reason this is shared: ten gold into a purse
/// holding five is fifteen in one row, not a second row the stack index
/// would refuse anyway.
///
/// FOUR OUTCOMES, from two independent questions. Does all of it go, or
/// only part? And is there already a stack at the destination for it to
/// join?
///
/// ```text
///                    nothing to join          a stack to join
/// all of it          the row changes hands    merge, and the row goes
/// part of it         the source keeps the     merge, and the source
///                    rest, a new row travels  keeps the rest
/// ```
///
/// `quantity` of None means all of it, which is what moving a thing
/// means. `objects::split` refuses zero and refuses more than there is,
/// and says why.
///
/// A NAMED THING CANNOT BE SPLIT and needs no guard here: `may_name`
/// refuses a name on a stack of more than one, so a named row is always
/// a single thing and every split of it takes the whole.
pub(crate) fn move_into(
    token: &str,
    obj: &objects::ObjectRow,
    object_id: &str,
    destination_contents: &[objects::Stack],
    holder: Option<&str>,
    quantity: Option<i64>,
) -> Result<Value, String> {
    let (keep, moved) = objects::split(obj.quantity, quantity)?;
    let target = objects::merge_into(destination_contents, &obj.item_key, obj.name.as_deref());

    if keep == 0 {
        if let Some(id) = target {
            let have = destination_contents
                .iter()
                .find(|s| s.id == id)
                .map(|s| s.quantity)
                .unwrap_or(0);
            let merged = supabase::rest_update(
                token,
                "objects",
                &[("id", &format!("eq.{}", id))],
                &json!({ "quantity": have + moved }),
            )?;
            // The row that moved is gone, because it was never a thing
            // in its own right - the same reasoning `take_object` uses.
            supabase::rest_delete(token, "objects", &[("id", &format!("eq.{}", object_id))])?;
            return Ok(merged);
        }
        return supabase::rest_update(
            token,
            "objects",
            &[("id", &format!("eq.{}", object_id))],
            &json!({ "holder_id": holder }),
        );
    }

    // Part of it. The source keeps the remainder whatever happens next.
    supabase::rest_update(
        token,
        "objects",
        &[("id", &format!("eq.{}", object_id))],
        &json!({ "quantity": keep }),
    )?;

    if let Some(id) = target {
        let have = destination_contents
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.quantity)
            .unwrap_or(0);
        return supabase::rest_update(
            token,
            "objects",
            &[("id", &format!("eq.{}", id))],
            &json!({ "quantity": have + moved }),
        );
    }

    supabase::rest_insert(
        token,
        "objects",
        &json!({
            "game_id": obj.game_id,
            "holder_id": holder,
            "item_key": obj.item_key,
            "quantity": moved,
            // THE OVERRIDE TRAVELS WITH THE SPLIT. If a DM said this
            // stack of coins is Large, the half that moves is still
            // Large - dropping it here would lose an edit silently,
            // which is the failure this codebase keeps meeting.
            "size_override": obj.size_override,
        }),
    )
}
