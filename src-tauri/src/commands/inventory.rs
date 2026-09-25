//! Getting things into hands, and out of them.
//!
//! PLUMBING, per the directory's rule. Every decision here is made in
//! `objects.rs` and tested there; what is left is the order of the
//! requests, which is the one thing these commands genuinely own.
//!
//! WHY THERE WAS NO ADD COMMAND UNTIL NOW
//!   There was one by accident. `set_item_equipped` upserted on
//!   (character, item_key), and an upsert on a pair that is not there
//!   INSERTS - so the way to give Rodnar a longsword was to equip a
//!   longsword he did not have. That is not a path anybody designed,
//!   and 026 closed it by taking an object id instead. These commands
//!   are the path that replaces it.
//!
//! ONE COMMAND FOR BOTH SIDES. `give_item` does not ask whether the
//! holder is a PC or an NPC, because since 022 there is nothing to ask:
//! a goblin is a characters row. The DM's reach is the policy's
//! business - `objects: holder or dm writes` lets a DM equip anyone in
//! their campaign and a player only themselves - and none of that is
//! restated here. A rule enforced in two places is a rule that will
//! disagree with itself.

use serde_json::{json, Value};
use tauri::State;

use crate::equipment;
use crate::objects;
use crate::supabase::{self, AppState};

/// Everything that can be added in this campaign: the global catalogue
/// with this game's overrides applied.
///
/// The same collapse `load_loadout` does, and for the same reason - a
/// campaign that redefines `longsword` should offer its longsword, not
/// both.
#[tauri::command]
pub fn list_catalogue(
    state: State<AppState>,
    game_id: String,
) -> Result<Vec<equipment::Item>, String> {
    let token = state.token()?;
    equipment::load_catalogue(&token, &game_id)
}

/// Put something in someone's hands.
///
/// Three steps, and the middle one is the only interesting part: what
/// is already held decides whether this is a new row or a bigger
/// number. `merge_into` makes that call and says why.
///
/// NOT AN UPSERT, deliberately. PostgREST resolves a conflict target
/// against a unique CONSTRAINT, and `objects_stack_idx` is a partial
/// index - it cannot back one, because the whole point of it is that
/// named objects are exempt. Reading first and then deciding is also
/// the only version that can produce the right error: an upsert that
/// silently merged a named axe into a stack would not look like a
/// failure.
#[tauri::command]
pub fn give_item(
    state: State<AppState>,
    character_id: String,
    item_key: String,
    quantity: Option<i64>,
    name: Option<String>,
) -> Result<Value, String> {
    let token = state.token()?;
    let quantity = objects::check_quantity(quantity.unwrap_or(1))?;
    let name = name.as_deref().and_then(objects::clean_name);

    // A named thing is one thing. Seven of them would each want the
    // name, which is the same question `may_name` refuses from the
    // other direction.
    if name.is_some() && quantity != 1 {
        return Err("only one thing can carry a name - give it singly".to_string());
    }

    let sheet = crate::character::load_profile(&token, &character_id)?;

    // A key with no catalogue row would load as an object the sheet
    // then skips - present in the table, absent from the inventory,
    // which is the worst of both. Refuse it here where there is
    // something to say about it.
    if equipment::load_item(&token, &sheet.game_id, &item_key)?.is_none() {
        return Err(format!("no item with key '{}' in this game", item_key));
    }

    let held = objects::load_held(&token, &sheet.entity_id)?;
    if let Some(id) = objects::merge_into(&held, &item_key, name.as_deref()) {
        let have = held
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.quantity)
            .unwrap_or(0);
        return supabase::rest_update(
            &token,
            "objects",
            &[("id", &format!("eq.{}", id))],
            &json!({ "quantity": have + quantity }),
        );
    }

    supabase::rest_insert(
        &token,
        "objects",
        &json!({
            "game_id": sheet.game_id,
            "holder_id": sheet.entity_id,
            "item_key": item_key,
            "quantity": quantity,
            "name": name,
        }),
    )
}

