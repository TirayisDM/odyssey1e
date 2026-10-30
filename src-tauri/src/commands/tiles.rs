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
use std::collections::{HashMap, HashSet};
use tauri::State;

use crate::commands::Ctx;
use crate::supabase::{self, AppState};
use crate::grid::Point;
use crate::ground::Bounds;
use crate::terrain::{Facing, Layer, Space, Stack, Terrain};

/// How many rows go out in one request.
///
/// A forty-by-forty room is 1,600 squares and fits in two. The number
/// is here so a big import fails somewhere locatable rather than all at
/// once, not because PostgREST cannot take more.
const CHUNK: usize = 1000;

/// The columns a read asks for. Spelled out rather than `*` so a new
/// column does not silently widen every payload.
const COLUMNS: &str =
    "x,y,z,surface,cover,sub,surface_hp,cover_hp,leads_to,to_x,to_y,to_z,facing,shape";

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
    /// Quarter turns clockwise from north. See 238.
    #[serde(default)]
    pub facing: i16,
    /// 254. Which PIECE of a wall this is - `corner`, `t`, or absent
    /// for a plain run. Chooses the PNG and nothing else.
    #[serde(default)]
    pub shape: Option<String>,
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
        let facing = Facing::of(self.facing)
            .ok_or_else(|| format!("{}: a facing is 0, 1, 2 or 3 - quarter turns from north", at))?;
        let stack = Stack {
            sub: word(self.sub.as_ref(), &at, "sub")?,
            surface,
            cover: word(self.cover.as_ref(), &at, "cover")?,
            facing,
        };
        stack.legal().map_err(|e| format!("{}: {}", at, e))?;

        // 254. AND THE SHAPE IS A WORD, SO IT IS CHECKED.
        //
        // The database holds free text - see the migration - so this
        // is the only thing standing between a typo and a square that
        // silently draws the plain tile. `shape.rs` owns the list.
        crate::shape::Shape::of_key(self.shape.as_deref().unwrap_or(""))
            .map_err(|e| format!("{}: {}", at, e))?;

        // 238. ONLY EIGHT WORDS MEAN ANYTHING BY TURNING. A quarter
        // turn on a rock is noise somebody will later read as
        // meaningful, so it is refused here rather than stored - the
        // database constrains the range and leaves the vocabulary to
        // Rust.
        if facing != Facing::North {
            let turns = stack.surface.faces() || stack.cover.is_some_and(|c| c.faces());
            if !turns {
                return Err(format!(
                    "{}: {} is the same whichever way it is turned",
                    at,
                    stack.surface
                ));
            }
        }
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

/// How big the map says it is, if it says.
#[derive(Debug, Clone, Copy, Deserialize)]
struct Extent {
    width: Option<i32>,
    height: Option<i32>,
}

impl Extent {
    /// 233. THE WIRE SHAPE BECOMES THE RULE SHAPE, the same way `Tile`
    /// becomes a `Stack`. `ground::Bounds` holds no serde, for the
    /// reason `terrain.rs` holds none: a rules type that knows how it
    /// is transported starts being shaped by the transport.
    fn bounds(&self) -> Bounds {
        Bounds::of(self.width, self.height)
    }
}

