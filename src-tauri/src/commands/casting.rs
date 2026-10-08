//! The spell catalogue, and what a caster has chosen from it.
//!
//! 106. Plumbing. Every number - how many may be prepared, how many
//! cantrips are known, what slots exist, the save DC - is casting.rs.
//!
//! 176. A CLERIC AND A WIZARD COME THROUGH THE SAME COMMANDS, which is
//! right: preparing a spell and spending a slot are the same act for
//! both, and the one thing that differs is what they may reach for.
//! That question is asked once, of `casting::may_reach`.
//!
//! THE CASTING CLASS'S LEVEL, NEVER THE CHARACTER'S. A Fighter 4 /
//! Cleric 1 prepares as a cleric 1, and reading their total of 5 would
//! hand them
//! a 3rd-level slot they have not earned. The same rule
//! `features::held` and `uses::Context` already follow.

use serde_json::{json, Value};
use tauri::State;

use crate::casting;
use crate::supabase::{self, AppState};

/// The spell catalogue, global rows and this game's own.
///
/// BY CLASS RATHER THAN A TABLE PER CLASS. `spells.classes` is an
/// array, so every class's list is the same query with a different
/// argument, and adding the wizard's 206 was a seed rather than a
/// schema change - see 101.
///
/// ONE TABLE IS THE POINT, not a convenience. Dispel Magic is ONE
/// SPELL on two lists; a table per class would hold it twice and let
/// the copies drift. A prayer and a spell differ in WHO MAY REACH FOR
/// ONE, not in where the row is kept.
///
/// 169. AND NO CLASS AT ALL IS THE WHOLE BOOK. An empty `class_key`
/// drops the filter, which is what the Spells tab reads: 161-167 took
/// the catalogue from 108 to 276 and only one class had a surface to
/// see any of it through.
///
/// 176. AND NOW NOBODY PASSES A CLASS. The sheet used to ask for the
/// cleric list, which offered a wizard the wrong spells and - worse -
/// made their own prepared ones render as nothing, because the lookup
/// that resolves a key could not find them. One catalogue, filtered
/// where the caster is known.
///
/// `special_text` IS SELECTED NOW, and its absence was the usual fault.
/// It is where the mechanical rider lives - the three rays of a
/// Scorching Ray, what a successful save is worth, which spells scale
/// with the slot - and the reference book has been dropping all of it
/// because one column list did not mention it. Same shape as 155's
/// `character_name` and 865c8be's `prof_bonus`: a column that exists,
/// is filled, and is never asked for.
#[tauri::command]
pub fn list_spells(
    state: State<AppState>,
    game_id: String,
    class_key: String,
) -> Result<Value, String> {
    let token = state.token()?;
    let scope = format!("(game_id.is.null,game_id.eq.{})", game_id);
    let holds = format!("cs.{{{}}}", class_key.trim());

    let mut query: Vec<(&str, &str)> = vec![
        (
            "select",
            "key,game_id,name,level,cast_type,category,school,save_ability,dice,\
             on_save,concentration,ritual,range,duration,casting_time,components,\
             material,classes,special_text,description",
        ),
        ("or", &scope),
        ("order", "level.asc,name.asc"),
    ];
    if !class_key.trim().is_empty() {
        query.push(("classes", &holds));
    }
    let rows = supabase::rest_get(&token, "spells", &query)?;

    // The game's own row wins over the global one of the same key - the
    // precedence every tenanted catalogue in this schema uses.
    let all = rows.as_array().cloned().unwrap_or_default();
    let mut out: Vec<Value> = Vec::new();
    for r in &all {
        let Some(key) = r.get("key").and_then(|v| v.as_str()) else {
            continue;
        };
        let mine = r.get("game_id").and_then(|v| v.as_str()).is_some();
        match out
            .iter()
            .position(|o| o.get("key").and_then(|v| v.as_str()) == Some(key))
        {
            Some(i) if mine => out[i] = r.clone(),
            Some(_) => {}
            None => out.push(r.clone()),
        }
    }
    Ok(json!(out))
}

/// What this caster can do today: their numbers, and what they hold.
///
/// 176. THE CLASS AND THE SOURCE ARE BOTH IN THE ANSWER, so the screen
/// can label itself - Prayers for a cleric, Spells for a wizard -
/// without deciding a rule of its own.
///
/// REFUSES A CHARACTER WHO DOES NOT CAST, rather than answering with
/// zeroes. A fighter has no prepared list and saying "0 of 0" would
/// invite the question of how to raise it.
#[tauri::command]
pub fn list_casting(state: State<AppState>, character_id: String) -> Result<Value, String> {
    let token = state.token()?;
    let sheet = crate::character::load_sheet(&token, &character_id)?;

    // 150. WHOEVER IS CASTING, which is a class row and not a creature
    // question - see casting::caster, and 022 for why there is no
    // second path for a monster.
    let Some(caster) = sheet.caster.clone() else {
        return Err(format!("{} does not cast spells", sheet.name));
    };
    let level = caster.level;
    let abil = sheet.ability_mod(&caster.ability);
    let pb = sheet.proficiency_bonus();

    let chosen = load_chosen(&token, &character_id, Some(caster.source))?;
    let of = |want: &str| -> Vec<String> {
        chosen
            .iter()
            .filter(|(_, st)| st == want)
            .map(|(k, _)| k.clone())
            .collect()
    };
    let prepared = of("prepared");
    let cantrips = of("cantrip");
    // 150. THE THIRD LIST, and empty for a cleric: known, written down,
    // and not prepared today. A wizard's book.
    let book = of("book");

    let slots = casting::slots_at(level);
    let spent = load_slots(&token, &character_id)?;
    let left = casting::slots_left(level, &spent);
    Ok(json!({
        "caster_level": level,
        "casting_ability": caster.ability,
        "ability_mod": abil,
        "class_key": caster.class_key,
        "source": caster.source,
        "save_dc": casting::save_dc(pb, abil),
        "attack_bonus": casting::attack_bonus(pb, abil),
        "prepared": prepared,
        "prepared_max": casting::prepared_max(level, abil),
        "cantrips": cantrips,
        "book": book,
        "cantrips_known": casting::cantrips_known(level),
        "top_slot": casting::top_slot(level),
        // 1st-level slots first. 107 made the spending real, so these
        // are three different facts and the tab needs all three: how
        // many they have, how many are gone, how many are left.
        "slots": slots,
        "slots_spent": spent,
        "slots_left": left,
    }))
}

