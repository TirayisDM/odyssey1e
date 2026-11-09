// 299. IS THE DIE STILL AN ICOSAHEDRON?
//
// `src/die.js` derives the solid rather than listing twenty rotations
// copied off a web page, and 290 wrote down why: twenty angles typed by
// hand are twenty chances to make a die that looks **very nearly
// right**, which is the failure a picture hides best. 290 also wrote
// down the test, and this is it, five cycles later:
//
//   *"twenty faces, all numbered once, opposites summing to 21, every
//   basis right-handed - four assertions that would catch a typo
//   instantly, and the harness for frontend code is still a browser
//   and a pair of eyes."*
//
// **WHAT IT BUYS:** a typo in fifty lines of arithmetic fails here, in
// a second, instead of producing a die somebody has to notice is subtly
// wrong while looking at it spin.
//
// **WHAT IT DOES NOT BUY, and nothing mechanical could:** whether the
// die LOOKS right. Not the lighting, not the spin, not 292's colours,
// not whether the number is legible at 140px, not whether the stage
// clips it. That is eyes, and `npm run rig` is one command away.
//
// IT NEEDS NO STUBS AT ALL, which is the reason this file is short and
// `map.smoke.mjs` is not. `die.js` touches no DOM, reads no state and
// imports nothing: it is `Math`, arrays, and the twelve coordinates an
// icosahedron is made of.
//
// AND IT FOUND NOTHING. Every assertion below passed the first time it
// was run. Said out loud rather than dressed up: the value here is the
// NEXT edit to that file, not a bug caught today.
//
// `npm test`.

import assert from "node:assert/strict";
import {
  faces,
  verts,
  place,
  show,
  boxFor,
  EDGE,
  CIRCUM,
  INRAD,
} from "../src/die.js";

const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const cross = (a, b) => [
  a[1] * b[2] - a[2] * b[1],
  a[2] * b[0] - a[0] * b[2],
  a[0] * b[1] - a[1] * b[0],
];
const near = (x, y, tol = 1e-9) => Math.abs(x - y) <= tol;

/* ---- 1. twenty faces, out of twelve vertices ----
   Not a count for its own sake: the faces are found by asking which
   triples of vertices are mutually one edge apart, so if that test
   drifts this number is the first thing that moves. Twelve vertices of
   degree five give 12*5/3 = 20. */
assert.equal(faces.length, 20, `the solid has ${faces.length} faces, expected 20`);

// Every normal distinct, which is what says they are twenty DIFFERENT
// faces rather than one found twenty times.
const normals = new Set(faces.map((f) => f.n.map((x) => x.toFixed(9)).join()));
assert.equal(normals.size, 20, `only ${normals.size} distinct face normals`);

assert.equal(verts.length, 12, `the solid has ${verts.length} vertices, expected 12`);

// EVERY VERTEX IN EXACTLY FIVE FACES. This is the property that says
// "icosahedron" rather than "twenty triangles": five around each
// corner, 12 * 5 / 3 = 20. A face set that is twenty triangles of the
// right size but the wrong incidence fails here and nowhere else.
const degree = new Array(12).fill(0);
for (const f of faces) for (const i of f.v) degree[i] += 1;
assert.deepEqual(
  degree,
  new Array(12).fill(5),
  `vertex degrees are ${degree.join(",")}, expected five faces at every corner`
);

// Every edge in exactly two faces, which is the other half of the same
// statement: a solid, not a sheet.
const edges = new Map();
for (const f of faces) {
  for (const [a, b] of [[0, 1], [1, 2], [0, 2]]) {
    const k = [f.v[a], f.v[b]].sort((x, y) => x - y).join("-");
    edges.set(k, (edges.get(k) || 0) + 1);
  }
}
assert.equal(edges.size, 30, `the solid has ${edges.size} edges, expected 30`);
assert.ok(
  [...edges.values()].every((n) => n === 2),
  "an edge belongs to a number of faces other than two, so this is not a closed solid"
);

/* ---- 2. each number once ---- */
const nums = faces.map((f) => f.num).sort((a, b) => a - b);
assert.deepEqual(
  nums,
  Array.from({ length: 20 }, (_, i) => i + 1),
  "the faces are not numbered 1 to 20 exactly once each"
);