/// Is every square inside the map it is being painted on?
///
/// 232. THE TRIGGER SAYS THIS TOO. This says it FIRST, naming the
/// square, before nine hundred rows go out and one of them is refused
/// by Postgres with a message about `new.x`.
///
/// 233. AND THE QUESTION IT ASKS IS `allows`, NOT `encloses`. An
/// unmeasured map takes anything - nobody has said where it stops -
/// which is the opposite of what the same map answers about a default
/// reaching a square. Both live in `ground.rs` so the two can never be
/// confused for one function again.
fn inside(e: Extent, tiles: &[Tile]) -> Result<(), String> {
    let b = e.bounds();
    for t in tiles {
        if b.allows(t.x, t.y) {
            continue;
        }
        // Which axis, so the message is about the mistake rather than
        // about the square in general.
        if let Some(w) = e.width.filter(|w| t.x < 0 || t.x >= *w) {
            return Err(format!(
                "{}: x {} is outside this map, which is {} squares across",
                t.at(),
                t.x,
                w
            ));
        }
        if let Some(h) = e.height.filter(|h| t.y < 0 || t.y >= *h) {
            return Err(format!(
                "{}: y {} is outside this map, which is {} squares down",
                t.at(),
                t.y,
                h
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
        "facing": t.facing,
        "shape": t.shape.as_deref().map(str::trim).filter(|s| !s.is_empty()),
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

    // 232. ONE ROW, ONCE, BEFORE ANYTHING GOES OUT. A map's extent is
    // a fact about the place rather than about each square, so it is
    // read once and checked against the whole batch.
    let bounds = supabase::rest_get(
        &token,
        "locations",
        &[
            ("select", "width,height"),
            ("id", &format!("eq.{}", location_id)),
        ],
    )?;
    let mut extents: Vec<Extent> = serde_json::from_value(bounds)
        .map_err(|e| format!("could not read the map's size: {}", e))?;
    let extent = extents.pop().ok_or("no such place")?;
    inside(extent, &tiles)?;

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

/* ================== 230. WHO IS STANDING WHERE ================== */

/// A creature on a map, and where.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Standing {
    pub id: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub z: i16,
}

/// Put a creature on a square, or lift them off it.
///
/// 230. DELIBERATELY DUMB. It does not ask whether the square exists,
/// whether a wall is there, or who else is standing on it.
///
/// NOT AN OVERSIGHT - A SEAM. "May this creature be here" is a rules
/// question with a growing number of answers: `stride.rs` already
/// knows what blocks a step, occupancy has exceptions the schema
/// deliberately does not encode (230 says why), and a DM placing a
/// monster mid-sentence should not be argued with by a writer.
///
/// So the write is a write. Whatever is judging legality calls
/// `stride::step` first and this second, and the two stay separable.
///
/// PASSING NO COORDINATES LIFTS THEM OFF THE MAP, which is a thing a
/// DM does constantly - a creature that has left, or was never placed.
#[tauri::command]
pub fn place_character(
    state: State<AppState>,
    character_id: String,
    x: Option<i32>,
    y: Option<i32>,
    z: Option<i16>,
) -> Result<(), String> {
    // THE SAME ALL-OR-NOTHING THE CONSTRAINT HOLDS, said here so the
    // refusal is a sentence rather than a row dump.
    let given = [x.is_some(), y.is_some(), z.is_some()];
    if given.iter().any(|g| *g) && !given.iter().all(|g| *g) {
        return Err("a position is x, y and z - give all three or none".to_string());
    }
    let token = state.token()?;
    let back = supabase::rest_update(
        &token,
        "characters",
        &[("id", &format!("eq.{}", character_id))],
        &json!({ "x": x, "y": y, "z": z }),
    )
    .map_err(|e| denied(e, "move a creature about the map"))?;

    // 217 AGAIN: a blocked UPDATE matches no row and returns success.
    match back.as_array().map(Vec::len).unwrap_or(0) {
        0 => Err("that creature did not move - it is not yours to place".to_string()),
        _ => Ok(()),
    }
}

/// Everyone standing on one map.
///
/// ONLY THE PLACED. A character in the room without coordinates is in
/// the room and not on the map, and a renderer asking "what do I draw"
/// should not have to filter them out itself.
#[tauri::command]
pub fn standing_in(state: State<AppState>, location_id: String) -> Result<Vec<Standing>, String> {
    let token = state.token()?;
    let rows = supabase::rest_get(
        &token,
        "characters",
        &[
            ("select", "id,name,x,y,z"),
            ("location_id", &format!("eq.{}", location_id)),
            ("x", "not.is.null"),
            ("order", "name.asc"),
        ],
    )?;
    serde_json::from_value(rows).map_err(|e| format!("could not read who is standing here: {}", e))
}

/* ===================== 231. TAKING A STEP ====================== */

/// Fetch the squares at these exact coordinates, and nothing else.
///
/// NOT THE WHOLE MAP. A step reads two squares, and pulling sixteen
/// hundred rows to look at two is the kind of thing that is invisible
/// until a map is large.
fn squares_at(
    token: &str,
    location_id: &str,
    spots: &[Spot],
) -> Result<HashMap<(i32, i32, i16), Stack>, String> {
    let clauses: Vec<String> = spots
        .iter()
        .map(|s| format!("and(x.eq.{},y.eq.{},z.eq.{})", s.x, s.y, s.z))
        .collect();
    let rows = supabase::rest_get(
        token,
        "tiles",
        &[
            ("select", COLUMNS),
            ("location_id", &format!("eq.{}", location_id)),
            ("or", &format!("({})", clauses.join(","))),
        ],
    )?;
    let tiles: Vec<Tile> =
        serde_json::from_value(rows).map_err(|e| format!("could not read the ground: {}", e))?;

    let mut out = HashMap::new();
    for t in &tiles {
        out.insert((t.x, t.y, t.z), t.stack()?);
    }
    Ok(out)
}

/// Where a creature is, as far as the map is concerned.
#[derive(Debug, Deserialize)]
struct Standpoint {
    name: String,
    /// 253. Which game, so a recorded move lands in the right one.
    game_id: String,
    /// 270. THE HOLDER, WHICH IS NOT THE CHARACTER. 031 moved objects
    /// onto entities and `list_inventory` carries the scar: passing a
    /// character id where a holder id belongs still compiles, and every
    /// inventory came back empty, so a give that had worked looked like
    /// a give that had failed. Here the same slip would refuse a dig
    /// for having no tools while the shovel sat in the pack - which is
    /// why this is a field on the standpoint and not a parameter
    /// anybody could pass the wrong variable to.
    ///
    /// NOT `#[serde(default)]`. A select that forgets this column fails
    /// loudly in the one place that would otherwise be silent.
    entity_id: Option<String>,
    location_id: Option<String>,
    x: Option<i32>,
    y: Option<i32>,
    z: Option<i16>,
}

/* 247. `step_to` AND `Moved` WERE DELETED HERE.

   230 built `step_to`: one step, refused unless adjacent. 241 deleted
   the dev hook and took its only caller with it, and 247 replaced it -
   `walk_to` to an adjacent square IS one step, priced by the same
   `stride::step`, so keeping both was two commands for one rule and
   the one nobody called was the one that would drift.

   Nothing is lost. If something ever wants "exactly one step and
   refuse the rest" - arrow keys, say - that is `walk_to` with a
   neighbouring square, and `stride::step` is still the only thing
   deciding. */

/* ============== 234. WHAT A ROOM IS MADE OF BY DEFAULT ============== */

/// The shape of `locations.recipe`, as stored.
///
/// 226 left the column deliberately unconstrained and said what it was
/// waiting for: "the shape it actually wants is defaults for `surface`
/// and `cover` per region". This is that shape, and it is the same
/// three words a tile row carries:
///
/// ```json
/// { "surface": "stone_floor", "cover": null, "sub": null }
/// ```
///
/// **IT IS STILL jsonb AND STILL UNCONSTRAINED BY THE DATABASE.** 226's
/// rule for the column stands - the DM authors it and play never
/// changes it - and a CHECK on a json shape is a constraint that only
/// ever refuses honest data when the shape grows. The validation is
/// here, where a refusal can be a sentence, and `Stack::legal()` does
/// the actual judging so a recipe cannot say something a tile could
/// not.
///
/// ANYTHING ELSE IN THE OBJECT IS LEFT ALONE. A recipe is also where a
/// generator's seed and its lighting will live, and this reads the
/// three keys it knows rather than refusing a document it did not
/// write.
fn recipe_of(v: &Value) -> Result<Option<Stack>, String> {
    let Some(obj) = v.as_object() else {
        return Ok(None);
    };
    let word = |k: &str| -> Option<String> {
        obj.get(k)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    let Some(surface) = word("surface") else {
        // A recipe with no surface declares no ground. It may still
        // carry a seed or a palette, which is not this function's
        // business.
        return Ok(None);
    };
    let tile = Tile {
        x: 0,
        y: 0,
        z: 0,
        surface,
        cover: word("cover"),
        sub: word("sub"),
        surface_hp: None,
        cover_hp: None,
        leads_to: None,
        to_x: None,
        to_y: None,
        to_z: None,
        // 254. NOR A SHAPE, for the same reason and more plainly: a
        // recipe says what a room is MADE of, and "this room is
        // corners by default" is not a sentence.
        shape: None,
        // 238. A RECIPE CANNOT DECLARE A FACING. "This room is stairs
        // climbing north by default" is not a thing anybody means - a
        // turn is a fact about one square, and a default that applied
        // one to every square in the room would be wrong everywhere it
        // was not placed deliberately.
        facing: 0,
    };
    // The square it reports is (0,0,0), which is a lie about WHERE and
    // true about WHAT - so the message is rewritten to say "the
    // recipe" rather than naming a square nobody painted.
    tile.stack()
        .map(Some)
        .map_err(|e| format!("the recipe is wrong: {}", e.replace("(0, 0, 0): ", "")))
}

/// How big a room is and what it is made of, in one read.
#[derive(Debug, Clone, Deserialize)]
struct MapOf {
    width: Option<i32>,
    height: Option<i32>,
    #[serde(default)]
    recipe: Value,
}

/// Read a location's extent and default ground.
fn map_of(token: &str, location_id: &str) -> Result<(Bounds, Option<Stack>), String> {
    let rows = supabase::rest_get(
        token,
        "locations",
        &[
            ("select", "width,height,recipe"),
            ("id", &format!("eq.{}", location_id)),
        ],
    )?;
    let mut found: Vec<MapOf> = serde_json::from_value(rows)
        .map_err(|e| format!("could not read the map's size: {}", e))?;
    let m = found.pop().ok_or("no such place")?;
    Ok((Bounds::of(m.width, m.height), recipe_of(&m.recipe)?))
}

/// Declare how big a room is and what it is made of.
///
/// 234. THE DEFAULT ONLY REACHES INSIDE A DECLARED ROOM - see
/// `ground::Bounds::encloses` - so a recipe without a width and height
/// is ground that covers nothing. This sets both in one call because
/// setting one without the other is almost always a mistake.
#[tauri::command]
pub fn set_map(
    state: State<AppState>,
    location_id: String,
    width: Option<i32>,
    height: Option<i32>,
    surface: Option<String>,
    cover: Option<String>,
    sub: Option<String>,
) -> Result<(), String> {
    let recipe = match surface.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        None => json!(null),
        Some(sfc) => json!({
            "surface": sfc,
            "cover": cover.as_deref().map(str::trim).filter(|s| !s.is_empty()),
            "sub": sub.as_deref().map(str::trim).filter(|s| !s.is_empty()),
        }),
    };
    // REFUSED HERE RATHER THAN STORED AND FOUND LATER. A recipe that
    // cannot be read is a room that silently has no floor.
    recipe_of(&recipe)?;

    if surface.is_some() && (width.is_none() || height.is_none()) {
        return Err(
            "default ground needs a width and a height - it only reaches inside a declared room"
                .to_string(),
        );
    }

    let token = state.token()?;
    let back = supabase::rest_update(
        &token,
        "locations",
        &[("id", &format!("eq.{}", location_id))],
        &json!({ "width": width, "height": height, "recipe": recipe }),
    )
    .map_err(|e| denied(e, "describe the map"))?;

    match back.as_array().map(Vec::len).unwrap_or(0) {
        0 => Err("that map was not changed - it is not yours to describe".to_string()),
        _ => Ok(()),
    }
}

/* ================= 235. THE MAP, RESOLVED FOR DRAWING ================= */

/// One square, as the renderer needs it.
///
/// 235. RESOLVED IN RUST, DELIBERATELY. The rule for what is at a
/// square - a row, else the default if the room encloses it, else
/// nothing - lives in `ground::at`. A renderer walking the grid in
/// JavaScript would have to reimplement it, and then **the picture
/// could disagree with where a creature may walk**, which is the
/// two-places fault in its most visible costume.
///
/// The sparse rows are small by design, so resolving them server-side
/// costs almost nothing and buys one rule in one language.
#[derive(Debug, Clone, Serialize)]
pub struct Square {
    pub x: i32,
    pub y: i32,
    pub surface: String,
    pub cover: Option<String>,
    /// Is there a ROW here, or is this the room's default?
    ///
    /// The renderer shows the two differently while a map is being
    /// authored: you want to see what you actually wrote down.
    pub drawn: bool,
    /// Does anything stop you entering? Decided here, by `footing`,
    /// so the picture and `stride` cannot differ about a wall.
    pub blocks: bool,
    /// Squares of movement spent entering, or null where nothing can.
    ///
    /// 241. SO A DM CAN CHECK THE RULES BY POINTING AT A SQUARE.
    /// Everything else about what crossing costs has only ever been
    /// visible in SQL or a test; hovering is the cheapest way to find
    /// out that the marsh you drew is difficult and the road beside it
    /// is not.
    pub cost: Option<i64>,
    /// Quarter turns clockwise from north, for the renderer to turn
    /// the tile by. Always 0 for a word that does not face.
    pub facing: i16,
    /// 254. Which piece of a wall, for the renderer alone.
    pub shape: Option<String>,
    /// A row outside the room's declared bounds.
    ///
    /// 232: shrinking a map does not delete what falls outside it, and
    /// "a tile outside the declared room should look WRONG, not
    /// vanish". This is the flag that lets it look wrong.
    pub outside: bool,
}

/// Everything one z level of a map needs to be drawn.
#[derive(Debug, Clone, Serialize)]
pub struct MapView {
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub z: i16,
    pub squares: Vec<Square>,
    pub standing: Vec<Standing>,
}

/// One level of a map, resolved and ready to draw.
#[tauri::command]
pub fn map_at(
    state: State<AppState>,
    location_id: String,
    z: i16,
) -> Result<MapView, String> {
    let token = state.token()?;
    let (bounds, default) = map_of(&token, &location_id)?;

    let rows = supabase::rest_get(
        &token,
        "tiles",
        &[
            ("select", COLUMNS),
            ("location_id", &format!("eq.{}", location_id)),
            ("z", &format!("eq.{}", z)),
        ],
    )?;
    let tiles: Vec<Tile> =
        serde_json::from_value(rows).map_err(|e| format!("could not read the map: {}", e))?;

    let mut drawn: HashMap<(i32, i32), Stack> = HashMap::new();
    // 254. THE SHAPE TRAVELS BESIDE THE STACK, NOT INSIDE IT.
    //
    // `Stack` is `terrain.rs`'s - the physics - and `ground::at`
    // resolves one from a row or the room's default. A shape is a
    // drawing and neither of them should learn about it, so it is kept
    // in its own map and joined back on at the end.
    //
    // A SQUARE FROM THE RECIPE HAS NO SHAPE, which falls out of this
    // for free: only drawn rows are in here, and a default describes
    // what a room is MADE of rather than how a wall is cut.
    let mut shapes: HashMap<(i32, i32), String> = HashMap::new();
    for t in &tiles {
        drawn.insert((t.x, t.y), t.stack()?);
        if let Some(sh) = t.shape.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            shapes.insert((t.x, t.y), sh.to_string());
        }
    }

    // WHICH SQUARES EXIST AT ALL: every drawn row, plus every square
    // the room encloses if it has a default. An unmeasured map or one
    // with no recipe is just its rows - which is right, because
    // `ground::at` would return nothing for the rest anyway.
    let mut wanted: Vec<(i32, i32)> = drawn.keys().copied().collect();
    if default.is_some() {
        if let (Some(w), Some(h)) = (bounds.width, bounds.height) {
            for y in 0..h {
                for x in 0..w {
                    if !drawn.contains_key(&(x, y)) {
                        wanted.push((x, y));
                    }
                }
            }
        }
    }
    wanted.sort_by_key(|(x, y)| (*y, *x));

    let squares = wanted
        .into_iter()
        .filter_map(|(x, y)| {
            let stack = crate::ground::at(drawn.get(&(x, y)).copied(), default, bounds, x, y)?;
            Some(Square {
                x,
                y,
                surface: stack.surface.key().to_string(),
                cover: stack.cover.map(|c| c.key().to_string()),
                facing: stack.facing.quarter(),
                drawn: drawn.contains_key(&(x, y)),
                blocks: !stack.enterable(),
                cost: stack.step_cost(),
                outside: !bounds.allows(x, y),
                shape: shapes.get(&(x, y)).cloned(),
            })
        })
        .collect();

    // Only the creatures on THIS level. A bridge deck and the floor
    // beneath it are different pictures.
    let here: Vec<Standing> = standing_in(state, location_id)?
        .into_iter()
        .filter(|c| c.z == z)
        .collect();

    Ok(MapView {
        width: bounds.width,
        height: bounds.height,
        z,
        squares,
        standing: here,
    })
}

/* ============== 263. SOMETHING CAN FINALLY BREAK ============== */

/// What one blow did.
///
/// SAID BACK RATHER THAN LEFT TO BE RE-READ. The threshold means the
/// amount that lands is not the amount that was typed, and a player
/// watching a wall not budge deserves to be told why.
#[derive(Debug, Serialize)]
pub struct Hit {
    pub layer: &'static str,
    pub word: String,
    /// What was typed.
    pub rolled: i64,
    /// What got through the threshold. Zero is a legitimate answer.
    pub landed: i64,
    pub hp_before: i64,
    pub hp_after: i64,
    pub wear: &'static str,
    /// Set when the layer stopped existing as it was.
    pub became: Option<String>,
    /// The square after, in the order the panel reads layers.
    pub now: Vec<String>,
    /// 271. Where the surplus ran, and what had nowhere to go.
    pub spill: Vec<String>,
    pub surplus: i64,
}

/// Which layer a caller means.
fn layer_of(k: &str) -> Result<Layer, String> {
    match k {
        "sub" => Ok(Layer::Sub),
        "surface" => Ok(Layer::Surface),
        "cover" => Ok(Layer::Cover),
        other => Err(format!("\"{}\" is not a layer - sub, surface or cover", other)),
    }
}

fn word_in(stack: &Stack, layer: Layer) -> Option<Terrain> {
    match layer {
        Layer::Sub => stack.sub,
        Layer::Surface => Some(stack.surface),
        Layer::Cover => stack.cover,
    }
}

/// Read one square the way the panel does: the row if there is one, the
/// recipe if there is not.
fn square_at(
    token: &str,
    location_id: &str,
    x: i32,
    y: i32,
    z: i16,
) -> Result<(Option<Tile>, Stack), String> {
    let (bounds, default) = map_of(token, location_id)?;
    let rows = supabase::rest_get(
        token,
        "tiles",
        &[
            ("select", COLUMNS),
            ("location_id", &format!("eq.{}", location_id)),
            ("x", &format!("eq.{}", x)),
            ("y", &format!("eq.{}", y)),
            ("z", &format!("eq.{}", z)),
        ],
    )?;
    let tiles: Vec<Tile> =
        serde_json::from_value(rows).map_err(|e| format!("could not read the square: {}", e))?;
    let tile = tiles.into_iter().next();
    let row = match tile.as_ref() {
        Some(t) => Some(t.stack()?),
        None => None,
    };
    let stack = crate::ground::at(row, default, bounds, x, y)
        .ok_or_else(|| format!("({}, {}, {}) is not mapped", x, y, z))?;
    Ok((tile, stack))
}

/// Read the eight squares around one, skipping any that are not mapped.
///
/// 271. ONE REQUEST, NOT EIGHT - the same reason `squares_at` exists.
///
/// AND UNMAPPED IS SKIPPED RATHER THAN AN ERROR. 226 says a missing row
/// means the square is not part of the room, so spoil that reaches the
/// edge of the map has simply run out of somewhere to go - which is
/// what `surplus` is for and not something to refuse a blow over.
fn ring_at(
    token: &str,
    location_id: &str,
    x: i32,
    y: i32,
    z: i16,
) -> Result<Vec<(i32, i32, Option<Tile>, Stack)>, String> {
    let (bounds, default) = map_of(token, location_id)?;
    let clauses: Vec<String> = crate::terrain::AROUND
        .iter()
        .map(|(dx, dy)| format!("and(x.eq.{},y.eq.{},z.eq.{})", x + dx, y + dy, z))
        .collect();
    let rows = supabase::rest_get(
        token,
        "tiles",
        &[
            ("select", COLUMNS),
            ("location_id", &format!("eq.{}", location_id)),
            ("or", &format!("({})", clauses.join(","))),
        ],
    )?;
    let tiles: Vec<Tile> =
        serde_json::from_value(rows).map_err(|e| format!("could not read the ground: {}", e))?;

    let mut out = Vec::new();
    for (dx, dy) in crate::terrain::AROUND {
        let (nx, ny) = (x + dx, y + dy);
        let tile = tiles
            .iter()
            .find(|t| t.x == nx && t.y == ny && t.z == z)
            .cloned();
        let row = match tile.as_ref() {
            Some(t) => Some(t.stack()?),
            None => None,
        };
        if let Some(stack) = crate::ground::at(row, default, bounds, nx, ny) {
            out.push((dx, dy, tile, stack));
        }
    }
    Ok(out)
}

/// How much of a layer a square is currently holding.
///
/// NULL MEANS WHOLE, which is the convention every layer uses, so this
/// is the one place that has to know it rather than three.
fn standing_at(tile: &Option<Tile>, which: Layer, word: Terrain) -> i64 {
    let stored = match which {
        Layer::Surface => tile.as_ref().and_then(|t| t.surface_hp),
        Layer::Cover => tile.as_ref().and_then(|t| t.cover_hp),
        Layer::Sub => None,
    };
    stored.unwrap_or_else(|| word.integrity().map(|i| i.hp).unwrap_or(0))
}

/// Put debris into one NEIGHBOURING square and write it.
///
/// -------------------------------------------------------------------
/// 271. A HOLE IS FILLED BY WEARING IT DOWN
/// -------------------------------------------------------------------
///
/// 262 ruled that you destroy a pit by filling it, so filling one IS
/// wearing its surface down - and `wear_away` already does every step
/// that follows from reaching zero. Spilling into a hole therefore
/// needs no second copy of destroy-at-zero, which is the whole reason
/// 269 extracted that function.
///
/// THE RECURSION TERMINATES, and a test says why: no volume leaves
/// spoil, so a hole that fills has nothing to pour onward.
#[allow(clippy::too_many_arguments)]
fn spill_into(
    token: &str,
    location_id: &str,
    x: i32,
    y: i32,
    z: i16,
    tile: &Option<Tile>,
    stack: Stack,
    debris: Terrain,
    amount: i64,
) -> Result<(), String> {
    let surface_hp = tile.as_ref().and_then(|t| t.surface_hp);
    let cover_hp = tile.as_ref().and_then(|t| t.cover_hp);
    match stack.space_for(debris, surface_hp, cover_hp) {
        Space::Hole(_) => {
            let had = standing_at(tile, Layer::Surface, stack.surface);
            wear_away(
                token,
                location_id,
                x,
                y,
                z,
                tile,
                stack,
                Layer::Surface,
                stack.surface,
                had - amount,
            )?;
            Ok(())
        }
        Space::Pile(_) => {
            // ADDED TO WHAT WAS THERE, which is the accumulation Dave
            // ruled. A square with no heap yet was holding nothing, and
            // `standing_at` must not be asked - it would read the mouth
            // as a full heap.
            let had = match stack.cover == Some(debris) {
                true => standing_at(tile, Layer::Cover, debris),
                false => 0,
            };
            let next = Stack {
                cover: Some(debris),
                ..stack
            };
            next.legal()?;
            save_square(
                token,
                location_id,
                x,
                y,
                z,
                tile.is_some(),
                &next,
                surface_hp,
                Some(had + amount),
            )
        }
        Space::None => Ok(()),
    }
}

/// Write a square's layers and hit points.
///
/// PATCH WHEN THE ROW EXISTS, INSERT WHEN IT DOES NOT, and never an
/// upsert of the whole row: `leads_to`, `to_x`, `to_y` and `to_z` live
/// on the same row and a full overwrite would quietly take a staircase
/// out of a square somebody hit.
///
/// 226. AND HITTING AN IMPLIED SQUARE WRITES IT DOWN. A square the room
/// merely implies has nowhere to record damage, so the first blow is
/// also the moment it stops being a recipe and becomes a row. That is
/// not a side effect to apologise for - it is the only honest place to
/// put the number.
#[allow(clippy::too_many_arguments)]
fn save_square(
    token: &str,
    location_id: &str,
    x: i32,
    y: i32,
    z: i16,
    existing: bool,
    stack: &Stack,
    surface_hp: Option<i64>,
    cover_hp: Option<i64>,
) -> Result<(), String> {
    let body = json!({
        "surface": stack.surface.key(),
        "cover": stack.cover.map(|t| t.key()),
        "sub": stack.sub.map(|t| t.key()),
        "surface_hp": surface_hp,
        "cover_hp": cover_hp,
    });
    if existing {
        let filter = [
            ("location_id", format!("eq.{}", location_id)),
            ("x", format!("eq.{}", x)),
            ("y", format!("eq.{}", y)),
            ("z", format!("eq.{}", z)),
        ];
        let filter: Vec<(&str, &str)> =
            filter.iter().map(|(k, v)| (*k, v.as_str())).collect();
        supabase::rest_update(token, "tiles", &filter, &body)
            .map_err(|e| denied(e, "change the map"))?;
        return Ok(());
    }
    let mut body = body;
    body["location_id"] = json!(location_id);
    body["x"] = json!(x);
    body["y"] = json!(y);
    body["z"] = json!(z);
    body["facing"] = json!(stack.facing as i16);
    supabase::rest_insert(token, "tiles", &json!([body]))
        .map_err(|e| denied(e, "change the map"))?;
    Ok(())
}

/// Which hit points the square still has after a blow.
///
/// 263. A LAYER THAT CHANGED WORD IS A NEW THING and starts whole, so
/// its number goes back to null rather than carrying the old one's
/// wounds across. A layer that did NOT change keeps what it had.
///
/// THAT SECOND HALF IS THE WHOLE POINT. The first version of this reset
/// both columns whenever anything was destroyed, which meant breaching
/// a wall standing on damaged ice silently healed the ice - a repair
/// nobody asked for, arriving as the ordinary answer to a different
/// question. 245 exists to stop exactly that, and it was reintroduced
/// three layers down from where 245 is written.
///
/// The test is the comparison itself: the words before and the words
/// after. Nothing here has to know WHICH destruction fired.
fn columns_after(
    before: &Stack,
    after: &Stack,
    hit: Layer,
    was_surface_hp: Option<i64>,
    was_cover_hp: Option<i64>,
    // What is left on the layer that was hit - `None` if it is gone.
    left: Option<i64>,
) -> (Option<i64>, Option<i64>) {
    let surface = match hit == Layer::Surface {
        true => left,
        false => (after.surface == before.surface).then_some(was_surface_hp).flatten(),
    };
    let cover = match hit == Layer::Cover {
        true => left,
        false => (after.cover == before.cover).then_some(was_cover_hp).flatten(),
    };
    // The column cannot hold a number for a cover that is not there.
    (surface, after.cover.is_some().then_some(cover).flatten())
}

/// What a square became, once something had worn a layer down.
struct Broke {
    became: Option<String>,
    now: Vec<String>,
    /// 271. Where the surplus ran, said in words for the panel.
    spill: Vec<String>,
    /// What had nowhere to go. See `Poured::surplus` - nothing in the
    /// vocabulary can reach this from a collapse, and it is reported
    /// rather than swallowed so that if anything ever does, it is on
    /// screen instead of in the ground.
    surplus: i64,
}

/// Take a layer down to `after`, and write whatever that makes it.
///
/// -------------------------------------------------------------------
/// 269. ONE DESTRUCTION PATH, NOW THAT TWO THINGS CAUSE IT
/// -------------------------------------------------------------------
///
/// Lifted whole out of `damage_square` without a line changed, because
/// `dig_square` needs every step of it and a second copy would be the
/// two-places problem in its most literal form - two routes to
/// destroying a square, drifting apart one bug fix at a time.
///
/// A BLOW AND A SHIFT DIFFER ONLY IN WHAT THEY SUBTRACT. One is dice
/// through a threshold, the other is hours at Dave's rate. Everything
/// after the subtraction - the destruction, the spoil and its amount,
/// which hit point columns survive, the row written - is the same
/// square either way, and now says so in one place.
#[allow(clippy::too_many_arguments)]
fn wear_away(
    token: &str,
    location_id: &str,
    x: i32,
    y: i32,
    z: i16,
    tile: &Option<Tile>,
    stack: Stack,
    which: Layer,
    word: Terrain,
    after: i64,
) -> Result<Broke, String> {
    let gone = after == 0;
    let mut next = match gone {
        true => stack.destroyed(which),
        false => stack,
    };

    let (mut surface_hp, mut cover_hp) = columns_after(
        &stack,
        &next,
        which,
        tile.as_ref().and_then(|t| t.surface_hp),
        tile.as_ref().and_then(|t| t.cover_hp),
        (!gone).then_some(after),
    );

    /* ---- 271. AND THE SPOIL POURS ----

       265 put the spoil on this square's cover with an amount, because
       a heap had only one place it could go. Dave's ruling gave it
       three, so WHERE is now a rule - `pour` - and this is the plumbing
       that reads the neighbourhood for it and writes what it decides.

       265'S POINT SURVIVES INTACT: the amount is still a quarter of
       what it came from, and `columns_after`'s null would still have
       meant a full heap however little was broken. The pour carries the
       number now instead of this block correcting it.                 */
    let mut spill: Vec<String> = Vec::new();
    let mut surplus = 0;
    if let Some((mess, amount)) = gone.then(|| word.spoil()).flatten() {
        let ring = ring_at(token, location_id, x, y, z)?;
        let home = next.space_for(mess, surface_hp, cover_hp);
        let mut rooms: Vec<(i32, i32, Space)> = vec![(0, 0, home)];
        for (dx, dy, t, st) in &ring {
            rooms.push((
                *dx,
                *dy,
                st.space_for(
                    mess,
                    t.as_ref().and_then(|t| t.surface_hp),
                    t.as_ref().and_then(|t| t.cover_hp),
                ),
            ));
        }
        let poured = crate::terrain::pour(amount, &rooms);
        surplus = poured.surplus;

        for (dx, dy, put) in poured.placed {
            if dx == 0 && dy == 0 {
                // THIS SQUARE IS FOLDED INTO THE ONE WRITE BELOW. The
                // row is already being written for the blow, and two
                // writes to one row is two chances to half-fail.
                match home {
                    // IT FELL IN. Dig through a floor into water and
                    // the diggings do not float on it.
                    Space::Hole(_) => {
                        // WHOLE UNLESS THE COLUMN SAYS OTHERWISE, and
                        // NOT what the tile used to say: a revealed
                        // surface is a different word, so
                        // `columns_after` has already nulled the number
                        // the old floor carried. Reading the tile here
                        // would charge the water for the floor's
                        // wounds.
                        let had = surface_hp.unwrap_or_else(|| {
                            next.surface.integrity().map(|i| i.hp).unwrap_or(0)
                        });
                        match had - put {
                            0 => {
                                next = next.destroyed(Layer::Surface);
                                surface_hp = None;
                            }
                            n => surface_hp = Some(n),
                        }
                    }
                    _ => {
                        next.cover = Some(mess);
                        cover_hp = Some(put);
                    }
                }
            } else if let Some((_, _, t, st)) =
                ring.iter().find(|(a, b, _, _)| *a == dx && *b == dy)
            {
                spill_into(token, location_id, x + dx, y + dy, z, t, *st, mess, put)?;
                spill.push(format!("{} into ({}, {})", put, x + dx, y + dy));
            }
        }
    }

    // A rule that produced a square nothing could read back would be
    // worse than a refusal. ASKED AFTER THE POUR, because the pour can
    // change which words are here - a hole that filled is no longer a
    // hole.
    next.legal()?;

    save_square(
        token,
        location_id,
        x,
        y,
        z,
        tile.is_some(),
        &next,
        surface_hp,
        cover_hp,
    )?;

    let became = gone.then(|| match word_in(&next, which) {
        Some(t) => t.key().to_string(),
        None => "nothing".to_string(),
    });
    let mut now = Vec::new();
    if let Some(t) = next.sub {
        now.push(t.key().to_string());
    }
    now.push(next.surface.key().to_string());
    if let Some(t) = next.cover {
        now.push(t.key().to_string());
    }
    Ok(Broke {
        became,
        now,
        spill,
        surplus,
    })
}

/// Hit one layer of one square.
///
/// 245 SAID DAMAGE IS PLAY, and this is the play half. It SUBTRACTS,
/// through `Integrity::lands`, and fires `destruction()` at zero. The
/// authoring half is `set_square_hp`, which assigns and fires nothing.
///
/// THE FIRST CALLER `Wear`, `integrity()` AND `destruction()` HAVE EVER
/// HAD outside `terrain.rs`. Thirteen words have carried hit points
/// since 226 and not one of them had ever been reachable.
#[tauri::command]
pub fn damage_square(
    state: State<AppState>,
    location_id: String,
    x: i32,
    y: i32,
    z: i16,
    layer: String,
    amount: i64,
) -> Result<Hit, String> {
    if amount < 0 {
        return Err("damage is not negative - to repair, set the hit points".to_string());
    }
    let which = layer_of(&layer)?;
    let token = state.token()?;
    let (tile, stack) = square_at(&token, &location_id, x, y, z)?;

    let word = word_in(&stack, which)
        .ok_or_else(|| format!("there is no {} on ({}, {}, {})", layer, x, y, z))?;
    let integ = word
        .integrity()
        .ok_or_else(|| format!("{} cannot be attacked", word.key()))?;

    // 266. AND IS ANYTHING STANDING ON IT?
    //
    // BEFORE ANY NUMBER MOVES. Dave ruled the whole strike is refused
    // rather than only the killing blow - you cannot reach the floor at
    // all while the wall stands, and a floor whittled to one hit point
    // that can never drop is a trap. This is the only door: a square is
    // destroyed here or nowhere, because `set_square_hp` refuses zero.
    stack.reachable(which)?;

    // WHERE THE NUMBER COMES FROM. Null means nobody has hurt this,
    // which is the maximum - not zero. Read the other way every
    // untouched door in the world would already be destroyed.
    let stored = match which {
        Layer::Surface => tile.as_ref().and_then(|t| t.surface_hp),
        Layer::Cover => tile.as_ref().and_then(|t| t.cover_hp),
        // Two columns, three layers - see `Stack::destroyed`.
        Layer::Sub => return Err("a sub has no hit points yet".to_string()),
    };
    let before = stored.unwrap_or(integ.hp);
    let landed = integ.lands(amount);
    let after = (before - landed).max(0);
    let broke = wear_away(
        &token,
        &location_id,
        x,
        y,
        z,
        &tile,
        stack,
        which,
        word,
        after,
    )?;
    Ok(Hit {
        layer: match which {
            Layer::Sub => "sub",
            Layer::Surface => "surface",
            Layer::Cover => "cover",
        },
        word: word.key().to_string(),
        rolled: amount,
        landed,
        hp_before: before,
        hp_after: after,
        wear: wear_word(integ.wear(after)),
        became: broke.became,
        now: broke.now,
        spill: broke.spill,
        surplus: broke.surplus,
    })
}

/* ============ 268. THE VERB THAT SPENDS THE DEBRIS ============ */

/// What one action of building did.
#[derive(Debug, Serialize)]
pub struct Built {
    /// Which layer the work went into.
    pub layer: &'static str,
    /// 269. How long it actually took, and that said in words.
    ///
    /// WHAT WAS DONE, NOT WHAT WAS ASKED FOR. A shift that finishes the
    /// wall early is not billed for the rest of the afternoon.
    pub spent: i64,
    pub took: String,
    pub workers: i64,
    /// What is being made, or the volume being filled.
    pub what: String,
    /// How much of the heap went in. Never more than one heap holds.
    pub placed: i64,
    /// What the work stands at now, out of what it wants.
    pub hp_now: i64,
    pub hp_full: i64,
    pub done: bool,
    /// Set when finishing it changed the word - a filled pit is rock.
    pub became: Option<String>,
    /// The heap that paid for it, and what is left in it.
    pub heap: String,
    pub heap_left: i64,
    /// 270. What it was shifted with. None is bare hands.
    pub tool: Option<String>,
    /// The square after, in the order the panel reads layers.
    pub now: Vec<String>,
}

/// Whether the worker is carrying something that will bite this, and
/// what it turned out to be.
///
/// -------------------------------------------------------------------
/// 270. CARRIED, NOT IN HAND
/// -------------------------------------------------------------------
///
/// Dave's ruling, and the reason it is the right one: `objects.slot`
/// says what is held, which is the correct question inside a six-second
/// round and the wrong one across a shift measured in hours. Nobody
/// digs all afternoon without taking the shovel out of the pack, and
/// refusals like "the shovel is in their pack" read as fussy rather
/// than tactical.
///
/// `Ok(None)` IS SUCCESS WITH BARE HANDS - sand and brambles, which the
/// rule in `terrain.rs` says need nothing. `Ok(Some(name))` names the
/// tool so the panel can say what the work was done with.
///
/// TWO REQUESTS AND A SERVER-SIDE FILTER, not an embedded join, for the
/// reason `load_loadout` states: a join that a policy trims looks like
/// missing data instead of a permission problem.
fn tooled(token: &str, who: &Standpoint, word: Terrain) -> Result<Option<String>, String> {
    let Some(wants) = word.bite() else {
        return Ok(None);
    };
    let refuse = || {
        format!(
            "{} has nothing to work {} with - it wants {}: {}",
            who.name,
            word.key(),
            wants.key(),
            wants.tools()
        )
    };

    let Some(entity_id) = who.entity_id.as_deref() else {
        return Err(refuse());
    };
    let held = supabase::rest_get(
        token,
        "objects",
        &[("select", "item_key"), ("holder_id", &format!("eq.{}", entity_id))],
    )?;
    let keys: Vec<String> = held
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|r| r.get("item_key"))
                .filter_map(|v| v.as_str())
                .map(crate::narrative::quoted)
                .collect()
        })
        .unwrap_or_default();
    // AN EMPTY `in.()` IS NOT VALID PostgREST, so this guard is
    // load-bearing rather than tidiness - the same note load_loadout
    // carries over the same hole.
    if keys.is_empty() {
        return Err(refuse());
    }

    let rows = supabase::rest_get(
        token,
        "items",
        &[
            ("select", "name,bites"),
            ("or", &format!("(game_id.is.null,game_id.eq.{})", who.game_id)),
            ("key", &format!("in.({})", keys.join(","))),
            // `cs` IS ARRAY-CONTAINS. The question is asked where the
            // rows are rather than dragged back and sifted here.
            ("bites", &format!("cs.{{{}}}", wants.key())),
            ("limit", "1"),
        ],
    )?;
    match rows
        .as_array()
        .and_then(|r| r.first())
        .and_then(|r| r.get("name"))
        .and_then(|v| v.as_str())
    {
        Some(name) => Ok(Some(name.to_string())),
        None => Err(refuse()),
    }
}