/// Put something down where you are standing.
///
/// IT LANDS ON A FLOOR NOW, not in limbo. Until 033 there was nowhere
/// for a dropped thing to BE, so this wrote NULL and 026 accepted that
/// as loot in limbo. Locations arrived and this did not follow, which
/// left the one command a player actually clicks still dropping things
/// out of the world.
///
/// THE PLACE IS DERIVED, NOT ASKED FOR. `drop_here` takes a location
/// and is the DM's tool for putting something in an arbitrary room.
/// This one is the player's: you drop what you are holding where you
/// are, and a character already knows where that is. Asking the caller
/// would let them drop a torch into a room they are not standing in.
///
/// A CHARACTER WHO IS NOWHERE DROPS INTO NOWHERE, and that is not a
/// failure. 035 made `location_id` nullable on purpose - a character
/// between scenes is not misfiled - so this keeps the old behaviour for
/// them and says plainly that is what happened.
///
/// THE SPLIT AND THE MERGE ARE `move_into`'s. This used to insert a
/// fresh row for the part that travelled, which lost `size_override`
/// and, worse, would hit `objects_stack_idx` the second time anybody
/// dropped the same plain thing in the same room - that index is on
/// (holder_id, item_key), and a floor is a holder.
#[tauri::command]
pub fn drop_object(
    state: State<AppState>,
    object_id: String,
    quantity: Option<i64>,
) -> Result<Value, String> {
    let token = state.token()?;
    let obj = objects::load_object(&token, &object_id)?;
    if obj.holder_id.is_none() {
        return Err("nobody is holding that".to_string());
    }

    // WHO is putting it down - through however many bags. Something in
    // a purse in a backpack is still Rodnar's to drop.
    let (world, holder_rows, _) = crate::holders::load_world(&token, &obj.game_id)?;
    let root = crate::holders::root_of(obj.holder_id.as_deref(), &world, &holder_rows);
    let carrier = match root {
        crate::holders::Root::Character { entity, .. } => entity,
        crate::holders::Root::Location { name, .. } => {
            return Err(format!("that is already lying in {} - nobody is carrying it", name))
        }
        _ => return Err("nobody is holding that".to_string()),
    };

    let (who, here) = standing(&token, &carrier)?;
    let (place_entity, place_name) = match here {
        Some(loc) => {
            let (e, n) = place(&token, &loc)?;
            (Some(e), n)
        }
        None => (None, "nowhere in particular".to_string()),
    };

    let (_, moved) = objects::split(obj.quantity, quantity)?;
    let thing = match equipment::load_item(&token, &obj.game_id, &obj.item_key)? {
        Some(i) => obj.name.clone().unwrap_or(i.name),
        None => obj.name.clone().unwrap_or_else(|| obj.item_key.clone()),
    };

    // A floor is a holder like any other, so what is already lying
    // there is what a merge looks at.
    let there = match &place_entity {
        Some(e) => objects::load_held(&token, e)?,
        None => Vec::new(),
    };
    crate::commands::containers::move_into(
        &token,
        &obj,
        &object_id,
        &there,
        place_entity.as_deref(),
        quantity,
    )?;

    // WHAT HAPPENED, in a sentence. The command says it rather than the
    // screen guessing: only this side knows which floor it landed on,
    // and a drop that quietly went somewhere else is exactly the kind of
    // thing nobody notices until an item cannot be found.
    Ok(json!({
        "said": format!(
            "{} is dropping {}{} at {}",
            who,
            if moved > 1 { format!("{} ", moved) } else { String::new() },
            thing,
            place_name
        ),
        "character": who,
        "item": thing,
        "quantity": moved,
        "location": place_name,
        "placed": place_entity.is_some(),
    }))
}

