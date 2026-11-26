// One-way double track on the map: where trains that take its other track cross over to it (`twinRamp`), and where its
// two tracks come together onto its way's track of two-way double (`twinMerges`, `twinRunPoints`). Run with:
// npx vite-node src/map/oneWayDouble.test.ts
import type { RideRail, TwinRun } from "./network";
import type { ClockEdge } from "../workers/protocol";

// network.ts reaches perfFlags.ts (through oneWayMarks.ts and rendererParts.ts), which reads the page's address as it loads
(globalThis as { location?: unknown }).location ??= { search: "" };
const { kAtY } = await import("./geo");
const { rideAt, rideBand, TWIN_LEAD_M, twinMerges, twinRamp, twinRunPoints } = await import("./network");
const { BAND_SHARE } = await import("./lineBands");

function assert(cond: unknown, msg: string): asserts cond {
  if (!cond) throw new Error(msg);
}
const near = (a: number, b: number) => Math.abs(a - b) < 1e-6;
const ease = (t: number) => t * t * (3 - 2 * t);

// 100 m of plain track, one-way double over two pieces (300 m and 200 m), then 100 m plain: samples every 50 m
{
  const path = [1 << 1, 2 << 1, (3 << 1) | 1, 4 << 1], lens = [100, 300, 200, 100];
  const v = twinRamp(path, lens, 15, 50, new Set([2, 3]));
  assert(v[0] === 0 && v[1] === 0 && v[2] === 0, `on its own track before the double: ${[...v.slice(0, 3)]}`);
  assert(near(v[3], ease(50 / TWIN_LEAD_M)) && v[4] === 1, `over to the other track along TWIN_LEAD_M (${TWIN_LEAD_M} m) in: ${v[3]}, ${v[4]}`);
  assert([...v.slice(4, 11)].every((x) => x === 1), "on the other track across the two pieces");
  assert(near(v[11], ease(50 / TWIN_LEAD_M)) && v[12] === 0 && v[13] === 0 && v[14] === 0, `back on its own before the double ends: ${[...v.slice(10)]}`);
  console.log("ok: over to the other track and back, eased in from each end");
}

// a short stretch eases over half its length each way
{
  const v = twinRamp([0, 2], [100, 100], 5, 50, new Set([1]));
  assert(v[2] === 0 && near(v[3], 1) && v[4] === 0, `100 m of one-way double: ${[...v]}`);
  assert([...twinRamp([0], [500], 11, 50, new Set([1]))].every((x) => x === 0), "none off one-way double track");
  console.log("ok: a short stretch, and none off it");
}

const K = kAtY(0), H = 2 * K;
const edge = (id: number, a: number, b: number, tracks: 1 | 2, oneWay: 0 | 1 | 2, pts: [number, number][]) =>
  [{ id, a, b, tracks, oneWay } as ClockEdge, Float32Array.from(pts.flatMap(([x, y]) => [x * K, y * K, 0]))] as const;
type Piece = ReturnType<typeof edge>;
const merges = (list: readonly Piece[], keep: number, stations: number[] = []) => twinMerges(list.map((x) => x[0]), list.map((x) => x[1]), keep, new Set(stations));
/** a piece's two tracks as drawn (x, y pairs): left and right of a to b in the local frame */
function drawnTracks(p: Piece, run: TwinRun): [number[], number[]] {
  const lp = twinRunPoints(p[1], run, 1 | (run.sign[0] ? 2 : 0) | (run.sign[1] ? 4 : 0));
  const q = lp.lpts, n = q.length / 3, out: [number[], number[]] = [[], []];
  for (let k = 0; k < n; k++) {
    const a = Math.max(0, k - 1), b = Math.min(n - 1, k + 1), dx = q[b * 3] - q[a * 3], dy = q[b * 3 + 1] - q[a * 3 + 1], d = Math.hypot(dx, dy) || 1;
    const w = (H * (typeof lp.lane === "number" ? lp.lane : lp.lane[k])) / 255;
    out[0].push(q[k * 3] - (dy / d) * w, q[k * 3 + 1] + (dx / d) * w);
    out[1].push(q[k * 3] + (dy / d) * w, q[k * 3 + 1] - (dx / d) * w);
  }
  return out;
}
function distTo(xy: readonly number[], x: number, y: number) {
  let best = Infinity;
  for (let k = 0; k + 3 < xy.length; k += 2) {
    const ax = xy[k], ay = xy[k + 1], dx = xy[k + 2] - ax, dy = xy[k + 3] - ay, l2 = dx * dx + dy * dy;
    const t = l2 > 0 ? Math.max(0, Math.min(1, ((x - ax) * dx + (y - ay) * dy) / l2)) : 0;
    best = Math.min(best, Math.hypot(ax + dx * t - x, ay + dy * t - y));
  }
  return best;
}

