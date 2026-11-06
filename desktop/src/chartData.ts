// Descriptive aggregation of stored instrument indices (port of gui/src/charts.rs).
// Group identity is part of the point identity; totals are never derived from A+B.
import type { Measurement } from "./types";

export type ChartRecord = Measurement & {
  group: number;
  source_file: string;
  index: number;
};
export interface ChartRow {
  label: string;
  group: number;
  point: number;
  sourceFile: string;
  ids: number[];
  values: [number, number, number];
  exact: [string, string, string] | null;
  errors: [number | null, number | null, number | null];
  warnings: number;
}
export const SERIES_NAMES = [
  "Clorofila a",
  "Clorofila b",
  "Clorofila total",
] as const;

class Statistics {
  n = 0;
  mean = 0;
  m2 = 0;
  push(value: number) {
    this.n += 1;
    const delta = value - this.mean;
    this.mean += delta / this.n;
    this.m2 += delta * (value - this.mean);
  }
  standardError(): number | null {
    return this.n > 1
      ? Math.sqrt(Math.max(this.m2, 0) / (this.n - 1) / this.n)
      : null;
  }
}

export function chartRows(records: ChartRecord[]): {
  raw: ChartRow[];
  means: ChartRow[];
} {
  const raw: ChartRow[] = [];
  const means = new Map<string, { row: ChartRow; stats: Statistics[] }>();
  for (const r of records) {
    const values: [number, number, number] = [
      r.chlorophyll_a_tenths / 10,
      r.chlorophyll_b_tenths / 10,
      r.chlorophyll_total_tenths / 10,
    ];
    raw.push({
      label: String(r.id),
      group: r.group,
      point: r.point,
      sourceFile: r.source_file,
      ids: [r.id],
      values,
      exact: [
        r.chlorophyll_a_index,
        r.chlorophyll_b_index,
        r.chlorophyll_total_index,
      ],
      errors: [null, null, null],
      warnings: r.warnings.length,
    });
    const key = `${r.group}\u0000${r.point}`;
    let entry = means.get(key);
    if (!entry) {
      entry = {
        row: {
          label: `G${r.group} · P${r.point}`,
          group: r.group,
          point: r.point,
          sourceFile: r.source_file,
          ids: [],
          values: [0, 0, 0],
          exact: null,
          errors: [null, null, null],
          warnings: 0,
        },
        stats: [new Statistics(), new Statistics(), new Statistics()],
      };
      means.set(key, entry);
    }
    entry.row.ids.push(r.id);
    entry.row.warnings += r.warnings.length;
    values.forEach((v, i) => entry!.stats[i].push(v));
  }
  const aggregated = [...means.values()].map(({ row, stats }) => {
    row.values = stats.map((s) => s.mean) as ChartRow["values"];
    row.errors = stats.map((s) => s.standardError()) as ChartRow["errors"];
    return row;
  });
  return { raw, means: aggregated };
}

/** "Nice" axis ticks from 0 up to at least `max`. */
export function niceTicks(max: number, target = 5) {
  if (!(max > 0)) return { top: 1, ticks: [0, 1] };
  const rough = max / target;
  const pow = 10 ** Math.floor(Math.log10(rough));
  const step = ([1, 2, 2.5, 5, 10].find((m) => m * pow >= rough) ?? 10) * pow;
  const top = Math.ceil(max / step) * step;
  const ticks: number[] = [];
  for (let v = 0; v <= top + step / 1000; v += step)
    ticks.push(Number(v.toFixed(10)));
  return { top, ticks };
}
