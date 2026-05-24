"use client";

import { useEffect, useState } from "react";
import { ArrowUpRight, X } from "lucide-react";
import { captureRexEvent } from "@/lib/posthog-client";

// Per-session dismissal. The chip is meant to be present-but-quiet across
// fresh visits, so we use sessionStorage (cleared when the tab closes) rather
// than localStorage (persists forever). A returning user from a new session
// gets one more look at the CTA; within a session, dismissing it sticks.
const SESSION_KEY = "rex.rubric_cta_dismissed";

// `?ref=rex` so the rubric side can see in analytics how much traffic the
// chip drives, separately from organic visits.
const RUBRIC_URL = "https://rubric.reneducation.com?ref=rex";

export function RubricCtaChip() {
  // `mounted` guards against SSR/CSR mismatch: at server-render time we have
  // no access to sessionStorage, so we render null until the client's effect
  // has read storage. Brief flash of nothing is acceptable for a soft CTA.
  const [mounted, setMounted] = useState(false);
  const [dismissed, setDismissed] = useState(false);

  useEffect(() => {
    try {
      setDismissed(sessionStorage.getItem(SESSION_KEY) === "1");
    } catch {
      // SSR-safe; private mode can throw on storage access. Default to shown.
    }
    setMounted(true);
  }, []);

  function dismiss() {
    setDismissed(true);
    try {
      sessionStorage.setItem(SESSION_KEY, "1");
    } catch {
      // Failure to persist is acceptable — chip stays hidden for this view.
    }
  }

  function trackRubricClick() {
    captureRexEvent("rubric_cta_click", {
      button_location: "header_chip",
      destination: RUBRIC_URL,
      ref: "rex",
    });
  }

  if (!mounted || dismissed) return null;

  return (
    <span className="inline-flex items-baseline gap-2">
      {/* Divider matches the visual weight of the "by ren." subscript so the
          chip reads as a sibling attribution, not as a louder element. */}
      <span aria-hidden className="text-border">|</span>
      <a
        href={RUBRIC_URL}
        target="_blank"
        rel="noopener noreferrer"
        onClick={trackRubricClick}
        className="group inline-flex items-baseline gap-1 text-[0.7rem] text-muted-foreground transition-colors hover:text-foreground"
      >
        <span>
          <span className="hidden sm:inline">Grade your own essays with </span>
          <span className="font-sans font-bold tracking-tight text-foreground">
            rubric
          </span>
        </span>
        <ArrowUpRight
          aria-hidden
          className="size-3 translate-y-0.5 transition-transform group-hover:translate-x-0.5 group-hover:-translate-y-0"
        />
      </a>
      <button
        type="button"
        onClick={dismiss}
        aria-label="Dismiss rubric link"
        className="text-muted-foreground/60 transition-colors hover:text-foreground"
      >
        <X className="size-3" />
      </button>
    </span>
  );
}
