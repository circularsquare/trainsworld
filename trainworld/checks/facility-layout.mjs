// Facility choices at grid boundaries and outside station buildings.
import assert from 'node:assert/strict';
import { loadApp } from './lib.mjs';
const { placeFacilities, LOT_STRIDE, ST_ROOM, ST_FLIP, ST_PARK, PLATFORM_POINTS } = await loadApp(`
  export {placeFacilities, LOT_STRIDE} from '@app/src/map/facilityPlace.ts';
  export {ST_ROOM, ST_FLIP, ST_PARK, PLATFORM_POINTS} from '@app/src/map/stationGeometry.ts';
`);

function layout(x, outlines, cache, trackY = -25) {
  const flags = [ST_ROOM | ST_PARK, ST_ROOM | ST_PARK, ST_PARK];
  const rows = flags.map((_, id) => ({ id, lat: 0, platform: 100 }));
  const stations = Float32Array.from(flags.flatMap((flag, i) => [x + 10 * i, i * 22, flag]));
  const lines = new Float32Array(rows.length * PLATFORM_POINTS * 2);
  for (const row of rows) for (let j = 0; j < PLATFORM_POINTS; j++)
    lines[(row.id * PLATFORM_POINTS + j) * 2] = 100 * (j / (PLATFORM_POINTS - 1) - 0.5);
  const lots = placeFacilities(rows, stations, lines, new Float32Array(rows.length * 4),
    new Map([[7, Float32Array.of(x - 500, trackY, 0, x + 500, trackY, 0)]]), new Map(), [9, 9, 10], outlines, cache);
  return {
    rooms: rows.map(({ id }) => stations[id * 3 + 2] & (ST_ROOM | ST_FLIP)),
    lots: Array.from({ length: lots.length / LOT_STRIDE }, (_, i) => {
      const k = i * LOT_STRIDE;
      return [lots[k] - x, lots[k + 1], ...lots.slice(k + 2, k + LOT_STRIDE)];
    }),
  };
}
const plain = { rooms: [ST_ROOM, 0, 0], lots: [
  [0, -58, 50, 0, 50, 16, 2, 0, 0, 2], [20, 108, 50, 0, 50, 8, 1, 0, 2, 1],
] };
assert.deepEqual(layout(0, []), plain);
assert.deepEqual(layout(-1280, []), plain);
for (const radius of [160, 40000]) {
  const x = -1280;
  const outline = { corners: [x - radius, -60, x + radius, -60, x + radius, 60, x - radius, 60], members: [1] };
  assert.deepEqual(layout(x, [outline]), {
    rooms: plain.rooms,
    lots: [[0, -82, 50, 0, 50, 16, 2, 0, 0, 2], plain.lots[1]],
  });
}

// A kept collision grid must follow changed track points and shifted platform centres.
const cache = {};
assert.deepEqual(layout(0, [], cache), plain);
assert.deepEqual(layout(0, [], cache), plain);
const moved = layout(0, [], undefined, -58);
assert.notDeepEqual(moved, plain);
assert.deepEqual(layout(0, [], cache, -58), moved);
assert.deepEqual(layout(-1280, [], cache), plain);
