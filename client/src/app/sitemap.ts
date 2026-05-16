import type { MetadataRoute } from "next";
import { SITE_URL } from "@/lib/seo";
import { listSubjects } from "@/lib/rex";
import { ANALYTICS_SUBJECTS } from "@/lib/analytics";

/**
 * Sitemap. Today this is essentially a single-URL sitemap because the app
 * is one SPA-shaped landing page; the meaningful URLs only emerge once we
 * add per-subject and per-document routes (e.g. /h2physics, /d/<uuid>).
 *
 * The sitemap is still worth generating now for three reasons:
 *
 *   1. It establishes the canonical entry point with a declared change
 *      frequency and priority. Search engines weight that signal even
 *      when there's only one URL.
 *   2. It's wired up so adding new routes later is *one push* away from
 *      crawl coverage — just append more entries.
 *   3. It pulls the live subject list from the rex API so when we *do*
 *      add `/[subject]` routes, the sitemap is already enumerating them.
 *
 * The API call is wrapped in a try/catch because a broken rex backend
 * shouldn't break sitemap generation — fall back to the static base URL
 * so crawlers still get something usable.
 */
export default async function sitemap(): Promise<MetadataRoute.Sitemap> {
  const now = new Date();
  const base: MetadataRoute.Sitemap = [
    {
      url: SITE_URL,
      lastModified: now,
      changeFrequency: "weekly",
      priority: 1.0,
    },
    {
      url: `${SITE_URL}/analytics`,
      lastModified: now,
      changeFrequency: "weekly",
      priority: 0.7,
    },
    // Per-subject analytics — real pages, real text content, real
    // indexable URLs. These are the SEO surface for queries like
    // "h2 physics topic distribution" or "gp essay coverage 2023".
    ...ANALYTICS_SUBJECTS.map((s) => ({
      url: `${SITE_URL}/analytics/${s.id}`,
      lastModified: now,
      changeFrequency: "weekly" as const,
      priority: 0.7,
    })),
  ];

  // When per-subject pages exist at `/[subject]`, uncomment the loop below
  // and the sitemap will enumerate them from the live rex API. Today the
  // app is one SPA-shaped landing page so subject URLs would 404 and pollute
  // Search Console. Leaving the wiring intact — just don't push to `base`.
  try {
    await listSubjects();
    // for (const s of subjects) {
    //   base.push({
    //     url: `${SITE_URL}/${s.id}`,
    //     lastModified: now,
    //     changeFrequency: "weekly",
    //     priority: 0.8,
    //   });
    // }
  } catch {
    // backend unavailable — sitemap still has the root URL.
  }

  return base;
}
