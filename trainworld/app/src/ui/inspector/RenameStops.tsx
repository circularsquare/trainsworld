// A line's stations renamed together from its panel: the Stops list turns into a box per station in the
// list's order, filled with the names they show. Rename sends the changed ones as one edit (one undo), each
// renamed as its own panel would (`renameStations` in workers/clockDemand.ts). A paste of several lines
// fills the box pasted into and those below it, so a list of names from elsewhere goes in at once. The station
// whose box has the focus is lit on the map, as a stop row under the pointer is, and ringed so it stands out
// among the line's other stations (the light alone is faint at most zooms).

import { useEffect, useRef, useState } from "preact/hooks";
import { t as tr } from "../../i18n";
import { complexOf } from "../../game/complexView";
import { edit, say, stationNames, stationsById, world } from "../../game/world";
import { setListHot } from "../../map/hovered";
import { live } from "../../map/live";
import { Marker } from "maplibre-gl";

/** The ring on the map round the station whose box has the focus. */
function ringAt(id: string | null, ring: { current: Marker | null }) {
  ring.current?.remove();
  ring.current = null;
  setListHot(id ? { stations: complexOf(id) } : null);
  const w = world.peek(), s = id && w ? stationsById(w).get(id) : undefined;
  if (!s || !live.map) return;
  // MapLibre places the marker by its element's transform, so the ring that pulses is a child
  const el = document.createElement("div");
  el.appendChild(document.createElement("div")).className = "rename-ring";
  ring.current = new Marker({ element: el }).setLngLat([s.lng, s.lat]).addTo(live.map);
}

/** One box per station shown, in `order` (station ids as the list shows them, top first); a bound station
 * stands for its complex, so two bound stations of one complex share a box. */
function renameRows(order: readonly string[]): { id: string; name: string }[] {
  const w = world.peek();
  if (!w) return [];
  const names = stationNames(w), byId = stationsById(w), seen = new Set<string>(), rows: { id: string; name: string }[] = [];
  for (const id of order) {
    const key = byId.get(id)?.unbound ? id : "c:" + [...complexOf(id)].sort().join(",");
    if (seen.has(key)) continue;
    seen.add(key);
    rows.push({ id, name: names.get(id) ?? "" });
  }
  return rows;
}

export function RenameStops({ order, colour, onClose }: { order: readonly string[]; colour: string; onClose: () => void }) {
  const [rows] = useState(() => renameRows(order));
  const [drafts, setDrafts] = useState<string[]>(() => rows.map((r) => r.name));
  const [busy, setBusy] = useState(false);
  const list = useRef<HTMLOListElement>(null);
  const ring = useRef<Marker | null>(null);
  const box = (k: number) => list.current?.querySelectorAll("input")[k];
  useEffect(() => {
    box(0)?.focus();
    box(0)?.select();
    return () => ringAt(null, ring);
  }, []);
  const changed = rows.map((r, k) => ({ r, name: drafts[k].trim() })).filter(({ r, name }) => name && name !== r.name);
  const save = async () => {
    if (busy) return;
    if (!changed.length) return onClose();
    const byId = stationsById(world.peek()!);
    const nodes = changed.map(({ r }) => byId.get(r.id)?.num);
    if (nodes.some((n) => n === undefined)) return onClose();
    setBusy(true);
    const a = await edit({ op: "renameStations", nodes: nodes as number[], names: changed.map((c) => c.name) });
    setBusy(false);
    if (a.ok) {
      say(tr("renameStops.done", { count: changed.length }));
      onClose();
    }
  };
  const paste = (k: number, e: ClipboardEvent) => {
    const lines = (e.clipboardData?.getData("text") ?? "").split(/\r?\n/).map((s) => s.trim());
    while (lines.length && !lines[lines.length - 1]) lines.pop();
    if (lines.length < 2) return;
    e.preventDefault();
    setDrafts((d) => d.map((v, j) => (j >= k && j - k < lines.length && lines[j - k] ? lines[j - k] : v)));
  };
  const key = (k: number, e: KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    } else if (e.key === "Enter") {
      e.preventDefault();
      const next = box(k + 1);
      if (next && !(e.ctrlKey || e.metaKey)) {
        next.focus();
        next.select();
      } else void save();
    }
  };
  return (
    <div class="rename-stops">
      <p class="muted small">{tr("renameStops.help")}</p>
      <ol ref={list} class="stops rename" style={{ "--c": colour }}>
        {rows.map((r, k) => (
          <li>
            <input class={"rename-box" + (drafts[k].trim() && drafts[k].trim() !== r.name ? " changed" : "")} value={drafts[k]} disabled={busy}
              aria-label={tr("renameStops.box", { name: r.name })} data-tip={drafts[k].trim() !== r.name ? r.name : undefined}
              onInput={(e) => { const v = e.currentTarget.value; setDrafts((d) => d.map((x, j) => (j === k ? v : x))); }}
              onFocus={() => ringAt(r.id, ring)} onBlur={() => ringAt(null, ring)}
              onPaste={(e) => paste(k, e)} onKeyDown={(e) => key(k, e)} />
          </li>
        ))}
      </ol>
      <div class="line-stop-actions">
        <button class="btn text on" disabled={busy || !changed.length} onClick={() => void save()}>
          {tr("renameStops.save", { count: changed.length })}
        </button>
        <button class="btn text" disabled={busy} onClick={onClose}>{tr("renameStops.cancel")}</button>
      </div>
    </div>
  );
}