/// A character's name and where they are standing, from their entity.
fn standing(token: &str, entity_id: &str) -> Result<(String, Option<String>), String> {
    let rows = supabase::rest_get(
        token,
        "characters",
        &[
            ("select", "name,location_id"),
            ("entity_id", &format!("eq.{}", entity_id)),
        ],
    )?;
    let r = rows
        .as_array()
        .and_then(|a| a.first())
        .ok_or_else(|| "nobody is holding that".to_string())?;
    Ok((
        r.get("name").and_then(|v| v.as_str()).unwrap_or("Someone").to_string(),
        r.get("location_id").and_then(|v| v.as_str()).map(str::to_string),
    ))
}

/// A place's holder identity and what it is called.
fn place(token: &str, location_id: &str) -> Result<(String, String), String> {
    let rows = supabase::rest_get(
        token,
        "locations",
        &[
            ("select", "entity_id,name"),
            ("id", &format!("eq.{}", location_id)),
        ],
    )?;
    let r = rows
        .as_array()
        .and_then(|a| a.first())
        .ok_or_else(|| "that place is not visible to you".to_string())?;
    let entity = r
        .get("entity_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "that place has no holder identity".to_string())?;
    Ok((
        entity.to_string(),
        r.get("name").and_then(|v| v.as_str()).unwrap_or("somewhere").to_string(),
    ))
}

/// Pick up something nobody is holding, merging it if it stacks.
///
/// The inverse of a drop, and it goes through the same stacking rule:
/// three rations picked up by someone carrying four become seven, and
/// the row that was on the floor is gone because it was never a thing
/// in its own right.
#[tauri::command]
pub fn take_object(
    state: State<AppState>,
    object_id: String,
    character_id: String,
) -> Result<Value, String> {
    let token = state.token()?;
    let obj = objects::load_object(&token, &object_id)?;
    if obj.holder_id.is_some() {
        return Err("somebody is already holding that".to_string());
    }

    let taker = crate::character::load_profile(&token, &character_id)?;
    let held = objects::load_held(&token, &taker.entity_id)?;
    if let Some(id) = objects::merge_into(&held, &obj.item_key, obj.name.as_deref()) {
        let have = held
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.quantity)
            .unwrap_or(0);
        let merged = supabase::rest_update(
            &token,
            "objects",
            &[("id", &format!("eq.{}", id))],
            &json!({ "quantity": have + obj.quantity }),
        )?;
        supabase::rest_delete(&token, "objects", &[("id", &format!("eq.{}", object_id))])?;
        return Ok(merged);
    }

    supabase::rest_update(
        &token,
        "objects",
        &[("id", &format!("eq.{}", object_id))],
        &json!({ "holder_id": taker.entity_id }),
    )
}

/// Give one object a name of its own, or take the name away.
///
/// Blank means "call it by its type" and is the way back to a plain
/// object - the same reading `clean_name` gives everywhere, and the
/// reason this is not two commands.
///
/// UNNAMING CAN COLLIDE. A plain handaxe beside a newly-plain one is
/// two rows the stack index forbids, and the refusal arrives from
/// Postgres rather than from here. That is the right place for it: the
/// index is the thing that knows, and duplicating the check would mean
/// two answers to maintain.
#[tauri::command]
pub fn rename_object(
    state: State<AppState>,
    object_id: String,
    name: String,
) -> Result<Value, String> {
    let token = state.token()?;
    let obj = objects::load_object(&token, &object_id)?;
    let clean = objects::clean_name(&name);

    if clean.is_some() {
        objects::may_name(obj.quantity)?;
    }

    supabase::rest_update(
        &token,
        "objects",
        &[("id", &format!("eq.{}", object_id))],
        &json!({ "name": clean }),
    )
}

