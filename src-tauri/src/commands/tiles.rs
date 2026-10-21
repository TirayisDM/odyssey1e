//! Reading and writing a map.
//!
//! 229. THE FIRST DOOR INTO `tiles`. 226 built the table and 228 fixed
//! its links, and until now nothing could put a row in either.
//!
//! THIN, LIKE EVERY COMMAND MODULE. The rules are in `terrain.rs` and
//! `grid.rs`; what is here is transport, the one check a database
//! cannot make for us, and the chunking.
//!
//! ---------------------------------------------------------------------
//! VALIDATION HAPPENS TWICE, AND THAT IS NOT A MISTAKE
//! ---------------------------------------------------------------------
//!
//! The CHECK constraints stay - they are the backstop for anything
//! reaching the table by any route, including psql. But a constraint
//! violation comes back as Postgres prose with a row dump in it, and
//! this project's standard is that a refusal arrives as a sentence.
//!
//! `Stack::legal()` already says "rock cannot be a cover". So the batch
//! is checked HERE FIRST, in full, before a single row goes out - and
//! the error names the square it came from, which the constraint
//! cannot do because it does not know which of nine hundred rows you
//! meant.
//!
//! ---------------------------------------------------------------------
//! THE SAME SQUARE TWICE IS REFUSED, NOT QUIETLY DEDUPED
//! ---------------------------------------------------------------------
//!
//! Postgres will not let one `insert ... on conflict do update` touch
//! the same key twice - "cannot affect row a second time" - so a batch
//! painting (3,4) twice fails as a whole, obscurely, after the round
//! trip.
//!
//! Silently keeping the last one would hide the bug in whatever drew
//! the batch. An editor that sends a square twice has a fault in its
//! brush, and the second value may not be the one the user drew last.
//! So it is named and refused.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use tauri::State;

use crate::supabase::{self, AppState};
use crate::terrain::{Stack, Terrain};

/// How many rows go out in one request.
///
/// A forty-by-forty room is 1,600 squares and fits in two. The number
/// is here so a big import fails somewhere locatable rather than all at
/// once, not because PostgREST cannot take more.
const CHUNK: usize = 1000;

/// The columns a read asks for. Spelled out rather than `*` so a new
/// column does not silently widen every payload.
const COLUMNS: &str = "x,y,z,surface,cover,sub,surface_hp,cover_hp,leads_to,to_x,to_y,to_z";

/// One square, as it crosses the wire.
///
/// TRANSPORT, NOT MEANING. `Stack` is the meaning, and [`Tile::stack`]
/// is the conversion. Keeping them apart is what keeps serde out of
/// `terrain.rs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tile {
    pub x: i32,
    pub y: i32,
    #[serde(default)]
    pub z: i16,
    pub surface: String,
    #[serde(default)]
    pub cover: Option<String>,
    #[serde(default)]
    pub sub: Option<String>,
    #[serde(default)]
    pub surface_hp: Option<i64>,
    #[serde(default)]
    pub cover_hp: Option<i64>,
    #[serde(default)]
    pub leads_to: Option<String>,
    #[serde(default)]
    pub to_x: Option<i32>,
    #[serde(default)]
    pub to_y: Option<i32>,
    #[serde(default)]
    pub to_z: Option<i16>,
}

/// A square to erase. Points rather than a rectangle: points compose,
/// and a rectangle is a shape the caller can expand.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Spot {
    pub x: i32,
    pub y: i32,
    #[serde(default)]
    pub z: i16,
}

/// Read one word, saying which square and which layer went wrong.
fn word(v: Option<&String>, at: &str, layer: &str) -> Result<Option<Terrain>, String> {
    match v.map(String::as_str).map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) => Terrain::of_key(s)
            .map(Some)
            .ok_or_else(|| format!("{}: \"{}\" is not a terrain ({})", at, s, layer)),
    }
}

impl Tile {
    fn at(&self) -> String {
        format!("({}, {}, {})", self.x, self.y, self.z)
    }

    /// What this square is MADE OF, checked.
    pub fn stack(&self) -> Result<Stack, String> {
        let at = self.at();
        let surface = word(Some(&self.surface), &at, "surface")?
            .ok_or_else(|| format!("{}: a square needs a surface", at))?;
        let stack = Stack {
            sub: word(self.sub.as_ref(), &at, "sub")?,
            surface,
            cover: word(self.cover.as_ref(), &at, "cover")?,
        };
        stack.legal().map_err(|e| format!("{}: {}", at, e))?;
        Ok(stack)
    }

