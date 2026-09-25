//! Buying something from somebody.
//!
//! PLUMBING, and unusually little of it - which is the point. A shop
//! was almost entirely expressible already: the keeper is a character
//! (022), the stock is a container (032), the till is coins (039), and
//! they stand somewhere (033). What this adds is the price gate between
//! two moves that already existed.
//!
//! THREE LAYERS, AND EACH KNOWS ONE THING.
//!   store.rs     what it costs
//!   currency.rs  which coins pay for it
//!   trade        that all of it lands or none of it does
//!
//! Nothing here decides any of those. What it does is find the
//! merchant, gather the purse, and put the three together.

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::{json, Value};
use tauri::State;

use crate::currency::{self, Coin};
use crate::holders::{self, Root};
use crate::store::{self, Counterparty, Disposition};
use crate::supabase::{self, AppState};

/// Who is selling this, or an error saying nobody is.
///
/// DERIVED FROM WHO HOLDS IT, not passed in. A thing on a shelf is
/// being sold by whoever owns the shelf, and asking the caller would
/// let them nominate a merchant who has never seen the goods.
fn seller_of(
    goods_holder: Option<&str>,
    world: &[holders::Obj],
    rows: &[holders::Holder],
) -> Result<String, String> {
    match holders::root_of(goods_holder, world, rows) {
        Root::Character { entity, .. } => Ok(entity),
        Root::Location { name, .. } => {
            Err(format!("that is lying in {} - nobody is selling it", name))
        }
        _ => Err("nobody is holding that, so nobody is selling it".to_string()),
    }
}

/// A character's coins, as stacks AND as the rows they live in.
///
/// TWO SHAPES OF THE SAME MONEY. `currency::pay` decides in kinds -
/// four gold, six silver - and `trade` moves rows. `currency::allocate`
/// turns one into the other; this gathers what it needs.
fn purse(
    profile_entity: &str,
    minted: &[(String, String, i64)],
    world: &[holders::Obj],
    rows: &[holders::Holder],
) -> (Vec<Coin>, HashMap<String, Vec<(String, i64)>>) {
    let mut stacks: Vec<Coin> = Vec::new();
    let mut where_: HashMap<String, Vec<(String, i64)>> = HashMap::new();

    for o in world {
        let Some((key, denom, value_cp)) = minted.iter().find(|(k, _, _)| *k == o.item_key) else {
            continue;
        };
        let mine = matches!(
            holders::root_of(o.holder_id.as_deref(), world, rows),
            Root::Character { ref entity, .. } if entity == profile_entity
        );
        if !mine {
            continue;
        }
        match stacks.iter_mut().find(|c| c.key == *key) {
            Some(c) => c.count += o.quantity,
            None => stacks.push(Coin {
                key: key.clone(),
                denom: denom.clone(),
                value_cp: *value_cp,
                count: o.quantity,
            }),
        }
        where_.entry(key.clone()).or_default().push((o.id.clone(), o.quantity));
    }
    (stacks, where_)
}

