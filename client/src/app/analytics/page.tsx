import Link from "next/link";
import { ArrowLeft, ArrowRight } from "lucide-react";
import { allAnalytics } from "@/lib/analytics";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import type { Metadata } from "next";

/**
 * Analytics index — landing page that lists each subject with its
 * headline numbers, deep-linking to the per-subject page. Static, no
 * API calls. Reachable from the homepage header and directly via SEO.
 */

export const dynamic = "force-static";

export const metadata: Metadata = {
  title: "Analytics",
  description:
    "Browse what's in the rex corpus: questions per year, top topics, school coverage. Aggregated dashboards for GP, H2 History, and H2 Physics.",
  alternates: { canonical: "/analytics" },
};

export default function AnalyticsIndexPage() {
  const subjects = allAnalytics();
  const grand = subjects.reduce(
    (acc, s) => acc + s.summary.total_documents,
    0,
  );

  return (
    <main className="mx-auto flex min-h-screen w-full max-w-[1500px] flex-col gap-10 px-6 py-14">
      <header className="flex items-baseline justify-between border-b border-border pb-6">
        <Link
          href="/"
          className="group flex items-baseline gap-2 text-muted-foreground transition-colors hover:text-foreground"
        >
          <ArrowLeft className="size-3 self-center transition-transform group-hover:-translate-x-0.5" />
          <span className="font-heading text-3xl tracking-tight text-foreground">
            r<span className="text-primary italic">e</span>x
          </span>
          <span className="text-[0.7rem]">/ analytics</span>
        </Link>
        <div className="text-xs text-muted-foreground">
          <span className="text-foreground/80 font-medium">
            {subjects.length} subjects
          </span>
          <span className="px-2 text-border">·</span>
          <span className="num">{grand.toLocaleString()}</span> documents
        </div>
      </header>

      <section className="flex flex-col gap-3">
        <h1 className="font-heading text-4xl leading-tight tracking-tight">
          What&apos;s in the archive.
        </h1>
        <p className="max-w-2xl text-base text-muted-foreground">
          Aggregated views of the rex corpus: questions per year, topic
          distribution, school coverage. One dashboard per subject.
        </p>
      </section>

      <section className="grid gap-3 md:grid-cols-2 lg:grid-cols-3">
        {subjects.map(({ id, label, blurb, summary }) => (
          <Link key={id} href={`/analytics/${id}`} className="group">
            <Card className="h-full transition-colors hover:border-primary/40">
              <CardHeader className="pb-2">
                <CardTitle className="flex items-center justify-between font-heading text-2xl tracking-tight">
                  {label}
                  <ArrowRight className="size-4 text-muted-foreground transition-transform group-hover:translate-x-0.5 group-hover:text-foreground" />
                </CardTitle>
                <p className="text-xs text-muted-foreground">{blurb}</p>
              </CardHeader>
              <CardContent className="grid grid-cols-3 gap-3 pt-2">
                <Stat label="Questions" value={summary.questions} />
                <Stat label="Schools" value={summary.tag_field_counts.schools} />
                <Stat label="Topics" value={summary.tag_field_counts.topics} />
              </CardContent>
            </Card>
          </Link>
        ))}
      </section>
    </main>
  );
}

function Stat({ label, value }: { label: string; value: number }) {
  return (
    <div className="flex flex-col gap-0.5">
      <div className="font-mono text-base font-medium tabular-nums text-foreground">
        {value.toLocaleString()}
      </div>
      <div className="text-[0.6rem] font-medium uppercase tracking-wider text-muted-foreground">
        {label}
      </div>
    </div>
  );
}