    /// A link is four facts or none.
    ///
    /// The constraint says the same thing; this says it with the square
    /// in the message, before the round trip.
    fn link_is_whole(&self) -> Result<(), String> {
        let has = self.leads_to.as_deref().map(str::trim).is_some_and(|s| !s.is_empty());
        let coords = self.to_x.is_some() && self.to_y.is_some() && self.to_z.is_some();
        let none = self.to_x.is_none() && self.to_y.is_none() && self.to_z.is_none();
        match (has, coords, none) {
            (true, true, _) | (false, _, true) => Ok(()),
            (true, false, _) => Err(format!(
                "{}: a link needs where you arrive - to_x, to_y and to_z",
                self.at()
            )),
            (false, _, false) => Err(format!(
                "{}: an arrival with nothing to arrive from - set leads_to or clear to_x/to_y/to_z",
                self.at()
            )),
        }
    }
}

/// Everything knowable about a batch before the network sees it.
///
/// ALL OF IT, NOT THE FIRST FAILURE'S REQUEST. The whole batch is
/// checked before anything is sent, so a bad square in the middle of
/// nine hundred costs nothing and is named.
fn check_batch(tiles: &[Tile]) -> Result<(), String> {
    if tiles.is_empty() {
        return Err("nothing to paint".to_string());
    }
    let mut seen: HashSet<(i32, i32, i16)> = HashSet::with_capacity(tiles.len());
    for t in tiles {
        t.stack()?;
        t.link_is_whole()?;
        if !seen.insert((t.x, t.y, t.z)) {
            return Err(format!(
                "{} is painted twice in one batch - a square can only be one thing",
                t.at()
            ));
        }
    }
    Ok(())
}

/// The row as it is written.
///
/// HIT POINTS ARE NOT IN IT, deliberately. Painting is AUTHORING and
/// damage is PLAY: an editor passing over a chipped wall should not
/// silently repair the dungeon. Leaving the columns out of the payload
/// leaves them untouched on an update, rather than overwriting them
/// with null.
fn row(location_id: &str, t: &Tile) -> Value {
    json!({
        "location_id": location_id,
        "x": t.x,
        "y": t.y,
        "z": t.z,
        "surface": t.surface.trim(),
        "cover": t.cover.as_deref().map(str::trim).filter(|s| !s.is_empty()),
        "sub": t.sub.as_deref().map(str::trim).filter(|s| !s.is_empty()),
        "leads_to": t.leads_to.as_deref().map(str::trim).filter(|s| !s.is_empty()),
        "to_x": t.to_x,
        "to_y": t.to_y,
        "to_z": t.to_z,
    })
}

/// 229. 217'S RULE: A WRITE THAT DID NOTHING MUST SAY SO.
///
/// "A blocked UPDATE or DELETE matches no row and returns success", so
/// a command reporting what it ASKED for rather than what happened is
/// the exact lie 217 swept this codebase for. A player painting the
/// DM's map is filtered by RLS and would otherwise be told it worked.
///
/// A SHORTFALL IS AN ERROR, NOT A NUMBER TO INSPECT. Half a map drawn
/// is not a success with a caveat - it is a map in a state nobody
/// asked for, and the caller needs to know before it draws it.
fn short(done: usize, asked: usize, verb: &str) -> Result<usize, String> {
    match done {
        n if n == asked => Ok(n),
        0 => Err(format!(
            "nothing was {} - this is not your map to change",
            verb
        )),
        n => Err(format!(
            "only {} of {} squares were {} - the rest were refused",
            n, asked, verb
        )),
    }
}

fn denied(e: String, what: &str) -> String {
    if e.contains("(401)") || e.contains("(403)") || e.contains("42501") {
        format!(
            "only the DM of this game can {} — you are signed in as a player",
            what
        )
    } else {
        e
    }
}