/// What one thing costs here.
///
/// Read-only, and the thing a shop screen calls for every row. The
/// haggle is passed in rather than rolled here - `store::haggle_margin`
/// turns a Persuasion against an Insight into it, and the dice happen
/// where every other roll in this app happens.
#[tauri::command]
pub fn quote_object(
    state: State<AppState>,
    object_id: String,
    haggle_margin: Option<i64>,
) -> Result<Value, String> {
    let token = state.token()?;
    let obj = crate::objects::load_object(&token, &object_id)?;
    let (world, rows, _) = holders::load_world(&token, &obj.game_id)?;
    let seller_entity = seller_of(obj.holder_id.as_deref(), &world, &rows)?;

    let keeper = supabase::rest_get(
        &token,
        "characters",
        &[
            ("select", "id,name,markup,disposition"),
            ("entity_id", &format!("eq.{}", seller_entity)),
        ],
    )?;
    let k = keeper
        .as_array()
        .and_then(|a| a.first())
        .cloned()
        .ok_or_else(|| "nobody is selling that".to_string())?;

    // NULL markup is not a shop. A goblin carrying a scimitar is not
    // offering it at list price.
    let markup = crate::supabase::numeric_at(&k, "markup").ok_or_else(|| {
        format!(
            "{} is not a merchant",
            k.get("name").and_then(|v| v.as_str()).unwrap_or("that creature")
        )
    })?;
    let disposition = Disposition::parse(k.get("disposition").and_then(|v| v.as_str()));

    let item = crate::equipment::load_item(&token, &obj.game_id, &obj.item_key)?
        .ok_or_else(|| format!("no item with key '{}'", obj.item_key))?;
    let base_cp = base_price(&token, &obj.game_id, &obj.item_key)?;

    // Condition is 1.0 everywhere: nothing in this schema records wear.
    let mut q = store::quote(base_cp, markup, 1.0, disposition)?;
    if let Some(m) = haggle_margin {
        q = store::haggle(&q, m);
    }

    Ok(json!({
        "item": obj.name.clone().unwrap_or(item.name),
        "quantity": obj.quantity,
        "merchant": k.get("name").and_then(|v| v.as_str()).unwrap_or(""),
        "merchant_id": k.get("id").and_then(|v| v.as_str()).unwrap_or(""),
        "disposition": disposition.as_str(),
        "buy_cp": q.buy_cp,
        "sell_cp": q.sell_cp,
        "haggled": q.haggled,
        "said": format!(
            "{} for {}",
            obj.name.clone().unwrap_or_else(|| item.key.clone()),
            currency::format_cp(q.buy_cp)
        ),
        "price": currency::format_cp(q.buy_cp),
        "shop_pays": currency::format_cp(q.sell_cp),
    }))
}

/// List price in copper, from the catalogue's own price and denom.
fn base_price(token: &str, game_id: &str, key: &str) -> Result<i64, String> {
    let rows = supabase::rest_get(
        token,
        "items",
        &[
            ("select", "key,game_id,price,denom"),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
            ("key", &format!("eq.{}", key)),
            ("order", "game_id.desc"),
        ],
    )?;
    let r = rows
        .as_array()
        .and_then(|a| a.first())
        .ok_or_else(|| format!("no item with key '{}'", key))?;
    let price = r.get("price").and_then(|v| v.as_i64()).unwrap_or(0);
    let denom = r.get("denom").and_then(|v| v.as_str()).unwrap_or("cp");
    currency::to_cp(price, denom)
}

/// Turn a Persuasion against an Insight into a haggle margin.
///
/// THE DICE HAPPEN WHERE EVERY OTHER ROLL DOES. This does not roll -
/// the two totals come from the roll box like any other contest, and
/// this is only the conversion. Positive is a discount the buyer won.
#[tauri::command]
pub fn haggle_margin(persuasion: i64, insight: i64) -> Value {
    let m = store::haggle_margin(persuasion, insight);
    json!({
        "margin": m,
        "said": match m {
            0 => "no ground given either way".to_string(),
            n if n > 0 => format!("{}% off", n),
            n => format!("{}% more", -n),
        },
    })
}

/* ============================ BOTH WAYS ============================ */

/// One thing somebody is putting on the table.
///
/// `count` of None means the whole row, which is what a sword is.
/// Arrows and coins are the reason it is an option at all.
#[derive(Debug, Clone, Deserialize)]
pub struct Offer {
    pub object: String,
    pub count: Option<i64>,
}

