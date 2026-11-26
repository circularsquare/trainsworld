// One-way stations paired as one stop (the track model's `pair_stations`, sim/src/track/wasm/pairs.rs): a
// station on each leg of a one-way pair, which a line lists once, each run stopping at the half on its own
// track. The station view's Pair starts it, the next station clicked on the map is the other half, and a pair is
// undone from the station view. Only the player knows which platforms of a big station go together, so the game
// never pairs them by itself (T-3605).

import { t } from "../i18n";
import { signal } from "@preact/signals";
import type { Selection, StationId } from "./types";
import { edit, say, world } from "./world";

/** The station a pair starts from while the player picks the other half; null otherwise. */
export const pairing = signal<StationId | null>(null);

export function startPair(id: StationId) {
  pairing.value = id;
}

/** Stop picking; true if a pair was being made (Esc takes one thing at a time). */
export function cancelPair(): boolean {
  if (pairing.peek() === null) return false;
  pairing.value = null;
  return true;
}

/** A map click while pairing: the station picked is paired with the one the pair started from, which stays
 * selected; a click on anything else stops pairing. A refused pick says why (the track model's issue) and keeps
 * picking. */
export async function pairPicked(picked: Selection): Promise<void> {
  const from = pairing.peek();
  if (from === null) return;
  if (picked?.kind !== "station") {
    pairing.value = null;
    return;
  }
  if (picked.station === from) return; // still picking
  const st = world.peek()?.save.stations ?? [];
  const sa = st.find((s) => s.id === from), sb = st.find((s) => s.id === picked.station);
  if (!sa || !sb) {
    pairing.value = null;
    return;
  }
  const a = await edit({ op: "pairStations", a: sa.num, b: sb.num });
  if (!a.ok) return; // still picking
  pairing.value = null;
  say(t("pairs.paired", { a: sa.name, b: sb.name }));
}

/** Undo the pair station `id` is in. */
export async function unpair(id: StationId) {
  const s = world.peek()?.save.stations.find((x) => x.id === id);
  if (!s) return;
  const r = await edit({ op: "unpairStation", node: s.num });
  if (r.ok) say(t("pairs.unpaired", { name: s.name }));
}
