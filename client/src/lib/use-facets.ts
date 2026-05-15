"use client";

import { useEffect, useState } from "react";
import { tagValues } from "./rex";
import type { Filters } from "./types";

/** The six tag fields rex's domain owns; keep in sync with rex-domain's TagField. */
export const FACET_FIELDS = [
  "topics",
  "schools",
  "paper_types",
  "source_types",
  "exam_systems",
  "question_types",
] as const;

export type FacetField = (typeof FACET_FIELDS)[number];

export interface FacetValue {
  value: string;
  count: number;
}

export type FacetMap = Partial<Record<FacetField, FacetValue[]>>;

interface UseFacetsResult {
  facets: FacetMap;
  loading: boolean;
  error: string | null;
}

/**
 * Fetch all six facet fields for a subject in parallel, filter-aware.
 *
 * Counts respect every selection except the one for the field itself —
 * i.e., the Schools facet shows counts as if you HAD applied all of the
 * other facets but NOT the schools filter. That's the standard "facet
 * partition" pattern: each facet's counts represent "what would happen
 * if I added a value here", not "what's left after applying everything
 * including this facet". Otherwise selecting a school would collapse the
 * Schools facet to count=1 for that school and zero everywhere else.
 *
 * Implemented client-side by sending each field's request with that
 * field's selections stripped from the filters.
 */
export function useFacets(
  subject: string,
  filters: Filters,
): UseFacetsResult {
  const [facets, setFacets] = useState<FacetMap>({});
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Memoize the filters as a JSON string so the effect dep is stable.
  // This is cheap; filters are small objects.
  const filtersKey = JSON.stringify(filters);

  useEffect(() => {
    if (!subject) {
      setFacets({});
      return;
    }
    let cancelled = false;
    setLoading(true);
    setError(null);

    Promise.all(
      FACET_FIELDS.map(async (field) => {
        // Strip THIS field's selections so the facet shows "what could I
        // add here", not "what's left after picking everything including
        // here".
        const stripped: Filters = { ...filters };
        delete (stripped as Record<string, unknown>)[field];
        try {
          const res = await tagValues(subject, field, stripped);
          return [field, res.values] as const;
        } catch {
          return [field, [] as FacetValue[]] as const;
        }
      }),
    ).then((entries) => {
      if (cancelled) return;
      const next: FacetMap = {};
      for (const [field, values] of entries) next[field] = values;
      setFacets(next);
      setLoading(false);
    });

    return () => {
      cancelled = true;
    };
    // filtersKey is intentional — we want to re-run when filter values change.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [subject, filtersKey]);

  return { facets, loading, error };
}
