/**
 * Analytics data layer.
 *
 * The actual data is produced upstream by `ren-subjects/analytics/{subject}/
 * build_dashboard.py` and dumped as `summary.json`. We commit a copy of those
 * JSONs into `src/data/analytics/` so they're statically bundled into the
 * Next build (zero runtime cost, no need for the rex backend on this page,
 * and the per-subject pages can be statically generated).
 *
 * To refresh: re-run the analytics script in ren-subjects, then copy the
 * three summary.json files into src/data/analytics/. A future improvement
 * would be a sync script, but with three subjects and infrequent rebuilds
 * the manual copy is fine.
 */

import gpData from "@/data/analytics/gp.json";
import historyData from "@/data/analytics/h2history.json";
import physicsData from "@/data/analytics/h2physics.json";

export type AnalyticsSummary = {
  total_documents: number;
  questions: number;
  notes: number;
  with_answer: number;
  with_mark: number;
  year_parseable: number;
  year_range: number[]; // always 2 elements in practice; widened so JSON imports type-narrow cleanly
  questions_per_year: Record<string, number>;
  tag_field_counts: Record<string, number>;
  top_topics: Record<string, number>;
  top_schools: Record<string, number>;
  /** Distribution of `question_types` tags (e.g. essay, MCQ, structured). */
  question_types: Record<string, number>;
  /** Distribution of `paper_types` tags (e.g. paper-1, paper-2, paper-3). */
  paper_types: Record<string, number>;
  /** Top 12 topics × every year they appear in. Sparse: missing years
   * mean zero. Use `topicYearSeries()` to densify into a chart-ready shape. */
  topic_by_year: Record<string, Record<string, number>>;
  /** Top 8 question types × every year. Same sparse layout. */
  qtype_by_year: Record<string, Record<string, number>>;
  /** Histogram of mark values → number of questions awarded that mark. */
  mark_distribution: Record<string, number>;
  /** Top 30 topic pairs that co-occur on the same document. */
  topic_cooccurrence: Array<{ a: string; b: string; count: number }>;
  /** For the top 5 topics: the most common keywords found inside their
   * questions. Drawn from `keywords_json` after stopword filtering. */
  topic_keywords: Record<string, Array<{ keyword: string; count: number }>>;
};

export type SubjectId = "gp" | "h2history" | "h2physics";

export const ANALYTICS_SUBJECTS: ReadonlyArray<{
  id: SubjectId;
  label: string;
  blurb: string;
}> = [
  {
    id: "gp",
    label: "General Paper",
    blurb: "Essay questions across 26 JCs, 314 topics, 2008–2025.",
  },
  {
    id: "h2history",
    label: "H2 History",
    blurb: "Source-based and essay questions, cold war + SEA themes.",
  },
  {
    id: "h2physics",
    label: "H2 Physics",
    blurb: "Mechanics through quantum; structured + MCQ paper coverage.",
  },
] as const;

const SUMMARIES: Record<SubjectId, AnalyticsSummary> = {
  gp: gpData as AnalyticsSummary,
  h2history: historyData as AnalyticsSummary,
  h2physics: physicsData as AnalyticsSummary,
};

export function getAnalytics(id: SubjectId): AnalyticsSummary {
  return SUMMARIES[id];
}

export function hasAnalytics(id: string): id is SubjectId {
  return id in SUMMARIES;
}

export function allAnalytics(): ReadonlyArray<{
  id: SubjectId;
  label: string;
  blurb: string;
  summary: AnalyticsSummary;
}> {
  return ANALYTICS_SUBJECTS.map((s) => ({ ...s, summary: SUMMARIES[s.id] }));
}

/**
 * Convert a `Record<string, number>` (e.g. `top_topics`) into an array of
 * `{ name, count }` sorted descending. Capped at `limit` so the bars stay
 * readable — beyond ~20 the labels overlap on mobile widths.
 *
 * `humanize` strips kebab-case for display (`technology-and-society` →
 * `Technology and society`). The kebab form is the canonical tag value
 * inside rex; we only humanize for display, never for filtering.
 */
export function topN(
  record: Record<string, number>,
  limit = 15,
  humanize = true,
): Array<{ name: string; raw: string; count: number }> {
  return Object.entries(record)
    .sort(([, a], [, b]) => b - a)
    .slice(0, limit)
    .map(([raw, count]) => ({
      raw,
      name: humanize ? raw.replaceAll("-", " ").replace(/^\w/, (c) => c.toUpperCase()) : raw,
      count,
    }));
}

/**
 * Convert `questions_per_year` into a year-ordered array suitable for a
 * Recharts BarChart. Years are coerced to numbers so the axis sorts
 * numerically rather than alphabetically.
 */
export function yearSeries(
  qpy: Record<string, number>,
): Array<{ year: number; count: number }> {
  return Object.entries(qpy)
    .map(([y, c]) => ({ year: Number(y), count: c }))
    .sort((a, b) => a.year - b.year);
}

/**
 * Densify a sparse `{seriesKey: {year: count}}` map into a Recharts-ready
 * `[{year, <key1>, <key2>, …}]` shape, filling missing cells with zero.
 *
 * Recharts' multi-series line/area charts expect one row per X tick with
 * a column per series. Our upstream JSON is the opposite shape (one entry
 * per series, with sparse year keys) because that's what compresses
 * cheaply — densifying client-side keeps the JSON small without losing
 * fidelity.
 *
 * Years on the X axis are the *union* of every year that appears in any
 * series, sorted ascending. Series order is preserved from the input
 * `Object.entries` so the largest series renders first (which matters for
 * stacked-area legibility).
 */
