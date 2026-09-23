//! Money: the ladder, and paying with actual coins.
//!
//! THE ALGORITHM IS HOPPER'S. odyssey-engine's `merchant.rs` has had a
//! worked coin system for a long time - pay smallest-denomination-first,
//! give change largest-first, track what was overpaid when exact change
//! cannot be made, and do every sum in the base unit. All of that ports
//! unchanged. What does NOT port is the ladder: HOPPER counts in bronze
//! and algareth at 20 and 40,000 copper, and this is 5e.
//!
//! COINS ARE ITEMS, which is the decision everything else rests on.
//! HOPPER's `is_coin()` is literally `has_content_tag("coin")`, and 032
//! seeded gold, silver and copper as `loot` rows carrying that tag - so
//! the dummy turned out to be the right shape and only needed the rules
//! written against it. A coin is carried, dropped, stolen, weighed and
//! put in a purse like anything else, because it IS anything else.
//!
//! NO ELECTRUM. It is in 5e as written and `items.denom` still permits
//! it, because a price could be written that way and a conversion that
//! refuses real data is worse than one nobody uses. But no electrum
//! piece is minted: there is no `coin_ep`, and none of the 68 priced
//! items uses it.
//!
//! WHERE A COIN'S VALUE COMES FROM. Nothing new is stored. A coin is
//! priced like any other item - `coin_gp` is price 1, denom gp - so its
//! worth in copper is what `to_cp` makes of those two columns. A gold
//! piece being worth one gold piece is not a fact worth a column.

// THE MERCHANT HALF IS AHEAD OF ITS CALLER. `pay` and `make_change`
// have nothing invoking them yet - Buy and Sell were explicitly left
// for later - and they are written now because the algorithm came from
// HOPPER's merchant.rs and porting it while the source was open is
// cheaper than porting it from memory in a month. The tests are what
// make that worth doing rather than deferring.
#![allow(dead_code)]

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/* ============================ THE LADDER ============================ */

pub const CP_PER_CP: i64 = 1;
pub const CP_PER_SP: i64 = 10;
pub const CP_PER_GP: i64 = 100;
pub const CP_PER_PP: i64 = 1_000;
/// Fifty copper. Convertible but never minted - see the header.
pub const CP_PER_EP: i64 = 50;

/// What one of these is worth in copper, or None if it is not a
/// denomination this game knows.
pub fn cp_per(denom: &str) -> Option<i64> {
    match denom.trim().to_lowercase().as_str() {
        "cp" => Some(CP_PER_CP),
        "sp" => Some(CP_PER_SP),
        "ep" => Some(CP_PER_EP),
        "gp" => Some(CP_PER_GP),
        "pp" => Some(CP_PER_PP),
        _ => None,
    }
}

/// A price, in copper.
///
/// EVERY SUM HAPPENS IN COPPER and never in mixed denominations. That
/// is the one rule that keeps this arithmetic honest: the moment
/// something adds gold to silver it is wrong, and the moment everything
/// is an integer of copper it cannot be.
pub fn to_cp(price: i64, denom: &str) -> Result<i64, String> {
    let per = cp_per(denom).ok_or_else(|| format!("'{}' is not a coin this game knows", denom))?;
    Ok(price * per)
}

/// Copper, written the way a person would say it.
///
/// Largest first, and denominations worth nothing are left out - "1 gp,
/// 2 cp" rather than "1 gp, 0 sp, 2 cp". Zero prints as "0 cp" rather
/// than as nothing, because a price of nothing still has to read as a
/// price.
///
/// Electrum never appears here even though `cp_per` knows it. A total
/// broken into electrum would be arithmetically right and would name a
/// coin nobody has.
pub fn format_cp(cp: i64) -> String {
    if cp < 0 {
        return format!("-{}", format_cp(-cp));
    }
    let mut left = cp;
    let mut parts: Vec<String> = Vec::new();
    for (value, label) in [
        (CP_PER_PP, "pp"),
        (CP_PER_GP, "gp"),
        (CP_PER_SP, "sp"),
    ] {
        let n = left / value;
        if n > 0 {
            parts.push(format!("{} {}", n, label));
            left %= value;
        }
    }
    if left > 0 || parts.is_empty() {
        parts.push(format!("{} cp", left));
    }
    parts.join(", ")
}

/* ============================ A PURSE ============================ */

