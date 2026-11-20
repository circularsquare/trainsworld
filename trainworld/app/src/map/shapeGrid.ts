/** Broad phase for the station facilities and buildings. Callers apply the exact tests.
 * A stamp visits a shape spanning several cells only once per query. */
export class ShapeGrid<T> {
  private cells = new Map<number, { shape: T; seen: number }[]>();
  private wide: { shape: T; seen: number }[] = [];
  private all: T[] = [];
  private serial = 0;

  constructor(private cell: number) {}

  add(shape: T, x0: number, y0: number, x1: number, y1: number) {
    const entry = { shape, seen: 0 };
    this.all.push(shape);
    const l = Math.floor(x0 / this.cell), r = Math.floor(x1 / this.cell);
    const t = Math.floor(y0 / this.cell), b = Math.floor(y1 / this.cell);
    // A very large building (or a degenerate rectangle) can be tested directly.
    if (!Number.isFinite(r - l + b - t) || (r - l + 1) * (b - t + 1) > 256) {
      this.wide.push(entry);
      return;
    }
    for (let x = l; x <= r; x++) for (let y = t; y <= b; y++) {
      const key = x * 1_000_003 + y, list = this.cells.get(key);
      if (list) list.push(entry); else this.cells.set(key, [entry]);
    }
  }

  near(x: number, y: number, radius: number): T[] {
    return this.box(x - radius, y - radius, x + radius, y + radius);
  }

  box(x0: number, y0: number, x1: number, y1: number): T[] {
    const l = Math.floor(x0 / this.cell), r = Math.floor(x1 / this.cell);
    const t = Math.floor(y0 / this.cell), b = Math.floor(y1 / this.cell);
    if (!Number.isFinite(r - l + b - t) || (r - l + 1) * (b - t + 1) > 256) return this.all.slice();
    const found = this.wide.map(e => e.shape), stamp = ++this.serial;
    for (let i = l; i <= r; i++)
      for (let j = t; j <= b; j++) {
        const list = this.cells.get(i * 1_000_003 + j);
        if (list) for (const e of list) if (e.seen !== stamp) {
          e.seen = stamp;
          found.push(e.shape);
        }
      }
    return found;
  }
}
