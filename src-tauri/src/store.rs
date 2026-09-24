//! What a shop charges, and what it will pay.
//!
//! THE MODEL IS HOPPER'S, from odyssey-engine's `merchant.rs`:
//!
//!   buy  = base x condition x markup x disposition   (floored)
//!   sell = buy x SELL_RATIO
//!
//! Four multipliers, and TWO OF THEM HAVE NOTHING TO FEED THEM HERE.
//! odyssey1e has no item condition and, until 040, no disposition - so
//! they arrive as PARAMETERS rather than as invented subsystems. Pass
//! 1.0 and the shop prices on markup alone, which is a real shop.
//!
//! That is the same seam `acquire.rs` leaves for traps: the shape is
//! right, the input is honest about not existing yet, and nothing here
//! pretends to know something it does not.
//!
//! EVERYTHING IS COPPER. `currency` made that the rule and this obeys
//! it - a price that is ever briefly in gold is a price that will
//! eventually be added to one in silver.
//!
//! ROUNDING IS THE SHOP'S. Both prices floor at one copper when the
//! thing is worth anything at all, because no shop sells for nothing
//! and no shop pays nothing for something. A base of zero stays zero:
//! a worthless thing is worthless, and inventing a copper for it would
//! make free goods a revenue stream.

use serde::{Deserialize, Serialize};

/* ============================ THE LADDER ============================ */

/// What a merchant pays, as a fraction of what they charge.
///
/// HOPPER's number, kept deliberately over 5e's informal half. Forty
/// percent is the difference between adventuring and shopkeeping being
/// the profitable trade, and a party that can buy at 100 and sell at 50
/// will eventually notice.
pub const SELL_RATIO: f64 = 0.40;

/// The most a haggle can move a price, either way.
///
/// Twenty five percent. Without a cap a lucky Persuasion roll against
/// an unlucky Insight halves the price of a suit of plate, which is a
/// worse outcome than losing the roll.
pub const HAGGLE_CAP: i64 = 25;

/// How a merchant feels about the customer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Disposition {
    Allied,
    Friendly,
    Warm,
    Neutral,
    Unfriendly,
    Hostile,
    /// Will not trade at any price. Not a multiplier - a refusal.
    SwornEnemy,
}

impl Disposition {
    /// The multiplier, or None when they will not deal with you.
    ///
    /// NONE IS NOT 'VERY EXPENSIVE'. A sworn enemy does not quote a
    /// high price, they decline - which is why this is an Option and
    /// not a large number. Collapsing the two would make every refusal
    /// purchasable by a rich enough party.
    pub fn factor(self) -> Option<f64> {
        Some(match self {
            Disposition::Allied => 0.80,
            Disposition::Friendly => 0.90,
            Disposition::Warm => 0.95,
            Disposition::Neutral => 1.00,
            Disposition::Unfriendly => 1.10,
            Disposition::Hostile => 1.25,
            Disposition::SwornEnemy => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Disposition::Allied => "allied",
            Disposition::Friendly => "friendly",
            Disposition::Warm => "warm",
            Disposition::Neutral => "neutral",
            Disposition::Unfriendly => "unfriendly",
            Disposition::Hostile => "hostile",
            Disposition::SwornEnemy => "sworn_enemy",
        }
    }

    /// The same words, but a typo is refused rather than quietly
    /// becoming neutral.
    ///
    /// TWO PARSERS ON PURPOSE. `parse` is for READING the database,
    /// where an unexpected value is somebody else's problem and neutral
    /// is the safe reading. This is for WRITING, where "freindly"
    /// silently becoming neutral would leave a DM wondering why their
    /// discount never applied. 028 is what that looks like when it goes
    /// unnoticed.
    pub fn parse_strict(s: &str) -> Result<Disposition, String> {
        match s.trim().to_lowercase().as_str() {
            "allied" => Ok(Disposition::Allied),
            "friendly" => Ok(Disposition::Friendly),
            "warm" => Ok(Disposition::Warm),
            "neutral" => Ok(Disposition::Neutral),
            "unfriendly" => Ok(Disposition::Unfriendly),
            "hostile" => Ok(Disposition::Hostile),
            "sworn_enemy" => Ok(Disposition::SwornEnemy),
            other => Err(format!(
                "'{}' is not a disposition - use allied, friendly, warm, \
                 neutral, unfriendly, hostile or sworn_enemy",
                other
            )),
        }
    }

    /// NULL in the database means neutral - a shopkeeper nobody has an
    /// opinion about is a shopkeeper who charges list.
    pub fn parse(s: Option<&str>) -> Disposition {
        match s.unwrap_or("neutral").trim().to_lowercase().as_str() {
            "allied" => Disposition::Allied,
            "friendly" => Disposition::Friendly,
            "warm" => Disposition::Warm,
            "unfriendly" => Disposition::Unfriendly,
            "hostile" => Disposition::Hostile,
            "sworn_enemy" => Disposition::SwornEnemy,
            _ => Disposition::Neutral,
        }
    }
}

/* ============================ THE QUOTE ============================ */

/// What one thing costs here, both ways.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quote {
    pub base_cp: i64,
    /// What the shop charges the customer.
    pub buy_cp: i64,
    /// What the shop pays the customer.
    pub sell_cp: i64,
    /// The haggle already applied, as a percentage. Positive is a
    /// discount the buyer won.
    pub haggled: i64,
}

