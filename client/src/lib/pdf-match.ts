/**
 * Client-side fuzzy page-finder for the rex PDF viewer.
 *
 * Same 3-gram Jaccard similarity the server uses at ingest time, but run
 * against pdfjs's extracted text (which is usually significantly cleaner
 * than what `pdf-extract` produced server-side). When a hit has no
 * `pdf_anchor.page_number` (the LowConfidence case), this re-runs the
 * match in the browser using the full question prose and can identify
 * the right page that the server missed.
 */

/** Strip non-alphanumerics, lowercase, single-space, trim. */
export function normalize(s: string): string {
  return s
    .toLowerCase()
    .replace(/[^a-z0-9 ]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

/** 3-gram character n-grams of a normalized string. */
export function ngrams3(s: string): Set<string> {
  const norm = normalize(s);
  const out = new Set<string>();
  if (norm.length < 3) return out;
  for (let i = 0; i <= norm.length - 3; i++) {
    out.add(norm.slice(i, i + 3));
  }
  return out;
}

/** Jaccard similarity between two n-gram sets. */
export function jaccard(a: Set<string>, b: Set<string>): number {
  if (a.size === 0 || b.size === 0) return 0;
  let inter = 0;
  for (const g of a) {
    if (b.has(g)) inter++;
  }
  return inter / (a.size + b.size - inter);
}

/**
 * Page locator. Iterates the PDF's pages, extracts text from each via
 * pdfjs's TextContent API, and returns the best Jaccard match.
 *
 * `signal` lets callers abort if the user clicks another hit while the
 * scan is in flight.
 */
// Minimal duck-typed surface we need from pdfjs's PDFDocumentProxy. Kept
// loose on purpose so callers can pass the real proxy without import-time
// dependence on `pdfjs-dist` types.
export interface PdfLike {
  numPages: number;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  getPage(n: number): Promise<any>;
}

export async function locateBestPage(
  pdf: PdfLike,
  target: string,
  opts?: { signal?: AbortSignal; threshold?: number },
): Promise<{ page: number; score: number } | null> {
  const threshold = opts?.threshold ?? 0.08;
  const signal = opts?.signal;
  const truncated = target.slice(0, 800);
  const targetGrams = ngrams3(truncated);
  if (targetGrams.size === 0) return null;

  let best = { page: 1, score: 0 };
  for (let p = 1; p <= pdf.numPages; p++) {
    if (signal?.aborted) return null;
    const page = await pdf.getPage(p);
    const tc = await page.getTextContent();
    // tc.items is (TextItem | TextMarkedContent)[]; we only care about
    // TextItem which has `str`. TextMarkedContent items don't and are skipped.
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const pageText = (tc.items as any[])
      .map((it) => (typeof it.str === "string" ? it.str : ""))
      .join(" ");
    const score = jaccard(targetGrams, ngrams3(pageText));
    if (score > best.score) best = { page: p, score };
  }
  return best.score >= threshold ? best : null;
}

// ────────────────────────────────────────────────────────────────────────
// In-page term highlighting (feature B)
// ────────────────────────────────────────────────────────────────────────

const STOPWORDS = new Set([
  "the", "a", "an", "and", "or", "of", "to", "in", "on", "at", "for", "by",
  "as", "is", "it", "be", "are", "was", "were", "from", "with", "this",
  "that", "these", "those", "how", "what", "when", "where", "why", "do",
  "does", "did", "far", "you", "agree", "extent",
]);

/** Tokenize the user's query into highlightable terms. */
export function tokenizeQuery(q: string): string[] {
  return q
    .split(/[^a-zA-Z0-9]+/)
    .map((t) => t.toLowerCase())
    .filter((t) => t.length >= 3 && !STOPWORDS.has(t));
}

function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (c) => {
    switch (c) {
      case "&": return "&amp;";
      case "<": return "&lt;";
      case ">": return "&gt;";
      case '"': return "&quot;";
      case "'": return "&#39;";
      default:  return c;
    }
  });
}

function escapeRegex(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/**
 * Highlight every occurrence of `terms` inside `text` by wrapping matches
 * in `<mark class="rex-mark">…</mark>`. Returns HTML that pdfjs's TextLayer
 * will render in place of the raw text item. The original text is
 * HTML-escaped first to prevent injection from PDF content.
 */
export function highlightInText(text: string, terms: string[]): string {
  const escaped = escapeHtml(text);
  if (terms.length === 0) return escaped;
  const pattern = terms.map(escapeRegex).join("|");
  if (!pattern) return escaped;
  const re = new RegExp(`(${pattern})`, "gi");
  return escaped.replace(re, '<mark class="rex-mark">$1</mark>');
}
