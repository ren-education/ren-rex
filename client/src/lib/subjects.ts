// Maps the backend's terse subject ids (e.g. "h2physics") to the canonical
// MOE-style display names used everywhere in the UI. Keep this table as the
// single source of truth — never hardcode "H2 Physics" in a component.
//
// Unknown ids (a new subject we haven't labelled yet) fall back to a
// best-effort prettification of the id itself so the UI degrades gracefully
// rather than throwing.

const SUBJECT_LABELS: Record<string, string> = {
  gp: "H1 General Paper",
  h2physics: "H2 Physics",
  h2history: "H2 History",
  h2chemistry: "H2 Chemistry",
  h2literature: "H2 Literature",
  h2math: "H2 Math",
  h2biology: "H2 Biology",
  h2econs: "H2 Economics",
  h2cll: "H2 Chinese Language Literature",
};

const SUBJECT_SHORT_LABELS: Record<string, string> = {
  gp: "GP",
  h2physics: "Physics",
  h2history: "History",
  h2chemistry: "Chemistry",
  h2literature: "Literature",
  h2math: "Math",
  h2biology: "Biology",
  h2econs: "Economics",
  h2cll: "Chinese Lit",
};

/** Long form, suitable for dropdowns and headings. "h2physics" → "H2 Physics". */
export function formatSubject(id: string): string {
  return SUBJECT_LABELS[id] ?? prettifyUnknown(id);
}

/** Short form, suitable for chips/meta strips. "h2physics" → "Physics". */
export function formatSubjectShort(id: string): string {
  return SUBJECT_SHORT_LABELS[id] ?? prettifyUnknown(id);
}

// Generic fallback: "hcchem" → "Hcchem", "h3econ" → "H3econ". Not pretty,
// but obviously a placeholder so we notice and add the proper mapping.
function prettifyUnknown(id: string): string {
  if (!id) return id;
  return id.charAt(0).toUpperCase() + id.slice(1);
}
