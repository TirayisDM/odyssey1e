// 235. DRAWING A MAP.
//
// The only file that knows what anything LOOKS like. Everything about
// what a square IS - a row or the room's default, whether it blocks,
// whether it is outside the walls - was decided in Rust by `map_at`,
// which resolves `ground::at` once so the picture and `stride::step`
// cannot disagree about a wall.
//
// THIS FILE IS THEREFORE REPLACEABLE, which is the point. MAP.md
// section 8 expects a Three.js renderer if the voxel idea ever
// happens, and section 6 insisted the reference engine's CSS renderer
// be lifted out of `tile.rs` for the same reason: a renderer tangled
// into the rules is one you cannot replace.
//
// ---------------------------------------------------------------------
// CANVAS, NOT DIVS, AND NOT WEBGL
// ---------------------------------------------------------------------
//
// A tactical map is NOT A GAME LOOP. It redraws when something changes
// - a creature moves, a door opens - a few times a second at most,
// against a worst case of perhaps ten thousand squares. Canvas is
// comfortable at fifty thousand draws a frame, so this uses about two
// per cent of it, and WebGL would be a GPU pipeline for a problem that
// is not GPU-bound.
//
// `imageSmoothingEnabled = false` is set ready for the 16x16 tiles in
// `G:\My Drive\osdungeon\art`, which MAP.md records as having never
// been drawn by any generation. Flat colours first: the model should
// be proved readable before anything is pretty.

/// How many screen pixels one five-foot square is.
///
/// 236. THIRTY-TWO, TO MATCH THE ART ONE FOR ONE. A tile drawn at 32
/// and shown at 32 is not scaled at all, which is the sharpest result
/// there is. The requirement was never the NUMBER 16 - it was that the
/// scale be a WHOLE NUMBER, because at 1.75 some source pixels become
/// two screen pixels and others become one, and pixel art falls apart
/// even with smoothing off.
const SQUARE = 32;

/* ---------------------------------------------------------------------
   236. TILE ART, LOADED LAZILY AND FAILING QUIETLY

   MAP.md records the failure this is shaped to avoid: the Apps Script
   prototype had a complete texture path - a table, a fetch, a
   blend-mode render - that NEVER FIRED, because no tile type was ever
   given a `texture` key. A pipeline that needs thirty-three drawings
   before it does anything is a pipeline that never runs.

   So: a terrain word is drawn as its PNG if one exists and as its flat
   colour if not. **Art arrives one tile at a time and nothing breaks.**

   EACH FILE IS ASKED FOR ONCE. A miss is remembered as a miss, so a
   terrain with no art does not re-request it on every redraw; a hit
   triggers one repaint, because the image arrives after the frame that
   wanted it.
   --------------------------------------------------------------------- */

const TILES = "assets/tiles/";
const art = new Map(); // key -> HTMLImageElement[] (empty = asked, absent)
let last = null; // the most recent draw, so a late image can repaint it

/* ---------------------------------------------------------------------
   242. A REDRAWN TILE THAT NOBODY CAN SEE

   Dave: "it seems to be using an old version of graphics". The files
   on disk were the newest ones. **The webview had cached them** - it
   asked for `grass1.png` once, at launch, and kept that answer for the
   rest of the session however many times the artist saved over it.

   The ordinary fix is a changing query string, and the honest token
   here is THE MOMENT THE CACHE WAS LAST EMPTIED, not a version number
   somebody has to remember to bump. A hand-maintained `?v=3` is one
   more thing that can be stale, and it would be stale in exactly the
   situation it exists for.

   This costs one disk read per tile per launch. These are local files
   of about two kilobytes.
   --------------------------------------------------------------------- */

let bust = Date.now();

/// Files that loaded and were the wrong size, by name.
///
/// 242. SAID IN THE PANE, NOT ONLY THE CONSOLE. `grass.png` has been
/// refused for being 58x57 since 240 and the only place that said so
/// was a `console.warn` in devtools, which is not open while somebody
/// is drawing tiles. A tile that silently does not appear is this
/// project's named defect: the map looks finished and is not.
const refused = new Set();

