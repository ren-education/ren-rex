import { readFileSync } from "node:fs";
import path from "node:path";
import Link from "next/link";
import { notFound } from "next/navigation";
import { ArrowLeft } from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import {
  ANALYTICS_SUBJECTS,
  densifyBySeries,
  getAnalytics,
  hasAnalytics,
  humanizeTag,
  type SubjectId,
} from "@/lib/analytics";
import {
  cleanAnalysisMarkdown,
  parseInsights,
  type InsightSection,
  type InsightSlot,
} from "@/lib/analytics-prose";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { StackedAreaOverTime } from "@/components/analytics-charts";
import type { Metadata } from "next";

/**
 * Per-subject analytics page. Statically generated for each known subject
 * (`generateStaticParams` returns the id list), so renders are pre-baked
 * at build time and served as plain HTML with no runtime API call.
 *
 * Layout philosophy: a long-form "report" — single column, narrow enough
 * to read like prose, one chart per row. Commentary from the upstream
 * `analysis.md` is parsed into H3 sections and routed to the chart card
 * that matches each section's topic (a "trend" insight renders right
 * after the topic-over-time chart, etc.). Anything that doesn't match a
 * specific chart slot falls through into a "More observations" section
 * at the bottom.
 *
 * Sections deliberately not surfaced here even though they're computable:
 *   - Questions per year (corpus volume, not subject content)
 *   - Top schools (which JCs the data came from, not subject content)
 *   - Top topics cloud (already implicit in the over-time chart's series)
 *   - School signatures / house styles (per-user direction)
 */

export const dynamic = "force-static";

export function generateStaticParams() {
  return ANALYTICS_SUBJECTS.map((s) => ({ subject: s.id }));
}

export async function generateMetadata({
  params,
}: {
  params: Promise<{ subject: string }>;
}): Promise<Metadata> {
  const { subject } = await params;
  if (!hasAnalytics(subject)) return {};
  const meta = ANALYTICS_SUBJECTS.find((s) => s.id === subject)!;
  const data = getAnalytics(subject);
  return {
    title: `${meta.label} analytics`,
    description: `${data.questions.toLocaleString()} questions across ${data.tag_field_counts.schools} schools and ${data.tag_field_counts.topics} topics. ${meta.blurb}`,
    alternates: { canonical: `/analytics/${subject}` },
  };
}