/// Attune to something, or break an attunement.
///
/// THERE WAS NO WAY TO DO THIS AT ALL. `objects.attuned` has existed
/// since 008 and is read onto every sheet, and nothing in the command
/// surface ever wrote it - The Ember has been attuned since the seed
/// and could not have been un-attuned. So this is the rule and the only
/// door to it arriving together.
///
/// THREE IS THE CAP and it is counted across everything the character
/// ultimately holds, not just what is equipped: a wand attuned at the
/// bottom of a backpack is still one of your three.
///
/// DIRECTLY HELD OR NOT, attunement does not care - unlike equipping,
/// which 031 restricted to things in hand. You attune to a thing you
/// carry, and a rod in a pack is carried.
#[tauri::command]
pub fn set_item_attuned(
    state: State<AppState>,
    object_id: String,
    attuned: bool,
) -> Result<Value, String> {
    let token = state.token()?;

    if attuned {
        let obj = objects::load_object(&token, &object_id)?;
        let holder = obj
            .holder_id
            .clone()
            .ok_or_else(|| "nobody is holding that - it cannot be attuned".to_string())?;

        // Whose is it, through however many bags.
        let (world, holder_rows, kinds) =
            crate::holders::load_world(&token, &obj.game_id)?;
        let root = crate::holders::root_of(Some(&holder), &world, &holder_rows);
        let mine = match root {
            crate::holders::Root::Character { entity, .. } => entity,
            _ => return Err("only a creature carrying it can attune to it".to_string()),
        };

        let name_of = |key: &str| {
            kinds
                .iter()
                .find(|k| k.key == key)
                .map(|_| key.to_string())
                .unwrap_or_else(|| key.to_string())
        };

        // Everything they hold, at any depth, that is already attuned.
        let already: Vec<String> = world
            .iter()
            .filter(|o| o.attuned && o.id != object_id)
            .filter(|o| {
                matches!(
                    crate::holders::root_of(o.holder_id.as_deref(), &world, &holder_rows),
                    crate::holders::Root::Character { ref entity, .. } if *entity == mine
                )
            })
            .map(|o| o.name.clone().unwrap_or_else(|| name_of(&o.item_key)))
            .collect();

        let incoming = obj.name.clone().unwrap_or_else(|| name_of(&obj.item_key));
        crate::carry::check_attunement(&already, &incoming)?;
    }

    supabase::rest_update(
        &token,
        "objects",
        &[("id", &format!("eq.{}", object_id))],
        &json!({ "attuned": attuned }),
    )
}

/// What this character has in coin.
///
/// EVERYTHING THEY HOLD, AT ANY DEPTH, which is the only reading that
/// works: coins live in a purse, and a purse lives in a backpack. A sum
/// over what is directly in hand would say a character with a full
/// purse is penniless.
///
/// The value of each coin is `currency::to_cp` of its own price and
/// denomination - nothing new is stored, because a gold piece being
/// worth a gold piece is not a fact worth a column.
#[tauri::command]
pub fn wallet(state: State<AppState>, character_id: String) -> Result<Value, String> {
    let token = state.token()?;
    let p = crate::character::load_profile(&token, &character_id)?;
    let minted = crate::currency::load_coins(&token, &p.game_id)?;
    let (world, holder_rows, _) = crate::holders::load_world(&token, &p.game_id)?;

    let mut purse: Vec<crate::currency::Coin> = Vec::new();
    for o in &world {
        let Some((key, denom, value_cp)) = minted.iter().find(|(k, _, _)| *k == o.item_key) else {
            continue;
        };
        let mine = matches!(
            crate::holders::root_of(o.holder_id.as_deref(), &world, &holder_rows),
            crate::holders::Root::Character { ref entity, .. } if *entity == p.entity_id
        );
        if !mine {
            continue;
        }
        // Stacks of the same coin in different pockets are one pile for
        // the purpose of paying with them.
        match purse.iter_mut().find(|c| c.key == *key) {
            Some(c) => c.count += o.quantity,
            None => purse.push(crate::currency::Coin {
                key: key.clone(),
                denom: denom.clone(),
                value_cp: *value_cp,
                count: o.quantity,
            }),
        }
    }

    purse.sort_by_key(|c| -c.value_cp);
    let total = crate::currency::total(&purse);
    Ok(json!({
        "total_cp": total,
        // What is in the purse, and what it is worth. Two different
        // questions - see currency::held.
        "said": crate::currency::held(&purse),
        "worth": crate::currency::format_cp(total),
        "coins": purse,
    }))
}