/* ---- 3. opposite faces sum to 21 ----
   As they do on a die you can hold. Every face must HAVE an opposite -
   a missing one would mean the numbering loop silently paired a face
   with itself or left one at 0. */
for (const a of faces) {
  const opp = faces.filter((f) => near(dot(a.n, f.n), -1, 1e-3));
  assert.equal(
    opp.length,
    1,
    `face ${a.num} has ${opp.length} opposite faces, expected exactly 1`
  );
  assert.equal(
    a.num + opp[0].num,
    21,
    `face ${a.num} is opposite ${opp[0].num}; they should sum to 21`
  );
}

/* ---- 4. every basis orthonormal and right-handed ----
   RIGHT-HANDED IS THE ONE THAT MATTERS TO A READER: a left-handed
   basis mirrors the digit, so the die would show a backwards 7 and
   nothing else would be wrong. `mx = unit(cross(my, n))` makes it so
   by the identity (b x a) x b = a; this is that identity, checked. */
for (const f of faces) {
  for (const [name, v] of [["n", f.n], ["mx", f.mx], ["my", f.my]]) {
    assert.ok(
      near(Math.hypot(...v), 1),
      `face ${f.num}: ${name} is not a unit vector`
    );
  }
  assert.ok(near(dot(f.mx, f.my), 0), `face ${f.num}: mx and my are not perpendicular`);
  assert.ok(near(dot(f.my, f.n), 0), `face ${f.num}: my and n are not perpendicular`);
  assert.ok(near(dot(f.mx, f.n), 0), `face ${f.num}: mx and n are not perpendicular`);
  assert.ok(
    near(dot(cross(f.mx, f.my), f.n), 1),
    `face ${f.num}: the basis is LEFT-handed, so its digit is mirrored`
  );

  // AND IT POINTS OUT. Orthonormal and right-handed both survive
  // negating every normal - `mx` flips with `n` and the triple product
  // is unchanged - while the die renders inside out, because
  // `backface-visibility` then culls the ten faces you wanted to see.
  // This is the question that notices, and it is why `die.js` carries
  // each face's centroid.
  assert.ok(
    dot(f.n, f.c) > 0,
    `face ${f.num}: the normal points INWARD, so the die is inside out`
  );
  // Exactly as far out as it should be: the normal is the unit vector
  // through the centroid, so projecting the centroid onto it gives the
  // inradius back.
  assert.ok(
    near(dot(f.n, f.c), INRAD, 1e-9),
    `face ${f.num}: the face plane is not at the inradius`
  );
  // And the three corners are on the circumsphere, on the outward side.
  for (const i of f.v) {
    assert.ok(near(Math.hypot(...verts[i]), CIRCUM), `face ${f.num}: a corner is off the circumsphere`);
    assert.ok(dot(f.n, verts[i]) > 0, `face ${f.num}: a corner is behind its own face`);
  }

  // SCREEN UP POINTS AT A CORNER, which is what stands the triangle on
  // its base with a point at the top and the digit the right way up.
  // Local -y is screen up, so one of the three corners must lie along
  // it.
  //
  // THIS IS THE LAST OF THE SILENT ONES. Drop the `-1` on `my` and the
  // basis is still orthonormal, still right-handed and still outward -
  // every other check here passes - while every digit on the die is
  // upside down over a triangle standing on its point. "Very nearly
  // right" is the whole failure mode `die.js` exists to avoid, and
  // this is the last shape of it the arithmetic can see.
  const up = [-f.my[0], -f.my[1], -f.my[2]];
  const atCorner = f.v.filter((i) => {
    const toCorner = [
      verts[i][0] - f.c[0],
      verts[i][1] - f.c[1],
      verts[i][2] - f.c[2],
    ];
    return near(dot(up, toCorner) / Math.hypot(...toCorner), 1);
  });
  // WHICH corner is not pinned down, on purpose. An equilateral
  // triangle's three corners are interchangeable, so aiming at any of
  // them stands the digit upright and the die is right either way.
  // `die.js` picks the first vertex because it has to pick something.
  assert.equal(
    atCorner.length,
    1,
    `face ${f.num}: screen-up points at ${atCorner.length} corners, ` +
      "expected exactly 1 - the digit is not upright"
  );
}

