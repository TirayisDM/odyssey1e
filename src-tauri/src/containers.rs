//! What a container will take, and how much of it.
//!
//! THE RULES ONLY. Moving a thing is `commands/containers.rs`; this
//! decides whether it may move, the same division `equipment.rs` and
//! `set_item_equipped` already have.
//!
//! WHY NOT A CONSTRAINT. Deciding whether a coin purse takes a sword
//! needs `items.accepts` on one row and `items.content_tags` on
//! another, and a CHECK sees neither. That is the reasoning 008 gave
//! for the one-armor rule and it applies unchanged. What DID stay in
//! the database is the cycle guard - `no_container_cycles` - because a
//! cycle is not a rule about the game, it is a thing that makes
//! `holder_character` walk until its guard trips.
//!
//! THE ARITHMETIC IS DAVE'S, from odyssey-engine's container.rs: a coin
//! purse is 5 slots and holds 25 coins, so a coin is 0.2 of a slot. The
//! test below says exactly that, because that ratio is the reason the
//! numbers are fractional and it would otherwise look arbitrary.
//!
//! SIZE ARRIVED IN 036, and it is the third axis rather than a
//! replacement for either of the other two. Tags say a greatsword is
//! not a coin; slots say it is bulky; NEITHER SAYS IT IS TOO BIG. Those
//! are different objections - a marble is untagged and tiny, and an
//! unrestricted thimble should still refuse it - and size is the one a
//! person reaches for first, which is the order they are checked in.
//!
//! The ladder is `vitality::size_rank`, the same six words 010 gives
//! creatures. One vocabulary, so "a Huge backpack holds Huge things"
//! needs no translation.
//!
//! STILL NOT CARRIED OVER, from the same prior art: open/closed and
//! locked state, and a nesting depth separate from the cycle guard.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::narrative::quoted;
use crate::supabase;

/* ============================ TYPES ============================ */

/// What a KIND of container will take and how much it holds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// Content tags it admits. EMPTY MEANS ANYTHING - a backpack, not a
    /// locked box. The distinction matters: an empty list is permissive
    /// here and restrictive nowhere.
    pub accepts: Vec<String>,
    /// None is a FAULT rather than "infinite". A container whose
    /// capacity nobody wrote down is a catalogue row somebody did not
    /// finish, and pretending it is bottomless hides that.
    pub capacity_slots: Option<f64>,
    /// The largest size it admits. NONE MEANS NO LIMIT, which is the
    /// opposite of what None means one field up - and deliberately so.
    /// `capacity_slots` is a quantity, so its absence is an unfinished
    /// row; this is a restriction, like `accepts`, and an absent
    /// restriction is not one. A sack cares how much goes in it, not
    /// how long any one thing is.
    pub holds_size: Option<String>,
}

/// How much room a thing takes, and what it counts as.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bulk {
    pub key: String,
    pub slots: f64,
    pub content_tags: Vec<String>,
    /// 036's ladder. Every catalogue row has one - the column is NOT
    /// NULL and defaults to med - so this is not an Option the way the
    /// limit above is.
    pub size: String,
}

/* ============================ RULES ============================ */

/// Whether this container admits this kind of thing.
///
/// Empty `accepts` admits everything. Otherwise the thing needs at
/// least ONE matching tag - not all of them - which is what lets a
/// scroll case take anything tagged either lore or paper.
pub fn admits(profile: &Profile, incoming: &Bulk, container_name: &str) -> Result<(), String> {
    if profile.accepts.is_empty() {
        return Ok(());
    }
    if incoming
        .content_tags
        .iter()
        .any(|t| profile.accepts.contains(t))
    {
        return Ok(());
    }
    Err(format!(
        "a {} takes only {} - that is {}",
        container_name,
        profile.accepts.join(" or "),
        if incoming.content_tags.is_empty() {
            "not tagged as anything".to_string()
        } else {
            incoming.content_tags.join(" and ")
        }
    ))
}