/// 255. Shapes asked for whose sprite does not exist, by name.
///
/// `shape.rs` stopped waiting for the art before admitting a word -
/// `center` and `diag` were asked for by name before either was drawn
/// - so the guard against a square quietly drawing the WRONG picture
/// moved here. A shaped wall with no shaped PNG falls back to the
/// plain one, which looks like an ordinary wall and is not what the
/// DM painted.
///
/// 257. EMPTIED AT THE START OF EVERY DRAW, which it was not, and the
/// readout lied about art that exists.
///
/// `tiles()` hands back its array IMMEDIATELY and fills it as the
/// images arrive, so the first frame misses every shaped sprite by
/// definition - the files had not loaded yet. Each load then calls
/// `repaintSoon()` and the next frame draws them correctly, but a name
/// added once stayed for the life of the session. Dave's map reported
/// `stone_wall_corner, stone_wall_diag` as having no art while both
/// were plainly drawn on the screen in front of him.
///
/// A warning that cries wolf is worse than no warning: the one time
/// it is right, it has already been learned to be ignored. Cleared
/// per frame, it says what THIS picture is missing.
const unshaped = new Set();

/// Throw away every image and ask for them all again.
///
/// 242. Called by "Reload art", because the loop while drawing a tile
/// is save, look, adjust - and restarting the app between each look is
/// a reason to stop adjusting.
export function forgetArt() {
  art.clear();
  refused.clear();
  unshaped.clear();
  bust = Date.now();
  if (last) draw(last.canvas, last.view);
}

/// 240. HOW MANY NUMBERED VARIANTS TO LOOK FOR BEYOND THE BASE.
///
/// Probing stops at the first miss, so a word with one tile costs two
/// requests and a word with none costs one. This only caps how far a
/// gap can be bridged: `grass1, grass2, grass4` would find three and
/// miss the fourth, which is a naming mistake worth noticing rather
/// than papering over.
const VARIANTS = 6;

/// Every square is 32x32. A tile that is not gets refused.
///
/// 240. A WRONG-SIZED TILE IS A SILENT BLUR. `grass.png` arrived at
/// 58x57 and canvas would have squashed it into the cell without
/// complaint - an ordinary-looking square that is quietly wrong, which
/// is this project's named defect. Better a missing tile and a flat
/// colour, which is obviously unfinished.
const EXPECTED = 32;

function load(src, onOk, onFail) {
  const img = new Image();
  img.onload = () => {
    if (img.naturalWidth !== EXPECTED || img.naturalHeight !== EXPECTED) {
      console.warn(
        `${src} is ${img.naturalWidth}x${img.naturalHeight}, not ${EXPECTED}x${EXPECTED} - not drawn`
      );
      refused.add(src.slice(TILES.length));
      onFail();
      return;
    }
    onOk(img);
  };
  img.onerror = onFail;
  // The token is appended here rather than built into the path so that
  // every request for a tile carries it, including the variant probes.
  img.src = `${src}?v=${bust}`;
}

/// The art for a word: its base tile and any numbered variants.
///
/// 240. VARIANTS SO A FLOOR DOES NOT VISIBLY REPEAT. `grass.png`,
/// `grass1.png`, `grass2.png` - whichever exist. Probing walks upward
/// and stops at the first miss, so nothing needs a manifest and a new
/// variant is a file, not a code change.
///
/// EACH FILE IS ASKED FOR ONCE. A miss is remembered, so a word with no
/// art does not re-request on every redraw; a hit repaints once,
/// because the image lands after the frame that wanted it.
/// 260. ONE REPAINT PER FRAME, however many images land in it.
///
/// A word can bring six files and a map can use a dozen words, so a
/// cold open used to redraw the whole map seventy-odd times in a
/// second - each one a `clearRect` followed by a fill. The picture was
/// correct at the end and visibly unstable on the way there.
///
/// COALESCED RATHER THAN THROTTLED: every arrival still causes a
/// redraw, it is just the same redraw. Nothing is dropped and nothing
/// is delayed past the next frame.
let frame = 0;
function repaintSoon() {
  if (frame) return;
  frame = requestAnimationFrame(() => {
    frame = 0;
    if (last) draw(last.canvas, last.view);
    // 258. AND THE PREVIEW, which is drawn from the same art and would
    // otherwise sit on the flat colour until something else moved. A
    // tile loading is exactly when a preview is wrong.
    if (lastSquare) drawSquareInto(lastSquare.canvas, lastSquare.square);
  });
}