/// Price one thing.
///
/// `markup` is the shop's own margin - 1.0 is list. `condition` is the
/// state of the goods, and is 1.0 everywhere today because nothing in
/// this schema records wear.
pub fn quote(
    base_cp: i64,
    markup: f64,
    condition: f64,
    disposition: Disposition,
) -> Result<Quote, String> {
    if base_cp < 0 {
        return Err("a price cannot be negative".to_string());
    }
    let disp = disposition
        .factor()
        .ok_or_else(|| "they will not trade with you".to_string())?;

    let buy = floor_price(base_cp as f64 * markup.max(0.0) * condition.max(0.0) * disp, base_cp);
    Ok(Quote {
        base_cp,
        buy_cp: buy,
        sell_cp: floor_price(buy as f64 * SELL_RATIO, base_cp),
        haggled: 0,
    })
}

/// A copper is the smallest thing anybody trades in.
///
/// Floors at one when the goods are worth anything, and keeps zero at
/// zero. The `base` argument is what decides which - not the computed
/// price, because a very cheap thing heavily discounted still costs
/// something and a free thing never does.
fn floor_price(computed: f64, base_cp: i64) -> i64 {
    if base_cp == 0 {
        return 0;
    }
    (computed.round() as i64).max(1)
}

/// Shift a quote by a haggle margin.
///
/// POSITIVE IS A DISCOUNT, which is HOPPER's convention and reads the
/// way people talk: "I got twenty percent off". A negative margin is
/// the merchant winning, and the price goes up - losing a haggle costs
/// you, which is what makes trying it a decision.
///
/// The sell price is recomputed FROM the haggled buy price rather than
/// shifted separately, so the shop's margin survives the negotiation.
pub fn haggle(q: &Quote, margin_pct: i64) -> Quote {
    let capped = margin_pct.clamp(-HAGGLE_CAP, HAGGLE_CAP);
    let shift = 1.0 - (capped as f64 / 100.0);
    let buy = floor_price(q.buy_cp as f64 * shift, q.base_cp);
    Quote {
        base_cp: q.base_cp,
        buy_cp: buy,
        sell_cp: floor_price(buy as f64 * SELL_RATIO, q.base_cp),
        haggled: capped,
    }
}

/// The margin a haggle won, from Persuasion against Insight.
///
/// THE DIFFERENCE IS THE PERCENTAGE. Beat them by ten and the price
/// moves ten percent your way; lose by ten and it moves ten against.
/// Symmetric, which is what makes it a gamble rather than a free roll.
///
/// A TIE MOVES NOTHING, the same rule every contest in this codebase
/// uses - see `acquire::taker_wins`. The roll decides a CHANGE, and a
/// tie is the absence of one.
pub fn haggle_margin(persuasion: i64, insight: i64) -> i64 {
    (persuasion - insight).clamp(-HAGGLE_CAP, HAGGLE_CAP)
}

/* ============================ BOTH WAYS ============================ */

/// Who is on the other side of the table.
///
/// THE SELL RATIO IS A MERCHANT'S, NOT A LAW. A shop buys at 0.40
/// because it has to make a living reselling. Two players swapping a
/// sword each are not making a living off one another, and valuing both
/// sides at 40% would mean an even trade left both of them poorer - the
/// goods would evaporate on the way across the table.
///
/// So a peer values goods at LIST in both directions. An even swap is
/// even, which is the only answer that lets a party divide loot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Counterparty {
    Merchant,
    Peer,
}

/// What one thing is worth in this trade, in the direction it moves.
///
/// `toward_you` is the whole asymmetry. The same sword is worth its buy
/// price coming from a shop and its sell price going back, and the gap
/// between those is the shop's trade.
pub fn worth(q: &Quote, toward_you: bool, with: Counterparty) -> i64 {
    match (with, toward_you) {
        (Counterparty::Merchant, true) => q.buy_cp,
        (Counterparty::Merchant, false) => q.sell_cp,
        (Counterparty::Peer, _) => q.base_cp,
    }
}