/// Hold a spell, or learn a cantrip.
///
/// ONE COMMAND FOR BOTH, because the difference is the spell's level
/// and the engine can see it. Asking the screen to pick between two
/// commands would be the screen deciding a rule.
#[tauri::command]
pub fn prepare_spell(
    state: State<AppState>,
    character_id: String,
    spell_key: String,
) -> Result<Value, String> {
    let token = state.token()?;
    let sheet = crate::character::load_sheet(&token, &character_id)?;
    // 150. WHOEVER IS CASTING, which is a class row and not a creature
    // question - see casting::caster, and 022 for why there is no
    // second path for a monster.
    let Some(caster) = sheet.caster.clone() else {
        return Err(format!("{} does not cast spells", sheet.name));
    };
    let level = caster.level;
    let abil = sheet.ability_mod(&caster.ability);

    let spell = one_spell(&token, &sheet.game_id, &spell_key)?;
    let on_lists: Vec<String> = spell
        .get("classes")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|c| c.as_str()).map(String::from).collect())
        .unwrap_or_default();
    let spell_level = spell.get("level").and_then(|v| v.as_i64()).unwrap_or(0);

    let chosen = load_chosen(&token, &character_id, Some(caster.source))?;
    // 150. WHAT THEY MAY REACH FOR - the cleric list, or this wizard's
    // own book. 176 adds the level, because a cantrip is a class-list
    // question even for a wizard, and THIS CALL HAPPENS BEFORE the
    // branch on level further down. See casting::may_reach.
    casting::may_reach(
        &caster,
        &on_lists,
        spell_level,
        chosen
            .iter()
            .find(|(k, _)| *k == spell_key)
            .map(|(_, st)| st.as_str()),
    )?;
    let held: Vec<String> = chosen
        .iter()
        .filter(|(_, st)| st == "prepared")
        .map(|(k, _)| k.clone())
        .collect();

    // A CANTRIP IS A DIFFERENT BUDGET, so it is checked against its own
    // count rather than through may_prepare - which refuses cantrips
    // outright, and says so, because being prepared is not what a
    // cantrip does.
    if spell_level == 0 {
        let known: Vec<&String> = chosen
            .iter()
            .filter(|(_, st)| st == "cantrip")
            .map(|(k, _)| k)
            .collect();
        if known.iter().any(|k| **k == spell_key) {
            return Err("already known".to_string());
        }
        let max = casting::cantrips_known(level);
        if known.len() as i64 >= max {
            return Err(format!(
                "that is {} cantrips and they know {} - forget one first",
                known.len(),
                max
            ));
        }
    } else {
        casting::may_prepare(spell_level, level, abil, &held, &spell_key)?;
    }

    supabase::rest_upsert(
        &token,
        "character_spells",
        &json!({
            "character_id": character_id,
            "spell_key": spell_key,
            "state": if spell_level > 0 { "prepared" } else { "cantrip" },
        }),
        "character_id,spell_key",
    )
}

/// Put one down.
///
/// A DELETE, UNLIKE A CHOICE OR AN EFFECT. A prepared list is what you
/// hold TODAY and changes at every long rest; there is no history worth
/// keeping in which spells you held last Tuesday.
#[tauri::command]
pub fn forget_spell(
    state: State<AppState>,
    character_id: String,
    spell_key: String,
) -> Result<Value, String> {
    let token = state.token()?;
    supabase::rest_delete(
        &token,
        "character_spells",
        &[
            ("character_id", &format!("eq.{}", character_id)),
            ("spell_key", &format!("eq.{}", spell_key)),
        ],
    )?;
    Ok(json!({ "ok": true }))
}

/// What this caster can cast right now, as turn actions.
///
/// 110. PREPARED AND CANTRIPS ONLY, which is the whole point - the
/// catalogue is 276 long and what a caster can do on their turn is the
/// dozen they are holding.
///
/// WHAT CANNOT BE CAST IS STILL LISTED, with the reason on it. Prayer
/// of Healing takes ten minutes and a fight is six seconds a round, so
/// it is shown as too long rather than hidden - anybody reaching for
/// it should be told why rather than wondering where it went.
#[tauri::command]
pub fn castable(state: State<AppState>, character_id: String) -> Result<Value, String> {
    let token = state.token()?;
    let sheet = crate::character::load_sheet(&token, &character_id)?;
    // 150. WHOEVER IS CASTING, which is a class row and not a creature
    // question - see casting::caster, and 022 for why there is no
    // second path for a monster.
    let Some(caster) = sheet.caster.clone() else {
        return Err(format!("{} does not cast spells", sheet.name));
    };
    let abil = sheet.ability_mod(&caster.ability);
    let pb = sheet.proficiency_bonus();

    // 178. WHAT IS UP TODAY, which is not everything they hold. A
    // wizard's carried books come back from `load_chosen` as `book`
    // since 172, and this took every key regardless - so the cast panel
    // offered a wizard their whole spellbook and the prepared budget
    // meant nothing. `casting::up_today` owns the rule.
    let chosen: Vec<(String, String)> = load_chosen(&token, &character_id, Some(caster.source))?
        .into_iter()
        .filter(|(_, st)| casting::up_today(st))
        .collect();
    if chosen.is_empty() {
        return Ok(json!([]));
    }
    let keys: Vec<String> = chosen.iter().map(|(k, _)| k.clone()).collect();
    let rows = supabase::rest_get(
        &token,
        "spells",
        &[
            ("select", "key,name,level,cast_type,casting_time,save_ability,dice,range,duration,concentration,grants"),
            ("key", &format!("in.({})", keys.join(","))),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", sheet.game_id)),
        ],
    )?;

    let spent = load_slots(&token, &character_id)?;
    let left = casting::slots_left(caster.level, &spent);

    let mut out: Vec<Value> = Vec::new();
    for r in rows.as_array().unwrap_or(&Vec::new()) {
        let c = crate::spellcast::cast(
            r.get("key").and_then(|v| v.as_str()).unwrap_or(""),
            r.get("name").and_then(|v| v.as_str()).unwrap_or(""),
            r.get("level").and_then(|v| v.as_i64()).unwrap_or(0),
            r.get("cast_type").and_then(|v| v.as_str()).unwrap_or(""),
            r.get("casting_time").and_then(|v| v.as_str()),
            r.get("save_ability").and_then(|v| v.as_str()),
            r.get("dice").and_then(|v| v.as_str()),
            pb,
            abil,
        );
        // WHY IT CANNOT BE CAST, where it cannot. Two different
        // problems and they want different words: it takes too long,
        // or there is no slot left to carry it.
        let blocked = if !c.cost.in_a_fight() {
            Some(format!(
                "takes {}",
                r.get("casting_time").and_then(|v| v.as_str()).unwrap_or("too long")
            ))
        } else if c.needs_slot && !slots_for(c.level, &left).is_empty() {
            None
        } else if c.needs_slot {
            Some(format!("no level {} slots left", c.level))
        } else {
            None
        };

        // 185. WHICH SLOTS COULD CARRY IT, which is the engine's
        // question and not the screen's. A 1st-level spell can go in
        // any slot from 1st up, and a wizard with four spent firsts and
        // three free thirds can still cast it - Tarren could not,
        // because every caller passed `atLevel: null`.
        //
        // EMPTY FOR A CANTRIP, which spends nothing.
        let levels = if c.needs_slot { slots_for(c.level, &left) } else { Vec::new() };

        out.push(json!({
            "levels": levels,
            "key": c.key, "name": c.name, "level": c.level,
            "stance": c.stance.as_str(),
            "cost": c.cost.as_str(),
            "to_hit": c.to_hit,
            "save_dc": c.save_dc,
            "save_ability": c.save_ability,
            "dice": c.dice,
            "needs_slot": c.needs_slot,
            "label": crate::spellcast::label(&c),
            "range": r.get("range"),
            "duration": r.get("duration"),
            "concentration": r.get("concentration"),
            "blocked": blocked,
            // 116. THE QUESTION THIS SPELL ASKS, if it asks one.
            // Protection from Energy is the only one so far: the panel
            // shows these five and the cast refuses without an answer,
            // rather than the engine picking an element for somebody.
            "choices": choices_in(r.get("grants")),
        }));
    }
    // Cantrips last: they cost nothing and are what is left when the
    // slots are gone, so the things that run out lead.
    out.sort_by_key(|v| {
        let lvl = v.get("level").and_then(|x| x.as_i64()).unwrap_or(0);
        (
            lvl == 0,
            lvl,
            v.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        )
    });
    Ok(json!(out))
}

