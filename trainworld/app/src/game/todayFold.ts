// `today`'s folds of one line's counts into another's (game/today.ts), kept free of the app's imports so
// trainworld/checks/line-branches-fold.mjs runs them under plain node.

import type { LineToday } from "./today";

/** `a` with `b`'s counts added, at the boardings per stop given: trains and riders add, the crowding is
 * the fuller's. */
export const addDays = (a: LineToday, b: LineToday, stops: number[]): LineToday => ({
  ...a, riders: a.riders + b.riders, stops, fares: a.fares + b.fares, dayFares: a.dayFares + b.dayFares,
  fullTrains: a.fullTrains + b.fullTrains, trains: a.trains + b.trains, leftBehind: a.leftBehind + b.leftBehind, trainsNow: a.trainsNow + b.trainsNow, fullNow: a.fullNow + b.fullNow,
  crowdLoad: Math.max(a.crowdLoad, b.crowdLoad),
});

/** Line branches: a head counts its branches' riders, trains and fares as its own. Unlike a twin's, a
 * branch's stops are not its head's stop for stop (it shares the trunk only so far, then goes its own
 * way), so its boardings go onto the head's stop at the same station, and those at a station the head
 * never stops at into the head's `beyond`, where the station figures still find them. The branches'
 * entries go. `stopsOf` gives each line's stops in the order its counts are in. */
export function foldBranches(days: Map<string, LineToday>, branches: ReadonlyMap<string, string>, stopsOf: (id: string) => readonly string[] | undefined) {
  for (const [b, h] of branches) {
    const x = days.get(b);
    if (!x) continue;
    days.delete(b);
    const hs = stopsOf(h) ?? [], bs = stopsOf(b) ?? [];
    // a head with no counts of its own still shows its branches'
    const a = days.get(h) ?? { riders: 0, stops: hs.map(() => 0), fares: 0, dayFares: 0, fullTrains: 0, trains: 0, leftBehind: 0, trainsNow: 0, fullNow: 0, crowdLoad: 0 };
    const stops = [...a.stops], beyond = { ...a.beyond };
    bs.forEach((s, k) => {
      const v = x.stops[k] ?? 0, i = hs.indexOf(s);
      if (!v) return;
      if (i >= 0 && i < stops.length) stops[i] += v;
      else beyond[s] = (beyond[s] ?? 0) + v;
    });
    days.set(h, { ...addDays(a, x, stops), ...(Object.keys(beyond).length ? { beyond } : {}) });
  }
}