/// Every square of one map, in the order a grid wants them.
///
/// `order=z,y,x` IS THE INDEX `one_square_one_row` ALREADY BUILT, so
/// this walks it without a sort - and it is the order a row-major
/// `Vec` per layer is reconstituted in.
///
/// THE WHOLE LOCATION, not a window. Phase 1 is a DM rendering one
/// place; a bounding box is what fog will want, and adding the
/// parameter now would be one nothing passes.
#[tauri::command]
pub fn list_tiles(state: State<AppState>, location_id: String) -> Result<Vec<Tile>, String> {
    let token = state.token()?;
    let rows = supabase::rest_get(
        &token,
        "tiles",
        &[
            ("select", COLUMNS),
            ("location_id", &format!("eq.{}", location_id)),
            ("order", "z.asc,y.asc,x.asc"),
        ],
    )?;
    serde_json::from_value(rows).map_err(|e| format!("could not read the map: {}", e))
}

/// Write squares, replacing whatever was there.
///
/// Returns how many were written.
#[tauri::command]
pub fn paint_tiles(
    state: State<AppState>,
    location_id: String,
    tiles: Vec<Tile>,
) -> Result<usize, String> {
    check_batch(&tiles)?;
    let token = state.token()?;
    let mut done = 0usize;
    for part in tiles.chunks(CHUNK) {
        let body: Vec<Value> = part.iter().map(|t| row(&location_id, t)).collect();
        let back = supabase::rest_upsert(&token, "tiles", &json!(body), "location_id,z,y,x")
            .map_err(|e| denied(e, "draw the map"))?;
        done += back.as_array().map(Vec::len).unwrap_or(0);
    }
    short(done, tiles.len(), "painted")
}

