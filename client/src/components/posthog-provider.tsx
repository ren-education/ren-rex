"use client";

import { useEffect } from "react";
import { usePathname } from "next/navigation";
import { captureRexEvent, initPostHog } from "@/lib/posthog-client";

let lastTrackedUrl = "";

export function PostHogProvider() {
  const pathname = usePathname();

  useEffect(() => {
    if (!initPostHog()) return;

    const { search } = window.location;
    const url = `${pathname}${search}`;
    if (url === lastTrackedUrl) return;
    lastTrackedUrl = url;

    const params = new URLSearchParams(search);
    captureRexEvent("page_view", {
      path: pathname,
      search: search || null,
      utm_source: params.get("utm_source"),
      utm_medium: params.get("utm_medium"),
      utm_campaign: params.get("utm_campaign"),
      referrer: document.referrer || null,
    });
  }, [pathname]);

  return null;
}
