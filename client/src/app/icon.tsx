import { ImageResponse } from "next/og";

/**
 * 32×32 favicon — the small icon browsers show in the tab. Rendered
 * server-side from JSX so it stays in sync with brand color tokens
 * without committing a binary asset. A bigger 180×180 variant lives
 * in apple-icon.tsx for iOS home-screen.
 *
 * Design intent: a single 'r' glyph on the sage-linen background.
 * Letterform sized aggressively (90% of cell height) so the mark reads
 * clearly even at the 16×16 sizes browsers downscale to.
 */
export const runtime = "edge";
export const size = { width: 32, height: 32 };
export const contentType = "image/png";

export default function Icon() {
  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          background: "#2a2d24",
          color: "#f4f1ea",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          fontFamily: "system-ui, -apple-system, sans-serif",
          fontSize: 24,
          fontWeight: 800,
          letterSpacing: "-0.06em",
        }}
      >
        r
      </div>
    ),
    { ...size },
  );
}