// Two-way double track from the south-west into a junction J (node 0), where a one-way double pair leaves north-east and
// one comes back into it beside it, each through a station on to node 3 or from node 4
{
  const list = [
    edge(10, 9, 0, 2, 0, [[-300, -400], [0, 0]]),
    edge(11, 0, 1, 2, 1, [[0, 0], [130, 190]]),
    edge(12, 1, 3, 2, 1, [[130, 190], [250, 380]]),
    edge(13, 2, 0, 2, 1, [[110, 200], [0, 0]]),
    edge(14, 4, 2, 2, 1, [[230, 390], [110, 200]]),
  ];
  for (const keep of [1, -1]) {
    const m = merges(list, keep, [1, 2]);
    assert(!m.has(10), "two-way double track is drawn as it is");
    const out = m.get(11), back = m.get(13);
    // onto the side trains keep to looking along its way (`LegShift.sign`: left of a to b in the local frame)
    assert(out && out.sign[0] === keep && !out.sign[1] && out.from[0] === 0, `the pair leaving J comes together there alone, onto its way's track: ${JSON.stringify(out)}`);
    assert(back && !back.sign[0] && back.sign[1] === keep, `the pair coming into J comes together there alone, onto its way's track: ${JSON.stringify(back)}`);
    assert(!m.has(12) && !m.has(14), "apart well before the stations, and on through them as it is");
    // the pair coming back lies right of the one leaving (on the map): keeping right, beyond the track the leaving one
    // comes together on, so the two ways cross at J and neither is held off the other; keeping left, away from it
    const held = keep === -1;
    assert(out.near[0].length === +held && back.near[1].length === +held, `held off each other, keep side ${keep}: ${out.near[0].length}, ${back.near[1].length}`);
  }
  // one-way double meeting single track only narrows as double track does there
  const single = [edge(1, 9, 0, 1, 0, [[-300, -400], [0, 0]]), edge(2, 0, 1, 2, 1, [[0, 0], [130, 190]])];
  assert(!merges(single, 1).size, "none where it meets single track");
  console.log("ok: one-way double comes together onto its way's track of two-way double, and only there");
}

