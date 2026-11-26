// How a stop's boardings share out between its two stations where its runs stop at stations of their own
// (game/stopStations.ts): the station views count each half's own.

import { lineDemand } from "./demand";
import { runAt } from "./stopStations";
import type { LineInput, StationId, WorldView } from "./types";

/** The part of line `l`'s boardings at its stop `k` that board at station `id`: all of them where the stop is one
 * station; where its two runs stop at stations of their own, the part boarding the run that stops at `id`, from
 * the forecast's boardings by run (`LineDemand.board`), half each while there is none. The day's counts are the
 * stop's, not split by run, and share out the same way. */
export function stopShare(w: WorldView, l: LineInput, k: number, id: StationId): number {
  const run = runAt(w, l, k, id);
  if (run === null) return 1;
  const n = l.stops.length;
  let r0 = 0, r1 = 0;
  for (const p of lineDemand(l.id)?.board ?? []) {
    if (p.length !== 2 * n) continue;
    r0 += p[k];
    r1 += p[n + (n - 1 - k)];
  }
  if (!(r0 + r1 > 0)) return 0.5;
  return (run === 0 ? r0 : r1) / (r0 + r1);
}
