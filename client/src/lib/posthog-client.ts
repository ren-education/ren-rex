"use client";

import posthog from "posthog-js";

let initialized = false;

export function initPostHog() {
  if (typeof window === "undefined") return false;
  if (initialized) return true;

  const key = process.env.NEXT_PUBLIC_POSTHOG_KEY;
  if (!key) return false;

  posthog.init(key, {
    api_host:
      process.env.NEXT_PUBLIC_POSTHOG_HOST ?? "https://us.i.posthog.com",
    capture_pageview: false,
  });
  initialized = true;
  return true;
}

export function captureRexEvent(
  event: string,
  properties?: Record<string, unknown>,
) {
  if (!initPostHog()) return;

  posthog.capture(event, {
    app: "rex",
    ...properties,
  });
}
