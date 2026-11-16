// Line branches check: a head's `today` row takes its branches' counts (app/src/game/todayFold.ts): riders,
// trains and fares add, a branch's boardings at a station the head stops at go onto the head's stop there,
// those at its own stations into the head's `beyond`, and the branch's own row goes.
//   node --experimental-strip-types trainworld/checks/line-branches-fold.mjs
import { foldBranches } from '../app/src/game/todayFold.ts';

const results = [];
const report = (name, ok, detail = '') => { results.push(ok); console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}  ${detail}`); };
const day = (riders, stops, extra = {}) => ({ riders, stops, fares: riders * 2, dayFares: riders * 3, fullTrains: 1, trains: 10, leftBehind: 0, trainsNow: 2, fullNow: 0, crowdLoad: 0.5, ...extra });

// trunk A-B-C; branch 1 to D-E (its stops A,B,C,D,E), branch 2 from B to F-G (A,B,F,G)
const stops = { h: ['A', 'B', 'C'], b1: ['A', 'B', 'C', 'D', 'E'], b2: ['A', 'B', 'F', 'G'] };
const days = new Map([
  ['h', day(60, [30, 20, 10])],
  ['b1', day(150, [10, 20, 30, 40, 50], { crowdLoad: 1.2 })],
  ['b2', day(10, [1, 2, 3, 4])],
  ['other', day(5, [5])],
]);
foldBranches(days, new Map([['b1', 'h'], ['b2', 'h']]), (id) => stops[id]);
const h = days.get('h');
report('the branches\' rows go, the others stay', !days.has('b1') && !days.has('b2') && days.get('other').riders === 5);
report('riders, trains and fares add', h.riders === 220 && h.trains === 30 && h.trainsNow === 6 && h.fares === 440 && h.dayFares === 660, JSON.stringify(h));
report('the crowding is the fullest', h.crowdLoad === 1.2);
report('trunk boardings go onto the head\'s stop at the same station', JSON.stringify(h.stops) === '[41,42,40]', JSON.stringify(h.stops));
report('a branch\'s own stations go into beyond', JSON.stringify(h.beyond) === '{"D":40,"E":50,"F":3,"G":4}', JSON.stringify(h.beyond));
const all = h.stops.reduce((a, b) => a + b, 0) + Object.values(h.beyond).reduce((a, b) => a + b, 0);
report('no boarding is lost or counted twice', all === 30 + 20 + 10 + 150 + 10, `${all}`);

// a head with no counts of its own still shows its branch's
const quiet = new Map([['b1', day(150, [10, 20, 30, 40, 50])]]);
foldBranches(quiet, new Map([['b1', 'h']]), (id) => stops[id]);
const q = quiet.get('h');
report('a head with no row takes its branch\'s', q?.riders === 150 && JSON.stringify(q.stops) === '[10,20,30]' && q.beyond.D === 40, JSON.stringify(q));

// no branch rows: nothing changes, no beyond appears
const plain = new Map([['h', day(60, [30, 20, 10])]]);
foldBranches(plain, new Map([['b1', 'h']]), (id) => stops[id]);
report('a branch with no row changes nothing', plain.get('h').riders === 60 && plain.get('h').beyond === undefined);

const failed = results.filter((x) => !x).length;
console.log(failed ? `${failed} FAILED` : `all ${results.length} passed`);
process.exit(failed ? 1 : 0);