export default async function SubjectAnalyticsPage({
  params,
}: {
  params: Promise<{ subject: string }>;
}) {
  const { subject } = await params;
  if (!hasAnalytics(subject)) notFound();
  const id = subject as SubjectId;
  const meta = ANALYTICS_SUBJECTS.find((s) => s.id === id)!;
  const data = getAnalytics(id);

  const topicTrend = densifyBySeries(data.topic_by_year);

  // Read companion narrative, strip meta sections, normalize tag/school
  // names, then parse into per-insight chunks routed by `slot`.
  let insights: InsightSection[] = [];
  try {
    const narrativePath = path.join(process.cwd(), "src/content/analytics", `${id}.md`);
    const raw = readFileSync(narrativePath, "utf-8");
    insights = parseInsights(cleanAnalysisMarkdown(raw));
  } catch {
    insights = [];
  }

  // Helper: pull all insights that belong to a given slot. Only the
  // slots whose corresponding chart is rendered on this page are inlined;
  // the rest fall through to a "More observations" section below.
  const bySlot = (slot: InsightSlot) => insights.filter((i) => i.slot === slot);
  const trendInsights = bySlot("trend");
  const cooccurInsights = bySlot("cooccur");
  const topicInsights = bySlot("topics");
  const renderedSlots = new Set<InsightSlot>(["trend", "cooccur", "topics"]);
  const otherInsights = insights.filter((i) => !renderedSlots.has(i.slot));

  return (
    <main className="mx-auto flex min-h-screen w-full max-w-3xl flex-col gap-10 px-6 py-14">
      <header className="flex items-baseline justify-between border-b border-border pb-6">
        <div className="flex items-baseline gap-2">
          <Link
            href="/"
            className="group flex items-baseline gap-2 text-muted-foreground transition-colors hover:text-foreground"
          >
            <ArrowLeft className="size-3 self-center transition-transform group-hover:-translate-x-0.5" />
            <span className="font-heading text-3xl tracking-tight text-foreground">
              r<span className="text-primary italic">e</span>x
            </span>
          </Link>
          <span className="text-[0.7rem] text-muted-foreground">/ analytics / {meta.label}</span>
        </div>
        <div className="text-xs text-muted-foreground">
          <span className="num">{data.year_range[0]}</span>–
          <span className="num">{data.year_range[1]}</span>
        </div>
      </header>

      <section className="flex flex-col gap-3">
        <h1 className="font-heading text-4xl leading-tight tracking-tight">
          {meta.label}.
        </h1>
        <p className="max-w-2xl text-base text-muted-foreground">{meta.blurb}</p>
      </section>

      {/* Slim 3-tile KPI strip. Coverage percentages (`with_mark`,
        * `with_answer`) are intentionally omitted — they describe the
        * corpus's data-completeness, not the subject. */}
      <section className="grid grid-cols-3 gap-3">
        <Stat label="Questions" value={data.questions.toLocaleString()} />
        <Stat label="Topics" value={data.tag_field_counts.topics?.toString() ?? "—"} />
        <Stat
          label="Year range"
          value={`${data.year_range[0]}–${data.year_range[1]}`}
        />
      </section>

      {/* === Topic share over time === */}
      <ChartSection
        title="Topic share over time"
        subtitle="Top topics, stacked by year. Read for slopes — which themes are rising or fading?"
        chart={<StackedAreaOverTime keys={topicTrend.keys} rows={topicTrend.rows} />}
        height={320}
      />
      <Insights items={trendInsights} />
      <Insights items={topicInsights} />

      {/* === Topic co-occurrences === */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-normal text-muted-foreground">
            Topic co-occurrences
          </CardTitle>
          <p className="text-xs text-muted-foreground/60">
            Topic pairs that show up on the same question most often. Treat
            these as <em>argument bundles</em>: evidence for one usually works
            for the other.
          </p>
        </CardHeader>
        <CardContent>
          <ol className="flex flex-col divide-y divide-border/60 text-sm">
            {data.topic_cooccurrence.slice(0, 12).map((p) => (
              <li
                key={`${p.a}__${p.b}`}
                className="flex items-center justify-between gap-3 py-2"
              >
                <span className="flex flex-wrap items-center gap-1.5">
                  <Badge variant="outline">{humanizeTag(p.a)}</Badge>
                  <span className="text-muted-foreground/60">+</span>
                  <Badge variant="outline">{humanizeTag(p.b)}</Badge>
                </span>
                <span className="font-mono tabular-nums text-muted-foreground">
                  {p.count}
                </span>
              </li>
            ))}
          </ol>
        </CardContent>
      </Card>
      <Insights items={cooccurInsights} />

      {/* === Keywords by top topic === */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-normal text-muted-foreground">
            Keywords by top topic
          </CardTitle>
          <p className="text-xs text-muted-foreground/60">
            Most frequent keywords inside questions tagged with each top
            topic. Drawn from extracted keywords, not raw question text.
          </p>
        </CardHeader>
        <CardContent>
          <div className="flex flex-col gap-4">
            {Object.entries(data.topic_keywords).map(([topic, kws]) => (
              <div key={topic} className="flex flex-col gap-1.5">
                <div className="text-xs font-medium text-foreground">
                  {humanizeTag(topic)}
                </div>
                <div className="flex flex-wrap gap-1">
                  {kws.map((kw) => (
                    <Badge
                      key={kw.keyword}
                      variant="secondary"
                      className="font-normal"
                    >
                      {humanizeTag(kw.keyword)}
                      <span className="ml-1 font-mono tabular-nums text-muted-foreground">
                        {kw.count}
                      </span>
                    </Badge>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>

      {otherInsights.length > 0 && (
        <section className="flex flex-col gap-4 border-t border-border pt-10">
          <h2 className="text-sm font-normal uppercase tracking-wider text-muted-foreground">
            More observations
          </h2>
          <Insights items={otherInsights} />
        </section>
      )}
    </main>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <Card className="gap-1 py-3">
      <CardContent className="flex flex-col gap-1 px-4">
        <div className="font-mono text-xl font-medium tabular-nums tracking-tight text-foreground">
          {value}
        </div>
        <div className="text-[0.65rem] font-medium uppercase tracking-wider text-muted-foreground">
          {label}
        </div>
      </CardContent>
    </Card>
  );
}

/**
 * Standard chart wrapper. Keeps the title/subtitle/height pattern in
 * one place so all charts read with the same visual rhythm and we
 * don't drift between calls.
 */
function ChartSection({
  title,
  subtitle,
  chart,
  height,
}: {
  title: string;
  subtitle?: string;
  chart: React.ReactNode;
  height: number;
}) {
  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-sm font-normal text-muted-foreground">
          {title}
        </CardTitle>
        {subtitle && (
          <p className="text-xs text-muted-foreground/60">{subtitle}</p>
        )}
      </CardHeader>
      <CardContent style={{ height }}>{chart}</CardContent>
    </Card>
  );
}

/**
 * Render a list of `InsightSection`s as compact markdown blocks. Each
 * insight slots between charts (interleaving) — heading is rendered
 * subdued and the body uses the same prose treatment as the longer
 * analysis section. Returns null when there are no insights for this
 * slot so we don't emit empty wrappers.
 */
function Insights({ items }: { items: InsightSection[] }) {
  if (items.length === 0) return null;
  return (
    <div className="flex flex-col gap-6">
      {items.map((insight) => (
        <article
          key={insight.heading}
          className={[
            "prose prose-sm max-w-none dark:prose-invert",
            "prose-headings:font-heading prose-headings:tracking-tight",
            // H4 below acts as the insight title; styled as a subtle
            // capsule-uppercase eyebrow so it doesn't compete with the
            // chart card titles directly above.
            "prose-p:text-foreground/85 prose-p:leading-relaxed",
            "prose-strong:text-foreground prose-strong:font-medium",
            "prose-a:text-primary prose-a:no-underline hover:prose-a:underline",
            "prose-code:font-mono prose-code:text-foreground prose-code:bg-muted prose-code:rounded prose-code:px-1 prose-code:py-0.5 prose-code:text-[0.85em] prose-code:before:content-none prose-code:after:content-none",
            "prose-pre:bg-muted prose-pre:text-foreground prose-pre:text-[0.78rem] prose-pre:leading-relaxed prose-pre:border prose-pre:border-border",
            "prose-blockquote:border-l-primary/40 prose-blockquote:text-muted-foreground prose-blockquote:not-italic",
            "prose-li:text-foreground/85 marker:text-muted-foreground",
            "prose-table:text-sm prose-th:font-medium prose-th:text-foreground prose-td:text-foreground/85",
            "prose-hr:border-border",
          ].join(" ")}
        >
          <h4 className="!mt-0 !mb-3 text-[0.7rem] font-medium uppercase tracking-wider text-muted-foreground">
            {insight.heading}
          </h4>
          <ReactMarkdown remarkPlugins={[remarkGfm]}>{insight.body}</ReactMarkdown>
        </article>
      ))}
    </div>
  );
}