/// Cast one: spend the slot, and say what to roll.
///
/// THE SLOT IS SPENT HERE AND THE DICE ARE ROLLED BY THE ROLL PATH,
/// deliberately. A cast is a cost plus a roll, and the roll already has
/// somewhere to go. Rolling here would be a second way to make a d20
/// happen.
///
/// `at_level` CARRIES AN UPCAST. Casting Cure Wounds with a 3rd-level
/// slot is 5e's own rule, and the reason 107 tracks slots by level
/// rather than by spell.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn cast_spell(
    state: State<AppState>,
    character_id: String,
    spell_key: String,
    at_level: Option<i64>,
    // 112. WHO IT IS AIMED AT, and it reaches the engine now. 111 put a
    // picker on the panel and used the answer only to write a log line
    // - so Luci cast Bless on Falon four times, four slots went, and
    // nothing was recorded, nothing landed on Falon, and no action row
    // was written. The slot was the only thing that moved.
    target_character_id: Option<String>,
    target_actor_id: Option<String>,
    target_label: Option<String>,
    encounter_id: Option<String>,
    actor_id: Option<String>,
    // 116. WHICH DAMAGE TYPE, for the one spell that asks. Protection
    // from Energy is "choose one of acid, cold, fire, lightning, or
    // thunder", and the choice belongs to the moment of casting rather
    // than to the catalogue - the same spell protects against cold on
    // Tuesday and fire on Wednesday.
    //
    // None FOR EVERY OTHER SPELL, which is all but one of them.
    choice: Option<String>,
) -> Result<Value, String> {
    // 156. THE WHOLE SESSION, not just the token: a heal writes a roll
    // row and `rolls.owner_uid` is who threw the dice.
    let session = state.current()?.ok_or_else(|| "not signed in".to_string())?;
    let token = session.access_token.clone();
    let sheet = crate::character::load_sheet(&token, &character_id)?;
    // 150. WHOEVER IS CASTING, which is a class row and not a creature
    // question - see casting::caster, and 022 for why there is no
    // second path for a monster.
    let Some(caster) = sheet.caster.clone() else {
        return Err(format!("{} does not cast spells", sheet.name));
    };

    // 178. UP TODAY, not merely written down. This asked only whether
    // the key was in the list, which 172 widened to include every spell
    // in a carried book - so a wizard could cast anything in the book
    // without preparing it, and the refusal below never fired.
    let held = load_chosen(&token, &character_id, Some(caster.source))?;
    let state = held.iter().find(|(k, _)| *k == spell_key).map(|(_, st)| st.as_str());
    match state {
        Some(st) if casting::up_today(st) => {}
        // Told apart, because they are different problems: one is
        // solved by preparing it this morning and the other by writing
        // it down first.
        Some(_) => return Err("that is in their book but not prepared today".to_string()),
        None => return Err("they do not have that prepared".to_string()),
    }

    let spell = one_spell_full(&token, &sheet.game_id, &spell_key)?;
    let mut c = crate::spellcast::cast(
        &spell_key,
        spell.get("name").and_then(|v| v.as_str()).unwrap_or(""),
        spell.get("level").and_then(|v| v.as_i64()).unwrap_or(0),
        spell.get("cast_type").and_then(|v| v.as_str()).unwrap_or(""),
        spell.get("casting_time").and_then(|v| v.as_str()),
        spell.get("save_ability").and_then(|v| v.as_str()),
        spell.get("dice").and_then(|v| v.as_str()),
        sheet.proficiency_bonus(),
        sheet.ability_mod(&caster.ability),
    );

    // 185. AND WHAT THE BIGGER SLOT ROLLS. `at_level` has chosen which
    // slot is SPENT since 107; this is the other half of the same
    // decision, and it had never been wired - a wizard spending a 3rd
    // level slot on Magic Missile got three darts for it.
    //
    // AFTER `cast`, not inside it, because `cast` takes the catalogue's
    // own dice and a Heal has already had the caster's modifier folded
    // in by the time this runs. Scaling what is on the card is the
    // right order: the card and the roll must agree.
    c.dice = crate::spellcast::upcast_dice(
        c.dice.as_deref(),
        spell.get("at_higher_dice").and_then(|v| v.as_str()),
        spell.get("level").and_then(|v| v.as_i64()).unwrap_or(0),
        at_level,
    );

    if !c.cost.in_a_fight() {
        return Err(format!(
            "{} takes {} - not something to do on a turn",
            c.name,
            spell.get("casting_time").and_then(|v| v.as_str()).unwrap_or("too long")
        ));
    }

    // A CANTRIP COSTS NOTHING, which is the whole of what makes it one.
    let mut used: Option<i64> = None;
    if c.needs_slot {
        let want = at_level.unwrap_or(c.level);
        if want < c.level {
            return Err(format!(
                "{} is a level {} spell - a level {} slot will not carry it",
                c.name, c.level, want
            ));
        }
        let spent = load_slots(&token, &character_id)?;
        casting::may_spend_slot(caster.level, &spent, want)?;
        let i = (want - 1) as usize;
        move_slot(&token, &character_id, want, spent[i], spent[i] + 1)?;
        used = Some(want);
    }

    // ---- THE RECORD. A cast is a thing that happened and 001's rule
    // covers it: the action row is what makes it findable later, and
    // its absence is why four slots vanished with nothing to show.
    let full = spell.get("duration").and_then(|v| v.as_str());
    let concentrates = spell
        .get("concentration")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // 156. AND A HEAL IS ROLLED HERE, which it never was. Cure Wounds
    // spent a slot, logged a line and changed nobody's hit points -
    // seven healing spells in the catalogue carry dice and not one of
    // them reached a hit point, because every route to `hp_events` ran
    // through `resolved.attack` and a heal is not an attack.
    //
    // THE DICE ALREADY HAVE THE MODIFIER IN THEM. `spellcast::cast`
    // folds it in for a Heal and only for a Heal - Cure Wounds is 1d8
    // plus your modifier and Sacred Flame is 1d8 flat - so this rolls
    // what it is handed and adds nothing.
    //
    // `death::healed` OWNS HOW MUCH LANDS, because the dice and the
    // delta are not the same number: hit points are a signed log that
    // runs below zero while the reading is floored at zero, so healing
    // Falon at a raw -1 has to write +9 to put him on 8. That is a rule
    // and it is tested; this is the plumbing that calls it.
    // 183. ONE PAIR OF SLOTS FOR BOTH BRANCHES - the heal below and
    // the automatic damage after it. They are mutually exclusive by
    // cast type, they both end in the same `write_action`, and two sets
    // of variables for one transaction would only invite a spell that
    // somehow filled both.
    let mut moved_said: Option<String> = None;
    let mut extra_rolls: Vec<Value> = Vec::new();
    let mut extra_hp: Value = Value::Null;

    if spell.get("cast_type").and_then(|v| v.as_str()) == Some("Heal") {
        if let (Some(on), Some(dice)) = (
            target_character_id.as_deref().filter(|s| !s.is_empty()),
            c.dice.as_deref().filter(|d| !d.trim().is_empty()),
        ) {
            let rolled = crate::dice::roll_formula(dice)?;
            let (hp_max, summed) = hp_state(&token, on)?;
            let delta = crate::death::healed(hp_max, summed, rolled.total);

            extra_rolls.push(json!({
                "game_id": sheet.game_id,
                "character_id": character_id,
                "owner_uid": session.user_id,
                "request": c.name,
                "label": format!("{} healing", c.name),
                "mode": "normal",
                "formula": dice,
                "detail": rolled.detail,
                "total": rolled.total,
                "natural_roll": rolled.natural,
                "status": "resolved",
                // 156. A ROLE OF ITS OWN, so the log can tell a heal from
                // the damage it is the mirror of, and so `write_action`
                // hangs the hit point event off the right dice.
                "role": "heal",
            }));

            // NO EVENT FOR NOTHING. Healing somebody already full is a
            // wasted spell and not a zero-delta row - 013's log is a
            // record of what CHANGED.
            if delta > 0 {
                extra_hp = json!({
                    "game_id": sheet.game_id,
                    "character_id": on,
                    "delta": delta,
                    "note": format!("{} healing", c.name),
                });
            }
            moved_said = Some(if delta > 0 {
                format!("healed {} ({} rolled {})", delta, dice, rolled.total)
            } else {
                format!("{} rolled {} - already at full health", dice, rolled.total)
            });
        }
    }

    // 183. AND A SPELL THAT SIMPLY LANDS, which until now could not
    // reach a hit point either. See `spellcast::lands_automatically`:
    // Magic Missile has no attack roll and no save, so neither the
    // Attack path nor 158's `resolve_spell_save` was ever going to
    // carry it, and it sat in `Utility` dealing nothing.
    //
    // THE MIRROR OF THE HEAL ABOVE, deliberately - same roll, same
    // `write_action`, opposite sign. The role is `damage`, which is
    // what `resolve_spell_save` already writes, so the log tells the
    // two apart without a new word.
    if crate::spellcast::lands_automatically(
        spell.get("cast_type").and_then(|v| v.as_str()).unwrap_or(""),
    ) {
        if let (Some(on), Some(dice)) = (
            target_character_id.as_deref().filter(|s| !s.is_empty()),
            c.dice.as_deref().filter(|d| !d.trim().is_empty()),
        ) {
            let rolled = crate::dice::roll_formula(dice)?;
            extra_rolls.push(json!({
                "game_id": sheet.game_id,
                "character_id": character_id,
                "owner_uid": session.user_id,
                "request": c.name,
                "label": format!("{} damage", c.name),
                "mode": "normal",
                "formula": dice,
                "detail": rolled.detail,
                "total": rolled.total,
                "natural_roll": rolled.natural,
                "status": "resolved",
                "role": "damage",
            }));
            if rolled.total > 0 {
                extra_hp = json!({
                    "game_id": sheet.game_id,
                    "character_id": on,
                    // SIGNED, and negative - 013's log, the same shape
                    // `resolve_spell_save` writes.
                    "delta": -rolled.total,
                    "note": c.name.clone(),
                });
            }
            moved_said = Some(format!("{} damage ({} rolled {})", rolled.total, dice, rolled.total));
        }
    }

    if let Some(enc) = encounter_id.as_deref().filter(|s| !s.is_empty()) {
        // 156. THROUGH `write_action`, so the action, the heal roll and
        // the hit point event land together or not at all - 012's rule,
        // which a plain insert here could not keep. It took `cost` being
        // added to that function's column list to become possible: a
        // bonus-action spell written through it would otherwise have
        // been stamped "action" and taken the caster's whole turn.
        let mut act = json!({
            "game_id": sheet.game_id,
            "character_id": character_id,
            "encounter_id": enc,
            "request": c.name,
            "label": crate::spellcast::label(&c),
            "key": "spell",
            "cost": c.cost.as_str(),
        });
        if let Some(a) = actor_id.as_deref().filter(|s| !s.is_empty()) {
            act["actor_id"] = json!(a);
        }
        if let Some(t) = target_actor_id.as_deref().filter(|s| !s.is_empty()) {
            act["target_actor_id"] = json!(t);
        }
        supabase::rpc(
            &token,
            "write_action",
            &json!({
                "p_action": act,
                "p_rolls": extra_rolls,
                "p_hp": extra_hp,
                "p_vitals": Value::Null,
            }),
        )?;
    }

    // ---- AND WHAT IT LEAVES BEHIND. Bless is a minute of +1d4 on
    // somebody; Cure Wounds is over the moment it lands. `lasts` tells
    // them apart, and an instantaneous spell makes no effect at all.
    //
    // 154. WHOSE EFFECT IT IS, which was the target's unconditionally
    // and is wrong for an attack. Luci cast Spiritual Weapon and a
    // ten-tick chip appeared on WEBBYS - the thing she was hitting -
    // saying a spectral mace was in play, with an empty grants list so
    // it did nothing at all. A floating weapon belongs to the caster
    // who maintains it, not to whoever it is swung at.
    //
    // `cast_type = 'Attack'` AND NOT THE STANCE, which is the narrower
    // test and the correct one. Stance lumps Attack together with Save,
    // and the lasting Save spells are mostly debuffs that genuinely DO
    // sit on their victim - Hold Person, Bane, Blindness, Bestow Curse.
    // Routing by stance would have moved all nineteen of them onto the
    // caster to fix two.
    //
    // THE ROUGH EDGE THAT REMAINS, named rather than quietly fudged:
    // Spirit Guardians, Blade Barrier and Guardian of Faith are Save
    // spells that hang around the CASTER, and they stay on the target
    // here. `spellcast`'s own header already says the four-word
    // vocabulary cannot tell Zone of Truth from Bane; this is the same
    // gap and it wants a column, not a cleverer guess.
    let holds_it = if spell.get("cast_type").and_then(|v| v.as_str()) == Some("Attack") {
        Some(character_id.as_str())
    } else {
        target_character_id.as_deref().filter(|s| !s.is_empty())
    };

    let mut landed: Option<String> = None;
    if let Some(on) = holds_it {
        let ticks = match crate::spellcast::lasts(full) {
            crate::spellcast::Lasts::Instant => None,
            crate::spellcast::Lasts::Ticks(n) => Some(Some(n)),
            crate::spellcast::Lasts::Indefinite => Some(None),
        };
        if let Some(for_ticks) = ticks {
            let now = crate::commands::effects::read_tick(&token, &sheet.game_id)?;

            // ONE CONCENTRATION AT A TIME, which is 5e and is the rule
            // a table forgets most often. Casting a second ends the
            // first - and it is the CASTER's concentration, so this
            // looks for what they are holding rather than what is on
            // the target.
            if concentrates {
                crate::commands::effects::drop_concentration(&token, &sheet.game_id, &character_id, now)?;
            }

            // 116. SETTLE WHAT THE SPELL ASKED. A grant naming several
            // damage types is a question; the answer goes onto the
            // effect row so the resistance that is RUNNING is one
            // concrete thing, and a DM who retunes the spell next month
            // does not retroactively change what is already on somebody
            // - 001's snapshot rule, the same reason the grants are
            // copied rather than looked up.
            let grants = settle_choices(
                spell.get("grants").unwrap_or(&json!([])),
                choice.as_deref(),
            )?;

            let (_, said) = crate::commands::effects::apply_inner(
                &token,
                &sheet.game_id,
                on,
                &spell_key,
                &c.name,
                for_ticks,
                c.dice.as_deref().and_then(dice_size),
                "replace",
                Some(&character_id),
                if concentrates { Some("concentration") } else { Some("spell") },
                None,
                // WHAT IT DOES WHILE IT LASTS, copied off the spell.
                &grants,
                now,
            )?;
            landed = Some(said);
        }
    }

    Ok(json!({
        "name": c.name,
        "label": crate::spellcast::label(&c),
        "stance": c.stance.as_str(),
        "cost": c.cost.as_str(),
        "to_hit": c.to_hit,
        "save_dc": c.save_dc,
        "save_ability": c.save_ability,
        "dice": c.dice,
        "slot_used": used,
        "target": target_label,
        // What is now true of the target, where anything is. None for
        // an instantaneous spell, which is most of the damage ones.
        "landed": landed,
        // 156. WHAT THE HEAL ACTUALLY DID, which is not always what the
        // dice said - capped at the maximum, and larger than the roll
        // when it has a hole to climb out of first. Said out loud for
        // the same reason 116 says why damage and hit points disagree:
        // a number that moves differently from the dice beside it looks
        // like a bug until something explains it.
        // 183. "moved", NOT "healed". This carried only a heal until the
        // Auto branch above started writing damage into it, and a key
        // called `healed` holding "7 damage" is the same kind of lie
        // 176 spent the day pulling out of this module.
        "moved": moved_said,
        "concentration": concentrates,
    }))
}