// South of Leganés Central (a player's save): two-way double from the north into J (node 0); the pair coming north
// into J (pieces of 41, 7, 5 and 175 m) and the pair leaving J southward laid on top of it, peeling off west (its side,
// keeping right) to 12 m apart by 96 m. Drawn, neither pair may lie on the other
{
  const peel: [number, number][] = [[0, 0], [0, 14], [-0.2, 19], [-0.5, 24], [-1, 29], [-1.6, 34], [-2.3, 38], [-3.1, 43], [-4.1, 48], [-5.3, 53], [-6.6, 58], [-7.7, 62], [-8.8, 67], [-9.6, 72], [-10.4, 77], [-11, 82], [-11.5, 86], [-11.8, 91], [-12, 96], [-12, 300]];
  const lay = (west: boolean) => [
    edge(455, 0, 212, 2, 0, [[0, 0], [0, -300]]),
    edge(447, 445, 0, 2, 1, [[0, 41], [0, 0]]),
    edge(454, 449, 445, 2, 1, [[0, 48], [0, 41]]),
    edge(448, 210, 449, 2, 1, [[0, 53], [0, 48]]),
    edge(263, 263, 210, 2, 1, [[0, 228], [0, 53]]),
    edge(452, 0, 493, 2, 1, peel.map(([x, y]) => [west ? x : -x, y] as [number, number])),
  ];
  const list = lay(true), m = merges(list, 1);
  const out = m.get(452), back = m.get(447);
  assert(out && out.sign[0] === 1 && out.near[0].length === 1, `the leaving pair comes together west at J, held off the other: ${JSON.stringify(out?.sign)}`);
  assert(back && back.sign[1] === 1 && back.near[1].length === 1, `the pair coming in comes together east at J, held off the other: ${JSON.stringify(back?.sign)}`);
  // the 41 m piece is too short for its tracks to be apart by its end: the run goes on through the plain nodes
  const on = [454, 448, 263].map((id) => m.get(id));
  assert(on.every((v) => v && v.sign[1] === 1 && !v.sign[0]), `on into the pieces beyond: ${on.map((v) => JSON.stringify(v?.sign))}`);
  assert(Math.abs(on[0]!.from[1] - 41 * K) < 0.01 * K && Math.abs(on[2]!.from[1] - 53 * K) < 0.01 * K, `measured from J: ${on.map((v) => (v?.from[1] ?? 0) / K)}`);
  // drawn: past J itself, every track of one at least the gap between double track's two from every track of the other
  const tr = (id: number) => drawnTracks(list.find((x) => x[0].id === id)!, m.get(id)!);
  const mine = tr(452), theirs = [447, 454, 448, 263].flatMap(tr);
  let least = Infinity, where = 0;
  for (const t of mine) {
    for (let k = 0; k < t.length; k += 2) {
      if (Math.hypot(t[k], t[k + 1]) < 1 * K) continue;
      for (const o of theirs) {
        const d = distTo(o, t[k], t[k + 1]);
        if (d < least) { least = d; where = t[k + 1] / K; }
      }
    }
  }
  assert(least > 2 * H - 0.15 * K, `the two pairs' tracks ${(least / K).toFixed(2)} m apart at ${where.toFixed(0)} m from J`);
  // the leaving pair's own track (west, keeping right: left of a to b in the local frame) runs on as it would, 2 m off
  // its centre line, held together or not
  const given = list.find((x) => x[0].id === 452)![1], line: number[] = [];
  for (let k = 0; k < given.length; k += 3) line.push(given[k], given[k + 1]);
  const west = mine[0];
  for (let k = 0; k < west.length; k += 2) {
    if (west[k + 1] > 150 * K) break;
    const off = distTo(line, west[k], west[k + 1]) - H;
    assert(Math.abs(off) < 0.05 * K && west[k] < 0, `its own track on as it would be, ${(off / K).toFixed(2)} m off at ${(west[k + 1] / K).toFixed(0)} m`);
  }
  // wholly apart past where they are quad track's spacing apart
  const lp = twinRunPoints(given, out, 1 | 2), lane = lp.lane as Uint8Array;
  for (let k = 0; k < lane.length; k++) if (lp.lpts[k * 3 + 1] > 100 * K) assert(lane[k] === 255 && lp.open![k] === 255, `apart by 100 m: ${lane[k]}`);
  // the other pair moved along its middle (ends and points as many as before) is drawn anew, not taken from the cache
  const moved = lay(true).map((x) => (x[0].id === 452 ? ([x[0], x[1].map((v, j) => (j === 10 * 3 ? v + 0.5 * K : v))] as unknown as Piece) : x));
  assert(merges(moved, 1).get(447)!.key !== back.key, "the key changes with the other pair's middle");
  assert(merges(lay(true), 1).get(447)!.key === back.key, "and stays as it is when nothing does");
  // peeling off east instead, over the other's track: the two ways cross there, and are drawn so
  const cross = merges(lay(false), 1);
  assert(!cross.get(452)!.near[0].length && !cross.get(447)!.near[1].length, "crossing, neither held off the other");
  console.log(`ok: two pairs coming together at one junction side by side are drawn apart (${(least / K).toFixed(2)} m at least), not one on the other`);
}

// a pair too short to come apart before a station: apart by there
{
  const list = [edge(1, 9, 0, 2, 0, [[0, -300], [0, 0]]), edge(2, 0, 5, 2, 1, [[0, 0], [0, 41]]), edge(3, 5, 6, 2, 1, [[0, 41], [0, 300]])];
  const m = merges(list, 1, [5]), run = m.get(2);
  assert(run && Math.abs(run.stop[0] - 41 * K) < 0.01 * K && !m.has(3), `stops at the station: ${JSON.stringify(run)}`);
  const lp = twinRunPoints(list[1][1], run, 1 | 2), n = lp.lpts.length / 3, lane = lp.lane as Uint8Array;
  assert(lane[n - 1] === 255 && Math.abs(lp.lpts[(n - 1) * 3]) < 0.01 * K, `apart and on its centre line at the station: ${lane[n - 1]}, ${lp.lpts[(n - 1) * 3] / K}`);
  assert(lane[0] === 0 && Math.abs(lp.lpts[0] + H) < 0.01 * K, `together on its track at J: ${lane[0]}, ${lp.lpts[0] / K}`);
  console.log("ok: a short pair is apart by the station it runs into");
}

