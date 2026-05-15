export type SearchMode = "Hybrid" | "Bm25Only" | "VectorOnly" | "Filter";
export type DocumentKind = "Question" | "Note";

export interface Tags {
  topics: string[];
  question_types: string[];
  exam_systems: string[];
  paper_types: string[];
  schools: string[];
  source_types: string[];
}

export interface PdfAnchor {
  pdf_path: string;
  page_number: number | null;
  bbox: { x: number; y: number; w: number; h: number } | null;
  confidence: number;
  fallback_reason: "LowConfidence" | "PdfReadFailed" | "PdfNotFound" | null;
}

export interface RexDocument {
  id: string;
  subject: string;
  kind: DocumentKind;
  parent_id: string | null;
  number: string | null;
  source: string;
  context: string | null;
  question: string | null;
  answer: string | null;
  notes: string | null;
  mark: number | null;
  options: string[] | null;
  keywords: string[];
  tags: Tags;
  pdf_anchor: PdfAnchor | null;
}

export interface Filters {
  subject?: string;
  topics?: string[];
  question_types?: string[];
  exam_systems?: string[];
  paper_types?: string[];
  schools?: string[];
  source_types?: string[];
  marks_range?: [number, number];
  kind?: DocumentKind;
}

export interface SearchRequest {
  text: string;
  mode?: SearchMode;
  filters?: Filters;
  limit?: number;
  exact?: boolean;
  rerank?: boolean;
}

export interface Highlight {
  field: "Question" | "Answer" | "Context" | "Notes";
  text: string;
}

export interface SearchHit {
  document: RexDocument;
  score: number;
  scores: {
    bm25: number | null;
    vector: number | null;
    rerank: number | null;
  };
  highlights: Highlight[];
}

export interface SearchMeta {
  mode: SearchMode;
  used_embedder: boolean;
  used_bm25: boolean;
  used_vector: boolean;
  used_reranker: boolean;
  fts5_query: string | null;
  total_matches: number | null;
  took_ms: number;
}

export interface SearchResponse {
  hits: SearchHit[];
  meta: SearchMeta;
}

export interface SubjectStats {
  id: string;
  item_count: number;
}
