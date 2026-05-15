"use client";

import { useEffect, useRef, useState } from "react";
import { Document, Page, pdfjs } from "react-pdf";
import "react-pdf/dist/Page/AnnotationLayer.css";
import "react-pdf/dist/Page/TextLayer.css";
import { ChevronLeft, ChevronRight, ExternalLink, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { SearchHit } from "@/lib/types";

// Use jsdelivr CDN for the worker so we don't need a custom Next asset rule.
// react-pdf ≥ 9 ships pdfjs ≥ 4 and expects the .mjs worker.
pdfjs.GlobalWorkerOptions.workerSrc =
  `https://cdn.jsdelivr.net/npm/pdfjs-dist@${pdfjs.version}/build/pdf.worker.min.mjs`;

interface Props {
  hit: SearchHit;
}

export function PdfViewer({ hit }: Props) {
  const docUrl = `/v1/documents/${hit.document.id}/pdf`;
  const hintedPage = hit.document.pdf_anchor?.page_number ?? null;

  const [numPages, setNumPages] = useState<number>(0);
  const [page, setPage] = useState<number>(hintedPage ?? 1);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [containerWidth, setContainerWidth] = useState<number>(600);

  // Reset state when the hit (document) changes.
  useEffect(() => {
    setPage(hintedPage ?? 1);
    setNumPages(0);
    setLoading(true);
    setError(null);
  }, [hit.document.id, hintedPage]);

  // Track the available width so the page can be scaled to fit.
  useEffect(() => {
    if (!containerRef.current) return;
    const ro = new ResizeObserver(([entry]) => {
      setContainerWidth(entry.contentRect.width);
    });
    ro.observe(containerRef.current);
    return () => ro.disconnect();
  }, []);

  const filename =
    hit.document.pdf_anchor?.pdf_path.split("/").pop() ?? "document.pdf";

  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      {/* Toolbar */}
      <div className="flex items-center gap-2 text-xs text-muted-foreground">
        <span className="font-heading truncate text-foreground text-sm">
          {filename}
        </span>
        {hit.document.pdf_anchor?.fallback_reason && (
          <span className="text-destructive/70 italic">
            ({hit.document.pdf_anchor.fallback_reason})
          </span>
        )}
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
          onLoadSuccess={({ numPages }) => {
            setNumPages(numPages);
            setLoading(false);
            // Clamp the hinted page if it overshoots the doc.
            if (hintedPage && hintedPage > numPages) {
              setPage(1);
            }
          }}
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
              className="mx-auto"
            />
          )}
        </Document>
      </div>

      {hintedPage && (
        <p className="smallcaps">
          Question anchored to page <span className="num text-foreground">{hintedPage}</span>
          {numPages ? <> of <span className="num">{numPages}</span></> : null}
        </p>
      )}
    </div>
  );
}
