/**
 * Cleans the raw `analysis.md` files shipped from ren-subjects into a
 * form that fits inside the rex analytics page:
 *
 *   1. Strips the *meta* sections (preamble, headline-numbers table,
 *      data-quality flags, how-to-extend, open-via, school house-style).
 *      Those answer questions about the *corpus* or the *pipeline*, not
 *      about the subject; on the rex page they're either redundant with
 *      the KPI strip or out of place.
 *   2. Normalises every kebab-case tag value to a human-readable form.
 *      `climate-change` → `Climate Change`. The original markdown leans
 *      heavily on the canonical ids (which match what's stored in the
 *      database), but for reading they're noise.
 *   3. Replaces school identifiers and common abbreviations with their
 *      full names. `hwa-chong-institution` and bare `HCI` both become
 *      "Hwa Chong Institution". The id list is the same as the one
 *      `SCHOOL_NAMES` in lib/analytics.ts uses for chart axes, so the
 *      whole page agrees on a single rendering.
 *
 * The function is pure and runs at build time (the page is a Server
 * Component with `force-static`), so there's no runtime cost. Kept in
 * its own file so the page stays focused on layout and the preprocessor
 * can be unit-tested independently if we ever care to.
 */

import { SCHOOL_NAMES, humanizeTag } from "./analytics";

/**
 * Heading text fragments that mark sections we want to drop entirely.
 * Matching is case-insensitive and substring-based, so the entries can
 * be short — anything that uniquely names the section.
 *
 * Order doesn't matter; every match is dropped.
 */
const DROP_SECTION_FRAGMENTS = [
  "headline numbers", // table that duplicates the KPI strip
  "data-quality flag", // pipeline-side notes
  "caveats",
  "how to extend",
  "open via",
  "house style", // explicit user request
  "school signature", // related school-house tangent in physics/history
];

/**
 * Tag values that look like kebab-case but should NOT be re-cased
 * (database identifiers used inside backticks where the canonical form
 * is intentionally exposed).
 */
const PRESERVE_KEBAB = new Set([
  "ren-rex/server/rex.db", // path
  "build_dashboard.py", // filename
]);

/**
 * Section keywords → which chart slot the section's content is most
 * relevant to. Used by the page to render each section right after the
 * matching chart (interleave commentary with graphs).
 */
export type InsightSlot =
  | "trend" // topic share over time
  | "qtype" // question type distribution / mix
  | "marks" // mark distribution
  | "cooccur" // topic co-occurrences
  | "essay" // sample prompts / essay families
  | "topics" // top topics, gravity centre
  | "other";

function detectSlot(heading: string): InsightSlot {
  const h = heading.toLowerCase();
  if (/(riser|declin|trend|syllabus shift|regime change|rising|fading)/.test(h)) return "trend";
  if (/(mark distribution|mark distribut|mark|kind of test|structure)/.test(h)) return "marks";
  if (/(co-?occur|study these together|argument bundle|essay vocabulary)/.test(h)) return "cooccur";
  if (/(essay|prompt|question shape)/.test(h)) return "essay";
  if (/(top topic|gravity centre|gravity center)/.test(h)) return "topics";
  if (/(question type|qtype)/.test(h)) return "qtype";
  return "other";
}

export interface InsightSection {
  heading: string; // already humanized
  body: string;    // raw markdown, ready to feed react-markdown
  slot: InsightSlot;
}

/**
 * Parse the cleaned markdown into discrete `## ` and `### ` chunks so
 * the page can route them to specific chart cards.
 *
 * H2-level sections are wrappers ("What the data is telling you"); the
 * H3 children are the *real* insights. We extract the H3 chunks and
 * discard their H2 parents — the page provides the layout.
 */
export function parseInsights(cleaned: string): InsightSection[] {
  const out: InsightSection[] = [];
  const lines = cleaned.split("\n");
  let currentHeading: string | null = null;
  let currentLines: string[] = [];
  for (const line of lines) {
    const m = /^###\s+(.+?)\s*$/.exec(line);
    if (m) {
      if (currentHeading) {
        out.push({
          heading: currentHeading,
          body: currentLines.join("\n").trim(),
          slot: detectSlot(currentHeading),
        });
      }
      // Strip the leading "1. " / "2. " counter — they were useful for
      // the standalone .md but read as noise once the prose lives next
      // to its chart.
      currentHeading = m[1].replace(/^\d+\.\s*/, "");
      currentLines = [];
    } else if (currentHeading) {
      currentLines.push(line);
    }
  }
  if (currentHeading) {
    out.push({
      heading: currentHeading,
      body: currentLines.join("\n").trim(),
      slot: detectSlot(currentHeading),
    });
  }
  return out;
}