/// How much this character is carrying, and what that costs them.
///
/// NOT ON THE SHEET, on purpose. `load_sheet` runs on every roll, and
/// this walks everything a character holds at any depth to sum it -
/// which is the right cost for an inventory screen and the wrong one
/// ahead of a d20. A panel asks when it paints.
///
/// EVERYTHING, AT ANY DEPTH. A backpack weighs what it weighs PLUS what
/// is in it, which is why this cannot be a sum over the loadout.
///
/// REPORTED, NOT REFUSED. See carry::burden - the base rule forbids
/// going over capacity and the variant slows you down instead, and
/// picking between them is a campaign's decision.
#[tauri::command]
pub fn encumbrance(state: State<AppState>, character_id: String) -> Result<Value, String> {
    let token = state.token()?;
    let p = crate::character::load_profile(&token, &character_id)?;
    let (world, holder_rows, kinds) = crate::holders::load_world(&token, &p.game_id)?;

    let mut carried = 0.0_f64;
    for o in &world {
        let mine = matches!(
            crate::holders::root_of(o.holder_id.as_deref(), &world, &holder_rows),
            crate::holders::Root::Character { ref entity, .. } if *entity == p.entity_id
        );
        if !mine {
            continue;
        }
        // `Kind::weight` is already TEXT - holders parsed it on the way
        // in, because most callers print a weight rather than sum one.
        // This is one of the few that sums, so it parses back.
        let each = kinds
            .iter()
            .find(|k| k.key == o.item_key)
            .and_then(|k| k.weight.as_deref())
            .and_then(|w| w.parse::<f64>().ok())
            .unwrap_or(0.0);
        carried += each * (o.quantity as f64);
    }

    let str_score = load_ability_score(&token, &character_id, "str")?;
    let size = p.vitals.size.as_deref();
    let capacity = crate::carry::carry_capacity(str_score, size);
    let state_ = crate::carry::burden(carried, str_score, size);

    Ok(json!({
        "carried": (carried * 100.0).round() / 100.0,
        "capacity": capacity,
        "strength": str_score,
        "burden": state_.as_str(),
        "said": format!(
            "{} lb of {} - {}",
            (carried * 100.0).round() / 100.0,
            capacity,
            state_.as_str()
        ),
    }))
}

/// One ability score, without loading a sheet to get it.
fn load_ability_score(token: &str, character_id: &str, code: &str) -> Result<i64, String> {
    let rows = supabase::rest_get(
        token,
        "character_abilities",
        &[
            ("select", "score"),
            ("character_id", &format!("eq.{}", character_id)),
            ("ability", &format!("eq.{}", code)),
        ],
    )?;
    Ok(rows
        .as_array()
        .and_then(|a| a.first())
        .and_then(|r| r.get("score"))
        .and_then(|x| x.as_i64())
        .unwrap_or(10))
}

/// Destroy an object outright.
///
/// Separate from dropping, because they are different events and only
/// one of them is reversible. A dropped thing can be picked up; this is
/// the sword that went into the lava.
///
/// A stack goes whole. Spending three of seven rations is a quantity
/// change and belongs to whatever rule spends them, not to a command
/// whose name says destroy.
#[tauri::command]
pub fn destroy_object(state: State<AppState>, object_id: String) -> Result<(), String> {
    let token = state.token()?;
    supabase::rest_delete(&token, "objects", &[("id", &format!("eq.{}", object_id))])
}