/// The damage types a spell makes its caster choose between.
///
/// 116. ASKED OF THE GRANTS rather than listed per spell, so a DM who
/// writes a new grant offering a choice gets the dropdown for free -
/// and a spell whose grants stop offering one stops asking. Null for
/// all but one spell in the catalogue.
fn choices_in(grants: Option<&Value>) -> Option<Vec<String>> {
    grants?
        .as_array()?
        .iter()
        .filter_map(|g| g.get("target")?.as_str())
        .find_map(crate::resist::choice_offered)
}

/// Replace any grant target that offers a choice with the one that was
/// chosen.
///
/// 116. REFUSED RATHER THAN GUESSED. Casting Protection from Energy
/// without naming a type fails with the five options in the message,
/// because picking one for the caster would be the engine deciding
/// which element they were afraid of. The refusal happens BEFORE the
/// effect is written - the slot is already spent by then, which is
/// 5e (a spell you botch the targeting of is still a spell you cast)
/// but a half-written effect would not be.
///
/// EVERY OTHER GRANT PASSES THROUGH UNTOUCHED, including the ones that
/// have nothing to do with resistance - Bless's die goes through this
/// function without noticing it.
fn settle_choices(grants: &Value, choice: Option<&str>) -> Result<Value, String> {
    let Some(rows) = grants.as_array() else {
        return Ok(json!([]));
    };
    let mut out = Vec::with_capacity(rows.len());
    for g in rows {
        let mut g = g.clone();
        let target = g.get("target").and_then(|t| t.as_str()).unwrap_or("").to_string();
        if crate::resist::choice_offered(&target).is_some() {
            // 143. AND THE QUALIFIER COMES BACK WITH IT. This used to
            // format the target inline, which was correct while a
            // target was two parts and would have silently dropped the
            // third the moment one existed - a spell granting
            // resistance only against ordinary weapons would have
            // landed as resistance against everything.
            let (degree, kind, nonmagical) = crate::resist::pick(&target, choice)?;
            g["target"] = json!(crate::resist::target_for(degree, &kind, nonmagical));
        }
        out.push(g);
    }
    Ok(json!(out))
}

