"use client";

import { useState, useTransition } from "react";
import { Search, FileText, Sparkles, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";
import { PdfViewerLoader } from "@/components/pdf-viewer-loader";
import type { SearchHit, SearchMeta, SearchMode, SubjectStats } from "@/lib/types";

interface Props {
  subjects: SubjectStats[];
  apiOnline: boolean;
}

const MODE_OPTIONS: { value: SearchMode; label: string }[] = [
  { value: "Hybrid",     label: "Hybrid"   },
  { value: "Bm25Only",   label: "Keyword"  },
  { value: "VectorOnly", label: "Semantic" },
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

export function SearchPanel({ subjects, apiOnline }: Props) {
  const [query, setQuery] = useState("");
  const [subject, setSubject] = useState<string>(subjects[0]?.id ?? "");
  const [mode, setMode] = useState<SearchMode>("Hybrid");
  const [hits, setHits] = useState<SearchHit[]>(apiOnline ? [] : DEMO_HITS);
  const [meta, setMeta] = useState<SearchMeta | null>(apiOnline ? null : DEMO_META);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isPending, startTransition] = useTransition();

  const selectedHit = hits.find((h) => h.document.id === selectedId) ?? null;

  function submit(e: React.FormEvent) {
    e.preventDefault();
    if (!query.trim()) return;
    setError(null);

    startTransition(async () => {
      try {
        const res = await fetch("/v1/search", {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({
            text: query,
            mode,
            filters: subject ? { subject } : {},
            limit: 20,
          }),
        });
        if (!res.ok) {
          const body = await res.json().catch(() => null);
          throw new Error(body?.error?.message ?? res.statusText);
        }
        const data = await res.json();
        setHits(data.hits);
        setMeta(data.meta);
        setSelectedId(data.hits[0]?.document.id ?? null);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      }
    });
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
            <SelectTrigger className="w-fit min-w-44">
              <SelectValue placeholder="All subjects" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="__all__">All subjects</SelectItem>
              {subjects.map((s) => (
                <SelectItem key={s.id} value={s.id}>
                  {s.id}{" "}
                  <span className="num text-muted-foreground ml-1">
                    ({s.item_count.toLocaleString()})
                  </span>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          <Tabs value={mode} onValueChange={(v) => setMode(v as SearchMode)}>
            <TabsList>
              {MODE_OPTIONS.map((m) => (
                <TabsTrigger key={m.value} value={m.value}>
                  {m.label}
                </TabsTrigger>
              ))}
            </TabsList>
          </Tabs>

          <Button
            type="submit"
            disabled={isPending || !query.trim()}
            size="default"
            className="ml-auto"
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

      {/* ─── Meta strip ─────────────────────────────────────────── */}
      {meta && (
        <div className="smallcaps flex flex-wrap items-center gap-x-4 gap-y-1">
          <span>
            <span className="text-foreground">
              <span className="num">{hits.length}</span> matches
            </span>
          </span>
          <span><span className="num">{meta.took_ms}</span>&nbsp;ms</span>
          <span>mode <span className="text-foreground">{meta.mode}</span></span>
          {meta.used_bm25     && <Badge variant="outline">bm25</Badge>}
          {meta.used_vector   && <Badge variant="outline">vector</Badge>}
          {meta.used_reranker && <Badge variant="outline">rerank</Badge>}
          {meta.fts5_query && (
            <span className="num text-muted-foreground/70 truncate max-w-[40ch]">
              fts5 → {meta.fts5_query}
            </span>
          )}
        </div>
      )}

      {/* ─── Split: results | viewer ────────────────────────────── */}
      <div className="grid grid-cols-1 gap-8 lg:grid-cols-[1fr_1fr] xl:grid-cols-[minmax(0,1fr)_minmax(0,1.05fr)]">
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

        <div className="lg:sticky lg:top-6 lg:h-[calc(100vh-3rem)]">
          <PdfViewerLoader hit={selectedHit} query={query} />
        </div>
      </div>
    </section>
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
      <div className="smallcaps flex items-center gap-x-2 gap-y-1">
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

      {d.context && <p className="leadin text-sm">{d.context}</p>}

      <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
        {d.mark != null && (
          <span><span className="num">{d.mark}</span> marks</span>
        )}
        {d.pdf_anchor && (
          <span className="inline-flex items-center gap-1.5">
            <FileText className="size-3.5" />
            <span className={cn("text-primary", isSelected && "underline underline-offset-4")}>
              {d.pdf_anchor.pdf_path.split("/").pop()}
            </span>
            {d.pdf_anchor.fallback_reason && (
              <span className="text-destructive/70 italic">
                ({d.pdf_anchor.fallback_reason})
              </span>
            )}
          </span>
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
