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
//!   buy_object   that all of it lands or none of it does
//!
//! Nothing here decides any of those. What it does is find the
//! merchant, gather the purse, and put the three together.

use std::collections::HashMap;

use serde_json::{json, Value};
use tauri::State;

use crate::currency::{self, Coin};
use crate::holders::{self, Root};
use crate::store::{self, Disposition};
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
/// four gold, six silver - and `buy_object` moves rows. The map is what
/// turns one into the other, and it is built here rather than inside
/// `pay` because a rule that knows about object ids is a rule that has
/// started doing plumbing.
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

/// Turn "four gold" into "two from this row, two from that one".
///
/// Coins of a kind may be split across a purse, a pocket and a pack,
/// and `pay` neither knows nor should. Taken in the order found, which
/// is `acquired_at` - oldest money first, which is as good a rule as
/// any and at least a stable one.
fn allocate(
    taken: &[(String, i64)],
    where_: &HashMap<String, Vec<(String, i64)>>,
) -> Result<Vec<Value>, String> {
    let mut plan: Vec<Value> = Vec::new();
    for (key, mut owed) in taken.iter().map(|(k, n)| (k.clone(), *n)) {
        let rows = where_
            .get(&key)
            .ok_or_else(|| format!("no {} found to pay with", key))?;
        for (object, have) in rows {
            if owed == 0 {
                break;
            }
            let take = owed.min(*have);
            plan.push(json!({ "object": object, "count": take }));
            owed -= take;
        }
        if owed > 0 {
            return Err(format!("{} short of {}", owed, key));
        }
    }
    Ok(plan)
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
    let markup = k
        .get("markup")
        .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .ok_or_else(|| {
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

/// Buy it.
///
/// Quote, find the coins, hand them over and take the goods - and the
/// last three happen inside `buy_object`, which is a single statement
/// as far as Postgres is concerned. Over REST they would be three round
/// trips, and the gap between the first and the second is a free item.
#[tauri::command]
pub fn buy(
    state: State<AppState>,
    object_id: String,
    buyer_id: String,
    haggle_margin: Option<i64>,
) -> Result<Value, String> {
    let token = state.token()?;
    let quoted = quote_object(state.clone(), object_id.clone(), haggle_margin)?;
    let price = quoted.get("buy_cp").and_then(|v| v.as_i64()).unwrap_or(0);
    let merchant_id = quoted
        .get("merchant_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let buyer = crate::character::load_profile(&token, &buyer_id)?;
    let minted = currency::load_coins(&token, &buyer.game_id)?;
    let (world, rows, _) = holders::load_world(&token, &buyer.game_id)?;

    let (mine, my_rows) = purse(&buyer.entity_id, &minted, &world, &rows);
    // Refuses before anything moves, and says what it costs against
    // what they have.
    let payment = currency::pay(&mine, price)?;
    let pay_plan = allocate(&payment.taken, &my_rows)?;

    // CHANGE COMES OUT OF THE TILL, so a shop that cannot break a gold
    // piece cannot take one. That is the merchant's problem to solve by
    // keeping small coin, which is what makes `make_change` refusing a
    // real event rather than a bug.
    let mut change_plan: Vec<Value> = Vec::new();
    if payment.overpaid_cp > 0 {
        let seller = crate::character::load_profile(&token, &merchant_id)?;
        let (till, till_rows) = purse(&seller.entity_id, &minted, &world, &rows);
        let owed = currency::make_change(&till, payment.overpaid_cp)?;
        change_plan = allocate(&owed, &till_rows)?;
    }

    supabase::rpc(
        &token,
        "buy_object",
        &json!({
            "p_object": object_id,
            "p_buyer": buyer_id,
            "p_merchant": merchant_id,
            "p_pay": pay_plan,
            "p_change": change_plan,
        }),
    )?;

    Ok(json!({
        "said": format!(
            "bought {} for {}{}",
            quoted.get("item").and_then(|v| v.as_str()).unwrap_or("it"),
            currency::format_cp(price),
            if payment.overpaid_cp > 0 {
                format!(", {} change", currency::format_cp(payment.overpaid_cp))
            } else {
                String::new()
            }
        ),
        "paid_cp": price,
        "change_cp": payment.overpaid_cp,
        "haggled": quoted.get("haggled").cloned().unwrap_or(json!(0)),
    }))
}
