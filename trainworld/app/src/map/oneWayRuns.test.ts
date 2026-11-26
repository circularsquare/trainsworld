// One-way track's chevrons by run (`oneWayTrackRuns`). Run with: npx vite-node src/map/oneWayRuns.test.ts
import type { Strokes } from "./network";
import type { OneWayEdge } from "./oneWayMarks";

// oneWayMarks.ts reaches perfFlags.ts (through rendererParts.ts), which reads the page's address as it loads
(globalThis as { location?: unknown }).location ??= { search: "" };
const { kAtY } = await import("./geo");
const { CHEVRON_STRIDE, oneWayTrackRuns } = await import("./oneWayMarks");

function assert(cond: unknown, msg: string): asserts cond {
  if (!cond) throw new Error(msg);
}

const k = kAtY(0);

/** A piece of track through `pts` (ground metres) from node `a` to node `b`, run its way (`way`). */
function edge(a: number, b: number, way: 1 | 2, pts: [number, number][], double = false): OneWayEdge {
  const n = pts.length - 1, seg = new Float32Array(n * 4), dist = new Float32Array(n);
  let d = 0;
  for (let i = 0; i < n; i++) {
    seg.set([pts[i][0] * k, pts[i][1] * k, pts[i + 1][0] * k, pts[i + 1][1] * k], i * 4);
    dist[i] = d;
    d += Math.hypot(pts[i + 1][0] - pts[i][0], pts[i + 1][1] - pts[i][1]) * k;
  }
  return { a, b, way, double, strokes: { seg, dist, count: n } as unknown as Strokes };
}

const marksOf = (out: number[]) => Array.from({ length: out.length / CHEVRON_STRIDE }, (_, i) => out.slice(i * CHEVRON_STRIDE, (i + 1) * CHEVRON_STRIDE));
/** ground metres from a mark to the nearest stop, as the shaders' stop clearance reads it */
const clearM = (m: number[]) => m[9];

// Two pairs of one-way double track, one each way, between a junction J to the south (node 0) and one to the
// north, through a pair of one-way stations (as Leganés Central): the southbound pair comes into J and the
// northbound leaves it, one in and one out, but a train never turns from one onto the other there.
{
  const at = { j: [-123, -191], west: [-10, 6], east: [0, 0], wn: [74, 137], en: [124, 193], north: [162, 251] } as const;
  const south = [edge(5, 1, 1, [[...at.wn], [...at.west]], true), edge(1, 0, 1, [[...at.west], [...at.j]], true)];
  const north = [edge(0, 2, 1, [[...at.j], [...at.east]], true), edge(2, 3, 1, [[...at.east], [...at.en]], true)];
  const stops = [at.j, at.west, at.east, at.north].map(([x, y]) => [x * k, y * k] as const);
  const out: number[] = [];
  oneWayTrackRuns([...south, ...north], stops, out);
  const marks = marksOf(out);
  // (a mark on one-way double track is one on each of its tracks)
  const southbound = marks.filter((m) => m[3] < 0), northbound = marks.filter((m) => m[3] > 0);
  assert(southbound.length + northbound.length === marks.length, "every mark points along the pairs");
  assert(southbound.some((m) => clearM(m) >= 100), `the southbound pair has a mark clear of the stations: ${southbound.map(clearM)}`);
  assert(northbound.some((m) => clearM(m) >= 100), `the northbound pair has a mark clear of the stations: ${northbound.map(clearM)}`);
  assert(southbound.every((m) => m[0] < 80 * k && m[2] < 0), "the southbound marks are on its pair, pointing its way");
  console.log("ok: a pair each way at one junction gets marks of its own");
}

// A run still goes on through a node where one piece follows another, so the step carries across them.
{
  const out: number[] = [];
  oneWayTrackRuns([edge(0, 1, 1, [[0, 0], [0, 200]]), edge(1, 2, 1, [[0, 200], [0, 400]])], [], out);
  assert(out.length / CHEVRON_STRIDE === 1, `one mark on 400 m of one run, ${out.length / CHEVRON_STRIDE} found`);
  console.log("ok: a run goes on through a plain node");
}