/// Whether this container is big enough for this thing.
///
/// CHECKED BEFORE THE SLOTS, because it is the objection a person
/// reaches for first and it gives the better sentence. "a coin purse
/// holds nothing bigger than tiny - a greatsword is large" says what is
/// wrong; "no room: greatsword needs 1.00 and coin purse has 5.00 free"
/// is arithmetic about a thing that was never going to fit.
///
/// A SIZE NOBODY RECOGNISES IS REFUSED, not waved through. The columns
/// are checked in 036 so this should be unreachable from the database,
/// but the alternative to refusing is admitting anything spelled
/// wrongly - and 008's two spellings of armour are what that looks like
/// when it goes unnoticed.
pub fn admits_size(profile: &Profile, incoming: &Bulk, container_name: &str) -> Result<(), String> {
    let limit = match profile.holds_size.as_deref() {
        // No restriction recorded, so there is not one. See the field.
        None => return Ok(()),
        Some(l) => l,
    };
    let cap = crate::vitality::size_rank(limit)
        .ok_or_else(|| format!("{} has a size limit nobody recognises: {}", container_name, limit))?;
    let mine = crate::vitality::size_rank(&incoming.size)
        .ok_or_else(|| format!("{} is a size nobody recognises: {}", incoming.key, incoming.size))?;
    if mine > cap {
        return Err(format!(
            "a {} holds nothing bigger than {} - {} is {}",
            container_name, limit, incoming.key, incoming.size
        ));
    }
    Ok(())
}

/// How much room is in use, given everything already inside.
pub fn used_slots(contents: &[(Bulk, i64)]) -> f64 {
    contents
        .iter()
        .map(|(b, qty)| b.slots * (*qty as f64))
        .sum()
}

/// Whether that many more will fit.
///
/// The comparison runs at a HUNDREDTH of a slot, because these are
/// fractions of a coin and binary floating point does not hold 0.2. Ten
/// coins at 0.2 sum to 1.9999999999999998, and a strict `>` against a
/// capacity of exactly that much would refuse a purse that is not full.
pub fn fits(
    profile: &Profile,
    contents: &[(Bulk, i64)],
    incoming: &Bulk,
    quantity: i64,
    container_name: &str,
) -> Result<(), String> {
    let capacity = profile.capacity_slots.ok_or_else(|| {
        format!(
            "{} has no capacity recorded - its catalogue row is unfinished",
            container_name
        )
    })?;
    let needed = incoming.slots * (quantity as f64);
    let free = capacity - used_slots(contents);
    if needed - free > 0.005 {
        return Err(format!(
            "no room: {} needs {:.2} and {} has {:.2} free of {:.2}",
            incoming.key, needed, container_name, free.max(0.0), capacity
        ));
    }
    Ok(())
}

/* ============================ NETWORK ============================ */

/// The container facts for one item key, or None if it is not a
/// container at all.
pub fn load_profile(
    token: &str,
    game_id: &str,
    key: &str,
) -> Result<Option<Profile>, String> {
    let rows = supabase::rest_get(
        token,
        "items",
        &[
            ("select", "key,game_id,kind,accepts,capacity_slots,holds_size"),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
            ("key", &format!("in.({})", quoted(key))),
            ("order", "game_id.desc"),
        ],
    )?;
    let Some(r) = rows.as_array().and_then(|a| a.first()) else {
        return Ok(None);
    };
    if r.get("kind").and_then(|x| x.as_str()) != Some("container") {
        return Ok(None);
    }
    Ok(Some(Profile {
        accepts: strings(r, "accepts"),
        capacity_slots: r.get("capacity_slots").and_then(as_f64),
        // Absent is no limit, which is why this is not defaulted to
        // anything. See the field.
        holds_size: r
            .get("holds_size")
            .and_then(|x| x.as_str())
            .map(str::to_string),
    }))
}

