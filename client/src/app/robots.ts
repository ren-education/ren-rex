import type { MetadataRoute } from "next";
import { SITE_URL } from "@/lib/seo";

/**
 * Robots policy. We allow all reputable crawlers everywhere, with two
 * narrow exceptions:
 *
 *   /api/   — Next API routes. Nothing useful here for an indexed result;
 *             crawling them just adds load without ranking benefit.
 *   /v1/    — The rex backend proxied via next.config.ts. PDF binaries and
 *             search endpoints — not pages, shouldn't be in the index.
 *
 * The single declaration block applies to every user-agent; if/when we
 * need to selectively block aggressive scrapers (AhrefsBot, SemrushBot,
 * GPTBot, etc.) we can add per-userAgent entries here.
 */
export default function robots(): MetadataRoute.Robots {
  return {
    rules: [
      {
        userAgent: "*",
        allow: "/",
        disallow: ["/api/", "/v1/"],
      },
    ],
    sitemap: `${SITE_URL}/sitemap.xml`,
    host: SITE_URL,
  };
}