/// A stack of one kind of coin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coin {
    pub key: String,
    /// What it is called - cp, sp, gp, pp. Carried so a purse can be
    /// read out in the coins somebody ACTUALLY holds; see `held`.
    pub denom: String,
    pub value_cp: i64,
    pub count: i64,
}

/// What a pile of coins is worth.
pub fn total(coins: &[Coin]) -> i64 {
    coins.iter().map(|c| c.value_cp * c.count).sum()
}

/// A purse read out in the coins that are in it.
///
/// NOT `format_cp`, AND THE DIFFERENCE MATTERS. Eighteen gold and eight
/// silver is 1880 copper, which `format_cp` renders as "1 pp, 8 gp,
/// 8 sp" - arithmetically right, and it names a platinum piece nobody
/// is carrying. That is the correct answer to "what is this worth" and
/// the wrong one to "what is in my purse".
///
/// Both exist because both questions get asked: this one when showing
/// somebody their money, `format_cp` when pricing something.
pub fn held(coins: &[Coin]) -> String {
    let mut sorted: Vec<&Coin> = coins.iter().filter(|c| c.count > 0).collect();
    sorted.sort_by_key(|c| -c.value_cp);
    if sorted.is_empty() {
        return "empty".to_string();
    }
    sorted
        .iter()
        .map(|c| format!("{} {}", c.count, c.denom))
        .collect::<Vec<_>>()
        .join(", ")
}

/// What changed hands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Payment {
    /// Which coins were handed over, and how many of each.
    pub taken: Vec<(String, i64)>,
    /// What they came to. Never less than the price, and more when
    /// exact payment was impossible.
    pub paid_cp: i64,
    /// The difference, which the seller owes back as change.
    pub overpaid_cp: i64,
}

/// Pay a price out of a purse.
///
/// SMALLEST DENOMINATION FIRST, which is HOPPER's rule and the one a
/// person actually follows: you spend the coppers before you break a
/// gold piece. It also leaves the purse full of large coins rather than
/// small ones, which is what makes the NEXT purchase payable.
///
/// OVERPAYMENT IS RETURNED, NOT AVOIDED. Buying a 5 cp rope with
/// nothing but gold hands over a gold piece and records 95 cp owed. The
/// alternative - refusing the sale because the coins do not divide - is
/// not how a shop works.
pub fn pay(purse: &[Coin], price_cp: i64) -> Result<Payment, String> {
    if price_cp < 0 {
        return Err("a price cannot be negative".to_string());
    }
    if price_cp == 0 {
        return Ok(Payment { taken: Vec::new(), paid_cp: 0, overpaid_cp: 0 });
    }
    let funds = total(purse);
    if funds < price_cp {
        return Err(format!(
            "that costs {} and you have {}",
            format_cp(price_cp),
            format_cp(funds)
        ));
    }

    let mut sorted: Vec<&Coin> = purse.iter().filter(|c| c.value_cp > 0 && c.count > 0).collect();
    sorted.sort_by_key(|c| c.value_cp);

    let mut remaining = price_cp;
    let mut taken: Vec<(String, i64)> = Vec::new();
    let mut paid = 0;

    for coin in sorted {
        if remaining == 0 {
            break;
        }
        // Rounded UP: three coppers against a five copper debt is all
        // three, not one. Integer ceiling without touching floats.
        let needed = (remaining + coin.value_cp - 1) / coin.value_cp;
        let give = needed.min(coin.count);
        if give == 0 {
            continue;
        }
        taken.push((coin.key.clone(), give));
        let value = give * coin.value_cp;
        paid += value;
        remaining = (remaining - value).max(0);
    }

    Ok(Payment { taken, paid_cp: paid, overpaid_cp: paid - price_cp })
}

/// Make change out of a till.
///
/// LARGEST FIRST, the mirror of paying: change should come back as few
/// coins as the till allows, and a shopkeeper who hands over ninety
/// five coppers has emptied their own drawer of exactly what the next
/// customer will need.
///
/// GREEDY, AND THAT IS ENOUGH HERE. Largest-first does not give optimal
/// change for every possible coin system, but it does for any ladder
/// where each denomination divides the next - and 1, 10, 100, 1000 is
/// one. Electrum at 50 would break that, which is a second reason it is
/// not minted.
pub fn make_change(till: &[Coin], amount_cp: i64) -> Result<Vec<(String, i64)>, String> {
    if amount_cp <= 0 {
        return Ok(Vec::new());
    }
    let mut sorted: Vec<&Coin> = till.iter().filter(|c| c.value_cp > 0 && c.count > 0).collect();
    sorted.sort_by_key(|c| -c.value_cp);

    let mut remaining = amount_cp;
    let mut out: Vec<(String, i64)> = Vec::new();
    for coin in sorted {
        if remaining == 0 {
            break;
        }
        if coin.value_cp > remaining {
            continue;
        }
        let give = (remaining / coin.value_cp).min(coin.count);
        if give == 0 {
            continue;
        }
        out.push((coin.key.clone(), give));
        remaining -= give * coin.value_cp;
    }

    if remaining > 0 {
        return Err(format!(
            "cannot make exact change - {} short",
            format_cp(remaining)
        ));
    }
    Ok(out)
}


