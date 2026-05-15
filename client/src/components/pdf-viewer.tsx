"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Document, Page, pdfjs } from "react-pdf";
import "react-pdf/dist/Page/AnnotationLayer.css";
import "react-pdf/dist/Page/TextLayer.css";
import { ChevronLeft, ChevronRight, Download, ExternalLink, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  highlightInText,
  locateBestPage,
  tokenizeQuery,
} from "@/lib/pdf-match";
import type { SearchHit } from "@/lib/types";

// Worker from jsdelivr. The version matches react-pdf's bundled pdfjs-dist
// (currently 4.x via react-pdf 9). Pinning the URL to `pdfjs.version`
// avoids version drift when react-pdf updates.
pdfjs.GlobalWorkerOptions.workerSrc =
  `https://cdn.jsdelivr.net/npm/pdfjs-dist@${pdfjs.version}/build/pdf.worker.min.mjs`;

interface Props {
  hit: SearchHit;
  /** The current user query; terms get highlighted on the rendered page. */
  query: string;
}

export function PdfViewer({ hit, query }: Props) {
  const docUrl = `/v1/documents/${hit.document.id}/pdf`;
  const hintedPage = hit.document.pdf_anchor?.page_number ?? null;

  const [numPages, setNumPages] = useState<number>(0);
  const [page, setPage] = useState<number>(hintedPage ?? 1);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [containerWidth, setContainerWidth] = useState<number>(600);

  // Track which hit is currently active so async page-locate can bail out
  // if the user clicks another hit mid-scan.
  const activeHitIdRef = useRef(hit.document.id);
  useEffect(() => {
    activeHitIdRef.current = hit.document.id;
  });

  // Reset state when the hit changes.
  useEffect(() => {
    setPage(hintedPage ?? 1);
    setNumPages(0);
    setLoading(true);
    setError(null);
  }, [hit.document.id, hintedPage]);

  // Track the available width so the page scales to fit.
  useEffect(() => {
    if (!containerRef.current) return;
    const ro = new ResizeObserver(([entry]) => {
      setContainerWidth(entry.contentRect.width);
    });
    ro.observe(containerRef.current);
    return () => ro.disconnect();
  }, []);

  // Build the target the page-locator will match against.
  const target = useMemo(() => {
    return [hit.document.context, hit.document.question, hit.document.notes]
      .filter(Boolean)
      .join(" ");
  }, [hit.document.context, hit.document.question, hit.document.notes]);

  // Tokenize the user query for in-page highlighting.
  const queryTerms = useMemo(() => tokenizeQuery(query), [query]);

  // Wrap each text item with <mark> tags on matched terms. pdfjs's
  // TextLayer renders the returned string as HTML in place of the raw
  // text item.
  const customTextRenderer = useCallback(
    (item: { str: string }) => highlightInText(item.str, queryTerms),
    [queryTerms],
  );

  const onLoadSuccess = useCallback(
    // react-pdf's onLoadSuccess gives us the underlying pdfjs PDFDocumentProxy.
    // We type it via the param shape we actually use to avoid a pdfjs-dist
    // direct import (which is a sibling dep, not a public re-export).
    async (pdf: Parameters<NonNullable<React.ComponentProps<typeof Document>["onLoadSuccess"]>>[0]) => {
      setNumPages(pdf.numPages);
      setLoading(false);

      const startingHit = activeHitIdRef.current;
      // Clamp hinted page first.
      if (hintedPage && hintedPage > pdf.numPages) {
        setPage(1);
      }

      if (!target.trim()) return;

      // Run client-side fuzzy match. pdfjs's text extraction is usually
      // cleaner than the server's pdf-extract, so this often finds the
      // right page even when the server set `page_number: null`.
      const located = await locateBestPage(pdf, target).catch(() => null);
      // Bail if the user clicked another hit while we were scanning.
      if (activeHitIdRef.current !== startingHit) return;

      if (located) {
        setPage(located.page);
      } else if (hintedPage) {
        setPage(hintedPage);
      }
    },
    [hintedPage, target],
  );

  const filename =
    hit.document.pdf_anchor?.pdf_path.split("/").pop() ?? "document.pdf";

  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      {/* Toolbar */}
      <div className="flex items-center gap-2 text-xs text-muted-foreground">
        <span className="font-heading truncate text-foreground text-sm">
          {filename}
        </span>
        <span className="ml-auto flex items-center gap-1">
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            disabled={page <= 1}
            onClick={() => setPage((p) => Math.max(1, p - 1))}
          >
            <ChevronLeft />
          </Button>
          <span className="num min-w-16 text-center">
            {numPages ? `${page} / ${numPages}` : "—"}
          </span>
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            disabled={!numPages || page >= numPages}
            onClick={() => setPage((p) => Math.min(numPages, p + 1))}
          >
            <ChevronRight />
          </Button>
          {/* Download: anchor with `download` attribute hints the browser to save
              rather than navigate. The filename comes from pdf_anchor.pdf_path so
              the saved file matches what the user sees in the toolbar. */}
          <a
            href={docUrl}
            download={filename}
            className="ml-2 inline-flex items-center gap-1 border-b border-accent text-primary hover:border-primary"
            title="Download PDF"
          >
            download <Download className="size-3" />
          </a>
          <a
            href={docUrl}
            target="_blank"
            rel="noopener noreferrer"
            className="ml-2 inline-flex items-center gap-1 border-b border-accent text-primary hover:border-primary"
          >
            open <ExternalLink className="size-3" />
          </a>
        </span>
      </div>

      {/* Render area */}
      <div
        ref={containerRef}
        className="relative flex-1 min-h-0 overflow-auto rounded-md border border-border bg-card p-3"
      >
        {error && (
          <div className="text-sm text-destructive">{error}</div>
        )}

        {loading && !error && (
          <div className="flex items-center justify-center gap-2 py-12 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            Loading PDF…
          </div>
        )}

        <Document
          file={docUrl}
          onLoadSuccess={onLoadSuccess}
          onLoadError={(err) => {
            setLoading(false);
            setError(err?.message ?? "Failed to load PDF");
          }}
          loading=""
          error=""
        >
          {!loading && !error && (
            <Page
              pageNumber={page}
              width={Math.max(320, containerWidth - 32)}
              renderTextLayer
              renderAnnotationLayer={false}
              customTextRenderer={customTextRenderer}
              className="mx-auto"
            />
          )}
        </Document>
      </div>

    </div>
  );
}