/// Who is standing on exactly this square.
fn who_is_on(
    token: &str,
    location_id: &str,
    x: i32,
    y: i32,
    z: i16,
) -> Result<Option<String>, String> {
    let rows = supabase::rest_get(
        token,
        "characters",
        &[
            ("select", "name"),
            ("location_id", &format!("eq.{}", location_id)),
            ("x", &format!("eq.{}", x)),
            ("y", &format!("eq.{}", y)),
            ("z", &format!("eq.{}", z)),
            ("limit", "1"),
        ],
    )?;
    Ok(rows
        .as_array()
        .and_then(|a| a.first())
        .and_then(|r| r.get("name"))
        .and_then(|n| n.as_str())
        .map(str::to_string))
}

/// Spend the heap you are standing in on the square beside you.
///
/// -------------------------------------------------------------------
/// 268. THE CONSUMER THE ECONOMY HAS BEEN MISSING
/// -------------------------------------------------------------------
///
/// 262 designed a destruction economy and 265 built its producer.
/// Every number it needs has existed since - a quantity, a material
/// and a rate - and nothing spent any of it. This is the verb.
///
/// **269 REPRICED IT IN TIME.** It was one action per heap; it is now a
/// shift of so many minutes by up to three workers, at five minutes a
/// hit point. A heap is an hour and a quarter of shifting and a stone
/// wall is five hours of masonry - and the four heaps that feed it are
/// five hours of hauling, which nothing arranged. What survives from
/// 268 is the heap CAP: you still cannot place stone you do not have.
/// That was never the rate, it only looked like one.
///
/// **YOU SHOVEL FROM THE PILE YOU ARE STANDING IN.** The source is the
/// builder's own square, not a parameter. That is a rule and not a
/// convenience: moving debris from somewhere else is HAULING, a
/// different verb with its own cost, and inventing it here as a free
/// side effect of building would price it at nothing.
///
/// **TWO ARMS, CHOSEN BY WHAT THE TARGET IS.** A volume is filled, a
/// floor is built on. You cannot wall a hole - fill it first - and
/// that refusal is the honest answer for a square that is both.
///
/// IT SPENDS BEFORE IT PLACES. There is no transaction across two
/// squares, so one of the two writes can fail alone. Spending first
/// means a failure costs the player their rubble; placing first would
/// mean a failure MINTS a wall, which is repeatable. The annoying
/// failure beats the exploitable one.
#[tauri::command]
pub fn build_square(
    state: State<AppState>,
    character_id: String,
    x: i32,
    y: i32,
    z: i16,
    word: Option<String>,
    minutes: i64,
    helpers: i64,
) -> Result<Built, String> {
    let token = state.token()?;
    // 269. HOW MANY HANDS, refused rather than capped - a fourth worker
    // quietly ignored would make the job take longer than the party was
    // told, with nothing on screen to explain why.
    let workers = crate::terrain::crew(1 + helpers)?;
    let shift = minutes.saturating_mul(crate::clock::MINUTE);

    let rows = supabase::rest_get(
        &token,
        "characters",
        &[
            ("select", "name,game_id,entity_id,location_id,x,y,z"),
            ("id", &format!("eq.{}", character_id)),
        ],
    )?;
    let mut who: Vec<Standpoint> =
        serde_json::from_value(rows).map_err(|e| format!("could not read the builder: {}", e))?;
    let who = who.pop().ok_or("no such creature")?;

    // CLONED, so `who` stays whole - 270 asks it for the holder further
    // down and a partial move here would take the name with it.
    let Some(location_id) = who.location_id.clone() else {
        return Err(format!("{} is not anywhere", who.name));
    };
    let (Some(fx), Some(fy), Some(fz)) = (who.x, who.y, who.z) else {
        return Err(format!("{} is not on the map - place them first", who.name));
    };

    // WITHIN ARM'S REACH, and on the same level. Building across a gap
    // is throwing rubble at a wall and hoping.
    if fz != z || (x - fx).abs() > 1 || (y - fy).abs() > 1 {
        return Err(format!(
            "{} is at ({}, {}) and cannot reach ({}, {}) - stand beside it",
            who.name, fx, fy, x, y
        ));
    }

    // THE HEAP IS UNDERFOOT.
    let (from_tile, from) = square_at(&token, &location_id, fx, fy, fz)?;
    let heap = from
        .cover
        .filter(|c| c.is_debris())
        .ok_or_else(|| format!("{} is not standing in any debris to build with", who.name))?;
    let heap_full = heap
        .integrity()
        .ok_or_else(|| format!("{} cannot be spent", heap.key()))?
        .hp;
    let heap_has = from_tile
        .as_ref()
        .and_then(|t| t.cover_hp)
        .unwrap_or(heap_full);

    // 270. AND THE BITE COMES OFF THE HEAP, NOT OFF THE WALL. You are
    // spending debris, and debris is shovel work whatever it used to
    // be - so building was priced by the tool rule with no second
    // decision taken anywhere. Filling a pit agrees from both sides at
    // once: the fill wants a scoop and so does the rubble going in.
    let tool = tooled(&token, &who, heap)?;

    let (tile, stack) = square_at(&token, &location_id, x, y, z)?;

    // NOBODY IS BUILT INTO. Dave's ruling. It names who, because "that
    // square is occupied" when the occupant is you reads as a bug.
    if let Some(name) = who_is_on(&token, &location_id, x, y, z)? {
        return Err(format!("{} is standing there", name));
    }

    let filling = stack.surface.is_volume();
    if filling && word.is_some() {
        return Err(format!(
            "there is {} here - fill it before you build on it",
            stack.surface.key()
        ));
    }

    // Which layer the work goes into, what it wants, and what it has.
    let (which, into, full, has) = match filling {
        true => {
            // 266 APPLIES UNCHANGED: filling is destroying the surface,
            // so a cover that could be attacked is still in the way.
            stack.reachable(Layer::Surface)?;
            let integ = stack
                .surface
                .integrity()
                .ok_or_else(|| format!("{} cannot be filled", stack.surface.key()))?;
            let has = tile.as_ref().and_then(|t| t.surface_hp).unwrap_or(integ.hp);
            (Layer::Surface, stack.surface, integ.hp, has)
        }
        false => {
            let key = word
                .as_deref()
                .ok_or("say what to build - a heap could raise any wall of its material")?;
            let wall = Terrain::of_key(key).ok_or_else(|| format!("\"{}\" is not a word", key))?;
            if !heap.may_raise(wall) {
                return Err(format!(
                    "{} cannot raise {} - debris builds a wall of its own material",
                    heap.key(),
                    wall.key()
                ));
            }
            if let Some(c) = stack.cover {
                if c != wall {
                    return Err(format!("{} is in the way", c.key()));
                }
            }
            let full = wall
                .integrity()
                .ok_or_else(|| format!("{} has no hit points", wall.key()))?
                .hp;
            // A wall part built holds what it holds; a square with no
            // cover holds nothing yet. Null means WHOLE for a thing
            // that exists, which is why it is only read when one does.
            let has = match stack.cover == Some(wall) {
                true => tile.as_ref().and_then(|t| t.cover_hp).unwrap_or(full),
                false => 0,
            };
            (Layer::Cover, wall, full, has)
        }
    };

    // HOW MUCH THIS ACTION MOVES. Filling counts a volume DOWN to
    // nothing and raising counts a wall UP to full, so "what is still
    // wanted" is the one of the two things that differs.
    let wanted = match filling {
        true => has,
        false => full - has,
    };
    if wanted <= 0 {
        return Err(format!("{} is already finished", into.key()));
    }
    // WHICHEVER RAN OUT FIRST - the heap, or the hours. 268 had only
    // the first of these; 269 gave the second a number.
    let bought = crate::terrain::earned(shift, workers);
    let crate::terrain::Work { placed, done } =
        crate::terrain::Work::of(heap_has.min(bought), wanted);
    // AND AN ACTION THAT MOVES NOTHING IS NOT AN ACTION. Nothing should
    // be able to leave a heap at zero - it is destroyed at zero and
    // `set_square_hp` refuses the number - but a no-op here would write
    // two squares and log a day's work for an empty barrow, which is
    // exactly the shape of thing that looks like an ordinary answer.
    if placed <= 0 {
        return Err(match bought {
            0 => format!(
                "{} is not long enough to shift anything - a hit point of work is five minutes",
                crate::clock::said(shift)
            ),
            _ => format!("there is nothing left in the {}", heap.key()),
        });
    }
    let left = match filling {
        true => has - placed,
        false => has + placed,
    };

    let next = match done {
        true if filling => stack.destroyed(Layer::Surface),
        // A finished wall is simply the word, whole.
        _ => Stack {
            cover: Some(into),
            ..stack
        },
    };
    next.legal()?;

    // 263'S COMPARISON, UNCHANGED AND NOW READ BACKWARDS. It asks which
    // words changed and keeps the numbers of the ones that did not -
    // nothing in it knows whether the change was a blow or a day's
    // work, which is why building needed no second version of it.
    let (surface_hp, cover_hp) = columns_after(
        &stack,
        &next,
        which,
        tile.as_ref().and_then(|t| t.surface_hp),
        tile.as_ref().and_then(|t| t.cover_hp),
        (!done).then_some(left),
    );

    /* ---- SPENT FIRST, for the reason in the doc above ---- */
    let heap_left = heap_has - placed;
    let spent = match heap_left {
        0 => from.destroyed(Layer::Cover),
        _ => from,
    };
    save_square(
        &token,
        &location_id,
        fx,
        fy,
        fz,
        from_tile.is_some(),
        &spent,
        from_tile.as_ref().and_then(|t| t.surface_hp),
        (heap_left > 0).then_some(heap_left),
    )?;

    save_square(
        &token,
        &location_id,
        x,
        y,
        z,
        tile.is_some(),
        &next,
        surface_hp,
        cover_hp,
    )?;

    /* ---- 253'S DIARY, FOR THE SAME REASONS ----

       Recorded whether or not anybody is counting, and FAILING QUIET:
       the wall is up, and refusing to report it in order to complain
       about the log would be a lie about the map.

       NO QUANTITY COLUMN. `squares` is movement's, and borrowing it for
       hit points of masonry would be the two-places problem with a
       straight face. The prose carries the amount until something
       actually needs to sum a day's work. */
    let fight = fighting_in(&token, &character_id);
    let mut row = json!({
        "game_id": who.game_id,
        "character_id": character_id,
        "key": "build",
        "cost": "build",
        "request": "build",
        "label": format!("placed {} of {}", placed, into.key()),
        "status": "done",
    });
    if let Some((actor_id, encounter_id)) = &fight {
        row["actor_id"] = json!(actor_id);
        row["encounter_id"] = json!(encounter_id);
    }
    let _ = supabase::rest_insert(&token, "actions", &json!([row]));

    /* ---- 269. AND THE CLOCK MOVES ----
       Charged for the work DONE, which is why `costs` rounds up where
       `earned` rounds down: a job that finished early is not billed for
       the shift that was asked for, and no point is ever had for less
       than five minutes. Failing quiet for 253's reason - the wall is
       up, and refusing to report it because the diary would not take
       the hour would be a lie about the map. */
    let spent = crate::terrain::costs(placed, workers);
    let _ = crate::commands::time::tick_on(&token, &who.game_id, spent);

    let became = (done && filling).then(|| match word_in(&next, which) {
        Some(t) => t.key().to_string(),
        None => "nothing".to_string(),
    });
    let mut now = Vec::new();
    if let Some(t) = next.sub {
        now.push(t.key().to_string());
    }
    now.push(next.surface.key().to_string());
    if let Some(t) = next.cover {
        now.push(t.key().to_string());
    }

    Ok(Built {
        layer: match which {
            Layer::Sub => "sub",
            Layer::Surface => "surface",
            Layer::Cover => "cover",
        },
        what: into.key().to_string(),
        placed,
        hp_now: match filling {
            true => full - left,
            false => left,
        },
        hp_full: full,
        done,
        became,
        heap: heap.key().to_string(),
        heap_left,
        now,
        tool,
        spent,
        took: crate::clock::said(spent),
        workers,
    })
}