/// Spread "four gold" across the rows the gold actually lives in.
///
/// A RULE WEARING A WRAPPER, which is why it is here. It lived in
/// `commands/store.rs` and had no tests, because that directory's own
/// header says nothing in it should want one - and anything that does
/// belongs a level up. This wants one: it is the step between deciding
/// in KINDS, which `pay` does, and moving ROWS, which the database
/// does, and getting it wrong means paying with coins somebody does not
/// have.
///
/// `held` maps a coin key to the rows holding it, each with its count.
/// Taken in the order given, which is `acquired_at` - oldest money
/// first. As good a rule as any and, more usefully, a stable one.
pub fn allocate(
    taken: &[(String, i64)],
    held: &HashMap<String, Vec<(String, i64)>>,
) -> Result<Vec<(String, i64)>, String> {
    let mut plan: Vec<(String, i64)> = Vec::new();
    for (key, want) in taken {
        let mut owed = *want;
        let rows = held
            .get(key)
            .ok_or_else(|| format!("no {} found to pay with", key))?;
        for (object, have) in rows {
            if owed == 0 {
                break;
            }
            let take = owed.min(*have);
            if take > 0 {
                plan.push((object.clone(), take));
                owed -= take;
            }
        }
        if owed > 0 {
            return Err(format!("{} short of {}", owed, key));
        }
    }
    Ok(plan)
}

/* ============================ NETWORK ============================ */

