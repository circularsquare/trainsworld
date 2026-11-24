// A bus takes the quicker road, not only the shorter (open-source issue 27): a trunk road counts at 1 / FAST_X
// of its length. Run with: npx vite-node src/map/roadQuickest.test.ts
import { FAST_X, metresPerUnit, RoadGraph, TRUNK, type Snap } from "./roadGraph";
import type { RoadLines } from "./roadTiles";

function assert(cond: unknown, msg: string): asserts cond {
  if (!cond) throw new Error(msg);
}

/** A street from (100, 500) to (900, 500), a stub street off each end where the bus starts and ends, and a trunk
 * road between the same two corners bowed out to (500, `bow`). */
function graph(bow: number): RoadGraph {
  const g = new RoadGraph();
  const lines = [[0, 500, 100, 500], [900, 500, 1000, 500], [100, 500, 900, 500], [100, 500, 500, bow, 900, 500]];
  const rl: RoadLines = { extent: 4096, lines: lines.map((l) => Int32Array.from(l)), weights: [1, 1, 1, TRUNK] };
  g.addTile(0, 0, rl);
  return g;
}
function snapOn(g: RoadGraph, edge: number, t: number): Snap {
  const a = g.ea[edge], b = g.eb[edge];
  return { edge, t, x: g.gx[a] + t * (g.gx[b] - g.gx[a]), y: g.gy[a] + t * (g.gy[b] - g.gy[a]), d: 0 };
}
const via = (pts: number[], x: number, y: number) => pts.some((_, i) => i % 2 === 0 && Math.abs(pts[i] - x) < 1 && Math.abs(pts[i + 1] - y) < 1);

// the trunk road 1.18 times as long as the street: quicker, so taken, its stretch marked fast
const near = graph(250);
const a = near.path(snapOn(near, 0, 0.5), snapOn(near, 1, 0.5), 1e6);
assert(a && via(a.pts, 500, 250), `the trunk road, ${a?.pts.join(",")}`);
assert(a.fast?.some(Boolean), "its stretch marked fast");
const long = 50 + 2 * Math.hypot(400, 250) + 50;
assert(Math.abs(a.m - long * metresPerUnit(500)) < 0.01 * a.m, `its length as it is (${a.m} m against ${long * metresPerUnit(500)} m)`);

// 1.6 times as long: past FAST_X, so the street
const far = graph(0);
assert(2 * Math.hypot(400, 500) / 800 > FAST_X, "the far bow is past FAST_X");
const b = far.path(snapOn(far, 0, 0.5), snapOn(far, 1, 0.5), 1e6);
assert(b && !via(b.pts, 500, 0) && !b.fast?.some(Boolean), `the street, ${b?.pts.join(",")}`);
assert(Math.abs(b.m - 900 * metresPerUnit(500)) < 0.01 * b.m, `the street's length (${b.m} m)`);

// a point on the trunk road itself: the part of it to either end counted at 1 / FAST_X too, so the route comes onto
// it from the end that is quicker, not the one that is shorter
{
  // a stub street down to X (1200, 1800), a short street from X to P (1000, 2000) and a long one to Q (3000, 2000),
  // and the trunk road from P to Q
  const g = new RoadGraph();
  const lines = [[1200, 1700, 1200, 1800], [1200, 1800, 1000, 2000], [1200, 1800, 3000, 2000], [1000, 2000, 3000, 2000]];
  g.addTile(0, 0, { extent: 4096, lines: lines.map((l) => Int32Array.from(l)), weights: [1, 1, 1, TRUNK] });
  const edgeAt = (x0: number, y0: number, x1: number, y1: number) => {
    for (let e = 0; e < g.edges; e++) {
      const a = g.ea[e], b = g.eb[e];
      const fits = (p: number, q: number) => g.gx[p] === x0 && g.gy[p] === y0 && g.gx[q] === x1 && g.gy[q] === y1;
      if (fits(a, b) || fits(b, a)) return e;
    }
    throw new Error(`no road from (${x0}, ${y0}) to (${x1}, ${y1})`);
  };
  const stub = edgeAt(1200, 1700, 1200, 1800), trunk = edgeAt(1000, 2000, 3000, 2000);
  const at = (x: number) => snapOn(g, trunk, (x - g.gx[g.ea[trunk]]) / (g.gx[g.eb[trunk]] - g.gx[g.ea[trunk]]));
  // at x = 2900: by P 283 of street and 1900 of trunk road, by Q 1811 and 100; shorter by Q, quicker by P
  const viaP = Math.hypot(200, 200) + 1900, viaQ = Math.hypot(1800, 200) + 100;
  assert(viaP > viaQ && Math.hypot(200, 200) + 1900 / FAST_X < Math.hypot(1800, 200) + 100 / FAST_X, "shorter by Q, quicker by P");
  const want = (50 + viaP) * metresPerUnit(2000);
  for (const [what, p] of [
    ["ending on it", g.path(snapOn(g, stub, 0.5), at(2900), 1e6)],
    ["starting on it", g.path(at(2900), snapOn(g, stub, 0.5), 1e6)],
  ] as const) {
    assert(p && via(p.pts, 1000, 2000) && !via(p.pts, 3000, 2000), `${what}: by P, ${p?.pts.join(",")}`);
    assert(Math.abs(p.m - want) < 0.01 * want, `${what}: its length as it is (${p.m} m against ${want} m)`);
  }
}

console.log("road quickest ok");