/// Say outright whether the holder is proficient with THIS object, or
/// stop saying and let the rule decide.
///
/// THE TRI-STATE, REACHABLE AT LAST. `objects.proficient_override` has
/// been read by `is_proficient` since 008 and written by exactly one
/// thing: `instantiate_npc`, which stamps TRUE on every item in a
/// statblock's kit. So a DM could give a goblin a weapon it was
/// mysteriously proficient with and had no way to say otherwise.
///
/// NULL IS NOT FALSE and that is the whole reason this takes an Option.
/// `None` clears the override and hands the question back to
/// `weapon_profs` - which is a different answer from `Some(false)`, and
/// the difference is whether the character being trained later changes
/// anything. Treating them as one would make "not proficient today"
/// mean "never proficient", silently, forever.
///
/// The escape hatch rather than the main road. Training belongs on the
/// character - see `set_proficiencies` - and this is for the single
/// weapon that does not follow from it.
#[tauri::command]
pub fn set_object_proficient(
    state: State<AppState>,
    object_id: String,
    proficient: Option<bool>,
) -> Result<Value, String> {
    let token = state.token()?;
    supabase::rest_update(
        &token,
        "objects",
        &[("id", &format!("eq.{}", object_id))],
        &json!({ "proficient_override": proficient }),
    )
}

/// Change what a thing is called and how many of it there are.
///
/// ONE COMMAND FOR BOTH because they constrain each other. `may_name`
/// refuses a name on a stack of seven, so setting the name and setting
/// the quantity in two calls means an order that works and an order
/// that does not - and whichever the screen picked, the other would be
/// a bug waiting for somebody to edit both at once.
///
/// Checked against the NEW quantity, not the old one. Naming a lone
/// dagger and raising it to three in the same breath is the case that
/// has to be refused, and only the new number can refuse it.
///
/// Omitting a field leaves it alone. A blank NAME is not omission - it
/// is "call it by its type again", which `clean_name` turns into NULL.
#[tauri::command]
pub fn edit_object(
    state: State<AppState>,
    object_id: String,
    name: Option<String>,
    quantity: Option<i64>,
    size_override: Option<String>,
    holds_size_override: Option<String>,
    weight_override: Option<String>,
    price_override: Option<String>,
    damage_number_override: Option<String>,
    damage_denomination_override: Option<String>,
    damage_types_override: Option<String>,
    properties_override: Option<String>,
    base_ac_override: Option<String>,
) -> Result<Value, String> {
    let token = state.token()?;
    let obj = objects::load_object(&token, &object_id)?;

    let qty = match quantity {
        Some(q) => objects::check_quantity(q)?,
        None => obj.quantity,
    };

    // A QUANTITY IS A WAY INTO A CONTAINER. Seventeen gold in a full
    // purse could become a hundred here with nothing consulted, and the
    // purse then refused ordinary moves with arithmetic that looked
    // wrong. `ignoring` leaves this row out of the existing total,
    // because qty REPLACES it rather than adding to it.
    //
    // Only when it GROWS. Emptying a full container is always allowed.
    if qty > obj.quantity {
        crate::commands::containers::guard_capacity(
            &token,
            obj.holder_id.as_deref(),
            &obj.game_id,
            &obj.item_key,
            qty,
            Some(&object_id),
        )?;
    }

    let mut patch = json!({ "quantity": qty });
    if let Some(raw) = name.as_deref() {
        let clean = objects::clean_name(raw);
        if clean.is_some() {
            objects::may_name(qty)?;
        }
        patch["name"] = json!(clean);
    } else if obj.name.is_some() {
        // The name is being kept, so it still has to survive the new
        // quantity. Raising a named sword to three is the same refusal
        // from the other direction.
        objects::may_name(qty)?;
    }

    // 036'S TWO OVERRIDES, and a blank clears one rather than skipping
    // it. That is the difference between "leave it alone" and "it is
    // ordinary after all", and only an explicit empty string can say
    // the second - which is why these are Option<String> and not
    // Option<Option<String>>: the screen sends "" to clear.
    //
    // The words are not checked here. 036 puts a CHECK on both columns,
    // so a size nobody recognises is refused by Postgres with its own
    // sentence, and a second copy of the ladder in this file is a second
    // place for it to be wrong.
    for (field, given) in [
        ("size_override", &size_override),
        ("holds_size_override", &holds_size_override),
    ] {
        if let Some(raw) = given.as_deref() {
            patch[field] = match raw.trim() {
                "" => Value::Null,
                v => json!(v),
            };
        }
    }


    // 049'S SEVEN, on the same convention as 036's two: an empty string
    // CLEARS the override and an absent field leaves it alone. The
    // difference matters - "as the catalogue says" and "the same as the
    // catalogue happens to say today" are different facts, and only the
    // first follows the catalogue when it changes.
    //
    // NUMBERS ARE PARSED HERE AND CHECKED IN POSTGRES. A word where a
    // die size belongs gets a sentence from this side, because "invalid
    // input syntax for type integer" is not one; the RANGES are 049's
    // CHECK constraints, because items has the same ones and a rule
    // written twice is 028.
    for (field, given) in [
        ("price_override", &price_override),
        ("damage_number_override", &damage_number_override),
        ("damage_denomination_override", &damage_denomination_override),
        ("base_ac_override", &base_ac_override),
    ] {
        let Some(raw) = given.as_deref().map(str::trim) else { continue };
        if raw.is_empty() {
            patch[field] = Value::Null;
            continue;
        }
        let n: i64 = raw
            .parse()
            .map_err(|_| format!("{} wants a whole number, not '{}'", field, raw))?;
        patch[field] = json!(n);
    }

    if let Some(raw) = weight_override.as_deref().map(str::trim) {
        if raw.is_empty() {
            patch["weight_override"] = Value::Null;
        } else {
            let w: f64 = raw
                .parse()
                .map_err(|_| format!("a weight wants a number, not '{}'", raw))?;
            patch["weight_override"] = json!(w);
        }
    }

    // LISTS REPLACE, so an empty one is a real answer and needs a way
    // to be said. Blank clears the override; the word `none` sets it to
    // nothing at all - which is how a greatsword loses `hvy`, and the
    // reason objects::Overrides holds Option<Vec> rather than Vec.
    for (field, given) in [
        ("damage_types_override", &damage_types_override),
        ("properties_override", &properties_override),
    ] {
        let Some(raw) = given.as_deref().map(str::trim) else { continue };
        if raw.is_empty() {
            patch[field] = Value::Null;
        } else if raw.eq_ignore_ascii_case("none") {
            patch[field] = json!(Vec::<String>::new());
        } else {
            let list: Vec<String> = raw
                .split(',')
                .map(|t| t.trim().to_lowercase())
                .filter(|t| !t.is_empty())
                .collect();
            patch[field] = json!(list);
        }
    }

    supabase::rest_update(
        &token,
        "objects",
        &[("id", &format!("eq.{}", object_id))],
        &patch,
    )
}

