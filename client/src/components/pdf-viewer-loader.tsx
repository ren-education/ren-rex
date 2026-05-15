"use client";

import dynamic from "next/dynamic";
import { Loader2 } from "lucide-react";
import type { SearchHit } from "@/lib/types";

// react-pdf depends on pdfjs which uses browser globals — disable SSR.
const PdfViewer = dynamic(
  () => import("./pdf-viewer").then((mod) => mod.PdfViewer),
  {
    ssr: false,
    loading: () => (
      <div className="flex h-full min-h-0 flex-col items-center justify-center gap-2 text-sm text-muted-foreground">
        <Loader2 className="size-4 animate-spin" />
        Loading viewer…
      </div>
    ),
  },
);

interface Props {
  hit: SearchHit | null;
}

export function PdfViewerLoader({ hit }: Props) {
  if (!hit) {
    return (
      <div className="flex h-full min-h-0 flex-col items-center justify-center gap-1 text-center text-sm text-muted-foreground">
        <p className="font-heading text-base">No document selected.</p>
        <p className="max-w-xs italic">
          Run a search and click a result to preview its PDF here.
        </p>
      </div>
    );
  }

  if (!hit.document.pdf_anchor) {
    return (
      <div className="flex h-full min-h-0 flex-col items-center justify-center gap-1 text-center text-sm text-muted-foreground">
        <p className="font-heading text-base text-foreground">No PDF attached.</p>
        <p className="max-w-xs italic">
          This document was indexed without a PDF anchor — there's nothing to render.
        </p>
      </div>
    );
  }

  return <PdfViewer hit={hit} />;
}
