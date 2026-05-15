import type { MetadataRoute } from "next";
import { BRAND } from "@/lib/seo";

/**
 * PWA manifest. Even without a service worker, this gives mobile users
 * "Add to home screen" with proper branding and avoids the default
 * favicon-as-app-icon fallback. The icons themselves are served by the
 * generated icon.tsx and apple-icon.tsx routes — Next auto-picks them up.
 */
export default function manifest(): MetadataRoute.Manifest {
  return {
    name: BRAND.fullName,
    short_name: BRAND.name,
    description: BRAND.description,
    start_url: "/",
    display: "standalone",
    background_color: "#f4f1ea",
    theme_color: "#f4f1ea",
    categories: ["education", "productivity", "reference"],
  };
}
