import type { BusPlaces } from "./busLinks";

/** Share layout keys, never stop ids: editing and boardings still use each line's own stop.
 * A bus place at a station uses its railway key, leaving railway complexes unchanged. */
export function groupPlaceKey(places: BusPlaces, stationKey: (id: string) => string): (id: string) => string {
  const keys = new Map<string, string>();
  places.places.forEach((place, i) => {
    const key = place.stations.length ? stationKey(place.stations[0]) : `bus-place:${i}`;
    for (const stop of place.stops) if (stop.station < 0) keys.set(String(stop.id), key);
  });
  return (id) => keys.get(id) ?? stationKey(id);
}
