import type {
  Filters,
  SearchRequest,
  SearchResponse,
  SubjectStats,
} from "./types";

// Important: this runs at request time on the SERVER (Server Components +
// Route Handlers). The Next.js rewrite in next.config.ts forwards
// `/v1/:path*` to REX_API_BASE for browser requests, so client-side fetches
// can use a relative path. From the server we hit the rex API directly.
const API_BASE = process.env.REX_API_BASE ?? "http://localhost:8080";

class RexApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string,
  ) {
    super(message);
    this.name = "RexApiError";
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  // On the client, use a relative URL so the Next rewrite handles the
  // proxy. On the server, prefix with API_BASE so we can call the rex
  // server directly.
  const url = typeof window === "undefined" ? `${API_BASE}${path}` : path;
  const res = await fetch(url, {
    ...init,
    headers: {
      "content-type": "application/json",
      ...init?.headers,
    },
    cache: "no-store",
  });

  if (!res.ok) {
    const body = (await res.json().catch(() => null)) as
      | { error?: { code?: string; message?: string } }
      | null;
    throw new RexApiError(
      res.status,
      body?.error?.code ?? "unknown",
      body?.error?.message ?? res.statusText,
    );
  }

  return (await res.json()) as T;
}

export function listSubjects(): Promise<SubjectStats[]> {
  return request<SubjectStats[]>("/v1/subjects");
}

export function search(req: SearchRequest): Promise<SearchResponse> {
  return request<SearchResponse>("/v1/search", {
    method: "POST",
    body: JSON.stringify(req),
  });
}

export interface FilterRequest {
  filters?: Filters;
  limit?: number;
  offset?: number;
}

export function filter(req: FilterRequest): Promise<SearchResponse> {
  return request<SearchResponse>("/v1/filter", {
    method: "POST",
    body: JSON.stringify(req),
  });
}

export interface TagValuesResponse {
  subject: string;
  field: string;
  values: Array<{ value: string; count: number }>;
}

export function tagValues(
  subject: string,
  field: string,
  filters?: Filters,
): Promise<TagValuesResponse> {
  return request<TagValuesResponse>(
    `/v1/subjects/${encodeURIComponent(subject)}/tag-values/${encodeURIComponent(field)}`,
    {
      method: "POST",
      body: JSON.stringify({ filters: filters ?? {} }),
    },
  );
}

export { RexApiError };