/// What a shift of digging did.
#[derive(Debug, Serialize)]
pub struct Dug {
    pub layer: &'static str,
    pub word: String,
    /// Hit points of work done. Five minutes each, divided by the crew.
    pub worked: i64,
    pub hp_before: i64,
    pub hp_after: i64,
    pub wear: &'static str,
    /// 270. What it was worked with. None is bare hands, which the rule
    /// allows for sand and brambles and nothing else.
    pub tool: Option<String>,
    pub spent: i64,
    pub took: String,
    pub workers: i64,
    /// Set when the layer stopped existing as it was.
    pub became: Option<String>,
    pub now: Vec<String>,
    /// 271. Where the diggings ran, and what had nowhere to go.
    pub spill: Vec<String>,
    pub surplus: i64,
}

/// Work a layer down with hours instead of dice.
///
/// -------------------------------------------------------------------
/// 269. A BLOW AND A SHIFT
/// -------------------------------------------------------------------
///
/// `damage_square` is a blow: the DM types what they rolled, it goes
/// through the threshold, and anything can deal it - a trebuchet, a
/// spell, a dragon. It needs no actor and 245 says the DM is not
/// argued with. **This is the other thing**: deliberate work by a
/// creature, priced in Dave's hours, which is what a pick is for.
///
/// Both subtract, and from the same number. Everything after the
/// subtraction is `wear_away`, shared, so the two cannot drift.
///
/// **NO THRESHOLD.** A threshold models one blow being too feeble to
/// mark the stuff at all - a dagger on masonry. Sustained work with
/// the right tool is not that, so an hour of digging is an hour of
/// progress.
///
/// WHICH IS EXACTLY THE HOLE TOOLS FILL, and they are the next cycle.
/// Until then bare hands dig stone, which is wrong and is not a
/// regression: `damage_square` has let anybody break anything with a
/// typed number since 263.
#[tauri::command]
pub fn dig_square(
    state: State<AppState>,
    character_id: String,
    x: i32,
    y: i32,
    z: i16,
    layer: String,
    minutes: i64,
    helpers: i64,
) -> Result<Dug, String> {
    let which = layer_of(&layer)?;
    let workers = crate::terrain::crew(1 + helpers)?;
    let shift = minutes.saturating_mul(crate::clock::MINUTE);
    let token = state.token()?;

    let rows = supabase::rest_get(
        &token,
        "characters",
        &[
            ("select", "name,game_id,entity_id,location_id,x,y,z"),
            ("id", &format!("eq.{}", character_id)),
        ],
    )?;
    let mut who: Vec<Standpoint> =
        serde_json::from_value(rows).map_err(|e| format!("could not read the digger: {}", e))?;
    let who = who.pop().ok_or("no such creature")?;

    // CLONED, so `who` stays whole - 270 asks it for the holder further
    // down and a partial move here would take the name with it.
    let Some(location_id) = who.location_id.clone() else {
        return Err(format!("{} is not anywhere", who.name));
    };
    let (Some(fx), Some(fy), Some(fz)) = (who.x, who.y, who.z) else {
        return Err(format!("{} is not on the map - place them first", who.name));
    };

    // WITHIN REACH, AND THEIR OWN SQUARE COUNTS. Digging the floor you
    // are standing on is the ordinary case - unlike building, where the
    // square has to be empty because you cannot raise a wall through
    // yourself. What happens to somebody standing on a floor that goes
    // is 262's unbuilt falling rule, and nothing here pretends to know.
    if fz != z || (x - fx).abs() > 1 || (y - fy).abs() > 1 {
        return Err(format!(
            "{} is at ({}, {}) and cannot reach ({}, {}) - stand beside it",
            who.name, fx, fy, x, y
        ));
    }

    let (tile, stack) = square_at(&token, &location_id, x, y, z)?;
    let word = word_in(&stack, which)
        .ok_or_else(|| format!("there is no {} on ({}, {}, {})", layer, x, y, z))?;
    let integ = word
        .integrity()
        .ok_or_else(|| format!("{} cannot be worked", word.key()))?;

    // 266 UNCHANGED: a wall standing on the floor is still in the way,
    // whether you are swinging a sword at it or a pick.
    stack.reachable(which)?;

    // 270. AND SOMETHING HAS TO BITE IT. This is what closes the hole
    // 269 flagged out loud - bare hands dug stone until this line.
    //
    // ASKED BEFORE ANY NUMBER MOVES, like 266's check, because a
    // refusal that had already spent the clock would be worse than no
    // refusal at all.
    let tool = tooled(&token, &who, word)?;

    let stored = match which {
        Layer::Surface => tile.as_ref().and_then(|t| t.surface_hp),
        Layer::Cover => tile.as_ref().and_then(|t| t.cover_hp),
        Layer::Sub => return Err("a sub has no hit points yet".to_string()),
    };
    let before = stored.unwrap_or(integ.hp);
    let worked = crate::terrain::earned(shift, workers).min(before);
    if worked <= 0 {
        return Err(format!(
            "{} is not long enough to make a mark - a hit point of work is five minutes",
            crate::clock::said(shift)
        ));
    }
    let after = before - worked;

    let broke = wear_away(
        &token,
        &location_id,
        x,
        y,
        z,
        &tile,
        stack,
        which,
        word,
        after,
    )?;

    let fight = fighting_in(&token, &character_id);
    let mut row = json!({
        "game_id": who.game_id,
        "character_id": character_id,
        "key": "dig",
        "cost": "dig",
        "request": "dig",
        "label": format!("worked {} off {}", worked, word.key()),
        "status": "done",
    });
    if let Some((actor_id, encounter_id)) = &fight {
        row["actor_id"] = json!(actor_id);
        row["encounter_id"] = json!(encounter_id);
    }
    let _ = supabase::rest_insert(&token, "actions", &json!([row]));

    let spent = crate::terrain::costs(worked, workers);
    let _ = crate::commands::time::tick_on(&token, &who.game_id, spent);

    Ok(Dug {
        layer: match which {
            Layer::Sub => "sub",
            Layer::Surface => "surface",
            Layer::Cover => "cover",
        },
        word: word.key().to_string(),
        worked,
        hp_before: before,
        hp_after: after,
        wear: wear_word(integ.wear(after)),
        tool,
        spent,
        took: crate::clock::said(spent),
        workers,
        became: broke.became,
        now: broke.now,
        spill: broke.spill,
        surplus: broke.surplus,
    })
}