/// Make another one.
///
/// THE COPY IS ANONYMOUS. A named object is a particular thing - 026
/// built naming so that Dawnbreaker is not one of seven longswords -
/// and two of them would make the name a lie. The clone comes off the
/// same catalogue key with no name, and the DM can christen it.
///
/// A CLONED CONTAINER IS EMPTY. 032's trigger gives the new row its own
/// entity because the catalogue says the key is a container, so the copy
/// really is a second chest rather than a second door onto the first -
/// which is what copying `entity_id` would have made, and what the
/// unique constraint on it would have refused anyway. Nothing inside
/// comes with it.
///
/// It stacks where stacking is what the rules say. An anonymous copy
/// landing in a holder who already has a plain stack of the same key
/// joins it, by `merge_into` - the same decision `give_item` makes,
/// because three daggers and a fourth is four daggers.
#[tauri::command]
pub fn clone_object(
    state: State<AppState>,
    object_id: String,
    quantity: Option<i64>,
) -> Result<Value, String> {
    let token = state.token()?;
    let obj = objects::load_object(&token, &object_id)?;
    let qty = objects::check_quantity(quantity.unwrap_or(1))?;

    // A CLONE LANDS IN THE SAME HOLDER, so it is a way into a container
    // and has to ask like any other. Cloning a coin inside a full purse
    // made a twenty-sixth coin in a purse that holds twenty-five,
    // because nothing on this path had ever been told about capacity.
    crate::commands::containers::guard_capacity(
        &token,
        obj.holder_id.as_deref(),
        &obj.game_id,
        &obj.item_key,
        qty,
        None,
    )?;

    // Only a held thing has a holder to stack within. One lying in a
    // room or nowhere at all simply gets a second row.
    if let Some(holder) = obj.holder_id.as_deref() {
        let held = objects::load_held(&token, holder)?;
        if let Some(id) = objects::merge_into(&held, &obj.item_key, None) {
            let have = held
                .iter()
                .find(|s| s.id == id)
                .map(|s| s.quantity)
                .unwrap_or(0);
            return supabase::rest_update(
                &token,
                "objects",
                &[("id", &format!("eq.{}", id))],
                &json!({ "quantity": have + qty }),
            );
        }
    }

    supabase::rest_insert(
        &token,
        "objects",
        &json!({
            "game_id": obj.game_id,
            "holder_id": obj.holder_id,
            "item_key": obj.item_key,
            "quantity": qty,
            // Deliberately absent. See the header.
            "name": Value::Null,
        }),
    )
}

