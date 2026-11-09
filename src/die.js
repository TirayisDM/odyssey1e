// 290/299. THE SHAPE OF THE SOLID.
//
// The only file that knows what an icosahedron IS. It touches no DOM,
// reads no state and imports nothing - it is `Math`, arrays, and the
// twelve coordinates an icosahedron is made of. `main.js` turns what it
// returns into twenty divs and a CSS transform.
//
// 299 LIFTED IT OUT OF `main.js` SO SOMETHING COULD ASSERT IT, which
// is the same reason 235 lifted the renderer into `map.js`: arithmetic
// buried inside a painter is arithmetic nothing can ask a question of.
// `tests/die.smoke.mjs` imports this under Node and needs **no stubs at
// all** - unlike `map.js`, which wants a fake canvas and a fake `Image`
// before it can be asked anything.
//
// ---------------------------------------------------------------------
// THE GEOMETRY IS DERIVED, NOT A TABLE OF ANGLES
// ---------------------------------------------------------------------
//
// The twelve vertices of an icosahedron are the cyclic permutations of
// (0, ±1, ±φ); a face is any three of them one edge apart; a face's
// normal is the direction of its centroid. **Twenty rotations copied
// off a web page would be twenty chances to typo a die that looks very
// nearly right** - and "very nearly right" is the failure mode a
// picture hides best.
//
// ONE MATRIX, USED TWICE. Each face gets an orthonormal basis whose z
// is its outward normal and whose -y points at one of its corners. That
// basis PLACES the face on the solid. Its TRANSPOSE - which for a
// rotation is its inverse - turns the solid until that face is square
// to the viewer with that corner up, which is how the number lands
// upright. No `acos`, no axis to normalise, no degenerate case at the
// poles and no sign to get backwards.
//
// `tests/die.smoke.mjs` asserts that claim as the thing it claims:
// `show(f) · place(f)` must be the identity.
//
// NO DEPENDENCIES AND NO BUILD STEP. Twenty divs, one transition and
// about fifty lines of arithmetic. three.js would light it better, and
// this project has no bundler, so it would mean vendoring a few hundred
// kilobytes into a desktop application for one die.
//
// WHAT IT DOES NOT DO. Advantage rolls two dice and the engine hands
// back only the one it kept; showing both means `faces` on
// `RollResult`, which is Rust and its own cycle. The damage dice are
// this same code with a different net - except the d10, which is a
// trapezohedron and not a Platonic solid at all.

const PHI = (1 + Math.sqrt(5)) / 2;

const sub = (a, b) => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const cross = (a, b) => [
  a[1] * b[2] - a[2] * b[1],
  a[2] * b[0] - a[0] * b[2],
  a[0] * b[1] - a[1] * b[0],
];
const mul = (a, k) => [a[0] * k, a[1] * k, a[2] * k];
const unit = (a) => mul(a, 1 / Math.hypot(a[0], a[1], a[2]));
const d2 = (a, b) => {
  const d = sub(a, b);
  return dot(d, d);
};

// TWELVE VERTICES: every cyclic permutation of (0, ±1, ±φ).
// The edge is 2 units long in these coordinates.
const verts = [];
for (const s of [1, -1])
  for (const t of [1, -1]) {
    verts.push([0, s, t * PHI]);
    verts.push([s, t * PHI, 0]);
    verts.push([t * PHI, 0, s]);
  }

