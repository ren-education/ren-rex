"use client";

import { useCallback, useEffect, useMemo, useState, useTransition } from "react";
import Link from "next/link";
import { Search, FileText, Sparkles, Loader2, BarChart3, ArrowUpRight } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { hasAnalytics } from "@/lib/analytics";
import { captureRexEvent } from "@/lib/posthog-client";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";
import { Tip } from "@/components/tip";
import { PdfViewerLoader } from "@/components/pdf-viewer-loader";
import { FacetBar } from "@/components/facet-bar";
import { useFacets, type FacetField } from "@/lib/use-facets";
import { filter as filterApi, search as searchApi } from "@/lib/rex";
import { formatSubject } from "@/lib/subjects";
import type {
  DocumentKind,
  Filters,
  SearchHit,
  SearchMeta,
  SearchMode,
  SubjectStats,
} from "@/lib/types";

interface Props {
  subjects: SubjectStats[];
  apiOnline: boolean;
}

const MODE_OPTIONS: { value: SearchMode; label: string; tip: string }[] = [
  {
    value: "Hybrid",
    label: "Hybrid",
    tip: "Best results. BM25 + semantic vector + cross-encoder rerank fused together.",
  },
  {
    value: "Bm25Only",
    label: "Keyword",
    tip: "Pure keyword search (FTS5/BM25). Fast and exact — use when you know the term.",
  },
  {
    value: "VectorOnly",
    label: "Semantic",
    tip: "Vector similarity only. Finds conceptual matches even when keywords don't overlap.",
  },
];

const DEMO_HITS: SearchHit[] = [
  {
    document: {
      id: "demo-1", subject: "h2history", kind: "Question",
      parent_id: null, number: "5", source: "content/prelims/2019/DHS/X.md",
      context: null,
      question: "How far do you agree that UN peacekeeping was better in the post-Cold War period than during the Cold War?",
      answer: null, notes: null, mark: 10, options: null, keywords: [],
      tags: { topics: ["cold-war"], question_types: [], exam_systems: [], paper_types: ["paper-1"], schools: ["dhs"], source_types: ["prelims"] },
      pdf_anchor: { pdf_path: "h2history/prelims/2019/DHS_H2_HIST_P1.pdf", page_number: null, bbox: null, confidence: 0.41, fallback_reason: "LowConfidence" },
    },
    score: 2.189,
    scores: { bm25: 2.189, vector: null, rerank: null },
    highlights: [
      { field: "Question", text: 'How far do you agree that UN peacekeeping was better in the post-<em class="match">Cold</em> <em class="match">War</em> period…' },
    ],
  },
  {
    document: {
      id: "demo-2", subject: "h2history", kind: "Question",
      parent_id: null, number: "2", source: "content/prelims/2013/MI/X.md",
      context: null,
      question: "To what extent has the end of the Cold War influenced the historical debate on the origins of the Cold War?",
      answer: null, notes: null, mark: 30, options: null, keywords: [],
      tags: { topics: ["cold-war"], question_types: [], exam_systems: [], paper_types: ["paper-2"], schools: ["mi"], source_types: ["prelims"] },
      pdf_anchor: null,
    },
    score: 2.173,
    scores: { bm25: 2.173, vector: null, rerank: null },
    highlights: [
      { field: "Question", text: 'To what extent has the end of the <em class="match">Cold</em> <em class="match">War</em> influenced the historical debate on the origins of the <em class="match">Cold</em> <em class="match">War</em>?' },
    ],
  },
];

const DEMO_META: SearchMeta = {
  mode: "Bm25Only", used_embedder: false, used_bm25: true,
  used_vector: false, used_reranker: false,
  fts5_query: "cold war", total_matches: null, took_ms: 15,
};

/** Selected-value sets per facet field. We keep these separate from the
 *  on-wire `Filters` shape so the UI's invariants (no nulls, sorted, deduped)
 *  are local; we synthesize the API payload in `buildFilters`. */
type FacetSelections = Partial<Record<FacetField, string[]>>;

/** Page size for the FILTER path (browse). Search is unpaginated and uses
 *  its own limit (see runQuery below). */
const FILTER_PAGE_SIZE = 20;

