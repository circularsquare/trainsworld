// Shared bus places in the group inspector, without WASM or a downloaded save.
// Run from the repo root: node trainworld/checks/group-bus-layout.mjs
import assert from 'node:assert/strict';
import { loadApp } from './lib.mjs';

const { busPlaces } = await loadApp('src/game/busLinks.ts');
const { groupPlaceKey } = await loadApp('src/game/groupPlaces.ts');
const { groupLayout, pathPlaces } = await loadApp('src/game/groupLayout.ts');

const stop = (id, lat, station = -1, lng = 0) => ({ id, lat, lng, station });
const a = [stop(101, .03), stop(102, .02), stop(103, .01)];
const b = [stop(201, .0101), stop(202, .0201), stop(203, .0301)];
const c = [stop(301, .02, -1, .0001), stop(302, .04)];
const far = [stop(401, .022), stop(402, .05)];
const lines = [a, b, c, far].map((stops, i) => ({ id: String(i + 1), num: i + 1, stops: stops.map(s => String(s.id)) }));
const w = { save: { lines, stations: [] }, lineStats: Object.fromEntries(lines.map((l, i) => [l.id, { bus: { stops: [a, b, c, far][i], built: true } }])) };
const places = busPlaces(w);
const key = groupPlaceKey(places, id => id);
assert.equal(key('102'), key('202'));
assert.equal(key('102'), key('301'));
assert.notEqual(key('102'), key('401'), 'a walk to another place does not align it');
const ins = lines.slice(0, 3).map(l => ({ num: l.num, places: pathPlaces(l.stops, [], key) }));
const before = groupLayout(lines.slice(0, 2).map(l => ({ num: l.num, places: pathPlaces(l.stops, [], id => id) })), { all: true });
assert.ok(!before.rows.some(r => r.cells[0] && r.cells[1]), 'the old railway-only keys reproduce the missing shared rows');
const lay = groupLayout(ins, { all: true });
const rowOf = id => lay.rows.findIndex(r => r.cells.some(c => c?.station === id));
for (const [x, y] of [['101', '203'], ['102', '202'], ['103', '201'], ['102', '301']]) assert.equal(rowOf(x), rowOf(y));
ins.forEach((l, col) => {
  const cells = lay.rows.map(r => r.cells[col]).filter(Boolean);
  const got = cells.map(c => c.station);
  const want = l.places.map(p => p.station);
  assert.ok(JSON.stringify(got) === JSON.stringify(want) || JSON.stringify(got) === JSON.stringify(want.toReversed()), 'route order and individual ids survive');
  cells.forEach(c => assert.equal(c.station, want[c.index], 'editing/boarding index remains local to the line'));
});
// Station-attached stops share the railway key; railway complexes and unknown ids are unchanged.
const railKey = id => ['10', '11'].includes(id) ? 'c0' : id;
const mixed = { ...w, save: { ...w.save, stations: [{ id: '10', lng: 0, lat: .02 }] } };
const mixedKey = groupPlaceKey(busPlaces(mixed), railKey);
assert.equal(mixedKey('102'), 'c0');
assert.equal(mixedKey('10'), 'c0');
assert.equal(mixedKey('11'), 'c0');
assert.equal(mixedKey('999'), '999');
const passed = pathPlaces(['10', '12'], [{ gap: 0, node: 11 }], mixedKey);
assert.deepEqual(passed[1], { key: 'c0', station: '11', stop: false, index: -1 });
const railway = ['10', '12', '13'];
assert.deepEqual(pathPlaces(railway, [], mixedKey), pathPlaces(railway, [], railKey));
const loop = pathPlaces(['101', '102', '103', '101'], [], key);
const loopLay = groupLayout([{ num: 1, places: loop }], { all: true });
assert.deepEqual(loopLay.rows.map(r => r.cells[0].station), ['101', '102', '103'], 'a loop keeps its stops once round');
const compact = groupLayout(ins);
assert.ok(compact.rows.some(r => r.cells[0]?.station === '102' && r.cells[2]?.station === '301' && r.named), 'the branch meeting is named in compact mode');
console.log('PASS shared bus stops, branches, opposite directions, separate places, individual ids and railway keys');