/// What kind of table this is, and at whose prices.
///
/// A MARKUP IS WHAT MAKES SOMEBODY A SHOP. No markup column, no
/// merchant - so two players trading get `Peer` with no configuration,
/// which is the case that would otherwise need a flag somebody forgets
/// to set.
fn counterparty(
    token: &str,
    character_id: &str,
) -> Result<(Counterparty, f64, Disposition), String> {
    let rows = supabase::rest_get(
        token,
        "characters",
        &[
            ("select", "markup,disposition"),
            ("id", &format!("eq.{}", character_id)),
        ],
    )?;
    let r = rows
        .as_array()
        .and_then(|a| a.first())
        .cloned()
        .ok_or_else(|| "no such creature, or they are not visible to you".to_string())?;
    let markup = crate::supabase::numeric_at(&r, "markup");
    let disposition = Disposition::parse(r.get("disposition").and_then(|v| v.as_str()));
    match markup {
        Some(m) => Ok((Counterparty::Merchant, m, disposition)),
        None => Ok((Counterparty::Peer, 1.0, Disposition::Neutral)),
    }
}

/// Value one side's offering, in the direction it is moving.
#[allow(clippy::too_many_arguments)]
fn value_side(
    token: &str,
    game_id: &str,
    offers: &[Offer],
    world: &[holders::Obj],
    toward_a: bool,
    with: Counterparty,
    markup: f64,
    disposition: Disposition,
    haggle_margin: Option<i64>,
) -> Result<(i64, Vec<Value>), String> {
    let mut total = 0;
    let mut lines: Vec<Value> = Vec::new();
    for off in offers {
        let o = world
            .iter()
            .find(|x| x.id == off.object)
            .ok_or_else(|| "something in that trade is not visible to you".to_string())?;
        let n = off.count.unwrap_or(o.quantity);
        if n <= 0 || n > o.quantity {
            return Err(format!("cannot offer {} of {} {}", n, o.quantity, o.item_key));
        }
        let base = base_price(token, game_id, &o.item_key)?;
        let mut q = store::quote(base, markup, 1.0, disposition)?;
        if let Some(m) = haggle_margin {
            q = store::haggle(&q, m);
        }
        let each = store::worth(&q, toward_a, with);
        total += each * n;
        lines.push(json!({
            "object": o.id,
            "item": o.name.clone().unwrap_or_else(|| o.item_key.clone()),
            "count": n,
            "each_cp": each,
            "line_cp": each * n,
        }));
    }
    Ok((total, lines))
}

/// What a proposed trade comes to, without doing it.
///
/// `a` is the side asking - usually a player - and the sign convention
/// is theirs: a positive `owed_cp` means A pays. See `store::settle`,
/// which exists to hold that convention in one place.
#[tauri::command]
pub fn quote_trade(
    state: State<AppState>,
    a_id: String,
    b_id: String,
    a_gives: Vec<Offer>,
    b_gives: Vec<Offer>,
    haggle_margin: Option<i64>,
) -> Result<Value, String> {
    let token = state.token()?;
    if a_id == b_id {
        return Err("a trade needs two sides".to_string());
    }
    let a = crate::character::load_profile(&token, &a_id)?;
    let (kind, markup, disposition) = counterparty(&token, &b_id)?;
    // A sworn enemy declines before anything is priced.
    disposition
        .factor()
        .ok_or_else(|| "they will not trade with you".to_string())?;

    let (world, _, _) = holders::load_world(&token, &a.game_id)?;

    let (incoming, in_lines) = value_side(
        &token, &a.game_id, &b_gives, &world, true, kind, markup, disposition, haggle_margin,
    )?;
    let (outgoing, out_lines) = value_side(
        &token, &a.game_id, &a_gives, &world, false, kind, markup, disposition, haggle_margin,
    )?;

    let bal = store::settle(incoming, outgoing);
    Ok(json!({
        "counterparty": match kind {
            Counterparty::Merchant => "merchant",
            Counterparty::Peer => "peer",
        },
        "disposition": disposition.as_str(),
        "incoming_cp": bal.incoming_cp,
        "outgoing_cp": bal.outgoing_cp,
        "owed_cp": bal.owed_cp,
        "receiving": in_lines,
        "giving": out_lines,
        "said": match bal.owed_cp {
            0 => "an even trade".to_string(),
            n if n > 0 => format!("you pay {}", currency::format_cp(n)),
            n => format!("they pay you {}", currency::format_cp(-n)),
        },
    }))
}