/// The die size out of a dice string, for an effect's magnitude.
///
/// `1d4` is a 4. Used by `highest` stacking so a better Bless beats a
/// worse one, and shown on the chip. A formula this cannot read gives
/// None rather than a guess.
fn dice_size(d: &str) -> Option<i64> {
    d.split('d').nth(1)?
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .ok()
}

/// Spend one.
///
/// BY LEVEL AND NOT BY SPELL. A slot is a slot - a 3rd-level slot can
/// carry a 1st-level spell, and asking which spell it was for would
/// make upcasting unrepresentable.
#[tauri::command]
pub fn spend_spell_slot(
    state: State<AppState>,
    character_id: String,
    slot_level: i64,
) -> Result<Value, String> {
    let token = state.token()?;
    let level = caster_level(&token, &character_id)?;
    let spent = load_slots(&token, &character_id)?;
    casting::may_spend_slot(level, &spent, slot_level)?;

    let i = (slot_level - 1) as usize;
    move_slot(&token, &character_id, slot_level, spent[i], spent[i] + 1)?;
    Ok(json!({ "left": casting::slots_left(level, &spent)[i] - 1 }))
}

/// Give one back, for a misclick or a DM's say-so.
#[tauri::command]
pub fn restore_spell_slot(
    state: State<AppState>,
    character_id: String,
    slot_level: i64,
) -> Result<Value, String> {
    let token = state.token()?;
    let level = caster_level(&token, &character_id)?;
    let spent = load_slots(&token, &character_id)?;
    if !(1..=9).contains(&slot_level) {
        return Err("a spell slot is level 1 to 9".to_string());
    }
    let i = (slot_level - 1) as usize;
    if spent[i] <= 0 {
        return Err("nothing to give back".to_string());
    }
    move_slot(&token, &character_id, slot_level, spent[i], spent[i] - 1)?;
    Ok(json!({ "left": casting::slots_left(level, &spent)[i] + 1 }))
}

