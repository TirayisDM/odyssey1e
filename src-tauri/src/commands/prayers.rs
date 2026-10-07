//! The spell catalogue, and what a caster has chosen from it.
//!
//! 106. Plumbing. Every number - how many may be prepared, how many
//! cantrips are known, what slots exist, the save DC - is prayers.rs.
//!
//! THE CLERIC'S LEVEL, NEVER THE CHARACTER'S. A Fighter 4 / Cleric 1
//! prepares as a cleric 1, and reading their total of 5 would hand them
//! a 3rd-level slot they have not earned. The same rule
//! `features::held` and `uses::Context` already follow.

use serde_json::{json, Value};
use tauri::State;

use crate::prayers;
use crate::supabase::{self, AppState};

/// The spell catalogue for one class, global rows and this game's own.
///
/// BY CLASS RATHER THAN A TABLE PER CLASS. `spells.classes` is an
/// array, so the cleric list is a query and the wizard list is a seed
/// rather than a schema - see 101.
#[tauri::command]
pub fn list_spells(
    state: State<AppState>,
    game_id: String,
    class_key: String,
) -> Result<Value, String> {
    let token = state.token()?;
    let rows = supabase::rest_get(
        &token,
        "spells",
        &[
            (
                "select",
                "key,game_id,name,level,cast_type,category,school,save_ability,dice,\
                 concentration,ritual,range,duration,casting_time,components,material,\
                 classes,description",
            ),
            ("classes", &format!("cs.{{{}}}", class_key)),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
            ("order", "level.asc,name.asc"),
        ],
    )?;

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

/// What this cleric can do today: their numbers, and what they hold.
///
/// REFUSES A CHARACTER WHO IS NOT A CLERIC, rather than answering with
/// zeroes. A fighter has no prepared list and saying "0 of 0" would
/// invite the question of how to raise it.
#[tauri::command]
pub fn list_prayers(state: State<AppState>, character_id: String) -> Result<Value, String> {
    let token = state.token()?;
    let sheet = crate::character::load_sheet(&token, &character_id)?;

    // 150. WHOEVER IS CASTING, which is a class row and not a creature
    // question - see prayers::caster, and 022 for why there is no
    // second path for a monster.
    let Some(caster) = sheet.caster.clone() else {
        return Err(format!("{} does not cast spells", sheet.name));
    };
    let level = caster.level;
    let abil = sheet.ability_mod(&caster.ability);
    let pb = sheet.proficiency_bonus();

    let chosen = load_chosen(&token, &character_id)?;
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

    let slots = prayers::slots_at(level);
    let spent = load_slots(&token, &character_id)?;
    let left = prayers::slots_left(level, &spent);
    Ok(json!({
        "caster_level": level,
        "casting_ability": caster.ability,
        "ability_mod": abil,
        "class_key": caster.class_key,
        "source": caster.source,
        "save_dc": prayers::save_dc(pb, abil),
        "attack_bonus": prayers::attack_bonus(pb, abil),
        "prepared": prepared,
        "prepared_max": prayers::prepared_max(level, abil),
        "cantrips": cantrips,
        "book": book,
        "cantrips_known": prayers::cantrips_known(level),
        "top_slot": prayers::top_slot(level),
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
pub fn prepare_prayer(
    state: State<AppState>,
    character_id: String,
    spell_key: String,
) -> Result<Value, String> {
    let token = state.token()?;
    let sheet = crate::character::load_sheet(&token, &character_id)?;
    // 150. WHOEVER IS CASTING, which is a class row and not a creature
    // question - see prayers::caster, and 022 for why there is no
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

    let chosen = load_chosen(&token, &character_id)?;
    // 150. WHAT THEY MAY REACH FOR - the cleric list, or this wizard's
    // own book. See prayers::may_reach.
    prayers::may_reach(
        &caster,
        &on_lists,
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
        let max = prayers::cantrips_known(level);
        if known.len() as i64 >= max {
            return Err(format!(
                "that is {} cantrips and they know {} - forget one first",
                known.len(),
                max
            ));
        }
    } else {
        prayers::may_prepare(spell_level, level, abil, &held, &spell_key)?;
    }

    supabase::rest_upsert(
        &token,
        "character_prayers",
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
pub fn forget_prayer(
    state: State<AppState>,
    character_id: String,
    spell_key: String,
) -> Result<Value, String> {
    let token = state.token()?;
    supabase::rest_delete(
        &token,
        "character_prayers",
        &[
            ("character_id", &format!("eq.{}", character_id)),
            ("spell_key", &format!("eq.{}", spell_key)),
        ],
    )?;
    Ok(json!({ "ok": true }))
}

/// What this cleric can cast right now, as turn actions.
///
/// 110. PREPARED AND CANTRIPS ONLY, which is the whole point - the
/// catalogue is 106 long and what a cleric can do on their turn is the
/// dozen they are holding.
///
/// WHAT CANNOT BE CAST IS STILL LISTED, with the reason on it. Prayer
/// of Healing takes ten minutes and a fight is six seconds a round, so
/// it is shown as too long rather than hidden - a cleric reaching for
/// it should be told why rather than wondering where it went.
#[tauri::command]
pub fn castable(state: State<AppState>, character_id: String) -> Result<Value, String> {
    let token = state.token()?;
    let sheet = crate::character::load_sheet(&token, &character_id)?;
    // 150. WHOEVER IS CASTING, which is a class row and not a creature
    // question - see prayers::caster, and 022 for why there is no
    // second path for a monster.
    let Some(caster) = sheet.caster.clone() else {
        return Err(format!("{} does not cast spells", sheet.name));
    };
    let abil = sheet.ability_mod(&caster.ability);
    let pb = sheet.proficiency_bonus();

    let chosen = load_chosen(&token, &character_id)?;
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
    let left = prayers::slots_left(caster.level, &spent);

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
        } else if c.needs_slot && left[(c.level - 1).max(0) as usize] == 0 {
            Some(format!("no level {} slots left", c.level))
        } else {
            None
        };

        out.push(json!({
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
pub fn cast_prayer(
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
    let token = state.token()?;
    let sheet = crate::character::load_sheet(&token, &character_id)?;
    // 150. WHOEVER IS CASTING, which is a class row and not a creature
    // question - see prayers::caster, and 022 for why there is no
    // second path for a monster.
    let Some(caster) = sheet.caster.clone() else {
        return Err(format!("{} does not cast spells", sheet.name));
    };

    let held = load_chosen(&token, &character_id)?;
    if !held.iter().any(|(k, _)| *k == spell_key) {
        return Err("they do not have that prepared".to_string());
    }

    let spell = one_spell_full(&token, &sheet.game_id, &spell_key)?;
    let c = crate::spellcast::cast(
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
        prayers::may_spend_slot(caster.level, &spent, want)?;
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

    if let Some(enc) = encounter_id.as_deref().filter(|s| !s.is_empty()) {
        let mut row = json!({
            "game_id": sheet.game_id,
            "character_id": character_id,
            "encounter_id": enc,
            "request": c.name,
            "label": crate::spellcast::label(&c),
            "key": "spell",
            "status": "resolved",
            "cost": c.cost.as_str(),
        });
        if let Some(a) = actor_id.as_deref().filter(|s| !s.is_empty()) {
            row["actor_id"] = json!(a);
        }
        if let Some(t) = target_actor_id.as_deref().filter(|s| !s.is_empty()) {
            row["target_actor_id"] = json!(t);
        }
        supabase::rest_insert(&token, "actions", &row)?;
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
    prayers::may_spend_slot(level, &spent, slot_level)?;

    let i = (slot_level - 1) as usize;
    move_slot(&token, &character_id, slot_level, spent[i], spent[i] + 1)?;
    Ok(json!({ "left": prayers::slots_left(level, &spent)[i] - 1 }))
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
    Ok(json!({ "left": prayers::slots_left(level, &spent)[i] + 1 }))
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
/// 150. Was `cleric_level` and found a cleric class by name, which is
/// the lookup that kept a Lich out of its own spell list.
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
fn load_chosen(token: &str, character_id: &str) -> Result<Vec<(String, String)>, String> {
    let rows = supabase::rest_get(
        token,
        "character_prayers",
        &[
            ("select", "spell_key,state"),
            ("character_id", &format!("eq.{}", character_id)),
        ],
    )?;
    Ok(rows
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
        .collect())
}

/// One catalogue row with everything casting needs.
fn one_spell_full(token: &str, game_id: &str, key: &str) -> Result<Value, String> {
    let rows = supabase::rest_get(
        token,
        "spells",
        &[
            ("select", "key,name,level,cast_type,casting_time,save_ability,dice,duration,concentration,grants"),
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