/// Bulk for a set of item keys, in one request.
pub fn load_bulk(token: &str, game_id: &str, keys: &[String]) -> Result<Vec<Bulk>, String> {
    if keys.is_empty() {
        return Ok(Vec::new());
    }
    let list: Vec<String> = keys.iter().map(|k| quoted(k)).collect();
    let rows = supabase::rest_get(
        token,
        "items",
        &[
            ("select", "key,game_id,slots,content_tags,size"),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", game_id)),
            ("key", &format!("in.({})", list.join(","))),
            ("order", "game_id.desc"),
        ],
    )?;
    let mut out: Vec<Bulk> = Vec::new();
    for r in rows.as_array().unwrap_or(&Vec::new()) {
        let key = r.get("key").and_then(|x| x.as_str()).unwrap_or("").to_string();
        // game_id.desc puts this campaign's override first, so the
        // first row for a key wins - the same precedence
        // collapse_overrides applies, done by the sort.
        if out.iter().any(|b| b.key == key) {
            continue;
        }
        out.push(Bulk {
            key,
            slots: r.get("slots").and_then(as_f64).unwrap_or(1.0),
            content_tags: strings(r, "content_tags"),
            // 036 makes the column NOT NULL with a default, so a row
            // without one has not been through that migration. "med" is
            // that migration's own default, which keeps a stale row
            // ordinary rather than unmeasurable.
            size: r
                .get("size")
                .and_then(|x| x.as_str())
                .unwrap_or("med")
                .to_string(),
        });
    }
    Ok(out)
}