/* ======================== THE WORKING PARTS ======================== */

/// What this character has spent, 1st-level first.
pub(crate) fn load_slots(token: &str, character_id: &str) -> Result<[i64; 9], String> {
    let rows = supabase::rest_get(
        token,
        "character_slots",
        &[
            ("select", "slot_level,spent"),
            ("character_id", &format!("eq.{}", character_id)),
        ],
    )?;
    let mut out = [0i64; 9];
    for r in rows.as_array().unwrap_or(&Vec::new()) {
        let Some(lvl) = r.get("slot_level").and_then(|v| v.as_i64()) else {
            continue;
        };
        if (1..=9).contains(&lvl) {
            out[(lvl - 1) as usize] = r.get("spent").and_then(|v| v.as_i64()).unwrap_or(0);
        }
    }
    Ok(out)
}

/// Move one level's spent count from `was` to `now`, and only if it is
/// still `was`.
///
/// 120. COMPARE AND SET, NOT READ-MODIFY-WRITE. This used to take an
/// absolute number and write it, which is correct exactly once:
///
/// ```text
/// let spent = load_slots(..)?;            // both casts read 0
/// may_spend_slot(level, &spent, want)?;   // both pass the check
/// write_slot(.., want, spent[i] + 1)?;    // both write 1
/// ```
///
/// Two castings, two effects on the target, two action rows - and one
/// slot gone. A double-click on `cast` did exactly that, and 120 guards
/// the button as well; this is the half that holds when the second
/// press arrives from somewhere a button guard cannot see.
///
/// THE PREDICATE IS THE WHOLE POINT. `spent=eq.{was}` means the UPDATE
/// matches nothing if anybody moved it first, and PostgREST hands back
/// the rows it touched - so an empty array IS the collision, reported
/// rather than silently lost.
///
/// NO ROW YET IS NOT A COLLISION. The first spend of a level has
/// nothing to update, so a `was` of zero that matches no row falls
/// through to an insert; a second one racing it loses on the primary
/// key, which is the same refusal by another route.
fn move_slot(
    token: &str,
    character_id: &str,
    level: i64,
    was: i64,
    now: i64,
) -> Result<(), String> {
    let landed = supabase::rest_update_if(
        token,
        "character_slots",
        &[
            ("character_id", &format!("eq.{}", character_id)),
            ("slot_level", &format!("eq.{}", level)),
            ("spent", &format!("eq.{}", was)),
        ],
        &json!({ "spent": now.max(0) }),
    )?;
    if landed {
        return Ok(());
    }

    if was == 0 {
        return supabase::rest_insert(
            token,
            "character_slots",
            &json!({
                "character_id": character_id,
                "slot_level": level,
                "spent": now.max(0),
            }),
        )
        .map(|_| ())
        .map_err(|_| STALE.to_string());
    }

    Err(STALE.to_string())
}

/// What a lost race is called, in one place so both callers say it.
const STALE: &str =
    "that slot moved while this was in flight - look at the sheet and try again";

/// The caster level, refusing anybody who does not cast.
///
/// 150. Was named `cleric_level` and found a cleric class by name,
/// which is the lookup that kept a Lich out of its own spell list. 176
/// renamed it for the same reason it renamed the module.
fn caster_level(token: &str, character_id: &str) -> Result<i64, String> {
    let sheet = crate::character::load_sheet(token, character_id)?;
    sheet
        .caster
        .as_ref()
        .map(|c| c.level)
        .ok_or_else(|| format!("{} does not cast spells", sheet.name))
}

/// Hand every slot back. Called by the rest path.
///
/// A DELETE RATHER THAN A ZERO because the rows carry nothing else -
/// an unspent level has no fact worth a row, and clearing them keeps
/// the table the size of what is actually spent.
pub(crate) fn clear_slots(token: &str, character_id: &str) -> Result<(), String> {
    supabase::rest_delete(
        token,
        "character_slots",
        &[("character_id", &format!("eq.{}", character_id))],
    )
}