/// State a layer's condition outright.
///
/// 245'S OTHER HALF. Painting is authoring, so authoring is allowed to
/// SAY a door is half shattered without anyone having hit it - that is
/// a DM placing a broken door, not a repair happening by accident.
///
/// NO THRESHOLD AND NO DESTRUCTION. This is not a blow. Zero is
/// refused for the same reason: a destroyed thing is a different word,
/// and reaching that by assignment would skip the rule that decides
/// which word. `None` puts it back to untouched.
#[tauri::command]
pub fn set_square_hp(
    state: State<AppState>,
    location_id: String,
    x: i32,
    y: i32,
    z: i16,
    layer: String,
    hp: Option<i64>,
) -> Result<(), String> {
    let which = layer_of(&layer)?;
    let token = state.token()?;
    let (tile, stack) = square_at(&token, &location_id, x, y, z)?;

    let word = word_in(&stack, which)
        .ok_or_else(|| format!("there is no {} on ({}, {}, {})", layer, x, y, z))?;
    let integ = word
        .integrity()
        .ok_or_else(|| format!("{} has no hit points to set", word.key()))?;

    if let Some(n) = hp {
        if n < 1 {
            return Err(format!(
                "{} is destroyed at zero, which is a different word - break it instead",
                word.key()
            ));
        }
        if n > integ.hp {
            return Err(format!(
                "{} is worth {} hit points at most",
                word.key(),
                integ.hp
            ));
        }
    }

    let (surface_hp, cover_hp) = match which {
        Layer::Surface => (hp, tile.as_ref().and_then(|t| t.cover_hp)),
        Layer::Cover => (tile.as_ref().and_then(|t| t.surface_hp), hp),
        Layer::Sub => return Err("a sub has no hit points yet".to_string()),
    };
    save_square(
        &token,
        &location_id,
        x,
        y,
        z,
        tile.is_some(),
        &stack,
        surface_hp,
        cover_hp,
    )
}

/* ========= 258. WHAT ONE SQUARE ACTUALLY HOLDS, FOR A PANEL ========= */

/// One layer of a square: what is STORED beside what the rules DERIVE.
///
/// THE TWO ARE SEPARATED ON PURPOSE AND THE PANEL SHOWS THEM APART.
/// This project keeps one copy of every derived fact and recomputes
/// the rest, and until now that decision has only been legible in
/// Rust. A DM looking at a door should be able to see that 25 of 25
/// hit points is a STORED number and `intact` is a VERDICT about it.
///
/// It is also the cheapest bug detector we have. Paint `door_broken`
/// today and the square reads `25 / 25 - intact`: the word says broken
/// and the rules say fine, which is the whole argument for this panel
/// in one line of it.
#[derive(Debug, Clone, Serialize)]
pub struct LayerDetail {
    /// `sub`, `surface` or `cover`.
    pub layer: &'static str,
    pub word: String,

    /* ---- stored ---- */
    /// The hit points column AS IT SITS, null included.
    ///
    /// NULL IS NOT ZERO AND NOT THE MAXIMUM - it is "nobody has
    /// damaged this". Kept separate from `hp_now` because the two
    /// being confusable is exactly the trap a damage control would
    /// fall into: writing the maximum on an undamaged square gives
    /// "undamaged" two spellings that will eventually disagree.
    pub stored_hp: Option<i64>,

    /* ---- derived, all null for a word with no integrity ---- */
    pub ac: Option<i64>,
    /// What it has now - the stored number, or the maximum if nothing
    /// has touched it.
    pub hp_now: Option<i64>,
    pub hp_max: Option<i64>,
    pub threshold: Option<i64>,
    /// Moving parts that can jam. Only a mechanism can be `broken`.
    pub mechanism: bool,
    /// The hit points at or below which a mechanism jams - `hp / 10`,
    /// one division rather than a number written beside every door.
    pub breaks_at: Option<i64>,
    /// `intact`, `broken` or `destroyed`. Null when nothing can hurt
    /// it, which is most of the vocabulary.
    pub wear: Option<&'static str>,
    /// What is left when it is destroyed, in words.
    pub destruction: String,
}

/// Something that hurts you for standing there.
#[derive(Debug, Clone, Serialize)]
pub struct HazardDetail {
    pub dice: String,
    pub kind: String,
}

/// Everything one square holds, for the tile panel.
///
/// ---------------------------------------------------------------------
/// A COMMAND OF ITS OWN RATHER THAN A FATTER `MapView`
/// ---------------------------------------------------------------------
///
/// `map_at` sends every square on the level and is called on every
/// redraw. Twenty more fields times four hundred squares is a large
/// answer to a question nobody asked: the panel wants ONE square, and
/// only when somebody points at it.
///
/// It also keeps `Square` honest. That struct exists so a renderer can
/// draw; this one exists so a person can read. They will want
/// different things and should be free to.
#[derive(Debug, Clone, Serialize)]
pub struct SquareDetail {
    pub x: i32,
    pub y: i32,
    pub z: i16,

    /// Is there a ROW here, or is this the room's recipe talking?
    ///
    /// 226: A MISSING ROW MEANS UNMAPPED, NOT EMPTY FLOOR. A square
    /// the recipe implies has no stored anything - no damage, no
    /// shape, no link - and the panel must not offer a tidy set of
    /// blanks that look like data.
    pub drawn: bool,
    /// A row beyond the room's declared bounds. 232 wants it to look
    /// wrong rather than vanish.
    pub outside: bool,

    pub facing: i16,
    pub shape: Option<String>,
    /// Bottom to top: `sub` if there is one, `surface` always, `cover`
    /// if there is one.
    pub layers: Vec<LayerDetail>,

    /* ---- crossing it, all derived by `Stack::footing` ---- */
    /// `normal`, `difficult` or `blocked`.
    pub going: &'static str,
    /// Squares of movement to enter, or null where nothing can.
    pub cost: Option<i64>,
    /// `swim`, `climb` or `fly` - a GATE, not a surcharge.
    pub needs: Option<&'static str>,
    pub blocks_sight: bool,
    pub hazard: Option<HazardDetail>,
    /// Stairs and ramps, which are how a creature changes level.
    pub links_levels: bool,

    /* ---- where it goes ---- */
    pub leads_to: Option<String>,
    pub to_x: Option<i32>,
    pub to_y: Option<i32>,
    pub to_z: Option<i16>,

    /* ---- what is on it ---- */
    pub standing: Vec<Standing>,
}

fn wear_word(w: crate::terrain::Wear) -> &'static str {
    match w {
        crate::terrain::Wear::Intact => "intact",
        crate::terrain::Wear::Broken => "broken",
        crate::terrain::Wear::Destroyed => "destroyed",
    }
}

/// 265. BOTH HALVES, IN ONE SENTENCE. A destruction says what becomes
/// of the layer AND what is left lying on the square, so the panel has
/// to say both or it is describing half of what will happen.
fn destruction_word(d: crate::terrain::Destruction) -> String {
    use crate::terrain::Remains::*;
    let what = match d.remains {
        Indestructible => return "cannot be attacked".to_string(),
        Becomes(t) => format!("becomes {}", t.key()),
        Clears => "clears".to_string(),
        Reveals => "reveals what is beneath".to_string(),
    };
    match d.spoil {
        None => what,
        Some(t) => format!("{}, leaving {}", what, t.key()),
    }
}

/// One layer, stored and derived.
fn layer_detail(
    layer: &'static str,
    t: crate::terrain::Terrain,
    stored_hp: Option<i64>,
) -> LayerDetail {
    let integ = t.integrity();
    // AN UNDAMAGED THING HAS ITS MAXIMUM. Null in the column means
    // nobody has hurt it, so the verdict is asked about a full one -
    // and `wear` is asked ONCE, here, rather than by the panel.
    let hp_now = integ.map(|i| stored_hp.unwrap_or(i.hp));
    LayerDetail {
        layer,
        word: t.key().to_string(),
        stored_hp,
        ac: integ.map(|i| i.ac),
        hp_now,
        hp_max: integ.map(|i| i.hp),
        threshold: integ.map(|i| i.threshold),
        mechanism: integ.is_some_and(|i| i.mechanism),
        breaks_at: integ.and_then(|i| i.breaks_at()),
        wear: integ.map(|i| wear_word(i.wear(hp_now.unwrap_or(0)))),
        destruction: destruction_word(t.destruction()),
    }
}

/// Everything about the square at (x, y, z), or null if nothing is
/// there at all.
///
/// NULL IS AN ANSWER, NOT A FAILURE. A map can be sparse - rows at
/// (0,0) and (10,10) and nothing between - so a click inside the
/// canvas can land where no square exists and no recipe implies one.
/// `ground::at` already says so by returning `None`, and an error
/// would make pointing at a gap look like a fault.
#[tauri::command]
pub fn inspect_square(
    state: State<AppState>,
    location_id: String,
    x: i32,
    y: i32,
    z: i16,
) -> Result<Option<SquareDetail>, String> {
    let token = state.token()?;
    let (bounds, default) = map_of(&token, &location_id)?;

    // ONE SQUARE, ASKED FOR AS ONE SQUARE. Filtering here rather than
    // fetching the level and searching it keeps the panel's cost
    // independent of how big the map has grown.
    let rows = supabase::rest_get(
        &token,
        "tiles",
        &[
            ("select", COLUMNS),
            ("location_id", &format!("eq.{}", location_id)),
            ("x", &format!("eq.{}", x)),
            ("y", &format!("eq.{}", y)),
            ("z", &format!("eq.{}", z)),
        ],
    )?;
    let tiles: Vec<Tile> =
        serde_json::from_value(rows).map_err(|e| format!("could not read the square: {}", e))?;
    let tile = tiles.into_iter().next();

    let row = match tile.as_ref() {
        Some(t) => Some(t.stack()?),
        None => None,
    };
    let Some(stack) = crate::ground::at(row, default, bounds, x, y) else {
        return Ok(None);
    };

    // THE STORED HIT POINTS COME FROM THE ROW, NOT THE RECIPE. A
    // square the room merely implies has never been damaged, because
    // there is nowhere to have recorded it.
    let mut layers = Vec::new();
    if let Some(sub) = stack.sub {
        // No hit points column for a sub: it is under the floor, and
        // nothing has ever reached it.
        layers.push(layer_detail("sub", sub, None));
    }
    layers.push(layer_detail(
        "surface",
        stack.surface,
        tile.as_ref().and_then(|t| t.surface_hp),
    ));
    if let Some(cover) = stack.cover {
        layers.push(layer_detail(
            "cover",
            cover,
            tile.as_ref().and_then(|t| t.cover_hp),
        ));
    }

    let f = stack.footing();
    let here: Vec<Standing> = standing_in(state, location_id)?
        .into_iter()
        .filter(|c| c.x == x && c.y == y && c.z == z)
        .collect();

    Ok(Some(SquareDetail {
        x,
        y,
        z,
        drawn: tile.is_some(),
        outside: !bounds.allows(x, y),
        facing: stack.facing.quarter(),
        shape: tile
            .as_ref()
            .and_then(|t| t.shape.as_deref())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        layers,
        going: match f.going {
            crate::terrain::Going::Normal => "normal",
            crate::terrain::Going::Difficult => "difficult",
            crate::terrain::Going::Blocked => "blocked",
        },
        cost: f.going.step_cost(),
        needs: f.needs.map(|m| match m {
            crate::terrain::Mode::Swim => "swim",
            crate::terrain::Mode::Climb => "climb",
            crate::terrain::Mode::Fly => "fly",
        }),
        blocks_sight: f.blocks_sight,
        hazard: f.hazard.map(|h| HazardDetail {
            dice: h.dice.to_string(),
            kind: h.kind.to_string(),
        }),
        links_levels: stack.surface.links_levels(),
        leads_to: tile.as_ref().and_then(|t| t.leads_to.clone()),
        to_x: tile.as_ref().and_then(|t| t.to_x),
        to_y: tile.as_ref().and_then(|t| t.to_y),
        to_z: tile.as_ref().and_then(|t| t.to_z),
        standing: here,
    }))
}

