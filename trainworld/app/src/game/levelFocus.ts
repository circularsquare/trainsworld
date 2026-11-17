// Layer focus: the elevation levels the map highlights. Levels left out are
// not hidden but drawn dim, and their track is no snap or join target, so the player can draw
// under or over it without the snap pulling to another depth. Kept as a bit mask, one bit per
// level -3..+3, in `DisplaySettings.levelFocus`.

import { LEVELS, type Level } from "./types";

/** The mask bit of a level (-3..+3); out-of-range values clamp to the nearest level. */
export const levelBit = (level: number) => 1 << (Math.max(-3, Math.min(3, Math.round(level))) + 3);

/** Every level highlighted: the map as it was before layer focus existed. */
export const ALL_LEVEL_BITS = LEVELS.reduce<number>((m, l) => m | levelBit(l), 0);

/** Whether `mask` highlights `level`. */
export const levelFocused = (mask: number, level: number) => (mask & levelBit(level)) !== 0;

/** A stored mask made safe: only the seven level bits, and never empty (all levels at least). */
export const normalizeLevelFocus = (mask: unknown): number => {
  const m = typeof mask === "number" && Number.isFinite(mask) ? Math.trunc(mask) & ALL_LEVEL_BITS : 0;
  return m || ALL_LEVEL_BITS;
};

/** How strongly levels left out of the focus are drawn: their share of full opacity. */
export const FOCUS_DIM = 0.3;
