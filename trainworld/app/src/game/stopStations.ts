// The stations a line stops at. A line lists each stop once, and its return run stops at that station but for
// a stop whose way back is at a station of its own: a platform beside it (T-3348) or the other half of a pair of
// one-way stations (game/pairs.ts), which run 0 never passes (`LineStats.backStops`, the track model's
// `line_back_stops`). That is one stop to the line and two stations on the map, so every view that asks where a
// line stops, or which lines stop at a station, reads both here: neither half is left showing no line.

import type { LineInput, StationId, WorldView } from "./types";

/** The station line `l`'s return run stops at for its stop `k`, where that is not the listed one; else undefined
 * (`backStops` is in the order of the line's stops). */
export function backStopAt(w: WorldView, l: LineInput, k: number): StationId | undefined {
  const back = w.lineStats[l.id]?.backStops;
  if (!back || back.length !== l.stops.length) return undefined;
  const b = back[k];
  return b !== undefined && b !== l.stops[k] ? b : undefined;
}

/** The stations of line `l`'s stop `k`: the listed one (run 0's), then the return run's where that differs. */
export function stopStations(w: WorldView, l: LineInput, k: number): StationId[] {
  const b = backStopAt(w, l, k);
  return b === undefined ? [l.stops[k]] : [l.stops[k], b];
}

/** The indices of line `l`'s stops at station `id`, either run's (a line may stop at one twice, T-797). */
export function stopsAtStation(w: WorldView, l: LineInput, id: StationId): number[] {
  const out: number[] = [];
  l.stops.forEach((s, k) => {
    if (s === id || backStopAt(w, l, k) === id) out.push(k);
  });
  return out;
}

/** Whether line `l` stops at station `id` either way. */
export const stopsAt = (w: WorldView, l: LineInput, id: StationId): boolean => stopsAtStation(w, l, id).length > 0;

/** The run (0 or 1) whose trains stop at station `id` for line `l`'s stop `k`: both (null) where it is the one
 * station of the stop. */
export function runAt(w: WorldView, l: LineInput, k: number, id: StationId): 0 | 1 | null {
  const b = backStopAt(w, l, k);
  if (b === undefined) return null;
  return id === b ? 1 : 0;
}

/** The station a line lists for station `id`: `id` itself, or the stop whose other half it is. */
export function listedFor(w: WorldView, l: LineInput, id: StationId): StationId {
  if (l.stops.includes(id)) return id;
  const k = stopsAtStation(w, l, id)[0];
  return k === undefined ? id : l.stops[k];
}
