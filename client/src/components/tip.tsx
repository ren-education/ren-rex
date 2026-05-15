"use client";

import * as React from "react";
import { cn } from "@/lib/utils";

/**
 * Minimal CSS-only hover tooltip themed via the rex token system.
 * Use for one-line hints; no JS portal / focus-trap / arrow positioning.
 * Wraps inline (display: inline-flex) so it composes inside flex rows.
 *
 *   <Tip label="Combines BM25 + vector + rerank">…</Tip>
 *
 * For multiline content, longer hints, or keyboard focus on the tip
 * itself, swap to a proper popover component (shadcn tooltip).
 */
interface TipProps extends React.HTMLAttributes<HTMLSpanElement> {
  label: string;
  side?: "top" | "bottom";
}

export function Tip({
  label,
  side = "top",
  children,
  className,
  ...rest
}: TipProps) {
  return (
    <span
      className={cn("group/tip relative inline-flex", className)}
      {...rest}
    >
      {children}
      <span
        role="tooltip"
        className={cn(
          "pointer-events-none absolute left-1/2 z-30 -translate-x-1/2 whitespace-normal",
          "max-w-[260px] rounded-md border border-border bg-popover px-2.5 py-1.5",
          "text-xs leading-snug text-popover-foreground shadow-sm",
          "opacity-0 transition-opacity duration-150 group-hover/tip:opacity-100 group-focus-within/tip:opacity-100",
          side === "top"
            ? "bottom-full mb-1.5"
            : "top-full mt-1.5",
        )}
      >
        {label}
      </span>
    </span>
  );
}