/* ================ 237. THE VOCABULARY, FOR A PICKER ================ */

/// One terrain word and where it may sit.
#[derive(Debug, Clone, Serialize)]
pub struct Word {
    pub key: String,
    /// 261. ASKED FOR SEPARATELY EVEN THOUGH IT IS CURRENTLY THE SAME
    /// ANSWER AS `surface`.
    ///
    /// `may()` happens to give `Sub` and `Surface` the same list today -
    /// ground or a wall. A picker that read `surface` and called it
    /// close enough would be a rule inferred rather than asked for, and
    /// the day `may()` splits them the dropdown would be quietly wrong
    /// with nothing to catch it. One more boolean costs nothing.
    pub sub: bool,
    pub surface: bool,
    pub cover: bool,
    /// 268. CAN ANY HEAP OF DEBRIS RAISE THIS?
    ///
    /// 237 again, and the reason it is a flag rather than a filter in
    /// JavaScript: "the words you can build" is `may_raise` asked over
    /// the vocabulary, and a dropdown that guessed by looking for keys
    /// ending in `_wall` would be the rule inferred rather than asked
    /// for. It would also be wrong - an ice wall ends in `_wall` and
    /// nothing can ever raise one.
    pub raisable: bool,
}

/// Every terrain word, with the layers it is legal in.
///
/// 237. SO A PICKER DOES NOT HOLD ITS OWN COPY. The vocabulary is a
/// rule and lives in `terrain.rs`; a dropdown with thirty-three words
/// typed into JavaScript would be the same list in a second place, and
/// the one that drifts is always the copy nobody tests.
///
/// NO STATE AND NO TOKEN. This is a constant, not a read - it is the
/// same answer for everybody, signed in or not, and asking Supabase for
/// it would be a round trip to learn something the binary already
/// knows.
///
/// NOT A DEV-HOOK CONVENIENCE EITHER: the painting UI that replaces the
/// hook needs exactly this, and so does anything that ever validates a
/// word on screen before sending it.
#[tauri::command]
pub fn terrain_words() -> Vec<Word> {
    crate::terrain::ALL
        .iter()
        .map(|t| Word {
            key: t.key().to_string(),
            sub: t.may(crate::terrain::Layer::Sub),
            surface: t.may(crate::terrain::Layer::Surface),
            cover: t.may(crate::terrain::Layer::Cover),
            raisable: crate::terrain::ALL.iter().any(|d| d.may_raise(*t)),
        })
        .collect()
}

/* ============== 247. WALKING TO A SQUARE ============== */

/// What a walk did.
#[derive(Debug, Clone, Serialize)]
pub struct Walk {
    pub name: String,
    /// Where it ended up, which is **not** always where it was sent.
    pub x: i32,
    pub y: i32,
    pub z: i16,
    /// Squares of movement spent, summed step by step.
    pub cost: i64,
    pub steps: usize,
    /// Why it stopped short, in `stride`'s own words, or null if it
    /// arrived.
    pub stopped: Option<String>,
    pub peril: Option<String>,
    /// 253. EVERY SQUARE THIS CREATURE HAS COVERED THIS ROUND,
    /// including the walk that just happened, or null when it is not
    /// in a fight.
    ///
    /// NULL IS NOT ZERO. Outside an encounter there is no round to
    /// count within - 054 - so there is no total to report, and a 0
    /// would read as "has not moved" rather than "nobody is counting".
    pub moved_this_round: Option<i64>,
}

/// The fight this creature is in, as the roster row and the encounter.
///
/// 253. SO A MOVE CAN BE COUNTED. `stamp_action_moment` fills an
/// action's `round` and `turn_actor_id` FROM ITS `encounter_id`, so an
/// action written without one is unattributable to a round - which is
/// 054's rule and exactly right for a DM shuffling creatures around
/// while drawing a map. Those moves are still recorded; they are just
/// not counted against anybody's turn.
///
/// ACTIVE ONLY. A draft encounter is a fight being assembled, and
/// movement in one is set-dressing rather than a turn being spent.
///
/// FAILING QUIET, like the resistances on a sheet: a creature that
/// cannot be looked up walks anyway and its move is recorded without
/// an encounter. Refusing to move somebody because the roster read was
/// slow would be a worse answer than an uncounted step.
fn fighting_in(token: &str, character_id: &str) -> Option<(String, String)> {
    let rows = supabase::rest_get(
        token,
        "encounter_actors",
        &[
            (
                "select",
                "id,encounter_id,encounters!encounter_actors_encounter_id_fkey(status)",
            ),
            ("character_id", &format!("eq.{}", character_id)),
        ],
    )
    .ok()?;
    rows.as_array()?.iter().find_map(|r| {
        let status = r
            .get("encounters")
            .and_then(|e| e.get("status"))
            .and_then(|v| v.as_str())?;
        if status != "active" {
            return None;
        }
        let actor = r.get("id").and_then(|v| v.as_str())?;
        let enc = r.get("encounter_id").and_then(|v| v.as_str())?;
        Some((actor.to_string(), enc.to_string()))
    })
}

/// Walk a creature toward a square, stopping at the first thing in the
/// way.
///
/// 247. ONE CLICK, A LINE OF STEPS. `step_to` prices ONE step and
/// refuses anything not adjacent, which made crossing a room six
/// clicks. This walks `grid::line` and prices each step with
/// `stride::step`, so the rule is unchanged and only the number of
/// times it is asked has gone up.
///
/// **THE LINE IS COMPUTED BEFORE ANYTHING IS FETCHED.** `grid::line`
/// is pure arithmetic, so the squares a walk could possibly touch are
/// known in advance and read in ONE request. That is the difference
/// between this and the obvious version, which would fetch a square,
/// decide, fetch the next, and make a forty-square walk forty round
/// trips.
///
/// Reads at most the squares on the line. A walk stopped by a wall
/// will have read a few squares beyond it, which is one request either
/// way.
#[tauri::command]
pub fn walk_to(
    state: State<AppState>,
    character_id: String,
    x: i32,
    y: i32,
    z: i16,
) -> Result<Walk, String> {
    let token = state.token()?;

    let rows = supabase::rest_get(
        &token,
        "characters",
        &[
            ("select", "name,game_id,entity_id,location_id,x,y,z"),
            ("id", &format!("eq.{}", character_id)),
        ],
    )?;
    let mut who: Vec<Standpoint> =
        serde_json::from_value(rows).map_err(|e| format!("could not read the creature: {}", e))?;
    let who = who.pop().ok_or("no such creature")?;

    let Some(location_id) = who.location_id else {
        return Err(format!("{} is not anywhere", who.name));
    };
    let (Some(fx), Some(fy), Some(fz)) = (who.x, who.y, who.z) else {
        return Err(format!("{} is not on the map - place them first", who.name));
    };

    let at = Point::new(fx, fy, fz);
    let toward = Point::new(x, y, z);

    // Every square the walk could touch, known before the first read.
    let spots: Vec<Spot> = crate::grid::line(at, toward)
        .map(|p| Spot { x: p.x, y: p.y, z: p.z })
        .collect();
    let ground = squares_at(&token, &location_id, &spots)?;

    // 234. THROUGH `ground::at`, so the room's default counts as floor
    // - a corridor nobody drew square by square is still walkable.
    let (bounds, default) = map_of(&token, &location_id)?;
    let resolve = |p: Point| {
        crate::ground::at(ground.get(&(p.x, p.y, p.z)).copied(), default, bounds, p.x, p.y)
    };

    // No modes yet: nothing records a swim or a fly speed, so every
    // creature walks and deep water perils all of them equally. 059
    // named `species.speed` as the column a movement system reads;
    // when that lands this is where the modes arrive with it.
    let went = crate::stride::walk(at, toward, resolve, &[])
        .map_err(|e| format!("{} did not move - {}", who.name, e))?;

    let back = supabase::rest_update(
        &token,
        "characters",
        &[("id", &format!("eq.{}", character_id))],
        &json!({ "x": went.reached.x, "y": went.reached.y, "z": went.reached.z }),
    )
    .map_err(|e| denied(e, "move a creature about the map"))?;

    // 217. A blocked UPDATE matches no row and returns success.
    if back.as_array().map(Vec::len).unwrap_or(0) == 0 {
        return Err(format!("{} did not move - not yours to move", who.name));
    }

    /* ---- 253. THE MOVE IS A THING THAT HAPPENED ----

       Recorded whether or not anybody is counting. Inside an active
       fight it carries the encounter and the roster row, and the
       trigger stamps the round and whose turn it was; outside one it
       is a row with no round, which 054 says cannot be attributed to
       a turn and therefore is not.

       `key` AND `cost` ARE BOTH "move", which is `Act::cost`'s
       fallback rule - a cost word used as a key IS that cost - so the
       count agrees whether it reads the column or the key.

       FAILING QUIET. The creature HAS MOVED: the position was written
       above and 217-checked. If the log write fails, the move still
       happened, and refusing to report it would be a lie about the
       map in order to complain about the diary. */
    let fight = fighting_in(&token, &character_id);
    let mut row = json!({
        "game_id": who.game_id,
        "character_id": character_id,
        "key": "move",
        "cost": "move",
        "request": "move",
        "label": format!("moved {} squares", went.cost),
        "status": "done",
        "squares": went.cost,
    });
    if let Some((actor_id, encounter_id)) = &fight {
        row["actor_id"] = json!(actor_id);
        row["encounter_id"] = json!(encounter_id);
    }
    let _ = supabase::rest_insert(&token, "actions", &json!([row]));

    // AND WHAT THAT MAKES THE ROUND'S TOTAL. Summed from the rows, not
    // from a counter - see 253. Only inside a fight, because outside
    // one there is no round to sum within.
    let moved_this_round = fight
        .as_ref()
        .and_then(|(actor_id, _)| moved_so_far(&token, actor_id));

    Ok(Walk {
        name: who.name,
        x: went.reached.x,
        y: went.reached.y,
        z: went.reached.z,
        cost: went.cost,
        steps: went.steps,
        stopped: went.stopped,
        peril: went.peril.map(|p| match p {
            crate::stride::Peril::Drowning => "drowning".to_string(),
        }),
        moved_this_round,
    })
}

/// Every square this roster row has covered in the round it is in.
///
/// 253. SUMMED IN RUST FROM THE ROWS, rather than asked of PostgREST
/// as an aggregate. The sum is three lines here and a `select=sum(...)`
/// there would be a second place that knows what counts as a move -
/// and `this_round` in `spent.rs` is already the first.
///
/// THE ROUND COMES OFF THE ROWS THEMSELVES. Asking the encounter for
/// its current round and filtering on it would be correct and would
/// also be a second read; the actions carry the stamp, so the newest
/// one names the round being counted.
fn moved_so_far(token: &str, actor_id: &str) -> Option<i64> {
    let rows = supabase::rest_get(
        token,
        "actions",
        &[
            ("select", "round,squares"),
            ("actor_id", &format!("eq.{}", actor_id)),
            ("squares", "not.is.null"),
            ("order", "round.desc"),
        ],
    )
    .ok()?;
    let rows = rows.as_array()?;
    let round = rows.iter().find_map(|r| r.get("round").and_then(|v| v.as_i64()))?;
    Some(
        rows.iter()
            .filter(|r| r.get("round").and_then(|v| v.as_i64()) == Some(round))
            .filter_map(|r| r.get("squares").and_then(|v| v.as_i64()))
            .sum(),
    )
}

/* ============ 245. THE SHELF: SAVE AS, LIST, LOAD, DELETE ============

   243 built `map_versions` and recorded why it is a separate table
   rather than a column on `tiles`: nothing that READS a map changes at
   all. These four commands are the only things that touch it.

   **DRAWING IS NOT SAVING AND NEVER WAS.** Every click is an INSERT,
   so a map survives closing the app whether anybody pressed anything
   or not. There is deliberately no `save_map` here, because a button
   that writes nothing while looking like it wrote something is this
   project's named defect.

   THE RULES ARE IN `shelf.rs`. What a name may be, what happens to a
   door whose destination is gone, and which squares a load has to
   remove - three questions with wrong answers that look like ordinary
   answers. This file does the fetching. */

/// A version's columns, without the blob.
///
/// 245. THE SQUARES ARE NOT IN A LISTING. A fully painted
/// forty-by-forty level is about 160 kB of jsonb, so fetching ten
/// versions to draw a dropdown would move a megabyte and a half to
/// display ten names. The room's size comes from `width`/`height`,
/// which 243 made columns for exactly this.
///
/// HOW MANY SQUARES EACH VERSION HOLDS IS NOT HERE, and it would be
/// useful - "4 squares" against "400" tells you which version is the
/// real one at a glance. PostgREST cannot compute
/// `jsonb_array_length` in a `select`, so it wants either a generated
/// column or the whole blob. It is worth a generated column the day
/// somebody misses it; it is not worth a megabyte today.
const VERSION_COLUMNS: &str = "id,name,saved_at,width,height";

