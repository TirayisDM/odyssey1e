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
const SQUARE = 28;

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
  rubble: "#7d7368",
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

export function draw(canvas, view) {
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
  canvas.width = cols * SQUARE;
  canvas.height = rows * SQUARE;

  const px = (x) => (x - box.minX) * SQUARE;
  const py = (y) => (y - box.minY) * SQUARE;

  ctx.clearRect(0, 0, canvas.width, canvas.height);

  for (const s of view.squares) {
    const x = px(s.x);
    const y = py(s.y);

    // THE GROUND. A square the room merely IMPLIES is drawn fainter
    // than one somebody wrote down, so that while a map is being
    // authored you can see what you actually authored.
    ctx.globalAlpha = s.drawn ? 1 : 0.55;
    ctx.fillStyle = paint(s.surface);
    ctx.fillRect(x, y, SQUARE, SQUARE);
    ctx.globalAlpha = 1;

    if (s.cover) {
      if (isSolid(s.cover)) {
        // A wall replaces the square.
        ctx.fillStyle = paint(s.cover);
        ctx.fillRect(x, y, SQUARE, SQUARE);
      } else {
        // Everything else is a layer ON the ground, so the ground
        // stays visible under it.
        ctx.globalAlpha = 0.6;
        ctx.fillStyle = paint(s.cover);
        ctx.fillRect(x, y, SQUARE, SQUARE);
        ctx.globalAlpha = 1;
      }
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
  return (
    `${cols}x${rows} · ${view.squares.length} squares` +
    (implied ? ` (${implied} from the recipe)` : "") +
    (stray ? ` · ${stray} outside the room` : "") +
    ` · ${view.standing.length} standing`
  );
}