/// Every coin this campaign mints, and what each is worth in copper.
///
/// FOUND BY TAG, not by a list of keys. `is_coin` in HOPPER is
/// `has_content_tag("coin")` and this is the same question asked of the
/// catalogue - so a campaign that adds its own trade bar tags it `coin`
/// and everything here starts counting it, with no code to change.
pub fn load_coins(token: &str, game_id: &str) -> Result<Vec<(String, String, i64)>, String> {
    let rows = crate::supabase::rest_get(
        token,
        "items",
        &[
            ("select", "key,game_id,price,denom"),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
            ("content_tags", "cs.{coin}"),
            ("order", "game_id.desc"),
        ],
    )?;
    let mut out: Vec<(String, String, i64)> = Vec::new();
    for r in rows.as_array().unwrap_or(&Vec::new()) {
        let key = r.get("key").and_then(|v| v.as_str()).unwrap_or("").to_string();
        // game_id.desc puts a campaign's own row first, so the first
        // one for a key wins - the same precedence collapse_overrides
        // applies, done by the sort.
        if key.is_empty() || out.iter().any(|(k, _, _)| *k == key) {
            continue;
        }
        let price = r.get("price").and_then(|v| v.as_i64()).unwrap_or(0);
        let denom = r.get("denom").and_then(|v| v.as_str()).unwrap_or("");
        // A coin nobody priced is worth nothing and is skipped rather
        // than counted as zero, which would quietly make a purse of
        // them read as empty.
        match to_cp(price, denom) {
            Ok(v) if v > 0 => out.push((key, denom.to_lowercase(), v)),
            _ => continue,
        }
    }
    Ok(out)
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    fn coin(key: &str, value: i64, count: i64) -> Coin {
        let denom = match value {
            1 => "cp", 10 => "sp", 100 => "gp", 1_000 => "pp", _ => "??",
        };
        Coin { key: key.into(), denom: denom.into(), value_cp: value, count }
    }

    fn purse() -> Vec<Coin> {
        vec![coin("coin_cp", 1, 7), coin("coin_sp", 10, 3), coin("coin_gp", 100, 2)]
    }

    /* ---------------- allocating across rows ------------------------ */

    fn rows(pairs: &[(&str, &str, i64)]) -> HashMap<String, Vec<(String, i64)>> {
        let mut m: HashMap<String, Vec<(String, i64)>> = HashMap::new();
        for (key, object, n) in pairs {
            m.entry((*key).into()).or_default().push(((*object).into(), *n));
        }
        m
    }

    #[test]
    fn coins_come_out_of_one_row_when_one_row_has_them() {
        let held = rows(&[("coin_gp", "purse", 18)]);
        assert_eq!(
            allocate(&[("coin_gp".into(), 15)], &held),
            Ok(vec![("purse".to_string(), 15)])
        );
    }

    #[test]
    fn and_across_several_when_they_are_scattered() {
        // Gold in a purse, a pocket and a pack - which is the ordinary
        // case once containers exist, and the reason this function does.
        let held = rows(&[
            ("coin_gp", "purse", 5),
            ("coin_gp", "pocket", 3),
            ("coin_gp", "pack", 10),
        ]);
        assert_eq!(
            allocate(&[("coin_gp".into(), 12)], &held),
            Ok(vec![
                ("purse".to_string(), 5),
                ("pocket".to_string(), 3),
                ("pack".to_string(), 4),
            ])
        );
    }

    #[test]
    fn it_stops_as_soon_as_the_debt_is_covered() {
        let held = rows(&[("coin_gp", "purse", 5), ("coin_gp", "pack", 10)]);
        assert_eq!(
            allocate(&[("coin_gp".into(), 5)], &held),
            Ok(vec![("purse".to_string(), 5)])
        );
    }

    #[test]
    fn more_than_is_held_is_refused_rather_than_part_paid() {
        let held = rows(&[("coin_gp", "purse", 2)]);
        let e = allocate(&[("coin_gp".into(), 5)], &held).unwrap_err();
        assert!(e.contains("3 short"), "{}", e);
    }

    #[test]
    fn a_coin_nobody_holds_is_refused() {
        let e = allocate(&[("coin_pp".into(), 1)], &rows(&[])).unwrap_err();
        assert!(e.contains("no coin_pp"), "{}", e);
    }

    #[test]
    fn several_kinds_are_planned_together() {
        let held = rows(&[("coin_gp", "purse", 2), ("coin_sp", "purse2", 9)]);
        let plan = allocate(
            &[("coin_gp".into(), 1), ("coin_sp".into(), 7)],
            &held,
        )
        .unwrap();
        assert_eq!(plan, vec![("purse".to_string(), 1), ("purse2".to_string(), 7)]);
    }

    #[test]
    fn empty_rows_in_the_map_are_skipped_not_counted() {
        let held = rows(&[("coin_gp", "spent", 0), ("coin_gp", "purse", 4)]);
        assert_eq!(
            allocate(&[("coin_gp".into(), 3)], &held),
            Ok(vec![("purse".to_string(), 3)])
        );
    }

    #[test]
    fn the_ladder_is_five_e_without_the_electrum() {
        assert_eq!(cp_per("cp"), Some(1));
        assert_eq!(cp_per("sp"), Some(10));
        assert_eq!(cp_per("gp"), Some(100));
        assert_eq!(cp_per("pp"), Some(1_000));
        // Convertible, never minted - a price could be written this way.
        assert_eq!(cp_per("ep"), Some(50));
        assert_eq!(cp_per("bp"), None, "bronze is HOPPER's, not 5e's");
    }

    #[test]
    fn a_price_becomes_copper() {
        assert_eq!(to_cp(50, "gp"), Ok(5_000));
        assert_eq!(to_cp(1, "cp"), Ok(1));
        assert!(to_cp(1, "quatloo").is_err());
    }

    #[test]
    fn copper_reads_the_way_a_person_says_it() {
        assert_eq!(format_cp(0), "0 cp");
        assert_eq!(format_cp(1), "1 cp");
        assert_eq!(format_cp(10), "1 sp");
        assert_eq!(format_cp(100), "1 gp");
        assert_eq!(format_cp(1_000), "1 pp");
        assert_eq!(format_cp(1_111), "1 pp, 1 gp, 1 sp, 1 cp");
    }

    #[test]
    fn empty_denominations_are_left_out() {
        // "1 gp, 2 cp", not "1 gp, 0 sp, 2 cp".
        assert_eq!(format_cp(102), "1 gp, 2 cp");
    }

    #[test]
    fn electrum_never_appears_in_a_total() {
        // 50 cp is arithmetically an electrum piece and nobody has one.
        assert_eq!(format_cp(50), "5 sp");
    }

    #[test]
    fn a_purse_reads_out_in_the_coins_it_holds() {
        // THE CASE THAT PROMPTED IT. 18 gold and 8 silver is 1880
        // copper, which format_cp calls "1 pp, 8 gp, 8 sp" - naming a
        // platinum piece nobody has.
        let p = vec![coin("coin_gp", 100, 18), coin("coin_sp", 10, 8)];
        assert_eq!(held(&p), "18 gp, 8 sp");
        assert_eq!(format_cp(total(&p)), "1 pp, 8 gp, 8 sp");
    }

    #[test]
    fn an_empty_purse_says_so() {
        assert_eq!(held(&[]), "empty");
        assert_eq!(held(&[coin("coin_gp", 100, 0)]), "empty");
    }

    #[test]
    fn a_purse_knows_what_it_is_worth() {
        assert_eq!(total(&purse()), 7 + 30 + 200);
        assert_eq!(format_cp(total(&purse())), "2 gp, 3 sp, 7 cp");
    }

    #[test]
    fn paying_spends_the_coppers_first() {
        // THE RULE. Five copper is paid with coppers, not by breaking a
        // silver, even though both would work.
        let p = pay(&purse(), 5).unwrap();
        assert_eq!(p.taken, vec![("coin_cp".to_string(), 5)]);
        assert_eq!(p.overpaid_cp, 0);
    }

    #[test]
    fn it_climbs_the_ladder_only_as_far_as_it_must() {
        // 7 cp then 2 sp covers 25; the gold is not touched.
        let p = pay(&purse(), 25).unwrap();
        assert_eq!(p.taken, vec![("coin_cp".into(), 7), ("coin_sp".into(), 2)]);
        assert_eq!(p.paid_cp, 27);
        assert_eq!(p.overpaid_cp, 2);
    }

    #[test]
    fn overpaying_is_recorded_not_refused() {
        // A 5 cp rope bought with nothing but gold.
        let gold = vec![coin("coin_gp", 100, 1)];
        let p = pay(&gold, 5).unwrap();
        assert_eq!(p.taken, vec![("coin_gp".to_string(), 1)]);
        assert_eq!(p.paid_cp, 100);
        assert_eq!(p.overpaid_cp, 95);
    }

    #[test]
    fn an_empty_purse_cannot_pay() {
        let e = pay(&[], 1).unwrap_err();
        assert!(e.contains("you have 0 cp"), "{}", e);
    }

    #[test]
    fn a_purse_that_is_short_says_by_how_much() {
        let e = pay(&purse(), 10_000).unwrap_err();
        assert!(e.contains("costs 10 pp"), "{}", e);
        assert!(e.contains("2 gp, 3 sp, 7 cp"), "{}", e);
    }

    #[test]
    fn nothing_costs_nothing() {
        let p = pay(&purse(), 0).unwrap();
        assert!(p.taken.is_empty());
        assert_eq!(p.overpaid_cp, 0);
    }

    #[test]
    fn change_comes_back_in_the_largest_coins_available() {
        let till = vec![coin("coin_cp", 1, 50), coin("coin_sp", 10, 20), coin("coin_gp", 100, 5)];
        // 95 back: no gold fits, so nine silver and five copper.
        assert_eq!(
            make_change(&till, 95),
            Ok(vec![("coin_sp".into(), 9), ("coin_cp".into(), 5)])
        );
    }

    #[test]
    fn a_till_without_small_coins_cannot_make_change() {
        let till = vec![coin("coin_gp", 100, 5)];
        let e = make_change(&till, 95).unwrap_err();
        assert!(e.contains("9 sp, 5 cp short"), "{}", e);
    }

    #[test]
    fn no_change_is_owed_on_an_exact_payment() {
        assert_eq!(make_change(&[], 0), Ok(Vec::new()));
    }

    #[test]
    fn paying_then_changing_balances() {
        // The property that matters: whatever was overpaid is exactly
        // what the till has to give back.
        let till = vec![coin("coin_cp", 1, 99), coin("coin_sp", 10, 9)];
        let p = pay(&vec![coin("coin_gp", 100, 1)], 5).unwrap();
        let back = make_change(&till, p.overpaid_cp).unwrap();
        let returned: i64 = back
            .iter()
            .map(|(k, n)| n * if k == "coin_sp" { 10 } else { 1 })
            .sum();
        assert_eq!(p.paid_cp - returned, 5);
    }
}