/// What this object can DO: every special attack its type offers, and
/// whether this particular one can still reach it.
///
/// THE TECHNIQUES BELONG TO THE TYPE AND THE MODES BELONG TO THE
/// OBJECT, which is the whole reason this is a command rather than a
/// query. 043 wrote three techniques for every weapon, each pinned to a
/// `mode`; `equipment::modes` derives the modes a weapon has from its
/// class and properties; and 049 lets an object override those
/// properties. So a greatsword reforged without `thr` really loses its
/// thrown techniques, and a viewer that listed the type's rows would
/// promise moves this object cannot make.
///
/// The unreachable ones are SHOWN, not filtered. 043's header says why
/// the opposite is dangerous: "a technique written in an unreachable
/// mode is not an error anywhere - it is simply never offered, which is
/// the worst kind of bug to find." A DM who edits a weapon out of a
/// mode should see what it cost, not watch three buttons quietly
/// disappear.
///
/// THE SAME MERGE BOTH OTHER SCREENS USE. `Overrides::apply` runs here
/// before the modes are derived, exactly as it does in `load_loadout`
/// and `holders::resolve` - three screens, one function, which is the
/// point 049 made about the two that used to disagree.
#[derive(serde::Serialize)]
pub struct WeaponMove {
    pub technique: crate::attack::Technique,
    /// False when this object cannot be used in that technique's mode.
    pub offered: bool,
    /// Which mode it needs, for saying so on screen.
    pub needs: String,
}

#[tauri::command]
pub fn object_techniques(
    state: State<AppState>,
    object_id: String,
) -> Result<Vec<WeaponMove>, String> {
    let token = state.token()?;

    // The whole row, because the override columns are what decide the
    // modes and `load_object` carries only a few of them.
    let rows = supabase::rest_get(
        &token,
        "objects",
        &[
            ("select", "*"),
            ("id", &format!("eq.{}", object_id)),
        ],
    )?;
    let row = rows
        .as_array()
        .and_then(|a| a.first())
        .ok_or_else(|| "no such object, or it is not visible to you".to_string())?;

    let game_id = row.get("game_id").and_then(|v| v.as_str()).unwrap_or("");
    let item_key = row.get("item_key").and_then(|v| v.as_str()).unwrap_or("");

    let Some(base) = equipment::load_item(&token, game_id, item_key)? else {
        return Err(format!("no item with key '{}' in this game", item_key));
    };
    // AS THIS ONE ACTUALLY IS, not as the catalogue describes the type.
    let item = objects::overrides_from_row(row).apply(&base);

    // Nothing that is not a weapon has techniques, and saying so is
    // cheaper than a query that returns nothing.
    if item.kind != "weapon" {
        return Ok(Vec::new());
    }

    let mine = equipment::modes(&item);
    let techniques = crate::attack::load_techniques(&token, game_id, &[item_key.to_string()])?;

    Ok(techniques
        .into_iter()
        .map(|t| WeaponMove {
            offered: mine.contains(&t.mode),
            needs: t.mode.as_str().to_string(),
            technique: t,
        })
        .collect())
}
