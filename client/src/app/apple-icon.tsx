import { ImageResponse } from "next/og";

/**
 * 180×180 Apple touch icon — used when iOS users save the site to their
 * home screen. Same design as icon.tsx but tuned for the larger canvas
 * (more letter weight, slightly more padding).
 */
export const runtime = "edge";
export const size = { width: 180, height: 180 };
export const contentType = "image/png";

export default function AppleIcon() {
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
          fontSize: 130,
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