/// Do it.
///
/// ONE PLAN, ONE CALL. The goods both ways, the coin that settles the
/// difference, and the change - assembled here and handed to `trade`,
/// which lands all of it or none. Over REST these would be six round
/// trips and the gap between any two of them is somebody robbed.
///
/// `free` is a gift: the goods move and no money does. A parameter
/// rather than a separate command, because a gift IS a trade where both
/// sides agree nothing is owed, and giving it its own path would be a
/// second place for the moves to be assembled.
#[tauri::command]
pub fn execute_trade(
    state: State<AppState>,
    a_id: String,
    b_id: String,
    a_gives: Vec<Offer>,
    b_gives: Vec<Offer>,
    haggle_margin: Option<i64>,
    free: Option<bool>,
) -> Result<Value, String> {
    let token = state.token()?;
    let quoted = quote_trade(
        state.clone(),
        a_id.clone(),
        b_id.clone(),
        a_gives.clone(),
        b_gives.clone(),
        haggle_margin,
    )?;
    let owed = quoted.get("owed_cp").and_then(|v| v.as_i64()).unwrap_or(0);
    let gift = free.unwrap_or(false);

    let a = crate::character::load_profile(&token, &a_id)?;
    let b = crate::character::load_profile(&token, &b_id)?;
    let minted = currency::load_coins(&token, &a.game_id)?;
    let (world, rows, _) = holders::load_world(&token, &a.game_id)?;

    // The goods, both directions.
    let mut moves: Vec<Value> = Vec::new();
    for off in &a_gives {
        moves.push(json!({ "object": off.object, "count": off.count, "to": "b" }));
    }
    for off in &b_gives {
        moves.push(json!({ "object": off.object, "count": off.count, "to": "a" }));
    }

    let mut paid = 0;
    let mut change = 0;
    if !gift && owed != 0 {
        // WHOEVER OWES, PAYS. The only asymmetry left, and it is decided
        // by a sign rather than by which argument came first.
        let (debtor, creditor, to_creditor, to_debtor) =
            if owed > 0 { (&a, &b, "b", "a") } else { (&b, &a, "a", "b") };
        let amount = owed.abs();

        let (funds, fund_rows) = purse(&debtor.entity_id, &minted, &world, &rows);
        let payment = currency::pay(&funds, amount)?;
        for (object, count) in currency::allocate(&payment.taken, &fund_rows)? {
            moves.push(json!({ "object": object, "count": count, "to": to_creditor }));
        }
        paid = payment.paid_cp;

        if payment.overpaid_cp > 0 {
            let (till, till_rows) = purse(&creditor.entity_id, &minted, &world, &rows);
            let owed_back = currency::make_change(&till, payment.overpaid_cp)?;
            for (object, count) in currency::allocate(&owed_back, &till_rows)? {
                moves.push(json!({ "object": object, "count": count, "to": to_debtor }));
            }
            change = payment.overpaid_cp;
        }
    }

    supabase::rpc(
        &token,
        "trade",
        &json!({ "p_a": a_id, "p_b": b_id, "p_moves": moves }),
    )?;

    Ok(json!({
        "said": if gift {
            "given".to_string()
        } else {
            match owed {
                0 => "traded, even".to_string(),
                n if n > 0 => format!("traded, paid {}", currency::format_cp(n)),
                n => format!("traded, received {}", currency::format_cp(-n)),
            }
        },
        "owed_cp": owed,
        "coins_handed_over_cp": paid,
        "change_cp": change,
        "moves": moves.len(),
    }))
}