function tiles(word) {
  if (art.has(word)) return art.get(word);
  const found = [];
  art.set(word, found);

  // Walk the numbered ones in order and stop at the first gap, so the
  // probe is bounded by what actually exists.
  const probe = (n) => {
    if (n > VARIANTS) return;
    load(
      `${TILES}${word}${n}.png`,
      (img) => {
        found.push(img);
        repaintSoon();
        probe(n + 1);
      },
      () => {}
    );
  };
  load(
    `${TILES}${word}.png`,
    (img) => {
      found.unshift(img);
      repaintSoon();
    },
    () => {}
  );
  probe(1);
  return found;
}

/// Which variant this square gets.
///
/// 240. DECIDED BY POSITION, NOT AT RANDOM. A square must look the
/// same on every redraw, or a floor would shimmer whenever a door
/// opened. Two primes so neighbours differ rather than striping.
function variant(list, x, y) {
  if (list.length < 2) return list[0];
  const i = (((x * 31 + y * 17) % list.length) + list.length) % list.length;
  return list[i];
}

// THE ONE PLACE COLOURS LIVE. Flat, and chosen to be told apart rather
// than to be handsome - this is a diagram until the tile art is wired.
const PAINT = {
  // ground
  rock: "#6b6b73",
  dirt: "#6b543c",
  mud: "#4f4030",
  sand: "#c2ab78",
  scree: "#8a8578",
  road: "#8c8275",
  stone_floor: "#56565e",
  wood_floor: "#7a5a37",
  marsh: "#4a5540",
  ice: "#b8d8e0",
  shallow_water: "#4a7f9e",
  deep_water: "#28527a",
  lava: "#c2451c",
  pit: "#17171b",
  ledge: "#5c5248",
  doorway: "#453b30",
  ramp: "#6f6354",
  stairs: "#7d7366",
  // covers
  grass: "#4e7a3c",
  snow: "#e8eef2",
  brush: "#44632f",
  trees: "#2f5223",
  vines: "#3d6b33",
  // 265. FOUR KINDS OF DEBRIS, and `rubble` is the stone one - it kept
  // its name because its art is already drawn and every row that says
  // `rubble` already means masonry. The other three have no art yet and
  // will show as flat colour, which is the renderer saying "unfinished"
  // out loud rather than guessing.
  rubble: "#7d7368",
  rubble_timber: "#6b5433",
  rubble_wood: "#8a7147",
  rubble_earth: "#6e5d45",
  door_open: "#8a6b3f",
  door_closed: "#8a6b3f",
  door_broken: "#6b4f2c",
  iron_grate: "#4a4a52",
  // walls
  stone_wall: "#3c3c44",
  timber_wall: "#5c4426",
  rock_wall: "#4a4a50",
  wooden_wall: "#4a3a22",
  ice_wall: "#8fb8c6",
};

const UNKNOWN = "#ff00ff"; // a word the palette forgot, shouting about it

const paint = (word) => PAINT[word] || UNKNOWN;

/// Which covers are drawn as SOLID rather than as a layer on the ground.
///
/// A wall REPLACES its square - you should not be able to see the
/// floor it stands on, because you cannot walk on it. Snow and grass
/// sit ON the ground and must not hide it.
const isSolid = (word) => word.endsWith("_wall") || word === "trees" || word.startsWith("door_");