export function SearchPanel({ subjects, apiOnline }: Props) {
  const [query, setQuery] = useState("");
  const [subject, setSubject] = useState<string>(subjects[0]?.id ?? "");
  const [mode, setMode] = useState<SearchMode>("Hybrid");
  const [selections, setSelections] = useState<FacetSelections>({});
  const [kind, setKind] = useState<DocumentKind | null>(null);
  // 0-indexed page within the FILTER path only. Reset to 0 on any change
  // that could shrink the total set (subject switch, facet toggle, kind
  // change, search submit). Pagination clicks are the only thing that
  // advance it.
  const [page, setPage] = useState(0);
  const [hits, setHits] = useState<SearchHit[]>(apiOnline ? [] : DEMO_HITS);
  const [meta, setMeta] = useState<SearchMeta | null>(apiOnline ? null : DEMO_META);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isPending, startTransition] = useTransition();

  const selectedHit = hits.find((h) => h.document.id === selectedId) ?? null;
  const normalizedSubject = subject && subject !== "__all__" ? subject : "";

  // Build the API Filters payload from the current selections + subject + kind.
  const apiFilters: Filters = useMemo(
    () => buildFilters(normalizedSubject, selections, kind),
    [normalizedSubject, selections, kind],
  );

  // Fetch facet counts whenever subject or filters change. The hook strips
  // the current field's selections per-facet so each one stays explorable.
  const { facets } = useFacets(normalizedSubject, apiFilters);

  // Reset facet selections AND kind AND page when the subject changes —
  // values from h2physics don't carry over to hcchem etc., and a Notes-only
  // filter could surprise the user after switching subjects.
  useEffect(() => {
    setSelections({});
    setKind(null);
    setPage(0);
  }, [normalizedSubject]);

  function toggleFacet(field: FacetField, value: string) {
    setSelections((prev) => {
      const cur = prev[field] ?? [];
      const next = cur.includes(value)
        ? cur.filter((v) => v !== value)
        : [...cur, value];
      return { ...prev, [field]: next };
    });
    setPage(0);
  }

  function clearFacet(field: FacetField) {
    setSelections((prev) => {
      const next = { ...prev };
      delete next[field];
      return next;
    });
    setPage(0);
  }

  function clearAllFacets() {
    setSelections({});
    setKind(null);
    setPage(0);
  }

  function setKindAndResetPage(k: DocumentKind | null) {
    setKind(k);
    setPage(0);
  }

  // Stable async runner used by both the explicit "Search" submit and the
  // implicit filter-only fetch triggered by facet/page changes.
  const runQuery = useCallback(
    (
      text: string,
      currentMode: SearchMode,
      filters: Filters,
      currentPage: number,
    ) => {
      setError(null);
      startTransition(async () => {
        try {
          if (text.trim()) {
            // Search path — page is ignored. Single ranked list, top-N.
            const res = await searchApi({
              text,
              mode: currentMode,
              filters,
              limit: 20,
            });
            setHits(res.hits);
            setMeta(res.meta);
            setSelectedId(res.hits[0]?.document.id ?? null);
          } else if (filters.subject || hasAnySelection(filters)) {
            // Filter path — paginated. limit + offset come from currentPage.
            const res = await filterApi({
              filters,
              limit: FILTER_PAGE_SIZE,
              offset: currentPage * FILTER_PAGE_SIZE,
            });
            setHits(res.hits);
            setMeta(res.meta);
            setSelectedId(res.hits[0]?.document.id ?? null);
          } else {
            // No subject, no filters, no query → genuinely empty state.
            setHits([]);
            setMeta(null);
            setSelectedId(null);
          }
        } catch (err) {
          setError(err instanceof Error ? err.message : String(err));
        }
      });
    },
    [],
  );

  // Re-fetch whenever filters or page change (including the initial mount
  // with a pre-selected subject). Query/mode changes still wait for explicit
  // Search submit. apiOnline gates the auto-fetch so demo data stays put
  // when the server isn't reachable.
  useEffect(() => {
    if (!apiOnline) return;
    runQuery(query, mode, apiFilters, page);
    // We intentionally omit `query` and `mode` from deps — filter changes
    // auto-re-run, but query/mode changes wait for explicit Search submit.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [apiFilters, page, runQuery, apiOnline]);

  function submit(e: React.FormEvent) {
    e.preventDefault();
    if (query.trim()) {
      captureRexEvent("search_submitted", {
        query: query.trim(),
        mode,
        subject: subject || null,
      });
    }
    // Reset to page 0 for any new search/filter. If page was already 0, the
    // useEffect won't re-fire on its own (deps unchanged), so call runQuery
    // explicitly. If page was non-zero, setPage(0) triggers the useEffect.
    if (page !== 0) {
      setPage(0);
    } else {
      runQuery(query, mode, apiFilters, 0);
    }
  }

  return (
    <section className="flex flex-col gap-6">
      {/* ─── Search controls ────────────────────────────────────── */}
      <form onSubmit={submit} className="flex flex-col gap-3">
        <div className="flex items-center gap-2 border-b-2 border-foreground/80 pb-2 focus-within:border-primary transition-colors">
          <Search className="size-5 text-muted-foreground" />
          <Input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search the archive…"
            className="font-heading flex-1 border-0 bg-transparent text-xl shadow-none focus-visible:ring-0 focus-visible:border-0 px-0 placeholder:italic placeholder:text-muted-foreground/70 h-auto"
          />
        </div>

        <div className="flex flex-wrap items-center gap-3">
          <Select value={subject} onValueChange={(v) => setSubject(v ?? "")}>
            {/* min-w-56 leaves room for the widest item ("H1 General Paper")
                plus the count plus the 32px indicator reserve (pr-8 on the
                SelectItem). The dropdown inherits its width from the trigger
                (base-ui: w-(--anchor-width)), so widening here widens both. */}
            <SelectTrigger className="w-full sm:w-fit sm:min-w-56">
              {/* base-ui's SelectValue shows the raw value string by default,
                  so even though SelectItem renders formatSubject(id), the
                  trigger would still say "gp". Pass a render fn so the
                  trigger goes through the same mapping. */}
              <SelectValue placeholder="All subjects">
                {(value) =>
                  !value || value === "__all__"
                    ? "All subjects"
                    : formatSubject(value)
                }
              </SelectValue>
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="__all__">All subjects</SelectItem>
              {subjects.map((s) => (
                <SelectItem key={s.id} value={s.id}>
                  <span>{formatSubject(s.id)}</span>
                  {/* ml-auto right-aligns the count inside the ItemText
                      flex row so labels and counts line up in a column —
                      and so the count never crashes into the check. */}
                  <span className="num text-muted-foreground ml-auto">
                    {s.item_count.toLocaleString()}
                  </span>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          <Tabs value={mode} onValueChange={(v) => setMode(v as SearchMode)}>
            <TabsList>
              {MODE_OPTIONS.map((m) => (
                <Tip key={m.value} label={m.tip} side="bottom">
                  <TabsTrigger value={m.value}>{m.label}</TabsTrigger>
                </Tip>
              ))}
            </TabsList>
          </Tabs>

          {/* Contextual analytics link. Lives AFTER the mode tabs because
            * it's adjacent in purpose ("what's in this corpus?" sits next
            * to "how should I search it?") but logically distinct from the
            * search controls — so it gets text-link styling rather than
            * the bordered button chrome the other controls use. The href
            * derives from the current `subject` state at render time. */}
          <Tip
            label={
              hasAnalytics(normalizedSubject)
                ? `Coverage and topic breakdown for ${formatSubject(normalizedSubject)}`
                : "Browse aggregated stats for all subjects"
            }
            side="bottom"
          >
            <Link
              href={
                hasAnalytics(normalizedSubject)
                  ? `/analytics/${normalizedSubject}`
                  : "/analytics"
              }
              className={cn(
                "hidden sm:inline-flex items-center gap-1.5 text-sm text-muted-foreground",
                "transition-colors hover:text-foreground",
                "[&_svg]:transition-transform [&:hover_.arrow]:translate-x-0.5 [&:hover_.arrow]:-translate-y-0.5",
              )}
            >
              <BarChart3 className="size-3.5" aria-hidden />
              <span>
                {hasAnalytics(normalizedSubject)
                  ? `${formatSubject(normalizedSubject)} analytics`
                  : "Analytics"}
              </span>
              <ArrowUpRight className="arrow size-3" aria-hidden />
            </Link>
          </Tip>

          <Button
            type="submit"
            disabled={isPending || !query.trim()}
            size="default"
            className="w-full sm:w-auto sm:ml-auto"
          >
            {isPending ? (
              <>
                <Loader2 className="size-3.5 animate-spin" />
                Searching
              </>
            ) : (
              <>
                <Sparkles className="size-3.5" />
                Search
              </>
            )}
          </Button>
        </div>
      </form>

      {error && (
        <div className="rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">
          {error}
        </div>
      )}

      {/* ─── Facet filters (when a subject is selected) ─────────── */}
      {normalizedSubject && (
        <FacetBar
          filters={apiFilters}
          facets={facets}
          onToggle={toggleFacet}
          onClear={clearFacet}
          onClearAll={clearAllFacets}
          onKindChange={setKindAndResetPage}
        />
      )}

      {/* ─── Meta strip ─────────────────────────────────────────── */}
      {/* `mode` already encodes which stages ran (Hybrid → all three;
          Bm25Only → only bm25; etc.) so we don't surface separate
          per-stage badges here. */}
      {meta && (
        <div className="smallcaps flex flex-wrap items-center gap-x-4 gap-y-1">
          <span className="text-foreground">
            <span className="num">{hits.length}</span> matches
          </span>
          <span><span className="num">{meta.took_ms}</span>&nbsp;ms</span>
          <span>mode <span className="text-foreground">{meta.mode}</span></span>
          {meta.fts5_query && (
            <span className="num text-muted-foreground/70 truncate max-w-[40ch]">
              fts5 → {meta.fts5_query}
            </span>
          )}
        </div>
      )}

      {/* ─── Split: results | viewer ────────────────────────────── */}
      <div className="grid grid-cols-1 gap-8 lg:grid-cols-[1fr_1fr] xl:grid-cols-[minmax(0,1fr)_minmax(0,1.05fr)]">
        <div className="flex flex-col">
          <ul className="flex flex-col">
            {hits.map((hit) => (
              <li
                key={hit.document.id}
                className={cn(
                  "paper cursor-pointer transition-colors",
                  hit.document.id === selectedId
                    ? "bg-accent/30 -mx-3 px-3"
                    : "hover:bg-accent/15 -mx-3 px-3",
                )}
                onClick={() => setSelectedId(hit.document.id)}
              >
                <HitCard hit={hit} isSelected={hit.document.id === selectedId} />
              </li>
            ))}
            {!hits.length && !error && (
              <li className="py-10 text-center text-sm text-muted-foreground italic">
                Type a query above to search the archive.
              </li>
            )}
          </ul>

          {/* Pagination bar — only on the filter path, when there's more
              than one page worth of results. Search results aren't paginated. */}
          {meta?.mode === "Filter"
            && meta.total_matches != null
            && meta.total_matches > FILTER_PAGE_SIZE && (
              <FilterPagination
                page={page}
                pageSize={FILTER_PAGE_SIZE}
                total={meta.total_matches}
                hitsOnPage={hits.length}
                onPrev={() => setPage((p) => Math.max(0, p - 1))}
                onNext={() => setPage((p) => p + 1)}
                disabled={isPending}
              />
            )}
        </div>

        <div className={cn(
          "lg:sticky lg:top-6 lg:h-[calc(100vh-3rem)]",
          selectedHit ? "h-[60vh] lg:h-auto" : "hidden lg:block",
        )}>
          <PdfViewerLoader hit={selectedHit} query={query} />
        </div>
      </div>
    </section>
  );
}

interface FilterPaginationProps {
  page: number;
  pageSize: number;
  total: number;
  hitsOnPage: number;
  onPrev: () => void;
  onNext: () => void;
  disabled?: boolean;
}

function FilterPagination({
  page,
  pageSize,
  total,
  hitsOnPage,
  onPrev,
  onNext,
  disabled,
}: FilterPaginationProps) {
  const start = page * pageSize + 1;
  const end = page * pageSize + hitsOnPage;
  const isFirstPage = page === 0;
  const isLastPage = end >= total;

  return (
    <div className="mt-4 flex items-center justify-between border-t border-border pt-3">
      <div className="smallcaps">
        Showing <span className="num text-foreground">{start.toLocaleString()}</span>
        –<span className="num text-foreground">{end.toLocaleString()}</span>
        {" of "}
        <span className="num text-foreground">{total.toLocaleString()}</span>
      </div>
      <div className="flex items-center gap-1">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={onPrev}
          disabled={disabled || isFirstPage}
        >
          ← Prev
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={onNext}
          disabled={disabled || isLastPage}
        >
          Next →
        </Button>
      </div>
    </div>
  );
}

function HitCard({ hit, isSelected }: { hit: SearchHit; isSelected: boolean }) {
  const d = hit.document;
  const metaBits = [
    d.number && `Q ${d.number}`,
    d.tags.schools[0]?.toUpperCase(),
    d.tags.paper_types[0]?.replace("-", " ").replace("paper", "Paper"),
    d.kind,
  ].filter(Boolean) as string[];

  return (
    <article data-slot="hit" className="flex flex-col gap-2 pt-6">
      <div className="smallcaps flex flex-wrap items-center gap-x-2 gap-y-1">
        {metaBits.map((m, i) => (
          <span key={i}>
            {m}
            {i < metaBits.length - 1 ? " ·" : ""}
          </span>
        ))}
        <span className="ml-auto num normal-case tracking-normal text-foreground/70">
          score <span className="text-primary">{hit.score.toFixed(3)}</span>
        </span>
      </div>

      {hit.highlights.length > 0 ? (
        <h2
          className={cn(
            "font-heading text-[19px] leading-snug",
            isSelected && "text-foreground",
          )}
          dangerouslySetInnerHTML={{ __html: hit.highlights[0].text }}
        />
      ) : (
        d.question && (
          <h2 className="font-heading text-[19px] leading-snug">
            {d.question}
          </h2>
        )
      )}

      {/* Topics — content classification (e.g. "Cold War", "Kinematics").
          Distinct from metadata facets above (school, paper-type); these
          describe what the question is *about*. Chips read better than a
          comma-separated list and naturally wrap when there are many. */}
      {d.tags.topics.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {d.tags.topics.map((topic) => (
            <Badge key={topic} variant="outline" className="font-normal">
              {topic}
            </Badge>
          ))}
        </div>
      )}

      {d.context && <p className="leadin text-sm">{d.context}</p>}

      <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
        {d.mark != null && (
          <span><span className="num">{d.mark}</span> marks</span>
        )}
        {d.pdf_anchor && (
          <a
            href={`/v1/documents/${d.id}/pdf`}
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-1.5"
            onClick={(e) => {
              e.stopPropagation();
              captureRexEvent("pdf_link_click", {
                document_id: d.id,
                subject: d.subject,
                pdf_path: d.pdf_anchor?.pdf_path ?? null,
              });
            }}
          >
            <FileText className="size-3.5" />
            <span className={cn("text-primary underline-offset-4", isSelected ? "underline" : "hover:underline")}>
              {d.pdf_anchor.pdf_path.split("/").pop()}
            </span>
          </a>
        )}
        <span className="num text-muted-foreground/70 ml-auto">
          {hit.scores.bm25   != null && <>bm25 {hit.scores.bm25.toFixed(2)}&nbsp;</>}
          {hit.scores.vector != null && <>vec {hit.scores.vector.toFixed(2)}&nbsp;</>}
          {hit.scores.rerank != null && <>rerank {hit.scores.rerank.toFixed(2)}</>}
        </span>
      </div>
    </article>
  );
}

// ────────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────────

function buildFilters(
  subject: string,
  selections: FacetSelections,
  kind: DocumentKind | null,
): Filters {
  const f: Filters = {};
  if (subject) f.subject = subject;
  if (kind)    f.kind    = kind;
  if (selections.topics?.length)         f.topics         = selections.topics;
  if (selections.schools?.length)        f.schools        = selections.schools;
  if (selections.paper_types?.length)    f.paper_types    = selections.paper_types;
  if (selections.source_types?.length)   f.source_types   = selections.source_types;
  if (selections.question_types?.length) f.question_types = selections.question_types;
  return f;
}

/** Are any facet fields (not counting subject) populated? */
function hasAnySelection(filters: Filters): boolean {
  return Boolean(
    filters.topics?.length
      || filters.schools?.length
      || filters.paper_types?.length
      || filters.source_types?.length
      || filters.question_types?.length,
  );
}
