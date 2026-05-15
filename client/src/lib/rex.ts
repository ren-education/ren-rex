import type {
  SearchRequest,
  SearchResponse,
  SubjectStats,
} from "./types";

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
  const res = await fetch(`${API_BASE}${path}`, {
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

export { RexApiError };
