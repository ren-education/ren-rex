"use client";

/**
 * Minimal shadcn-style chart primitives. Wraps Recharts and exposes the
 * chart palette through CSS variables so charts respect the active rex
 * theme (sage-linen → sage progression on `--chart-1` … `--chart-5`).
 *
 * Why a custom file instead of the full shadcn/ui chart component:
 *   - We only need ChartContainer + a themed tooltip, not the full
 *     legend/cursor/gradient kit.
 *   - The official component pulls in extra config-driven indirection;
 *     stripping it keeps charts predictable to read at the call site.
 *   - The CSS-variable bridge below is the only non-obvious bit, and
 *     we'd be writing it the same way regardless.
 */

import * as React from "react";
import * as RechartsPrimitive from "recharts";
import { cn } from "@/lib/utils";

/** Map of dataKey → display label / color slot. */
export type ChartConfig = {
  [k: string]: {
    label?: React.ReactNode;
    /** Reference a CSS var on the theme (e.g. "chart-1", "primary", "muted"). */
    colorVar?: string;
  };
};

const ChartContext = React.createContext<{ config: ChartConfig } | null>(null);

function useChart() {
  const ctx = React.useContext(ChartContext);
  if (!ctx) throw new Error("Chart components must be inside <ChartContainer>");
  return ctx;
}

/**
 * Wraps a Recharts container and injects per-series CSS variables so
 * Recharts series can render with `fill="var(--color-<key>)"`. This is the
 * mechanism that lets the same chart component re-color when `data-theme`
 * flips on <html>.
 */
export function ChartContainer({
  config,
  className,
  children,
  ...rest
}: React.ComponentProps<"div"> & {
  config: ChartConfig;
  children: React.ReactElement;
}) {
  const cssVars = React.useMemo<React.CSSProperties>(() => {
    const out: Record<string, string> = {};
    for (const [key, value] of Object.entries(config)) {
      if (value.colorVar) {
        out[`--color-${key}`] = `var(--${value.colorVar})`;
      }
    }
    return out as React.CSSProperties;
  }, [config]);

  return (
    <ChartContext.Provider value={{ config }}>
      <div
        data-chart=""
        style={cssVars}
        className={cn(
          // Mirrors shadcn's chart wrapper styling: small text in muted-fg
          // for axis labels, semi-transparent grid lines, etc.
          "flex aspect-video justify-center text-xs",
          "[&_.recharts-cartesian-axis-tick_text]:fill-muted-foreground",
          "[&_.recharts-cartesian-grid_line[stroke='#ccc']]:stroke-border/50",
          "[&_.recharts-curve.recharts-tooltip-cursor]:stroke-border",
          "[&_.recharts-polar-grid_[stroke='#ccc']]:stroke-border",
          "[&_.recharts-radial-bar-background-sector]:fill-muted",
          "[&_.recharts-rectangle.recharts-tooltip-cursor]:fill-muted/40",
          "[&_.recharts-reference-line_[stroke='#ccc']]:stroke-border",
          "[&_.recharts-sector[stroke='#fff']]:stroke-transparent",
          "[&_.recharts-sector]:outline-none",
          "[&_.recharts-surface]:outline-none",
          className,
        )}
        {...rest}
      >
        <RechartsPrimitive.ResponsiveContainer>
          {children}
        </RechartsPrimitive.ResponsiveContainer>
      </div>
    </ChartContext.Provider>
  );
}

/** Pass-through; just exists so consumers import a single namespace. */
export const ChartTooltip = RechartsPrimitive.Tooltip;

/**
 * Themed tooltip body. Renders the active data point as
 *   <label>: <number>
 * with rex's surface tokens. Used by the per-subject charts via the
 * `content={<ChartTooltipContent />}` prop on Recharts `<Tooltip>`.
 */
export function ChartTooltipContent({
  active,
  payload,
  label,
  labelKey,
  nameKey,
  className,
}: {
  active?: boolean;
  payload?: Array<{ name?: string; value?: number; dataKey?: string; payload?: Record<string, unknown> }>;
  label?: string | number;
  labelKey?: string;
  nameKey?: string;
  className?: string;
}) {
  const { config } = useChart();
  if (!active || !payload?.length) return null;

  const headline = (() => {
    if (labelKey && payload[0]?.payload && labelKey in payload[0].payload) {
      return String(payload[0].payload[labelKey]);
    }
    return label;
  })();

  return (
    <div
      className={cn(
        "rounded-md border border-border bg-popover px-3 py-2",
        "shadow-sm text-xs text-popover-foreground",
        "min-w-[8rem]",
        className,
      )}
    >
      {headline !== undefined && (
        <div className="mb-1 font-medium text-foreground/90">{headline}</div>
      )}
      <div className="flex flex-col gap-0.5">
        {payload.map((item, i) => {
          const key = nameKey ? String(item.payload?.[nameKey] ?? item.name ?? "") : (item.dataKey ?? item.name ?? "");
          const label = config[String(key)]?.label ?? item.name ?? key;
          return (
            <div key={i} className="flex items-center justify-between gap-3">
              <span className="flex items-center gap-1.5 text-muted-foreground">
                <span
                  className="size-2 rounded-[2px]"
                  style={{ background: `var(--color-${key})` }}
                  aria-hidden
                />
                {label}
              </span>
              <span className="font-mono tabular-nums text-foreground">
                {item.value?.toLocaleString()}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
