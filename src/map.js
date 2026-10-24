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
      onFail();
      return;
    }
    onOk(img);
  };
  img.onerror = onFail;
  img.src = src;
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
function tiles(word) {
  if (art.has(word)) return art.get(word);
  const found = [];
  art.set(word, found);

  const repaint = () => {
    if (last) draw(last.canvas, last.view);
  };
  // Walk the numbered ones in order and stop at the first gap, so the
  // probe is bounded by what actually exists.
  const probe = (n) => {
    if (n > VARIANTS) return;
    load(
      `${TILES}${word}${n}.png`,
      (img) => {
        found.push(img);
        repaint();
        probe(n + 1);
      },
      () => {}
    );
  };
  load(
    `${TILES}${word}.png`,
    (img) => {
      found.unshift(img);
      repaint();
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
function lay(ctx, word, x, y, facing, col, row) {
  const img = variant(tiles(word), col, row);
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

export function draw(canvas, view) {
  last = { canvas, view };
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
    lay(ctx, s.surface, x, y, s.facing, s.x, s.y);
    ctx.globalAlpha = 1;

    if (s.cover) {
      if (isSolid(s.cover)) {
        // A wall replaces the square.
        lay(ctx, s.cover, x, y, s.facing, s.x, s.y);
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
        lay(ctx, s.cover, x, y, s.facing, s.x, s.y);
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
  const drawnArt = [...art.values()].reduce((n, v) => n + v.length, 0);
  return (
    `${cols}x${rows} · ${view.squares.length} squares` +
    (implied ? ` (${implied} from the recipe)` : "") +
    (stray ? ` · ${stray} outside the room` : "") +
    ` · ${view.standing.length} standing` +
    (drawnArt ? ` · ${drawnArt} tile image${drawnArt === 1 ? "" : "s"}` : "")
  );
}