/// (spell key, prepared) for this character.
/// The target's save against a spell already cast, and what it costs
/// them.
///
/// 158. THE SECOND HALF OF A SAVE SPELL. Casting one spends the slot,
/// logs the DC and stops, because a save is the TARGET'S roll and the
/// engine does not roll for somebody else's character on its own
/// initiative. So Sacred Flame reported "DEX save DC 14, 1d8" and
/// nobody ever rolled the d8 - eleven spells with dice and no path to a
/// hit point between them.
///
/// THE DM ASKS FOR IT, which is what makes this allowable. The roll
/// happens because somebody pressed the button next to this target's
/// name, not because a spell decided to roll it for them - the same
/// distinction that lets a DM roll a monster's save at the table.
///
/// THE SAME CHAIN AN ATTACK ALREADY HAS: one d20, judged against a
/// number, and the dice that follow from the verdict, landing as an
/// action, its rolls and one hit point event through `write_action` -
/// together or not at all.
///
/// IT COSTS THE SAVER NOTHING. A save is not an action and
/// `stamp_action_cost` would have stamped one, quietly eating the
/// target's turn; 156 added `cost` to `write_action` and this is the
/// first caller that needs it.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn resolve_spell_save(
    state: State<AppState>,
    caster_character_id: String,
    spell_key: String,
    target_character_id: String,
    target_actor_id: Option<String>,
    encounter_id: Option<String>,
    mode: Option<String>,
    // 185. THE SLOT IT WAS CAST WITH, because a save spell rolls its
    // damage HERE and not at the cast. A Fireball upcast to 5th showed
    // 10d6 on the card and would have rolled the catalogue's 8d6 - the
    // card and the roll disagreeing, which is the one thing 154 built
    // `spellcast::cast` to prevent.
    //
    // CARRIED BY THE CALLER rather than stored on the action: the
    // prompt that offers the save is the thing that just did the cast,
    // and it already holds the rest of what this needs. A column on
    // `actions` would be the better answer the day something other
    // than that prompt can resolve a save.
    at_level: Option<i64>,
) -> Result<Value, String> {
    let session = state.current()?.ok_or_else(|| "not signed in".to_string())?;
    let token = session.access_token.clone();
    let mode = mode.unwrap_or_else(|| "normal".to_string());

    // --- the DC, which belongs to the CASTER and not to this roll ---
    let caster_sheet = crate::character::load_sheet(&token, &caster_character_id)?;
    let Some(caster) = caster_sheet.caster.clone() else {
        return Err(format!("{} does not cast spells", caster_sheet.name));
    };
    let spell = one_spell_full(&token, &caster_sheet.game_id, &spell_key)?;
    let Some(ability) = spell
        .get("save_ability")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return Err(format!(
            "{} forces no saving throw",
            spell.get("name").and_then(|v| v.as_str()).unwrap_or(&spell_key)
        ));
    };
    let name = spell
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(&spell_key)
        .to_string();
    let dc = casting::save_dc(
        caster_sheet.proficiency_bonus(),
        caster_sheet.ability_mod(&caster.ability),
    );

    // --- the roll, which belongs to the TARGET ---
    let target = crate::character::load_sheet(&token, &target_character_id)?;
    // " vs spell" IS NOT DECORATION. 098 gave the Ny'ook a bonus that
    // applies only against a spell and `resolve_request` has always read
    // it off those two words. This save IS against a spell, so it is the
    // one caller that can say so without being asked.
    let resolved =
        crate::character::resolve_request(&target, &format!("{} save vs spell", ability), &mode);
    // AND WHAT IS RUNNING ON THEM. A Bless on the saver is a d4 on this
    // d20; appended to the formula rather than folded into the modifier
    // so the roll shows what it rolled.
    let formula = match crate::effect_grants(&token, &target, &resolved) {
        Ok(terms) if !terms.is_empty() => format!("{}{}", resolved.formula, terms.join("")),
        _ => resolved.formula.clone(),
    };
    let rolled = crate::dice::roll_formula(&formula)?;
    let saved = rolled.total >= dc;

    // --- and what it costs them ---
    let mut rolls: Vec<Value> = vec![json!({
        "game_id": target.game_id,
        "character_id": target.character_id,
        "character_name": target.character_id.as_ref().map_or_else(
            || Some(target.name.clone()), |_| None),
        "owner_uid": session.user_id,
        "request": format!("{} save vs {}", ability, name),
        "label": format!("{} Save vs {}", ability.to_uppercase(), name),
        "mode": mode,
        "formula": formula,
        "detail": rolled.detail,
        "total": rolled.total,
        "natural_roll": rolled.natural,
        "status": "resolved",
        "role": "check",
        // JUDGED AGAINST THE DC, so the row says what it was trying to
        // beat - 009's whole point, and what lets the log read
        // "16 vs DC 14, made it" instead of a bare number.
        "target_value": dc,
        "target_kind": "dc",
        "target_label": name,
        "success": saved,
    })];

    let mut hp = Value::Null;
    let mut took: Option<i64> = None;
    // 185. SCALED FIRST, by the same rule the card used. One function,
    // two callers, so the number shown and the number rolled cannot
    // drift apart.
    let scaled = crate::spellcast::upcast_dice(
        spell.get("dice").and_then(|v| v.as_str()),
        spell.get("at_higher_dice").and_then(|v| v.as_str()),
        spell.get("level").and_then(|v| v.as_i64()).unwrap_or(0),
        at_level,
    );
    if let Some(dice) = scaled.as_deref().map(str::trim).filter(|d| !d.is_empty()) {
        let dmg = crate::dice::roll_formula(dice)?;
        // 158 OWNS WHAT LANDS. None means these dice were never save
        // damage - Bane's penalty, Geas's daily toll - and nothing is
        // rolled at anybody.
        if let Some(amount) = crate::spellcast::save_damage(
            dmg.total,
            saved,
            spell.get("on_save").and_then(|v| v.as_str()),
        ) {
            took = Some(amount);
            rolls.push(json!({
                "game_id": target.game_id,
                "character_id": target.character_id,
                "owner_uid": session.user_id,
                "request": name,
                "label": if saved {
                    format!("{} damage (saved, half)", name)
                } else {
                    format!("{} damage", name)
                },
                "mode": "normal",
                "formula": dice,
                "detail": dmg.detail,
                "total": dmg.total,
                "natural_roll": dmg.natural,
                "status": "resolved",
                "role": "damage",
            }));
            if amount > 0 {
                hp = json!({
                    "game_id": target.game_id,
                    "character_id": target_character_id,
                    // SIGNED, and negative: 013's log again.
                    "delta": -amount,
                    "note": format!("{} ({})", name, if saved { "saved" } else { "failed" }),
                });
            }
        }
    }

    if let Some(enc) = encounter_id.as_deref().filter(|s| !s.is_empty()) {
        let mut act = json!({
            "game_id": target.game_id,
            // THE SAVER'S ACTION, because it is their roll and their
            // hit points - 155's trigger names the log line off this.
            "character_id": target_character_id,
            "encounter_id": enc,
            "request": format!("{} save vs {}", ability, name),
            "label": format!("{} Save vs {} — DC {}", ability.to_uppercase(), name, dc),
            "key": resolved.key,
            // NOTHING. A save is not a thing you spend a turn on.
            "cost": "free",
        });
        if let Some(t) = target_actor_id.as_deref().filter(|s| !s.is_empty()) {
            act["actor_id"] = json!(t);
        }
        supabase::rpc(
            &token,
            "write_action",
            &json!({
                "p_action": act,
                "p_rolls": rolls,
                "p_hp": hp,
                "p_vitals": Value::Null,
            }),
        )?;
    }

    Ok(json!({
        "spell": name,
        "ability": ability,
        "dc": dc,
        "rolled": rolled.total,
        "detail": rolled.detail,
        "saved": saved,
        "damage": took,
        "said": format!(
            "{} rolled {} vs DC {} - {}{}",
            target.name,
            rolled.total,
            dc,
            if saved { "made it" } else { "failed" },
            match took {
                Some(n) if n > 0 => format!(", took {}", n),
                Some(_) => ", no damage".to_string(),
                None => String::new(),
            }
        ),
    }))
}

/// One creature's hit point maximum and the raw sum of its events.
///
/// 156. RAW, AND DELIBERATELY NOT FLOORED. `death::healed` needs to know
/// how far below zero the log actually runs, because that is the
/// difference between healing Falon to 8 and healing him to 7. Handing
/// it the floored reading would hide the hole it exists to account for -
/// see the rule's own comment.
///
/// BY CHARACTER AND NOT BY ACTOR. 023 moved damage onto the character
/// for everyone, monsters included, so there is no second place to look
/// and no question of which id a heal should land under.
fn hp_state(token: &str, character_id: &str) -> Result<(i64, i64), String> {
    let rows = supabase::rest_get(
        token,
        "characters",
        &[
            ("select", "hp_max"),
            ("id", &format!("eq.{}", character_id)),
        ],
    )?;
    let hp_max = rows
        .as_array()
        .and_then(|a| a.first())
        .and_then(|r| r.get("hp_max"))
        .and_then(|v| v.as_i64())
        // REFUSED RATHER THAN GUESSED. A creature with no maximum has no
        // ceiling to heal toward, and inventing one would be the engine
        // deciding how tough somebody is.
        .ok_or_else(|| "that creature has no hit point maximum to heal toward".to_string())?;

    let events = supabase::rest_get(
        token,
        "hp_events",
        &[
            ("select", "delta"),
            ("character_id", &format!("eq.{}", character_id)),
        ],
    )?;
    let summed = events
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|r| r.get("delta").and_then(|v| v.as_i64()))
        .sum();
    Ok((hp_max, summed))
}