/**
 * Replace every `[a-z]+(-[a-z]+)+` token with its humanized form, except
 * when the token is in PRESERVE_KEBAB or looks like a URL/path.
 *
 * We do this on the raw markdown text so it applies inside backticks,
 * inside code fences, and in prose alike — wherever a tag value appears,
 * the reader sees the proper name.
 */
function normalizeKebab(md: string): string {
  return md.replace(/\b[a-z]+(?:-[a-z]+){1,}\b/g, (token) => {
    if (PRESERVE_KEBAB.has(token)) return token;
    if (token.includes("/") || token.includes(".")) return token;
    // School names go through SCHOOL_NAMES; everything else gets the
    // generic Title Case treatment via humanizeTag.
    return humanizeTag(token);
  });
}

/**
 * Replace bare school abbreviations (HCI, NYJC, etc.) in prose with
 * their full names. We scope this to whole-word matches so we don't
 * mangle unrelated uppercase tokens.
 */
function expandSchoolAbbreviations(md: string): string {
  let out = md;
  for (const abbr of Object.keys(SCHOOL_NAMES)) {
    if (abbr.includes("-")) continue; // canonical id already handled above
    const upper = abbr.toUpperCase();
    const re = new RegExp(`\\b${upper}\\b`, "g");
    out = out.replace(re, SCHOOL_NAMES[abbr]);
  }
  return out;
}

/**
 * Strip the preamble (everything before the first H2) and every section
 * whose heading text matches one of DROP_SECTION_FRAGMENTS.
 *
 * Sections are bounded by `^##? ` headings. We walk top-down, keeping or
 * dropping each chunk based on its leading heading.
 */
function stripSections(md: string): string {
  const lines = md.split("\n");
  const chunks: { heading: string; body: string[] }[] = [];
  let pending: { heading: string; body: string[] } | null = null;

  for (const line of lines) {
    const isH2H3 = /^##?\s+/.test(line); // H1 too (will be dropped as preamble)
    if (isH2H3) {
      if (pending) chunks.push(pending);
      pending = { heading: line, body: [line] };
    } else if (pending) {
      pending.body.push(line);
    }
    // Else: preamble before any heading — drop.
  }
  if (pending) chunks.push(pending);

  const kept: string[] = [];
  for (const c of chunks) {
    const text = c.heading.toLowerCase();
    // Drop top-level title (single `#`) and any matching fragment.
    if (/^#\s/.test(c.heading)) continue;
    if (DROP_SECTION_FRAGMENTS.some((frag) => text.includes(frag))) continue;
    kept.push(c.body.join("\n"));
  }
  return kept.join("\n\n").trim();
}

/**
 * Drop the per-section "school house style" subsection that lives under
 * "What the data is telling you" in every analysis file. Because that
 * lives at H3 (not H2), the H2-level `stripSections` doesn't catch it.
 *
 * Removes from `### N. ... house style ...` (or `### N. ... school signatures ...`)
 * through the line just before the next `### ` or `## ` heading.
 */
function stripHouseStyleSubsection(md: string): string {
  const lines = md.split("\n");
  const out: string[] = [];
  let dropping = false;
  for (const line of lines) {
    const isH2H3 = /^##?#?\s+/.test(line);
    if (isH2H3) {
      const lower = line.toLowerCase();
      if (/^###\s/.test(line) && /(house style|school signature|each school)/.test(lower)) {
        dropping = true;
        continue;
      }
      // Any other heading resets the drop state.
      dropping = false;
    }
    if (!dropping) out.push(line);
  }
  return out.join("\n");
}

export function cleanAnalysisMarkdown(raw: string): string {
  let md = raw;
  md = stripSections(md);
  md = stripHouseStyleSubsection(md);
  md = expandSchoolAbbreviations(md);
  md = normalizeKebab(md);
  return md;
}