/// What is on the shelf.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    pub id: String,
    pub name: String,
    pub saved_at: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

/// What a save put away.
#[derive(Debug, Clone, Serialize)]
pub struct Saved {
    pub id: String,
    pub name: String,
    /// How many drawn squares went onto the shelf. Not how big the
    /// room is: a 20x20 room with four painted squares saves four,
    /// because the other 396 are the recipe and are saved as the
    /// recipe.
    pub squares: usize,
}

/// What a load actually did. 217: not what it was asked to do.
#[derive(Debug, Clone, Serialize)]
pub struct Loaded {
    pub name: String,
    pub written: usize,
    /// Squares that were on the map and are not in the version.
    pub removed: usize,
    /// Doors whose destination no longer exists. See
    /// [`crate::shelf::keep_link`].
    pub links_dropped: usize,
}

/// The whole of a version, blob included.
#[derive(Debug, Deserialize)]
struct Shelved {
    // No `id`: the caller already has it, and a field selected and
    // never read is a column asked for out of habit.
    location_id: String,
    name: String,
    width: Option<i32>,
    height: Option<i32>,
    #[serde(default)]
    recipe: Value,
    squares: Vec<Tile>,
}

/// A location's map as it is stored, verbatim.
#[derive(Debug, Deserialize)]
struct Room {
    width: Option<i32>,
    height: Option<i32>,
    #[serde(default)]
    recipe: Value,
}

/// One square, as a RESTORE writes it.
///
/// 245. THIS IS NOT [`row`], AND THE DIFFERENCE IS DAMAGE.
///
/// `row` is what PAINTING sends, and painting makes fresh terrain -
/// it omits `surface_hp` and `cover_hp` entirely, so the columns take
/// their null default and a newly painted wall is undamaged. Correct
/// there, and silently wrong here: Dave decided a version keeps
/// damage, and reusing `row` would have dropped every hit point on
/// the way back in while reporting a successful load.
///
/// The nulls are explicit because an upsert REPLACES the row. A square
/// that was damaged when the map was saved over and is undamaged in
/// the version must come back undamaged, which means saying null
/// rather than saying nothing.
fn restored(location_id: &str, t: &Tile) -> Value {
    json!({
        "location_id": location_id,
        "x": t.x,
        "y": t.y,
        "z": t.z,
        "surface": t.surface.trim(),
        "cover": t.cover.as_deref().map(str::trim).filter(|s| !s.is_empty()),
        "sub": t.sub.as_deref().map(str::trim).filter(|s| !s.is_empty()),
        "surface_hp": t.surface_hp,
        "cover_hp": t.cover_hp,
        "leads_to": t.leads_to.as_deref().map(str::trim).filter(|s| !s.is_empty()),
        "to_x": t.to_x,
        "to_y": t.to_y,
        "to_z": t.to_z,
        "facing": t.facing,
        "shape": t.shape.as_deref().map(str::trim).filter(|s| !s.is_empty()),
    })
}

/// Of these places, which still exist.
///
/// Asked in one request with `id=in.(...)` rather than fetching every
/// location, and asked only about the ids a version actually mentions.
///
/// **RLS MAKES "INVISIBLE" AND "GONE" THE SAME ANSWER.** A location in
/// a game this DM does not run is not returned, so a link to it reads
/// as dead and gets dropped. That is accepted rather than worked
/// around: `leads_to` points inside the DM's own world in every case
/// anybody has built, the cost of a wrong drop is one doorway needing
/// its destination set again, and the alternative is a load that fails
/// on a foreign key with a message about a constraint.
fn surviving(token: &str, ids: &HashSet<String>) -> Result<HashSet<String>, String> {
    if ids.is_empty() {
        return Ok(HashSet::new());
    }
    let list: Vec<&str> = ids.iter().map(String::as_str).collect();
    let rows = supabase::rest_get(
        token,
        "locations",
        &[
            ("select", "id"),
            ("id", &format!("in.({})", list.join(","))),
        ],
    )?;
    #[derive(Deserialize)]
    struct Id {
        id: String,
    }
    let found: Vec<Id> = serde_json::from_value(rows)
        .map_err(|e| format!("could not check where the doors go: {}", e))?;
    Ok(found.into_iter().map(|r| r.id).collect())
}

/// Put the whole map on the shelf under a name.
///
/// EVERY LEVEL, because 243 has no `z`: a dungeon is its floors, and
/// versions taken a level at a time can be restored out of step with
/// each other.
///
/// THE RECIPE GOES VERBATIM. 226 calls it the map's PROVENANCE - the
/// account of what produced the room, as authored - and it is
/// deliberately unconstrained, so parsing and re-emitting it could
/// quietly lose a key some later generator put there. It is copied,
/// not understood.
#[tauri::command]
pub fn save_map_as(
    state: State<AppState>,
    location_id: String,
    name: String,
) -> Result<Saved, String> {
    let name = crate::shelf::name_for(&name)?;
    let ctx = Ctx::of(&state)?;

    let rooms = supabase::rest_get(
        &ctx.token,
        "locations",
        &[
            ("select", "width,height,recipe"),
            ("id", &format!("eq.{}", location_id)),
        ],
    )?;
    let mut rooms: Vec<Room> =
        serde_json::from_value(rooms).map_err(|e| format!("could not read the room: {}", e))?;
    let room = rooms.pop().ok_or("no such place")?;

    let rows = supabase::rest_get(
        &ctx.token,
        "tiles",
        &[
            ("select", COLUMNS),
            ("location_id", &format!("eq.{}", location_id)),
            ("order", "z.asc,y.asc,x.asc"),
        ],
    )?;
    let squares: Vec<Tile> =
        serde_json::from_value(rows).map_err(|e| format!("could not read the map: {}", e))?;

    // A ROOM THAT IS NOTHING IS NOT A VERSION. An empty `squares` is
    // legal and meaningful - 243 says so - but only alongside a recipe
    // or some bounds. With none of the three there is no map here, and
    // saving one would put a row on the shelf that restores to the
    // same nothing.
    if squares.is_empty() && room.width.is_none() && room.height.is_none() && room.recipe.is_null()
    {
        return Err(
            "there is no map here to save - draw something, or describe the room first".to_string(),
        );
    }

    let back = supabase::rest_insert(
        &ctx.token,
        "map_versions",
        &json!([{
            "location_id": location_id,
            "name": name,
            "saved_by": ctx.profile_id,
            "width": room.width,
            "height": room.height,
            "recipe": room.recipe,
            "squares": squares,
        }]),
    )
    .map_err(|e| taken(e, &name))?;

    let id = back
        .get(0)
        .and_then(|r| r.get("id"))
        .and_then(Value::as_str)
        // 217 in its own shape: an insert RLS refused comes back as an
        // empty array with a 201, which is a success that saved nothing.
        .ok_or("nothing was saved - this is not your map to save")?
        .to_string();

    Ok(Saved {
        id,
        name,
        squares: squares.len(),
    })
}

/// What is on this location's shelf, newest first.
#[tauri::command]
pub fn list_map_versions(
    state: State<AppState>,
    location_id: String,
) -> Result<Vec<Version>, String> {
    let token = state.token()?;
    let rows = supabase::rest_get(
        &token,
        "map_versions",
        &[
            ("select", VERSION_COLUMNS),
            ("location_id", &format!("eq.{}", location_id)),
            ("order", "saved_at.desc"),
        ],
    )?;
    serde_json::from_value(rows).map_err(|e| format!("could not read the shelf: {}", e))
}

/// Put a saved map back, replacing what is drawn now.
///
/// THE LOCATION IS NOT A PARAMETER. It comes off the version's own
/// row, so there is no way to ask for a version to be loaded somewhere
/// it did not come from. Copying a map into a DIFFERENT place is a
/// separate thing, with its own problem - every `leads_to` would point
/// at doors belonging to the original - and Dave settled that it is
/// not this.
///
/// ---------------------------------------------------------------------
/// THE ORDER OF THE THREE WRITES, WHICH IS THE WHOLE DESIGN
/// ---------------------------------------------------------------------
///
/// PostgREST gives no transaction, so three requests can stop after
/// one or two and **every possible stopping point has to leave a state
/// a DM can see and recover from.** That rules the obvious order out:
/// delete the map, then write the version. Stop in the middle of that
/// and the map is EMPTY.
///
/// So:
///
///   1. the room - width, height, recipe. The cheapest write and the
///      least destructive. Stop here and the squares are the old
///      ones against the new room's size, which 232 draws with a red
///      border because it asked for a square outside the room to look
///      WRONG rather than vanish.
///   2. the version's squares, upserted. Nothing is deleted, so stop
///      here and the map is the version PLUS leftovers - every square
///      the version names is already correct.
///   3. the surplus - what was on the map and is not in the version.
///      [`crate::shelf::surplus`] computes it; stop here and you are
///      in state 2, which is where a second Load puts you right.
///
/// **NO BOUNDS CHECK ON THE WAY IN.** `paint_tiles` refuses a square
/// outside the declared room, and a restore must not: 232 decided
/// that shrinking a map does not delete what falls outside it, so a
/// version can legitimately carry squares the room no longer contains.
/// Refusing them here would make a version unloadable because of a
/// decision taken after it was saved.
#[tauri::command]
pub fn load_map_version(state: State<AppState>, version_id: String) -> Result<Loaded, String> {
    let token = state.token()?;

    let rows = supabase::rest_get(
        &token,
        "map_versions",
        &[
            ("select", "location_id,name,width,height,recipe,squares"),
            ("id", &format!("eq.{}", version_id)),
        ],
    )?;
    let mut found: Vec<Shelved> = serde_json::from_value(rows)
        .map_err(|e| format!("could not read that saved map: {}", e))?;
    let version = found.pop().ok_or("no such saved map")?;
    let location_id = version.location_id;

    // The doors first, because dropping one changes what gets written.
    let wanted: HashSet<String> = version
        .squares
        .iter()
        .filter_map(|t| t.leads_to.clone())
        .collect();
    let live = surviving(&token, &wanted)?;
    let mut squares = version.squares;
    let mut links_dropped = 0usize;
    for t in squares.iter_mut() {
        if t.leads_to.is_some()
            && crate::shelf::keep_link(t.leads_to.as_deref(), &live).is_none()
        {
            t.leads_to = None;
            t.to_x = None;
            t.to_y = None;
            t.to_z = None;
            links_dropped += 1;
        }
    }

    // What is there now, so the surplus can be worked out before
    // anything is written.
    let now = supabase::rest_get(
        &token,
        "tiles",
        &[
            ("select", "x,y,z"),
            ("location_id", &format!("eq.{}", location_id)),
            ("order", "z.asc,y.asc,x.asc"),
        ],
    )?;
    let now: Vec<Spot> =
        serde_json::from_value(now).map_err(|e| format!("could not read the map: {}", e))?;
    let current: Vec<crate::shelf::At> = now.iter().map(|s| (s.x, s.y, s.z)).collect();
    let coming: Vec<crate::shelf::At> = squares.iter().map(|t| (t.x, t.y, t.z)).collect();
    let surplus = crate::shelf::surplus(&current, &coming);

    /* ---- 1. the room ---- */
    let back = supabase::rest_update(
        &token,
        "locations",
        &[("id", &format!("eq.{}", location_id))],
        &json!({
            "width": version.width,
            "height": version.height,
            "recipe": version.recipe,
        }),
    )
    .map_err(|e| denied(e, "load a saved map"))?;
    if back.as_array().map(Vec::len).unwrap_or(0) == 0 {
        return Err("nothing was loaded - that map is not yours to change".to_string());
    }

    /* ---- 2. the squares ---- */
    let mut written = 0usize;
    for part in squares.chunks(CHUNK) {
        let body: Vec<Value> = part.iter().map(|t| restored(&location_id, t)).collect();
        let back = supabase::rest_upsert(&token, "tiles", &json!(body), "location_id,z,y,x")
            .map_err(|e| halfway(e, written, "written"))?;
        written += back.as_array().map(Vec::len).unwrap_or(0);
    }
    if written != squares.len() {
        return Err(format!(
            "only {} of {} squares were written - the room was resized and the rest refused, \
             so load it again",
            written,
            squares.len()
        ));
    }

    /* ---- 3. the surplus ---- */
    let mut removed = 0usize;
    for part in surplus.chunks(CHUNK) {
        let clauses: Vec<String> = part
            .iter()
            .map(|(x, y, z)| format!("and(x.eq.{},y.eq.{},z.eq.{})", x, y, z))
            .collect();
        let or = format!("({})", clauses.join(","));
        removed += supabase::rest_delete_counted(
            &token,
            "tiles",
            &[("location_id", &format!("eq.{}", location_id)), ("or", &or)],
        )
        .map_err(|e| halfway(e, removed, "removed"))?;
    }

    Ok(Loaded {
        name: version.name,
        written,
        removed,
        links_dropped,
    })
}

/// Take a version off the shelf.
#[tauri::command]
pub fn delete_map_version(state: State<AppState>, version_id: String) -> Result<usize, String> {
    let token = state.token()?;
    let gone = supabase::rest_delete_counted(
        &token,
        "map_versions",
        &[("id", &format!("eq.{}", version_id))],
    )
    .map_err(|e| denied(e, "delete a saved map"))?;
    match gone {
        0 => Err("nothing was deleted - that saved map is not yours, or is already gone"
            .to_string()),
        n => Ok(n),
    }
}

/// A name already on this shelf, said as a sentence.
///
/// 243 made `(location_id, name)` unique so that "Save as" over an
/// existing name is REFUSED rather than quietly replacing the thing
/// you were keeping. The constraint does the refusing; this does the
/// explaining, because `one_name_one_version` is not a sentence.
fn taken(e: String, name: &str) -> String {
    if e.contains("one_name_one_version") || e.contains("23505") {
        format!(
            "there is already a saved map called \"{}\" here - delete it first, \
             or save under another name",
            name
        )
    } else {
        denied(e, "save a map")
    }
}

