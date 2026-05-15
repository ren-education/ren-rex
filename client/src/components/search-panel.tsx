"use client";

import { useState, useTransition } from "react";
import type { SearchHit, SubjectStats } from "@/lib/types";

interface Props {
  subjects: SubjectStats[];
}

export function SearchPanel({ subjects }: Props) {
  const [query, setQuery] = useState("");
  const [subject, setSubject] = useState<string>(subjects[0]?.id ?? "");
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [isPending, startTransition] = useTransition();

  function onSubmit(e: React.FormEvent) {
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
            mode: "Hybrid",
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
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
        setHits([]);
      }
    });
  }

  return (
    <section className="flex flex-col gap-6">
      <form onSubmit={onSubmit} className="flex flex-col gap-3 sm:flex-row">
        <select
          value={subject}
          onChange={(e) => setSubject(e.target.value)}
          className="rounded-md border border-neutral-300 bg-transparent px-3 py-2 text-sm dark:border-neutral-700"
        >
          <option value="">All subjects</option>
          {subjects.map((s) => (
            <option key={s.id} value={s.id}>
              {s.id} ({s.item_count})
            </option>
          ))}
        </select>
        <input
          type="search"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search questions, notes, PDFs..."
          className="flex-1 rounded-md border border-neutral-300 bg-transparent px-3 py-2 text-sm dark:border-neutral-700"
        />
        <button
          type="submit"
          disabled={isPending || !query.trim()}
          className="rounded-md bg-neutral-900 px-4 py-2 text-sm font-medium text-white disabled:opacity-50 dark:bg-neutral-100 dark:text-neutral-900"
        >
          {isPending ? "Searching..." : "Search"}
        </button>
      </form>

      {error && (
        <div className="rounded-md border border-red-300 bg-red-50 px-3 py-2 text-sm text-red-800 dark:border-red-900 dark:bg-red-950 dark:text-red-200">
          {error}
        </div>
      )}

      <ul className="flex flex-col gap-3">
        {hits.map((hit) => (
          <li
            key={hit.document.id}
            className="rounded-md border border-neutral-200 p-4 dark:border-neutral-800"
          >
            <div className="flex items-center justify-between text-xs text-neutral-500">
              <span>
                {hit.document.subject} · {hit.document.kind}
                {hit.document.number ? ` · ${hit.document.number}` : ""}
              </span>
              <span>score {hit.score.toFixed(3)}</span>
            </div>
            {hit.document.question && (
              <p className="mt-2 text-sm">{hit.document.question}</p>
            )}
            {hit.highlights.length > 0 && (
              <div className="mt-2 flex flex-col gap-1 text-xs text-neutral-600 dark:text-neutral-400">
                {hit.highlights.map((h, i) => (
                  <div
                    key={i}
                    dangerouslySetInnerHTML={{
                      __html: `<span class="font-medium">${h.field}:</span> ${h.text}`,
                    }}
                  />
                ))}
              </div>
            )}
          </li>
        ))}
      </ul>
    </section>
  );
}