/// Put squares back to unmapped.
///
/// NOT "EMPTY FLOOR" - 226 is explicit that a missing row means the
/// square was never mapped, which is why this deletes rather than
/// writing some blank terrain.
#[tauri::command]
pub fn erase_tiles(
    state: State<AppState>,
    location_id: String,
    spots: Vec<Spot>,
) -> Result<usize, String> {
    if spots.is_empty() {
        return Err("nothing to erase".to_string());
    }
    let token = state.token()?;
    let mut done = 0usize;
    for part in spots.chunks(CHUNK) {
        // PostgREST's `or=(and(..),and(..))`. One request per chunk
        // rather than one per square, which an eraser dragged across a
        // room would otherwise make hundreds of.
        let clauses: Vec<String> = part
            .iter()
            .map(|s| format!("and(x.eq.{},y.eq.{},z.eq.{})", s.x, s.y, s.z))
            .collect();
        let or = format!("({})", clauses.join(","));
        done += supabase::rest_delete_counted(
            &token,
            "tiles",
            &[("location_id", &format!("eq.{}", location_id)), ("or", &or)],
        )
        .map_err(|e| denied(e, "change the map"))?;
    }
    // A SQUARE THAT WAS NEVER MAPPED IS NOT A FAILURE - erasing blank
    // is a no-op a brush does constantly - so `erase` only complains
    // when NOTHING went at all, which is the shape of a refusal.
    match done {
        0 => Err("nothing was erased - those squares are not yours to change, or were never mapped"
            .to_string()),
        n => Ok(n),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(surface: &str) -> Tile {
        Tile {
            x: 3,
            y: 4,
            z: 0,
            surface: surface.into(),
            cover: None,
            sub: None,
            surface_hp: None,
            cover_hp: None,
            leads_to: None,
            to_x: None,
            to_y: None,
            to_z: None,
        }
    }

    #[test]
    fn a_good_square_becomes_a_stack() {
        let mut tile = t("dirt");
        tile.cover = Some("grass".into());
        let s = tile.stack().unwrap();
        assert_eq!(s.surface, Terrain::Dirt);
        assert_eq!(s.cover, Some(Terrain::Grass));
    }

    #[test]
    fn a_word_nobody_wrote_is_named_with_its_square() {
        let mut tile = t("dirt");
        tile.cover = Some("marshmallow".into());
        let e = tile.stack().unwrap_err();
        assert!(e.contains("(3, 4, 0)"), "{}", e);
        assert!(e.contains("marshmallow"), "{}", e);
        assert!(e.contains("cover"), "{}", e);
    }

    #[test]
    fn a_layer_mix_up_says_which_layer() {
        // The constraint would say this too, in Postgres prose with a
        // row dump. This says it with the square.
        let mut tile = t("dirt");
        tile.cover = Some("rock".into());
        let e = tile.stack().unwrap_err();
        assert!(e.contains("cannot be a cover"), "{}", e);
        assert!(e.contains("(3, 4, 0)"), "{}", e);
    }

    #[test]
    fn grass_is_not_a_surface_and_the_message_says_so() {
        let e = t("grass").stack().unwrap_err();
        assert!(e.contains("cannot be a surface"), "{}", e);
    }

    #[test]
    fn an_empty_cover_is_no_cover_rather_than_a_bad_one() {
        // An untouched control sends "", the way 033 found with parents.
        let mut tile = t("dirt");
        tile.cover = Some("   ".into());
        assert_eq!(tile.stack().unwrap().cover, None);
    }

    #[test]
    fn a_link_without_an_arrival_is_refused_before_the_round_trip() {
        let mut tile = t("stairs");
        tile.leads_to = Some("some-uuid".into());
        let e = tile.link_is_whole().unwrap_err();
        assert!(e.contains("where you arrive"), "{}", e);
    }

    #[test]
    fn an_arrival_with_no_link_is_refused_too() {
        let mut tile = t("stairs");
        tile.to_x = Some(1);
        tile.to_y = Some(2);
        tile.to_z = Some(0);
        let e = tile.link_is_whole().unwrap_err();
        assert!(e.contains("nothing to arrive from"), "{}", e);
    }

    #[test]
    fn a_whole_link_passes_and_so_does_no_link_at_all() {
        let mut tile = t("ledge");
        tile.leads_to = Some("some-uuid".into());
        tile.to_x = Some(1);
        tile.to_y = Some(2);
        tile.to_z = Some(-1);
        assert!(tile.link_is_whole().is_ok());
        assert!(t("ledge").link_is_whole().is_ok());
    }

    #[test]
    fn a_link_from_a_ledge_is_allowed_because_228_said_so() {
        // 226 would have refused this; falling off a cliff lands you in
        // a different location at coordinates it chose.
        let mut tile = t("ledge");
        tile.leads_to = Some("valley".into());
        tile.to_x = Some(12);
        tile.to_y = Some(3);
        tile.to_z = Some(0);
        assert!(check_batch(&[tile]).is_ok());
    }

    #[test]
    fn the_same_square_twice_is_refused_rather_than_deduped() {
        // Postgres cannot upsert one key twice in a statement, and
        // keeping the last one silently would hide a fault in whatever
        // drew the batch.
        let e = check_batch(&[t("dirt"), t("rock")]).unwrap_err();
        assert!(e.contains("painted twice"), "{}", e);
        assert!(e.contains("(3, 4, 0)"), "{}", e);
    }

    #[test]
    fn the_same_x_and_y_at_another_height_is_a_different_square() {
        let mut upper = t("stone_floor");
        upper.z = 1;
        assert!(check_batch(&[t("dirt"), upper]).is_ok());
    }

    #[test]
    fn a_bad_square_anywhere_stops_the_whole_batch() {
        let mut bad = t("dirt");
        bad.x = 99;
        bad.surface = "custard".into();
        let e = check_batch(&[t("dirt"), bad, t("rock")]).unwrap_err();
        assert!(e.contains("custard") && e.contains("(99, 4, 0)"), "{}", e);
    }

    #[test]
    fn an_empty_batch_is_a_mistake_and_not_a_no_op() {
        assert!(check_batch(&[]).is_err());
    }

    #[test]
    fn a_write_that_did_nothing_says_so() {
        // 217: "a blocked UPDATE or DELETE matches no row and returns
        // success". A player painting the DM's map is filtered by RLS
        // and must not be told it worked.
        assert_eq!(short(40, 40, "painted"), Ok(40));

        let e = short(0, 40, "painted").unwrap_err();
        assert!(e.contains("not your map"), "{}", e);

        let e = short(12, 40, "painted").unwrap_err();
        assert!(e.contains("only 12 of 40"), "{}", e);
    }

    #[test]
    fn a_painted_row_carries_no_hit_points() {
        // PAINTING IS AUTHORING; DAMAGE IS PLAY. Omitting the columns
        // leaves them untouched on an update rather than nulling them.
        let r = row("loc", &t("dirt"));
        assert!(r.get("surface_hp").is_none());
        assert!(r.get("cover_hp").is_none());
        assert_eq!(r["surface"], "dirt");
        assert_eq!(r["location_id"], "loc");
    }

    #[test]
    fn a_painted_row_sends_null_rather_than_an_empty_string() {
        let mut tile = t("dirt");
        tile.cover = Some("".into());
        assert!(row("loc", &tile)["cover"].is_null());
    }
}