/// Where a two-sided trade lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Balance {
    /// What is coming to you, valued in the direction it moves.
    pub incoming_cp: i64,
    /// What is going the other way.
    pub outgoing_cp: i64,
    /// Positive means YOU owe coin. Negative means they do. Zero means
    /// the goods balanced and no money changes hands, which is what a
    /// straight swap looks like.
    pub owed_cp: i64,
}

/// Settle a trade.
///
/// Deliberately trivial, and deliberately its own function. The
/// arithmetic is a subtraction; what it carries is the SIGN CONVENTION,
/// and a sign convention invented separately at three call sites is
/// three chances to pay somebody for taking your sword.
pub fn settle(incoming_cp: i64, outgoing_cp: i64) -> Balance {
    Balance {
        incoming_cp,
        outgoing_cp,
        owed_cp: incoming_cp - outgoing_cp,
    }
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    fn list(base: i64) -> Quote {
        quote(base, 1.0, 1.0, Disposition::Neutral).unwrap()
    }

    #[test]
    fn a_neutral_shop_at_list_charges_list() {
        let q = list(1_500);
        assert_eq!(q.buy_cp, 1_500);
    }

    #[test]
    fn a_shop_pays_forty_percent_of_what_it_charges() {
        // The decision: HOPPER's 0.40 rather than 5e's informal half.
        assert_eq!(list(1_500).sell_cp, 600);
        assert_eq!(list(100).sell_cp, 40);
    }

    #[test]
    fn disposition_moves_the_price_both_ways() {
        let allied = quote(1_000, 1.0, 1.0, Disposition::Allied).unwrap();
        let hostile = quote(1_000, 1.0, 1.0, Disposition::Hostile).unwrap();
        assert_eq!(allied.buy_cp, 800);
        assert_eq!(hostile.buy_cp, 1_250);
    }

    #[test]
    fn a_sworn_enemy_does_not_quote_a_price() {
        // NOT a very large number. Collapsing refusal into expense
        // would make every enemy purchasable by a rich enough party.
        assert_eq!(Disposition::SwornEnemy.factor(), None);
        let e = quote(1_000, 1.0, 1.0, Disposition::SwornEnemy).unwrap_err();
        assert!(e.contains("will not trade"), "{}", e);
    }

    #[test]
    fn markup_is_the_shops_own_margin() {
        let gouging = quote(1_000, 1.5, 1.0, Disposition::Neutral).unwrap();
        assert_eq!(gouging.buy_cp, 1_500);
    }

    #[test]
    fn the_multipliers_compound() {
        // 1000 x 1.2 markup x 0.9 friendly = 1080.
        let q = quote(1_000, 1.2, 1.0, Disposition::Friendly).unwrap();
        assert_eq!(q.buy_cp, 1_080);
    }

    #[test]
    fn condition_is_a_parameter_that_nothing_feeds_yet() {
        // Half-ruined goods at half price. Nothing in this schema
        // records wear, so every live caller passes 1.0 - the seam is
        // real and the subsystem is honestly absent.
        let q = quote(1_000, 1.0, 0.5, Disposition::Neutral).unwrap();
        assert_eq!(q.buy_cp, 500);
    }

    #[test]
    fn nothing_is_ever_sold_for_nothing() {
        // A 1 cp candle, heavily discounted, still costs a copper.
        let q = quote(1, 1.0, 1.0, Disposition::Allied).unwrap();
        assert_eq!(q.buy_cp, 1);
        assert_eq!(q.sell_cp, 1);
    }

    #[test]
    fn but_a_worthless_thing_stays_worthless() {
        // Otherwise free goods become a revenue stream.
        let q = list(0);
        assert_eq!(q.buy_cp, 0);
        assert_eq!(q.sell_cp, 0);
    }

    #[test]
    fn winning_a_haggle_is_a_discount() {
        let q = haggle(&list(1_000), 20);
        assert_eq!(q.buy_cp, 800);
        assert_eq!(q.haggled, 20);
    }

    #[test]
    fn losing_one_costs_you() {
        // The reason trying it is a decision rather than a free roll.
        let q = haggle(&list(1_000), -20);
        assert_eq!(q.buy_cp, 1_200);
    }

    #[test]
    fn the_shops_margin_survives_the_negotiation() {
        // Sell is recomputed from the haggled buy, not shifted on its
        // own - so talking the price down does not also make them pay
        // you more for the same goods.
        let q = haggle(&list(1_000), 20);
        assert_eq!(q.sell_cp, 320); // 800 x 0.40
    }

    #[test]
    fn a_haggle_cannot_halve_a_suit_of_plate() {
        let q = haggle(&list(150_000), 90);
        assert_eq!(q.haggled, HAGGLE_CAP);
        assert_eq!(q.buy_cp, 112_500); // 25% off, not 90%
    }

    #[test]
    fn persuasion_against_insight_sets_the_margin() {
        assert_eq!(haggle_margin(18, 8), 10);
        assert_eq!(haggle_margin(8, 18), -10);
    }

    #[test]
    fn a_tied_haggle_moves_nothing() {
        // The same rule every contest here uses - see acquire.
        assert_eq!(haggle_margin(14, 14), 0);
        assert_eq!(haggle(&list(1_000), 0).buy_cp, 1_000);
    }

    #[test]
    fn a_landslide_still_respects_the_cap() {
        assert_eq!(haggle_margin(30, 1), HAGGLE_CAP);
        assert_eq!(haggle_margin(1, 30), -HAGGLE_CAP);
    }

    /* ---------------- two-sided ------------------------------------- */

    #[test]
    fn a_shop_buys_low_and_sells_high() {
        let q = list(1_000);
        assert_eq!(worth(&q, true, Counterparty::Merchant), 1_000);
        assert_eq!(worth(&q, false, Counterparty::Merchant), 400);
    }

    #[test]
    fn a_peer_values_it_the_same_both_ways() {
        // THE CASE THAT MATTERS. At a merchant's 0.40, two players
        // swapping a sword each would both come away poorer - the goods
        // would evaporate crossing the table.
        let q = list(1_000);
        assert_eq!(worth(&q, true, Counterparty::Peer), 1_000);
        assert_eq!(worth(&q, false, Counterparty::Peer), 1_000);
    }

    #[test]
    fn an_even_swap_between_peers_costs_nothing() {
        let mine = worth(&list(1_000), false, Counterparty::Peer);
        let theirs = worth(&list(1_000), true, Counterparty::Peer);
        assert_eq!(settle(theirs, mine).owed_cp, 0);
    }

    #[test]
    fn positive_means_you_pay() {
        // Buying: goods come in, nothing goes out.
        let b = settle(1_425, 0);
        assert_eq!(b.owed_cp, 1_425);
    }

    #[test]
    fn negative_means_they_pay() {
        // Selling: goods go out, nothing comes in.
        let b = settle(0, 600);
        assert_eq!(b.owed_cp, -600);
    }

    #[test]
    fn a_part_exchange_settles_the_difference() {
        // Their 15gp longsword for your 10gp mace, at a shop: you pay
        // 1500 and they credit you 400 (0.40 of the mace's 1000).
        let theirs = worth(&list(1_500), true, Counterparty::Merchant);
        let yours = worth(&list(1_000), false, Counterparty::Merchant);
        let b = settle(theirs, yours);
        assert_eq!(b.incoming_cp, 1_500);
        assert_eq!(b.outgoing_cp, 400);
        assert_eq!(b.owed_cp, 1_100);
    }

    #[test]
    fn the_same_part_exchange_between_peers_is_nearly_even() {
        let theirs = worth(&list(1_500), true, Counterparty::Peer);
        let yours = worth(&list(1_000), false, Counterparty::Peer);
        assert_eq!(settle(theirs, yours).owed_cp, 500);
    }

    #[test]
    fn giving_costs_the_receiver_nothing_and_the_giver_everything() {
        // Give is a trade where one side offers nothing back.
        assert_eq!(settle(0, 0).owed_cp, 0);
    }

    #[test]
    fn writing_a_disposition_refuses_a_typo() {
        // The difference between the two parsers. Reading tolerates;
        // writing does not, because a silent neutral leaves a DM
        // wondering why their discount never applied.
        assert_eq!(Disposition::parse_strict("warm"), Ok(Disposition::Warm));
        assert_eq!(Disposition::parse_strict("  HOSTILE "), Ok(Disposition::Hostile));
        assert!(Disposition::parse_strict("freindly").is_err());
        assert!(Disposition::parse_strict("").is_err());
        // ...whereas reading the same typo gives neutral and moves on.
        assert_eq!(Disposition::parse(Some("freindly")), Disposition::Neutral);
    }

    #[test]
    fn null_disposition_reads_as_neutral() {
        assert_eq!(Disposition::parse(None), Disposition::Neutral);
        assert_eq!(Disposition::parse(Some("")), Disposition::Neutral);
        assert_eq!(Disposition::parse(Some("HOSTILE")), Disposition::Hostile);
        assert_eq!(Disposition::parse(Some("nonsense")), Disposition::Neutral);
    }
}
