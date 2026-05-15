import { ImageResponse } from "next/og";
import { BRAND } from "@/lib/seo";

/**
 * Dynamically-rendered Open Graph image (also reused as the Twitter card).
 * Rendered server-side using Next's @vercel/og engine the first time a
 * crawler hits the URL, then CDN-cached. No design files / Photoshop /
 * static asset commit needed — the design lives in this component.
 *
 * Constraints to keep in mind:
 *   - 1200×630 is the OG standard. Twitter's "summary_large_image" uses
 *     the same dimensions, so one asset covers both.
 *   - @vercel/og runs on the Edge runtime, which means *no @import in CSS,
 *     no DOM APIs, no React effects*. Pure JSX layout only.
 *   - Custom fonts must be loaded via fetch() and passed to options.fonts.
 *     Skipped here on purpose — system fonts render fast and consistently
 *     across crawlers' rendering.
 */
export const runtime = "edge";
export const alt = `${BRAND.fullName} — ${BRAND.tagline}`;
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

export default async function OpenGraphImage() {
  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          flexDirection: "column",
          justifyContent: "space-between",
          padding: "80px 96px",
          background:
            "linear-gradient(135deg, #f4f1ea 0%, #e8e3d5 60%, #d9d3bf 100%)",
          color: "#2a2d24",
          fontFamily: "system-ui, -apple-system, sans-serif",
        }}
      >
        <div style={{ display: "flex", alignItems: "baseline", gap: 20 }}>
          <div
            style={{
              fontSize: 96,
              fontWeight: 700,
              letterSpacing: "-0.04em",
              lineHeight: 1,
            }}
          >
            rex
          </div>
          <div
            style={{
              fontSize: 32,
              color: "#6b6f5e",
              letterSpacing: "-0.01em",
            }}
          >
            by ren.
          </div>
        </div>

        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: 18,
          }}
        >
          <div
            style={{
              fontSize: 64,
              fontWeight: 600,
              letterSpacing: "-0.025em",
              lineHeight: 1.1,
              maxWidth: 940,
            }}
          >
            Search 25,000+ A-Level questions, notes, and marking schemes.
          </div>
          <div
            style={{
              fontSize: 32,
              color: "#6b6f5e",
              letterSpacing: "-0.01em",
              maxWidth: 940,
            }}
          >
            Find the exact page in the source PDF instantly. H2 Physics,
            H2 History, General Paper, and more.
          </div>
        </div>

        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            borderTop: "2px solid #2a2d2422",
            paddingTop: 28,
            fontSize: 24,
            color: "#6b6f5e",
            letterSpacing: "0.02em",
          }}
        >
          <div>rex.reneducation.com</div>
          <div>singapore JC prelims · free</div>
        </div>
      </div>
    ),
    { ...size },
  );
}