/// The extent to draw: the declared room, plus anything outside it.
///
/// 232 decided that shrinking a map does not delete what falls outside
/// it - "a tile outside the declared room should look WRONG, not
/// vanish" - so the canvas has to be big enough to show both.
function extent(view) {
  let minX = 0;
  let minY = 0;
  let maxX = (view.width || 1) - 1;
  let maxY = (view.height || 1) - 1;
  if (!view.width || !view.height) {
    minX = Infinity;
    minY = Infinity;
    maxX = -Infinity;
    maxY = -Infinity;
  }
  for (const s of view.squares) {
    minX = Math.min(minX, s.x);
    minY = Math.min(minY, s.y);
    maxX = Math.max(maxX, s.x);
    maxY = Math.max(maxY, s.y);
  }
  if (!Number.isFinite(minX)) return null; // nothing to draw at all
  return { minX, minY, maxX, maxY };
}

/// Draw one terrain word into a square: its art if there is any, its
/// flat colour if not, turned by `facing` quarter turns.
///
/// 238. THE ART IS DRAWN ONCE, WITH THE TOP OF THE IMAGE AS THE
/// DIRECTION OF TRAVEL, and turned here. A stairway climbing east is
/// the same file as one climbing north.
///
/// A FLAT COLOUR IS NOT TURNED, because a rectangle has no top - and
/// doing the transform anyway would cost four canvas operations per
/// square on a map with no art at all.
function lay(ctx, word, x, y, facing, col, row, shape) {
  // 254. A SHAPE PICKS A DIFFERENT SPRITE OF THE SAME WORD.
  //
  // `stone_wall` + `corner` is `stone_wall_corner.png`. The shape is
  // a drawing and nothing else - the square blocks, breaks and costs
  // exactly what a plain wall does, which is why it is one column on
  // the row rather than five more terrain words.
  //
  // FALLING BACK TO THE PLAIN TILE when the shaped art does not
  // exist, the same way a missing tile falls back to a flat colour:
  // art arrives one piece at a time and nothing breaks. `shape.rs`
  // refuses a shape nobody has drawn, so this only catches the gap
  // between a word being allowed and its PNG arriving.
  const shaped = shape ? tiles(word + "_" + shape) : [];
  if (shape && !shaped.length) unshaped.add(word + "_" + shape);
  const img = variant(shaped.length ? shaped : tiles(word), col, row);
  if (!img) {
    ctx.fillStyle = paint(word);
    ctx.fillRect(x, y, SQUARE, SQUARE);
    return;
  }
  if (!facing) {
    ctx.drawImage(img, x, y, SQUARE, SQUARE);
    return;
  }
  // Turn about the square's own centre, then draw as if at the origin.
  const half = SQUARE / 2;
  ctx.save();
  ctx.translate(x + half, y + half);
  ctx.rotate((facing * Math.PI) / 2);
  ctx.drawImage(img, -half, -half, SQUARE, SQUARE);
  ctx.restore();
}

/// 258. THE ONE SQUARE THE TILE PANEL IS SHOWING.
///
/// Kept so that art arriving late refreshes the preview as well as the
/// map - see `repaintSoon` above.
let lastSquare = null;

/// Draw ONE square, composed exactly as the map composes it, into its
/// own canvas.
///
/// ---------------------------------------------------------------------
/// WHY THIS IS HERE AND NOT IN THE PANEL
/// ---------------------------------------------------------------------
///
/// The obvious panel code is `img.src = "assets/tiles/" + cover +
/// ".png"`, and it would be wrong four ways at once: it ignores the
/// shape suffix, it ignores numbered variants, it never turns the tile
/// by its facing, and it skips the 32x32 guard. It would also be the
/// filename rule in a SECOND place, which is the fault this project
/// keeps naming.
///
/// So the panel asks the renderer for a picture instead. The square it
/// passes carries its real `x` and `y`, so `variant()` hashes to the
/// SAME variant the map drew - a preview that showed a different
/// grass to the one on screen would be a small lie told confidently.
///
/// THE BUFFER STAYS 32x32. 236 chose art drawn at 32 and never
/// resampled; the panel makes it bigger with CSS and
/// `image-rendering: pixelated`, which is the browser doing an integer
/// nearest-neighbour rather than us smearing it.
export function drawSquareInto(canvas, square) {
  if (!canvas) return;
  lastSquare = square ? { canvas, square } : null;
  canvas.width = SQUARE;
  canvas.height = SQUARE;
  const ctx = canvas.getContext("2d");
  ctx.imageSmoothingEnabled = false;
  ctx.clearRect(0, 0, SQUARE, SQUARE);
  if (!square) return;

  // THE SAME ORDER AS `draw`, for the same reasons - a solid cover
  // replaces the square, anything else layers on the ground.
  lay(ctx, square.surface, 0, 0, square.facing, square.x, square.y, null);
  if (square.cover) {
    const solid = isSolid(square.cover);
    const has = tiles(square.cover).length > 0;
    ctx.globalAlpha = solid || has ? 1 : 0.6;
    lay(ctx, square.cover, 0, 0, square.facing, square.x, square.y, square.shape);
    ctx.globalAlpha = 1;
  }
}