// A line's return run on its own way back over the other way's one-way double, 12 m beside its own banded stroke in
// one ribbon (Leganés): zoomed out it is drawn on that stroke, at any line width, and close in on its own track
{
  const kk = kAtY(0), M = (m: number) => m * kk;
  // its own stroke runs east along y = 0 (local y runs south), 0-500 m; the ribbon's middle 6 m south of it (right of
  // its travel: -6 m left), its slot 0.6 line widths left of that, a ribbon of several lines (its width share)
  const fwd: number[] = [], fb: number[] = [];
  for (let x = 0; x <= 500; x += 50) {
    fwd.push(M(x), 0, 0);
    fb.push(0.6, 1 - BAND_SHARE, -6);
  }
  const rail: RideRail[] = [{ pts: new Float32Array(fwd), band: new Float32Array(fb), reverse: false }];
  // where the shader draws a point: off its stroke's travel to the left on the map (local y south: the frame's right)
  // by its shift in line widths `u` m wide and its metres
  const drawn = (x: number, y: number, tx: number, ty: number, shift: number, m: number, u: number) => {
    const tl = Math.hypot(tx, ty), o = (shift * u + m) * kk;
    return [x + (ty / tl) * o, y - (tx / tl) * o];
  };
  // its way back runs west 12 m south, drawn a to b east (`reverse`)
  const back = new Float32Array([M(100), M(12), 0, M(400), M(12), 0]);
  const r = rideBand({ lpts: back, lane: 255, src: null }, true, rail);
  assert(r, "a way back beside its line's own stroke has bands");
  const n = r.pts.length / 3;
  assert(n >= 30, `drawn through points every few metres: ${n}`);
  for (const u of [20, 3]) for (let k = 0; k < n; k++) {
    const x = r.pts[k * 3], y = r.pts[k * 3 + 1];
    const [bx, by] = drawn(x, y, -1, 0, r.band[k * 3], r.band[k * 3 + 2], u);
    const [fx, fy] = drawn(x, 0, 1, 0, 0.6, -6, u);
    assert(Math.abs(bx - fx) < 1e-6 && Math.abs(by - fy) < M(1e-3), `on its line's stroke at ${(x / kk).toFixed(0)} m, line ${u} m wide: ${(by / kk).toFixed(3)} m, not ${(fy / kk).toFixed(3)} m`);
    assert(Math.abs(r.band[k * 3 + 1] - (1 - BAND_SHARE)) < 1e-6, `as narrow as its line's band: ${r.band[k * 3 + 1]}`);
  }
  const mid = rideAt(r, 0.5);
  assert(near(mid[0], -0.6) && Math.abs(mid[2] - -6) < 1e-3, `its trains half way along take the same: ${mid}`);
  // the same way (two one-way tracks side by side both its line's way): the same place, not turned round
  const same = rideBand({ lpts: back, lane: 255, src: null }, false, rail)!;
  const [sx, sy] = drawn(same.pts[3], same.pts[4], 1, 0, same.band[3], same.band[5], 20), [fx, fy] = drawn(same.pts[3], 0, 1, 0, 0.6, -6, 20);
  assert(Math.abs(sx - fx) < 1e-6 && Math.abs(sy - fy) < M(1e-3), `the same way, on its line's stroke: ${sy / kk} m, not ${fy / kk} m`);
  // further off than RIDE_FAR_M (30 m), across it, or beside a stroke with no ribbon: none
  assert(rideBand({ lpts: new Float32Array([M(100), M(35), 0, M(400), M(35), 0]), lane: 255, src: null }, true, rail) === null, "35 m off: on its own track");
  assert(rideBand({ lpts: new Float32Array([M(250), M(-100), 0, M(250), M(100), 0]), lane: 255, src: null }, true, rail) === null, "across it: on its own track");
  const alone: RideRail[] = [{ pts: rail[0].pts, band: new Float32Array(fb.map((v, i) => (i % 3 === 1 ? 0 : v))), reverse: false }];
  assert(rideBand({ lpts: back, lane: 255, src: null }, true, alone) === null, "its line alone (no ribbon): on its own track");
  // 20-30 m off: let go smoothly, all of it within 20 m
  const out = rideBand({ lpts: new Float32Array([M(100), M(25), 0, M(400), M(25), 0]), lane: 255, src: null }, true, rail)!;
  assert(out && out.band[1] > 0 && out.band[1] < 1 - BAND_SHARE, `25 m off: part way (${out?.band[1]})`);
  console.log("ok: a way back beside its line drawn on its stroke zoomed out, at any width; on its own track off it");
}