/* ---- 5. show(f) is place(f)'s inverse ----
   THE CLAIM `die.js` MAKES, TESTED AS THE THING IT CLAIMS: "one matrix,
   used twice - its transpose, which for a rotation is its inverse". A
   transpose written with an index or a sign wrong is exactly the
   failure that design exists to avoid, and it would show as a die that
   turns to nearly the right face.

   THE TOLERANCE IS 1e-5 AND THAT IS NOT FLOATING POINT. `m3` writes six
   decimal places because it is building a CSS string, so the round trip
   through `matrix3d(...)` is where the precision goes. Measured
   deviation is about 9e-7. */
const cols = (s) => {
  assert.ok(s.startsWith("matrix3d(") && s.endsWith(")"), "not a matrix3d string");
  const n = s.slice("matrix3d(".length, -1).split(",").map(Number);
  assert.equal(n.length, 16, "matrix3d takes 16 numbers");
  assert.ok(n.every(Number.isFinite), "matrix3d has a NaN in it");
  return [
    [n[0], n[1], n[2]],
    [n[4], n[5], n[6]],
    [n[8], n[9], n[10]],
  ];
};

// Column-major, so `m[c][r]`.
const matmul = (A, B) =>
  [0, 1, 2].map((c) =>
    [0, 1, 2].map((r) => A[0][r] * B[c][0] + A[1][r] * B[c][1] + A[2][r] * B[c][2])
  );

let worst = 0;
for (const f of faces) {
  const I = matmul(cols(show(f)), cols(place(f)));
  for (let c = 0; c < 3; c++) {
    for (let r = 0; r < 3; r++) {
      worst = Math.max(worst, Math.abs(I[c][r] - (c === r ? 1 : 0)));
    }
  }
}
assert.ok(
  worst < 1e-5,
  `show(f) . place(f) is not the identity - worst deviation ${worst.toExponential(2)}`
);

/* ---- 6. the radii are an icosahedron's ----
   Closed forms for edge a: circumradius a*sqrt(phi^2+1)/2, inradius
   phi^2*a/(2*sqrt(3)). Hard-coding the numbers would just be the same
   typo in two places, so these are written from phi. */
const PHI = (1 + Math.sqrt(5)) / 2;
assert.equal(EDGE, 2, "the coordinates give an edge of 2");
assert.ok(near(CIRCUM, (EDGE * Math.sqrt(PHI * PHI + 1)) / 2), "wrong circumradius");
assert.ok(near(INRAD, (PHI * PHI * EDGE) / (2 * Math.sqrt(3))), "wrong inradius");
// And a vertex really is that far out, which ties the radius to the
// coordinates rather than to the formula.
for (const f of faces) {
  assert.ok(near(Math.hypot(...f.n), 1), `face ${f.num}: normal not normalised`);
}
assert.ok(INRAD < CIRCUM, "the inradius must be inside the circumradius");

/* ---- 7. the box one face is clipped from ----
   `ty` is the trickiest number in the die: a triangle's centroid sits a
   sixth of its height below the middle of its bounding box, and the
   centroid is where the normal comes through - so the box has to ride
   that much high. Wrong, and every face is offset by a few pixels in a
   way that reads as "the die looks a bit odd". */
const SIZE = 140;
const box = boxFor(SIZE);
assert.ok(near(box.ty, -box.h / 6), "ty is not a sixth of the height, negative");
assert.ok(near(box.h, (box.w * Math.sqrt(3)) / 2), "the triangle is not equilateral");
assert.ok(near(box.w, EDGE * (SIZE / (2 * CIRCUM))), "the side is not the edge at scale");
assert.ok(near(box.tz, INRAD * (SIZE / (2 * CIRCUM))), "tz is not the inradius at scale");
// The face sits inside the sphere the corners are on, or the die pokes
// through itself.
assert.ok(box.tz < SIZE / 2, "the face is further out than the circumradius");
assert.ok(box.fontPx > 0 && box.fontPx < box.h, "the digit does not fit its face");
// Scaling is linear in the size, which is what lets the stage pick one.
const half = boxFor(SIZE / 2);
assert.ok(near(half.w, box.w / 2) && near(half.tz, box.tz / 2), "boxFor is not linear");

console.log(
  `die.js: 20 faces, 1-20 once, opposites sum to 21, every basis ` +
    `right-handed, show . place = I to ${worst.toExponential(1)}.`
);