/// Which square a point on the canvas is over, or null off the map.
///
/// 241. THE SAME `extent` THE DRAWING USES, so a click lands where the
/// eye says it does. Hit-testing a grid is arithmetic - `floor(px /
/// 32)` plus the origin - which is why there is no listener per
/// square and why a forty-by-forty room costs nothing to make
/// clickable.
export function squareAt(view, px, py) {
  const box = extent(view);
  if (!box) return null;
  const x = Math.floor(px / SQUARE) + box.minX;
  const y = Math.floor(py / SQUARE) + box.minY;
  if (x < box.minX || x > box.maxX || y < box.minY || y > box.maxY) return null;
  return { x, y };
}

export function draw(canvas, view) {
  last = { canvas, view };
  // 257. See the note on `unshaped`. This set describes the picture
  // about to be drawn, not every picture ever drawn.
  unshaped.clear();
  const ctx = canvas.getContext("2d");
  ctx.imageSmoothingEnabled = false;

  const box = extent(view);
  if (!box) {
    canvas.width = 1;
    canvas.height = 1;
    return "nothing is mapped on this level";
  }

  const cols = box.maxX - box.minX + 1;
  const rows = box.maxY - box.minY + 1;
  // 260. ONLY WHEN IT ACTUALLY CHANGED. Assigning `width` resizes the
  // element as well as clearing the bitmap, and a redraw follows every
  // paint, erase and walk - so the usual case is writing the same two
  // numbers back and making the browser lay the pane out again for
  // nothing. `clearRect` below does the clearing either way.
  if (canvas.width !== cols * SQUARE) canvas.width = cols * SQUARE;
  if (canvas.height !== rows * SQUARE) canvas.height = rows * SQUARE;

  const px = (x) => (x - box.minX) * SQUARE;
  const py = (y) => (y - box.minY) * SQUARE;

  ctx.clearRect(0, 0, canvas.width, canvas.height);

  for (const s of view.squares) {
    const x = px(s.x);
    const y = py(s.y);

    // THE GROUND. A square the room merely IMPLIES is drawn fainter
    // than one somebody wrote down, so that while a map is being
    // authored you can see what you actually authored.
    // 257. NO SHAPE ON THE GROUND. The shape is which piece of a WALL
    // this is, and 254 said so in the migration: *"the only shaped
    // thing on it is the wall."* Passing it here asked for
    // `dirt_corner.png` and `dirt_diag.png`, which nobody will ever
    // draw, and then named them as missing art.
    //
    // The FACING still goes to the ground, because a ramp climbs a
    // direction and stairs do. Only the shape is the wall's alone.
    ctx.globalAlpha = s.drawn ? 1 : 0.55;
    lay(ctx, s.surface, x, y, s.facing, s.x, s.y, null);
    ctx.globalAlpha = 1;

    if (s.cover) {
      if (isSolid(s.cover)) {
        // A wall replaces the square.
        lay(ctx, s.cover, x, y, s.facing, s.x, s.y, s.shape);
      } else {
        // Everything else is a layer ON the ground, so the ground
        // stays visible under it.
        //
        // A PNG COVER NEEDS NO ALPHA FROM US: the art carries its own
        // transparency, so snow drawn over a road shows the road
        // without being faded. The 60% is for a FLAT COLOUR, which is
        // an opaque rectangle and would hide what is beneath.
        const has = tiles(s.cover).length > 0;
        ctx.globalAlpha = has ? 1 : 0.6;
        lay(ctx, s.cover, x, y, s.facing, s.x, s.y, s.shape);
        ctx.globalAlpha = 1;
      }
    }

    // 258. THE SELECTED SQUARE, so the panel and the map agree about
    // which one is being talked about. Drawn in the square loop rather
    // than after it, because a creature token standing on the square
    // should sit ON the outline the way it sits on the terrain.
    if (view.selected && view.selected.x === s.x && view.selected.y === s.y) {
      ctx.strokeStyle = "#ffd24d";
      ctx.lineWidth = 2;
      ctx.strokeRect(x + 1, y + 1, SQUARE - 2, SQUARE - 2);
    }

    // A row the declared room does not contain. 232 asked for this to
    // look wrong.
    if (s.outside) {
      ctx.strokeStyle = "#ff4d4d";
      ctx.lineWidth = 2;
      ctx.strokeRect(x + 1, y + 1, SQUARE - 2, SQUARE - 2);
    }
  }

  // THE GRID LAST, over everything, so a wall does not swallow its own
  // edges and a reader can count squares.
  ctx.strokeStyle = "rgba(255,255,255,0.08)";
  ctx.lineWidth = 1;
  for (let c = 0; c <= cols; c++) {
    ctx.beginPath();
    ctx.moveTo(c * SQUARE + 0.5, 0);
    ctx.lineTo(c * SQUARE + 0.5, canvas.height);
    ctx.stroke();
  }
  for (let r = 0; r <= rows; r++) {
    ctx.beginPath();
    ctx.moveTo(0, r * SQUARE + 0.5);
    ctx.lineTo(canvas.width, r * SQUARE + 0.5);
    ctx.stroke();
  }

  // CREATURES ON TOP OF EVERYTHING, because a map you cannot find
  // yourself on is a decoration.
  for (const who of view.standing) {
    const x = px(who.x) + SQUARE / 2;
    const y = py(who.y) + SQUARE / 2;
    ctx.beginPath();
    ctx.arc(x, y, SQUARE * 0.33, 0, Math.PI * 2);
    ctx.fillStyle = "#f2e6c8";
    ctx.fill();
    ctx.strokeStyle = "#1b1b20";
    ctx.lineWidth = 2;
    ctx.stroke();

    ctx.fillStyle = "#1b1b20";
    ctx.font = `bold ${Math.round(SQUARE * 0.42)}px system-ui, sans-serif`;
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillText((who.name || "?").charAt(0).toUpperCase(), x, y + 1);
  }

  const implied = view.squares.filter((s) => !s.drawn).length;
  const stray = view.squares.filter((s) => s.outside).length;
  const drawnArt = [...art.values()].reduce((n, v) => n + v.length, 0);
  return (
    `${cols}x${rows} · ${view.squares.length} squares` +
    (implied ? ` (${implied} from the recipe)` : "") +
    (stray ? ` · ${stray} outside the room` : "") +
    ` · ${view.standing.length} standing` +
    (drawnArt ? ` · ${drawnArt} tile image${drawnArt === 1 ? "" : "s"}` : "") +
    // 242. THE REFUSED ONES BY NAME. A wrong-sized tile does not
    // appear, and without this the only clue is that one kind of
    // ground looks unfinished while the artist is sure it is not.
    (refused.size
      ? ` · NOT ${EXPECTED}x${EXPECTED}, so not drawn: ${[...refused].join(", ")}`
      : "") +
    // 255. A SHAPE WITH NO SPRITE, by name. Without this a `center`
    // wall draws as a plain one and looks finished.
    (unshaped.size
      ? ` · no art yet, drawn plain: ${[...unshaped].join(", ")}`
      : "")
  );
}