/// A write that stopped part way through, said so the DM knows what
/// state the map is in.
///
/// 217 is about a write that did nothing reporting success. This is
/// its larger cousin: a write that did SOME of it, where the honest
/// report has to name where it got to, because "it failed" would
/// leave a DM guessing whether to load again or start over.
///
/// Loading again is always the right answer - see the ordering note on
/// [`load_map_version`] - so it says so.
fn halfway(e: String, done: usize, verb: &str) -> String {
    format!(
        "the load stopped after {} squares were {} - the map is part old and part new. \
         Load the same saved map again to finish it. ({})",
        done, verb, e
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /* ===== 258. ONE LAYER, STORED BESIDE DERIVED ===== */
    //
    // `inspect_square` itself is plumbing - a token, a filtered read,
    // a struct - and this project does not test plumbing. `layer_detail`
    // is not: it decides what an absent hit points column MEANS, which
    // is a rule wearing a helper's clothes.

    use crate::terrain::Terrain;

    #[test]
    fn an_undamaged_thing_reads_as_its_maximum() {
        // NULL IS NOT ZERO. The column is empty because nobody has hurt
        // this door, so the verdict must be asked about a whole one.
        // Read the other way, every untouched door in the world would
        // be `destroyed`.
        let l = layer_detail("cover", Terrain::DoorClosed, None);
        assert_eq!(l.stored_hp, None, "nothing was written and nothing is invented");
        assert_eq!(l.hp_now, Some(25));
        assert_eq!(l.hp_max, Some(25));
        assert_eq!(l.wear, Some("intact"));
    }

    #[test]
    fn a_door_at_the_jamming_point_reads_broken() {
        // 25 / 10 is 2, and `breaks_at` is the only place that division
        // happens.
        let l = layer_detail("cover", Terrain::DoorClosed, Some(2));
        assert_eq!(l.breaks_at, Some(2));
        assert_eq!(l.wear, Some("broken"));
        assert!(l.mechanism);

        // One point above it is still a working door.
        let ok = layer_detail("cover", Terrain::DoorClosed, Some(3));
        assert_eq!(ok.wear, Some("intact"));
    }

    #[test]
    fn a_wall_is_never_broken_only_destroyed() {
        // A wall at a tenth of its hit points is a BREACH, which is
        // passable. Only a mechanism jams.
        let l = layer_detail("cover", Terrain::StoneWall, Some(1));
        assert!(!l.mechanism);
        assert_eq!(l.breaks_at, None);
        assert_eq!(l.wear, Some("intact"));
        assert_eq!(
            layer_detail("cover", Terrain::StoneWall, Some(0)).wear,
            Some("destroyed")
        );
    }

    #[test]
    fn a_word_nothing_can_hurt_offers_no_numbers_at_all() {
        // NOT ZEROES. A floor with "AC 0, 0 / 0 hp" would be a panel
        // inventing a fight nobody can have.
        // 264. DIRT CAN BE DUG NOW, so the word with no numbers at all
        // has to be one that is genuinely scenery.
        let l = layer_detail("cover", Terrain::Grass, None);
        assert_eq!(l.ac, None);
        assert_eq!(l.hp_now, None);
        assert_eq!(l.hp_max, None);
        assert_eq!(l.wear, None);
        assert_eq!(l.destruction, "cannot be attacked");
    }

    #[test]
    fn destruction_is_said_in_words_including_what_is_left() {
        // 265. BOTH HALVES. A wall clears AND leaves debris; a door
        // clears and leaves nothing, and the sentence has to tell them
        // apart or the panel is describing half of what will happen.
        assert_eq!(
            layer_detail("cover", Terrain::StoneWall, None).destruction,
            "clears, leaving rubble"
        );
        assert_eq!(
            layer_detail("cover", Terrain::DoorClosed, None).destruction,
            "clears"
        );
        // A dug floor is the case the pair exists for: it opens AND
        // piles the diggings up.
        assert_eq!(
            layer_detail("surface", Terrain::Dirt, None).destruction,
            "reveals what is beneath, leaving rubble_earth"
        );
    }

    /* ===== 263. WHAT A BLOW LEAVES IN THE OTHER COLUMN ===== */

    #[test]
    fn breaching_a_wall_does_not_heal_the_floor_under_it() {
        // THE FAULT THIS FUNCTION WAS EXTRACTED TO FIX. Damaged ice,
        // a stone wall standing on it, the wall destroyed. The ice is
        // not party to that and keeps its wounds.
        let before = Stack::of(Terrain::Ice).under(Terrain::StoneWall);
        let after = before.destroyed(Layer::Cover);
        assert_eq!(after.cover, None, "271: the pour places the heap, not this");

        let (s, c) = columns_after(&before, &after, Layer::Cover, Some(5), Some(0), None);
        assert_eq!(s, Some(5), "the ice was damaged and nothing touched it");
        assert_eq!(c, None, "there is nothing up there to hold a number");

        // AND STILL NULL ONCE THE POUR HAS PLACED THE HEAP: rubble is a
        // new thing and starts whole, which is the fault this function
        // was extracted to fix and is unaffected by 271.
        let settled = Stack {
            cover: Some(Terrain::Rubble),
            ..after
        };
        let (s, c) = columns_after(&before, &settled, Layer::Cover, Some(5), Some(0), None);
        assert_eq!(s, Some(5), "the ice still keeps its wounds");
        assert_eq!(c, None, "rubble is a new thing and starts whole");
    }

    #[test]
    fn a_lid_coming_off_takes_both_columns_with_it() {
        // Dave's square. The surface becomes water and the snow is
        // gone, so neither number describes anything that still exists.
        let before = Stack::of(Terrain::Ice)
            .over(Terrain::DeepWater)
            .under(Terrain::Snow);
        let after = before.destroyed(Layer::Surface);
        let (s, c) = columns_after(&before, &after, Layer::Surface, Some(3), None, None);
        assert_eq!(s, None);
        assert_eq!(c, None);
    }

    #[test]
    fn a_blow_that_does_not_finish_the_job_writes_only_its_own_column() {
        let st = Stack::of(Terrain::StoneFloor).under(Terrain::DoorClosed);
        let (s, c) = columns_after(&st, &st, Layer::Cover, None, None, Some(12));
        assert_eq!(s, None, "an untouched floor stays untouched");
        assert_eq!(c, Some(12));
    }

    #[test]
    fn an_empty_cover_cannot_carry_a_number() {
        // `only_a_cover_can_be_damaged` in the migration refuses this
        // at the database. It should never get that far.
        let before = Stack::of(Terrain::Doorway).under(Terrain::DoorClosed);
        let after = before.destroyed(Layer::Cover);
        assert_eq!(after.cover, None, "a door clears");
        let (_, c) = columns_after(&before, &after, Layer::Cover, None, Some(2), None);
        assert_eq!(c, None);
    }

    #[test]
    fn painting_door_broken_does_not_make_a_broken_door() {
        // THIS TEST PINS A KNOWN FAULT RATHER THAN A BEHAVIOUR WE WANT.
        //
        // `door_broken` is a terrain word a DM can paint; `Wear::Broken`
        // is a verdict derived from hit points. Nothing connects them,
        // so a freshly painted broken door is mechanically pristine -
        // the word says broken and the rules say fine.
        //
        // The panel puts those two lines next to each other, which is
        // how it was found. When a later cycle makes painting stamp the
        // condition, THIS TEST SHOULD FAIL - and that failure is the
        // signal to delete it, not to work around it.
        let l = layer_detail("cover", Terrain::DoorBroken, None);
        assert_eq!(l.hp_now, Some(25));
        assert_eq!(
            l.wear,
            Some("intact"),
            "if this now says broken, painting has started stamping the \
             condition and this test has done its job"
        );
    }

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
            facing: 0,
            shape: None,
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
    fn a_turn_on_something_symmetrical_is_refused() {
        // 238. A quarter turn on a rock is noise somebody will later
        // read as meaningful.
        let mut r = t("rock");
        r.facing = 1;
        let e = r.stack().unwrap_err();
        assert!(e.contains("same whichever way"), "{}", e);
        assert!(e.contains("(3, 4, 0)"), "{}", e);
    }

    #[test]
    fn a_turn_on_something_that_faces_is_kept() {
        let mut s = t("stairs");
        s.facing = 3;
        assert_eq!(s.stack().unwrap().facing, Facing::West);
    }

    #[test]
    fn a_door_may_turn_even_though_its_doorway_is_the_surface() {
        // The cover is what faces here, and either layer should allow
        // the turn.
        let mut d = t("doorway");
        d.cover = Some("door_closed".into());
        d.facing = 2;
        assert_eq!(d.stack().unwrap().facing, Facing::South);
    }

    #[test]
    fn a_facing_outside_the_quarter_turns_is_refused() {
        let mut bad = t("stairs");
        bad.facing = 4;
        let e = bad.stack().unwrap_err();
        assert!(e.contains("0, 1, 2 or 3"), "{}", e);
    }

    #[test]
    fn a_recipe_cannot_declare_a_facing() {
        // "This room is stairs climbing north by default" is not a
        // thing anybody means.
        let r = recipe_of(&json!({"surface": "stairs", "facing": 2}))
            .unwrap()
            .unwrap();
        assert_eq!(r.facing, Facing::North);
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
    fn a_position_is_three_facts_or_none() {
        // The constraint says this too; this says it as a sentence.
        let all = [true, true, true];
        let none = [false, false, false];
        let part = [true, false, true];
        let whole = |g: [bool; 3]| g.iter().any(|x| *x) && !g.iter().all(|x| *x);
        assert!(!whole(all));
        assert!(!whole(none));
        assert!(whole(part), "two of three must be refused");
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
    fn an_unmeasured_map_takes_anything() {
        // Null is unbounded, not zero - which is every map today.
        let e = Extent { width: None, height: None };
        let mut far = t("dirt");
        far.x = 999999;
        assert!(inside(e, &[far]).is_ok());
    }

    #[test]
    fn a_square_off_the_east_edge_is_refused_with_its_coordinates() {
        let e = Extent { width: Some(40), height: Some(30) };
        let mut off = t("dirt");
        off.x = 40;
        let err = inside(e, &[off]).unwrap_err();
        assert!(err.contains("x 40 is outside"), "{}", err);
        assert!(err.contains("40 squares across"), "{}", err);
    }

    #[test]
    fn the_last_square_is_inside_and_the_next_one_is_not() {
        // 0..width-1, so 39 is the far edge of a 40-wide room.
        let e = Extent { width: Some(40), height: Some(30) };
        let mut edge = t("dirt");
        edge.x = 39;
        edge.y = 29;
        assert!(inside(e, std::slice::from_ref(&edge)).is_ok());
        edge.y = 30;
        assert!(inside(e, std::slice::from_ref(&edge)).is_err());
    }

    #[test]
    fn a_negative_coordinate_is_outside_a_measured_map() {
        // (0,0) is the corner, so negatives are out of bounds rather
        // than merely unusual.
        let e = Extent { width: Some(10), height: Some(10) };
        let mut neg = t("dirt");
        neg.x = -1;
        assert!(inside(e, &[neg]).is_err());
    }

    #[test]
    fn a_width_without_a_height_bounds_only_one_axis() {
        // Odd, legal, and the trigger checks the two separately too.
        let e = Extent { width: Some(5), height: None };
        let mut tall = t("dirt");
        tall.x = 2;
        tall.y = 900;
        assert!(inside(e, std::slice::from_ref(&tall)).is_ok());
        tall.x = 5;
        assert!(inside(e, std::slice::from_ref(&tall)).is_err());
    }

    /* ---------------- 234. the recipe ---------------- */

    #[test]
    fn the_picker_gets_every_word_and_its_layers() {
        // The guard against a dropdown growing its own copy.
        let w = terrain_words();
        assert_eq!(w.len(), crate::terrain::ALL.len());

        let floor = w.iter().find(|x| x.key == "stone_floor").unwrap();
        assert!(floor.surface && !floor.cover);

        let grass = w.iter().find(|x| x.key == "grass").unwrap();
        assert!(grass.cover && !grass.surface);

        // 263. NOTHING IS LEGAL IN BOTH ANY MORE. This asserted FIVE -
        // the walls - until applying a destruction showed that a wall
        // used as a floor becomes rubble, and rubble is not a floor.
        let both = w.iter().filter(|x| x.surface && x.cover).count();
        assert_eq!(both, 0, "a word is either ground or it stands on it");

        // 261. AND THE SUB LAYER IS CARRIED RATHER THAN INFERRED.
        //
        // `may()` gives `Sub` and `Surface` the same list, so this
        // assertion is a tautology TODAY and says so plainly. It is
        // here for the day the two layers diverge: the picker asks for
        // `sub` and would follow a change silently, and this is the
        // line that makes the change visible instead.
        for x in &w {
            assert_eq!(x.sub, x.surface, "{} - the two ground layers have                  split, which is fine; the pickers are already asking                  the right question, so update this test", x.key);
        }
        assert!(w.iter().find(|x| x.key == "deep_water").unwrap().sub);
        assert!(!w.iter().find(|x| x.key == "grass").unwrap().sub);
    }

    #[test]
    fn a_recipe_is_the_same_three_words_a_tile_carries() {
        let r = recipe_of(&json!({"surface": "stone_floor", "cover": null, "sub": null}))
            .unwrap()
            .unwrap();
        assert_eq!(r.surface, Terrain::StoneFloor);
        assert_eq!(r.cover, None);
    }

    #[test]
    fn a_recipe_may_declare_a_cover_too() {
        // A snowed-over field is a legitimate default.
        let r = recipe_of(&json!({"surface": "dirt", "cover": "snow"}))
            .unwrap()
            .unwrap();
        assert_eq!(r.cover, Some(Terrain::Snow));
    }

    #[test]
    fn a_recipe_cannot_say_what_a_tile_could_not() {
        // Stack::legal does the judging, so the two can never disagree.
        let e = recipe_of(&json!({"surface": "grass"})).unwrap_err();
        assert!(e.contains("cannot be a surface"), "{}", e);
        assert!(e.contains("the recipe is wrong"), "{}", e);
        assert!(!e.contains("(0, 0, 0)"), "it should not name a square: {}", e);
    }

    #[test]
    fn a_recipe_with_no_surface_declares_no_ground() {
        // It may still carry a seed or a palette, which is not this
        // function's business.
        assert_eq!(recipe_of(&json!({"seed": 42})).unwrap(), None);
        assert_eq!(recipe_of(&json!(null)).unwrap(), None);
        assert_eq!(recipe_of(&json!({"surface": "  "})).unwrap(), None);
    }

    #[test]
    fn a_recipe_keeps_whatever_else_it_carries() {
        // Reads the three keys it knows rather than refusing a
        // document it did not write.
        let r = recipe_of(&json!({"surface": "rock", "seed": 7, "lighting": "dim"}))
            .unwrap()
            .unwrap();
        assert_eq!(r.surface, Terrain::Rock);
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
