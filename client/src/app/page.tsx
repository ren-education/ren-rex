import { SearchPanel } from "@/components/search-panel";
import { RubricCtaChip } from "@/components/rubric-cta-chip";
import { listSubjects } from "@/lib/rex";
import type { SubjectStats } from "@/lib/types";

export const dynamic = "force-dynamic";

const UPCOMING_SUBJECTS = [
  "H2 Economics",
  "H2 Literature",
  "H2 Mathematics",
];

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
    <main className="mx-auto flex min-h-screen w-full max-w-[1500px] flex-col gap-6 sm:gap-10 px-6 py-8 sm:py-14">
      <header className="flex flex-wrap items-baseline justify-between gap-y-2 border-b border-border pb-6">
        <div className="flex items-baseline gap-2">
          {/* rex wordmark — sage italic e in serif */}
          <span className="font-heading text-3xl tracking-tight">
            r<span className="text-primary italic">e</span>x
          </span>
          {/* "by ren." attribution — small enough to read as a subscript-
              style credit rather than co-equal billing. The "ren." keeps
              its brand weight (black sans + period) so it still nods to
              the ren wordmark identity, just at attribution scale. */}
          <span className="text-[0.7rem] text-muted-foreground">
            by{" "}
            <a
              href="https://reneducation.com"
              target="_blank"
              rel="noopener noreferrer"
              className="font-sans font-bold tracking-tight text-foreground transition-opacity hover:opacity-70"
            >
              ren.
            </a>
          </span>
          <RubricCtaChip />
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
        <h1 className="font-heading text-3xl sm:text-4xl leading-tight tracking-tight">
          Search the archive.
        </h1>
        <p className="max-w-2xl text-base text-muted-foreground">
          Find questions, notes, and PDF pages across subjects. Hybrid search
          combines keyword matching with semantic similarity.
        </p>
        <div className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 border-y border-border bg-accent/35 px-4 py-3 text-sm">
          <span className="font-medium text-foreground">
            More subjects coming soon
          </span>
          <span className="text-muted-foreground">
            {UPCOMING_SUBJECTS.join(", ")}, and more.
          </span>
        </div>
      </section>

      <SearchPanel subjects={subjects} apiOnline={apiOnline} />
    </main>
  );
}