export function densifyBySeries(
  bySeries: Record<string, Record<string, number>>,
): { keys: string[]; rows: Array<Record<string, number> & { year: number }> } {
  const keys = Object.keys(bySeries);
  const allYears = new Set<number>();
  for (const seriesData of Object.values(bySeries)) {
    for (const y of Object.keys(seriesData)) allYears.add(Number(y));
  }
  const years = [...allYears].sort((a, b) => a - b);
  const rows = years.map((year) => {
    const row: Record<string, number> & { year: number } = { year };
    for (const k of keys) {
      row[k] = bySeries[k]?.[String(year)] ?? 0;
    }
    return row;
  });
  return { keys, rows };
}

/**
 * Mark distribution → chart-ready array of `{mark, count}`. Marks are
 * coerced to numbers and sorted; mark-0 stays in if present (real data
 * sometimes has 0-mark rubric rows worth keeping visible).
 */
export function markSeries(
  dist: Record<string, number>,
): Array<{ mark: number; count: number }> {
  return Object.entries(dist)
    .map(([m, c]) => ({ mark: Number(m), count: c }))
    .sort((a, b) => a.mark - b.mark);
}

/**
 * Display-name overrides for school taxonomy values. The canonical id is
 * kebab-case (e.g. `hwa-chong-institution`), but the source corpus also
 * uses abbreviations (`HCI`, `RI`, `NYJC`) and partial names in prose.
 * Centralising the mapping here means every surface — chart axes,
 * markdown narrative, topic clouds — agrees on "Hwa Chong Institution"
 * instead of mixing forms.
 */
export const SCHOOL_NAMES: Record<string, string> = {
  // Canonical ids
  "anderson-junior-college": "Anderson Junior College",
  "anderson-serangoon-jc": "Anderson Serangoon Junior College",
  "anglo-chinese-junior-college": "Anglo-Chinese Junior College",
  "catholic-junior-college": "Catholic Junior College",
  "dunman-high-school": "Dunman High School",
  "eunoia-junior-college": "Eunoia Junior College",
  "hwa-chong-institution": "Hwa Chong Institution",
  "innova-junior-college": "Innova Junior College",
  "jurong-junior-college": "Jurong Junior College",
  "jurong-pioneer-junior-college": "Jurong Pioneer Junior College",
  "meridian-junior-college": "Meridian Junior College",
  "millennia-institute": "Millennia Institute",
  "nanyang-junior-college": "Nanyang Junior College",
  "national-junior-college": "National Junior College",
  "pioneer-junior-college": "Pioneer Junior College",
  "raffles-institution": "Raffles Institution",
  "raffles-junior-college": "Raffles Junior College",
  "river-valley-high-school": "River Valley High School",
  "serangoon-junior-college": "Serangoon Junior College",
  "st-andrews-junior-college": "St Andrew's Junior College",
  "tampines-junior-college": "Tampines Junior College",
  "tampines-meridian-junior-college": "Tampines Meridian Junior College",
  "temasek-junior-college": "Temasek Junior College",
  "victoria-junior-college": "Victoria Junior College",
  "yishun-innova-junior-college": "Yishun Innova Junior College",
  "yishun-junior-college": "Yishun Junior College",
  // Common abbreviations as they appear in prose narratives.
  acjc: "Anglo-Chinese Junior College",
  ajc: "Anderson Junior College",
  asjc: "Anderson Serangoon Junior College",
  cjc: "Catholic Junior College",
  dhs: "Dunman High School",
  ejc: "Eunoia Junior College",
  hci: "Hwa Chong Institution",
  jjc: "Jurong Junior College",
  jpjc: "Jurong Pioneer Junior College",
  mi: "Millennia Institute",
  njc: "National Junior College",
  nyjc: "Nanyang Junior College",
  pjc: "Pioneer Junior College",
  ri: "Raffles Institution",
  rvhs: "River Valley High School",
  sajc: "St Andrew's Junior College",
  srjc: "Serangoon Junior College",
  tjc: "Tampines Junior College",
  tmjc: "Tampines Meridian Junior College",
  vjc: "Victoria Junior College",
  yijc: "Yishun Innova Junior College",
  yjc: "Yishun Junior College",
};

/**
 * Display-string for any kebab-case tag value. Three rules:
 *   1. If the value is a known school id (or abbreviation), use its full
 *      proper name from SCHOOL_NAMES.
 *   2. Otherwise replace dashes with spaces and Title-Case each word.
 *   3. Single-word inputs get their first letter capitalised — preserves
 *      the previous behaviour for tag values that are already one word
 *      (`essay`, `calculation`).
 */
/** Minor words that stay lowercase in title case unless they're the first
 * word. Mirrors AP-style title case roughly (Chicago is slightly stricter
 * but the difference doesn't matter for tag display). */
const TITLE_CASE_LOWER = new Set([
  "a", "an", "the",
  "and", "but", "or", "nor", "for", "so", "yet",
  "at", "by", "in", "of", "on", "to", "up", "vs", "via", "with",
]);

export function humanizeTag(raw: string): string {
  if (!raw) return raw;
  const lower = raw.toLowerCase();
  if (SCHOOL_NAMES[lower]) return SCHOOL_NAMES[lower];
  return lower
    .split("-")
    .map((word, i) => {
      if (word.length === 0) return word;
      if (i > 0 && TITLE_CASE_LOWER.has(word)) return word;
      return word[0].toUpperCase() + word.slice(1);
    })
    .join(" ");
}