// TWENTY FACES: every triple of vertices that are mutually one edge
// apart. Nothing else in the solid is 2 units away, so the test for a
// face is the same test as for an edge, three times.
const faces = [];
for (let i = 0; i < 12; i++)
  for (let j = i + 1; j < 12; j++)
    for (let k = j + 1; k < 12; k++) {
      if (Math.abs(d2(verts[i], verts[j]) - 4) > 1e-9) continue;
      if (Math.abs(d2(verts[j], verts[k]) - 4) > 1e-9) continue;
      if (Math.abs(d2(verts[i], verts[k]) - 4) > 1e-9) continue;
      const v0 = verts[i], v1 = verts[j], v2 = verts[k];
      const c = mul(
        [v0[0] + v1[0] + v2[0], v0[1] + v1[1] + v2[1], v0[2] + v1[2] + v2[2]],
        1 / 3
      );
      // The centroid points out of the face because the solid is
      // centred on the origin, so the outward normal is free.
      const n = unit(c);
      // Local -y is screen up, and it is aimed at a corner: that is
      // what makes the triangle stand on its base with a point at the
      // top. `mx` is then forced, and the basis is right-handed by the
      // identity (b × a) × b = a for unit perpendicular a, b - so the
      // digits are never mirrored.
      const my = mul(unit(sub(v0, c)), -1);
      const mx = unit(cross(my, n));
      // 299. `v` and `c` are carried so the face can be asked what it
      // IS rather than only how it is turned. `buildDie` ignores them.
      //
      // THEY ARE WHAT MAKES THE ORIENTATION CHECKABLE. A test can see
      // that a basis is orthonormal and right-handed without noticing
      // that every normal points the wrong way - negate them all and
      // those properties still hold, while the die renders inside out
      // because `backface-visibility` then culls the ten faces you
      // wanted. `dot(n, c) > 0` is the question that catches it, and
      // it needs the centroid.
      faces.push({ n, mx, my, num: 0, v: [i, j, k], c });
    }

// OPPOSITE FACES SUM TO 21, as they do on a die you can hold.
let next = 1;
for (const a of faces) {
  if (a.num) continue;
  const b = faces.find((f) => !f.num && f !== a && dot(a.n, f.n) < -0.999);
  a.num = next;
  b.num = 21 - next;
  next += 1;
}

export const EDGE = 2;
export const CIRCUM = Math.sqrt(1 + PHI * PHI);
export const INRAD = (PHI * PHI * EDGE) / (2 * Math.sqrt(3));

// CSS matrix3d is column-major, and the columns are where the local
// axes end up.
//
// SIX DECIMAL PLACES, which is the only reason the transpose identity
// in `tests/die.smoke.mjs` is checked to 1e-5 rather than to machine
// precision. The limit is this string format, not the arithmetic.
const m3 = (cx, cy, cz) =>
  "matrix3d(" +
  [cx[0], cx[1], cx[2], 0,
   cy[0], cy[1], cy[2], 0,
   cz[0], cz[1], cz[2], 0,
   0, 0, 0, 1]
    .map((x) => x.toFixed(6))
    .join(",") +
  ")";

export { faces, verts };

/** Put this face on the solid. */
export const place = (f) => m3(f.mx, f.my, f.n);

/**
 * Turn the solid until this face is the one you are looking at.
 * The transpose of `place`, written out as its rows.
 */
export const show = (f) =>
  m3(
    [f.mx[0], f.my[0], f.n[0]],
    [f.mx[1], f.my[1], f.n[1]],
    [f.mx[2], f.my[2], f.n[2]]
  );

/**
 * The box one face is clipped from, in pixels, for a die whose corners
 * sit on a sphere of diameter `sizePx`.
 *
 * 299 MOVED THIS OUT OF `buildDie` WITH THE REST. `ty` is the single
 * trickiest number in the whole die: a triangle's centroid is a sixth
 * of its height below the middle of the box it is clipped from, and the
 * centroid is the point the normal passes through - so the box has to
 * sit that much high. A typo there produces a die that is very nearly
 * right, which is exactly what nobody notices. Leaving it behind in a
 * DOM function would have defeated the extraction.
 */
export function boxFor(sizePx) {
  const scale = sizePx / (2 * CIRCUM);
  const w = EDGE * scale;              // the triangle's side
  const h = (w * Math.sqrt(3)) / 2;    // and its height
  return {
    w,
    h,
    tz: INRAD * scale,                 // centre to face, straight out
    ty: -h / 6,                        // see above
    fontPx: w * 0.26,
  };
}