fn as_f64(v: &Value) -> Option<f64> {
    // PostgREST sends `numeric` as a JSON string, not a number, so a
    // plain as_f64 returns None on every one of these.
    v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

fn strings(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/* ============================ TESTS ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    fn purse() -> Profile {
        Profile {
            accepts: vec!["coin".to_string()],
            capacity_slots: Some(5.0),
            holds_size: Some("tiny".to_string()),
        }
    }

    fn backpack() -> Profile {
        Profile {
            accepts: Vec::new(),
            capacity_slots: Some(20.0),
            holds_size: Some("med".to_string()),
        }
    }

    /// A container with no size limit recorded - which is NOT the same
    /// as a limit of zero. See the field.
    fn sack() -> Profile {
        Profile {
            accepts: Vec::new(),
            capacity_slots: Some(20.0),
            holds_size: None,
        }
    }

    fn bulk(key: &str, slots: f64, tags: &[&str]) -> Bulk {
        sized(key, slots, tags, "med")
    }

    fn sized(key: &str, slots: f64, tags: &[&str], size: &str) -> Bulk {
        Bulk {
            key: key.to_string(),
            slots,
            content_tags: tags.iter().map(|s| s.to_string()).collect(),
            size: size.to_string(),
        }
    }

    fn coin() -> Bulk {
        sized("coin_gp", 0.2, &["coin"], "tiny")
    }

    fn sword() -> Bulk {
        bulk("longsword", 2.0, &[])
    }

    fn greatsword() -> Bulk {
        sized("greatsword", 2.0, &[], "lg")
    }

    #[test]
    fn a_coin_purse_takes_coins() {
        assert!(admits(&purse(), &coin(), "Coin Purse").is_ok());
    }

    #[test]
    fn a_coin_purse_refuses_a_sword() {
        let e = admits(&purse(), &sword(), "Coin Purse").unwrap_err();
        assert!(e.contains("takes only coin"), "{}", e);
    }

    #[test]
    fn an_empty_accepts_list_takes_anything() {
        // Permissive, not restrictive. A backpack is not a locked box,
        // and reading the empty list the other way would make every
        // unrestricted container refuse everything.
        assert!(admits(&backpack(), &sword(), "Backpack").is_ok());
        assert!(admits(&backpack(), &coin(), "Backpack").is_ok());
    }

    #[test]
    fn one_matching_tag_is_enough() {
        let case = Profile {
            accepts: vec!["lore".to_string(), "paper".to_string()],
            capacity_slots: Some(5.0),
            holds_size: None,
        };
        assert!(admits(&case, &bulk("scroll", 0.5, &["paper"]), "Scroll Case").is_ok());
    }

    #[test]
    fn five_slots_is_twenty_five_coins() {
        // Dave's own arithmetic, from odyssey-engine's coin_purse():
        // "5.0 slots = 25 coins max". This is the reason the numbers
        // are fractional and it would look arbitrary undocumented.
        assert!(fits(&purse(), &[], &coin(), 25, "Coin Purse").is_ok());
        assert!(fits(&purse(), &[], &coin(), 26, "Coin Purse").is_err());
    }

    #[test]
    fn what_is_already_inside_takes_up_room() {
        let inside = vec![(coin(), 20)];
        assert_eq!(used_slots(&inside), 4.0);
        assert!(fits(&purse(), &inside, &coin(), 5, "Coin Purse").is_ok());
        assert!(fits(&purse(), &inside, &coin(), 6, "Coin Purse").is_err());
    }

    #[test]
    fn floating_point_does_not_cost_a_coin() {
        // Ten coins at 0.2 sum to 1.9999999999999998. A strict
        // comparison would refuse the tenth coin of a purse that is not
        // full, which is the sort of bug nobody thinks to look for.
        let inside = vec![(coin(), 10)];
        assert!(fits(&purse(), &inside, &coin(), 15, "Coin Purse").is_ok());
    }

    #[test]
    fn a_container_with_no_capacity_is_a_fault_not_a_bag_of_holding() {
        let broken = Profile {
            accepts: Vec::new(),
            capacity_slots: None,
            holds_size: None,
        };
        let e = fits(&broken, &[], &coin(), 1, "Mystery Sack").unwrap_err();
        assert!(e.contains("unfinished"), "{}", e);
    }

    #[test]
    fn the_refusal_says_what_the_thing_actually_is() {
        // An error that names only the container tells the player
        // nothing they did not already know.
        let e = admits(&purse(), &bulk("arrow", 0.05, &["ammunition"]), "Coin Purse")
            .unwrap_err();
        assert!(e.contains("ammunition"), "{}", e);
    }
    /* ---------- size, 036 ---------- */

    // THE CASE containers.rs ASKED FOR IN ITS HEADER since 032: a
    // greatsword fails to fit in a purse for a second and better reason
    // than its tags.
    #[test]
    fn a_purse_holds_nothing_bigger_than_a_coin() {
        let e = admits_size(&purse(), &greatsword(), "coin purse").unwrap_err();
        assert!(e.contains("nothing bigger than tiny"), "{}", e);
        assert!(e.contains("greatsword is lg"), "{}", e);
    }

    #[test]
    fn a_coin_goes_in_a_purse() {
        assert!(admits_size(&purse(), &coin(), "coin purse").is_ok());
    }

    // Equal fits. A Medium backpack takes a Medium longsword - the
    // limit is the largest it ADMITS, not the largest it is bigger
    // than.
    #[test]
    fn the_limit_is_inclusive() {
        assert!(admits_size(&backpack(), &sword(), "backpack").is_ok());
    }

    #[test]
    fn a_backpack_refuses_a_greatsword() {
        assert!(admits_size(&backpack(), &greatsword(), "backpack").is_err());
    }

    // NONE IS NO LIMIT, and this is the test that keeps it from being
    // read as zero. capacity_slots means the opposite by the same
    // absence, which is exactly why this is written down.
    #[test]
    fn a_container_with_no_size_limit_takes_anything() {
        assert!(admits_size(&sack(), &greatsword(), "sack").is_ok());
        assert!(admits_size(&sack(), &sized("ship", 1.0, &[], "grg"), "sack").is_ok());
    }

    // 008's two spellings of armour, refused before they can happen
    // again. A word off the ladder is an error, not a pass.
    #[test]
    fn a_size_nobody_recognises_is_refused_not_waved_through() {
        let odd = sized("thing", 1.0, &[], "small");
        let e = admits_size(&backpack(), &odd, "backpack").unwrap_err();
        assert!(e.contains("nobody recognises"), "{}", e);

        let bad_limit = Profile {
            accepts: Vec::new(),
            capacity_slots: Some(20.0),
            holds_size: Some("medium".to_string()),
        };
        assert!(admits_size(&bad_limit, &coin(), "crate").is_err());
    }

    // Size and slots are DIFFERENT OBJECTIONS, and this is the pair
    // that proves neither implies the other: a greatsword is 2 slots
    // and a purse has 5 free, so the slot check alone would let it in.
    #[test]
    fn room_is_not_the_same_question_as_size() {
        assert!(fits(&purse(), &[], &greatsword(), 1, "coin purse").is_ok());
        assert!(admits_size(&purse(), &greatsword(), "coin purse").is_err());
    }

}
