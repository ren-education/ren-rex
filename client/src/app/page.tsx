import { SearchPanel } from "@/components/search-panel";
import { listSubjects } from "@/lib/rex";
import type { SubjectStats } from "@/lib/types";

export const dynamic = "force-dynamic";

const FALLBACK_SUBJECTS: SubjectStats[] = [
  { id: "h2history", item_count: 1866 },
  { id: "h2physics", item_count: 7758 },
  { id: "hcchem",    item_count: 2412 },
  { id: "h2econs",   item_count: 3014 },
  { id: "english",   item_count: 812  },
];

export default async function Home() {
  let subjects: SubjectStats[] = [];
  let apiOnline = true;
  try {
    subjects = await listSubjects();
  } catch {
    subjects = FALLBACK_SUBJECTS;
    apiOnline = false;
  }

  const totalItems = subjects.reduce((n, s) => n + s.item_count, 0);

  return (
    <main className="mx-auto flex min-h-screen max-w-5xl flex-col gap-10 px-6 py-14">
      <header className="flex items-baseline justify-between border-b border-border pb-6">
        <div className="font-heading text-3xl tracking-tight">
          r<span className="text-primary italic">e</span>x
        </div>
        <div className="text-xs text-muted-foreground">
          <span className="text-foreground/80 font-medium">
            {subjects.length} subjects
          </span>
          <span className="px-2 text-border">·</span>
          <span className="num">{totalItems.toLocaleString()}</span> items
          {!apiOnline && (
            <>
              <span className="px-2 text-border">·</span>
              <span className="text-destructive">offline (demo data)</span>
            </>
          )}
        </div>
      </header>

      <section className="flex flex-col gap-3">
        <h1 className="font-heading text-4xl leading-tight tracking-tight">
          Search the archive.
        </h1>
        <p className="max-w-2xl text-base text-muted-foreground">
          Find questions, notes, and PDF pages across subjects. Hybrid search
          combines keyword matching with semantic similarity.
        </p>
      </section>

      <SearchPanel subjects={subjects} apiOnline={apiOnline} />
    </main>
  );
}