fn load_chosen(
    token: &str,
    character_id: &str,
    source: Option<casting::Source>,
) -> Result<Vec<(String, String)>, String> {
    let rows = supabase::rest_get(
        token,
        "character_spells",
        &[
            ("select", "spell_key,state"),
            ("character_id", &format!("eq.{}", character_id)),
        ],
    )?;
    let mut out: Vec<(String, String)> = rows
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|r| {
            Some((
                r.get("spell_key")?.as_str()?.to_string(),
                r.get("state")
                    .and_then(|v| v.as_str())
                    .unwrap_or("prepared")
                    .to_string(),
            ))
        })
        .collect();

    // 172. AND WHAT IS WRITTEN IN THE BOOKS THEY ARE CARRYING.
    //
    // A wizard's book is not rows on the character any more - it is a
    // physical object with writing in it, so the spells they may
    // prepare from depend on what they have in hand. Everything that
    // asks what a caster holds comes through here, which is why this is
    // the only place that had to learn it: `may_reach` is unchanged and
    // still just asks whether the state is `book`.
    //
    // ONLY FOR A BOOK CASTER. A cleric reaches the whole list every day
    // and would pay three queries for an answer that cannot change what
    // they may do.
    //
    // `character_spells` WINS A TIE. A spell already prepared is
    // prepared, whatever the book says; adding `book` beside it would
    // make `find` return whichever came first.
    if source == Some(casting::Source::Book) {
        for key in in_books(token, character_id)? {
            if !out.iter().any(|(k, _)| *k == key) {
                out.push((key, "book".to_string()));
            }
        }
    }
    Ok(out)
}

/// Every spell written in a book this character is carrying.
///
/// 172. A BOOK IN HAND OR IN A PACK, and nothing further. 038 caps
/// container depth at one, so those two are every place a carried
/// object can be - there is no third level to walk.
///
/// A SCROLL DOES NOT COUNT, which is the rule and not an oversight: a
/// wizard copies a scroll into the book and prepares from the book. The
/// test is `items.spell_levels`, which only a book has.
///
/// THREE QUERIES, PAID BY WIZARDS. The count to watch, and the same
/// bargain `load_sheet` makes for its prepared list: a cleric, a
/// fighter and every creature in the bestiary pay none of it.
/// The slot levels that could carry a spell of `level`, and have one
/// left.
///
/// 185. FROM THE SPELL'S OWN LEVEL UPWARD. A 1st-level spell goes in a
/// 1st, 2nd or 3rd slot; a 3rd-level spell never goes in a 1st. Empty
/// means there is nothing left to cast it with, which is what `blocked`
/// then says in words.
fn slots_for(level: i64, left: &[i64; 9]) -> Vec<i64> {
    (level.max(1)..=9)
        .filter(|l| left[(*l - 1) as usize] > 0)
        .collect()
}

fn in_books(token: &str, character_id: &str) -> Result<Vec<String>, String> {
    // Who they are as a holder - 031.
    let me = supabase::rest_get(
        token,
        "characters",
        &[
            ("select", "entity_id"),
            ("id", &format!("eq.{}", character_id)),
        ],
    )?;
    let Some(entity) = me
        .as_array()
        .and_then(|a| a.first())
        .and_then(|r| r.get("entity_id"))
        .and_then(|v| v.as_str())
    else {
        return Ok(Vec::new());
    };

    // Anything they carry that is itself a holder: a pack, a chest.
    let carried = supabase::rest_get(
        token,
        "objects",
        &[
            ("select", "entity_id"),
            ("holder_id", &format!("eq.{}", entity)),
            ("entity_id", "not.is.null"),
        ],
    )?;
    let mut holders = vec![entity.to_string()];
    for r in carried.as_array().unwrap_or(&Vec::new()) {
        if let Some(e) = r.get("entity_id").and_then(|v| v.as_str()) {
            holders.push(e.to_string());
        }
    }

    // THE WRITING, WITH THE THING IT IS WRITTEN ON. 153's lesson: the
    // relationship is named even though `scribed_spells` has only one
    // foreign key to `objects` today, because "only one today" is what
    // 139 falsified for `encounter_actors`.
    let rows = supabase::rest_get(
        token,
        "scribed_spells",
        &[
            (
                "select",
                "spell_key,objects!scribed_spells_object_id_fkey!inner(item_key,holder_id)",
            ),
            ("objects.holder_id", &format!("in.({})", holders.join(","))),
        ],
    )?;

    // Which items are books. Small, and the only honest way to tell a
    // book from a scroll without hardcoding four keys that live in the
    // catalogue.
    let books = supabase::rest_get(
        token,
        "items",
        &[
            ("select", "key"),
            ("spell_levels", "not.is.null"),
            // 175. A SCROLL IS A ONE-PAGE BOOK AND HAS A `spell_levels`
            // TOO, so the column alone would let a wizard prepare
            // straight off a scroll - the one thing the rule is firm
            // about. You copy a scroll into the book and prepare from
            // the book. The tag is the discriminator.
            ("content_tags", "not.cs.{scroll}"),
        ],
    )?;
    let book_keys: std::collections::HashSet<String> = books
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|r| r.get("key")?.as_str().map(String::from))
        .collect();

    let mut out = Vec::new();
    for r in rows.as_array().unwrap_or(&Vec::new()) {
        let on = r.get("objects").and_then(|o| o.get("item_key")).and_then(|v| v.as_str());
        let Some(on) = on else { continue };
        if !book_keys.contains(on) {
            continue;
        }
        if let Some(k) = r.get("spell_key").and_then(|v| v.as_str()) {
            out.push(k.to_string());
        }
    }
    Ok(out)
}

/// One catalogue row with everything casting needs.
fn one_spell_full(token: &str, game_id: &str, key: &str) -> Result<Value, String> {
    let rows = supabase::rest_get(
        token,
        "spells",
        &[
            // 185. `at_higher_dice` IS LISTED HERE, and an explicit column
            // list is a second schema - 021's lesson, and the reason
            // `prof_bonus`, `character_name` and `special_text` each
            // existed unread for weeks. A column the query never asks
            // for does not exist as far as the engine is concerned.
            ("select", "key,name,level,cast_type,casting_time,save_ability,dice,duration,concentration,grants,on_save,at_higher_dice"),
            ("key", &format!("eq.{}", key)),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
            ("order", "game_id.asc.nullslast"),
        ],
    )?;
    rows.as_array()
        .and_then(|a| a.first())
        .cloned()
        .ok_or_else(|| format!("no spell with key '{}'", key))
}

/// One catalogue row, this game's before the global one.
fn one_spell(token: &str, game_id: &str, key: &str) -> Result<Value, String> {
    let rows = supabase::rest_get(
        token,
        "spells",
        &[
            ("select", "key,name,level,classes"),
            ("key", &format!("eq.{}", key)),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
            ("order", "game_id.asc.nullslast"),
        ],
    )?;
    rows.as_array()
        .and_then(|a| a.first())
        .cloned()
        .ok_or_else(|| format!("no spell with key '{}'", key))
}
