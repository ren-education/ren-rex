"use client";

/**
 * Client wrapper for Recharts-backed analytics visualisations.
 *
 * Recharts uses React.createContext under the hood, which can only run in
 * Client Components — so all chart rendering has to live in a "use client"
 * boundary. The server page (analytics/[subject]/page.tsx) does the data
 * shaping (`densifyBySeries`) and hands plain serialisable arrays across
 * the boundary; this file does nothing but render them.
 *
 * After the focus pass that dropped the question-type, paper-type,
 * mark-distribution, and year-volume charts, only the stacked-area trend
 * chart remains. If we re-add a bar/horizontal-bar chart later, look at
 * the git history for `HorizontalBarChart` — it had the right shape and
 * styling already.
 */

import {
  Area,
  AreaChart,
  CartesianGrid,
  Legend,
  XAxis,
  YAxis,
} from "recharts";
import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from "@/components/ui/chart";
import { humanizeTag } from "@/lib/analytics";

/**
 * Stacked-area chart over time. Used for topic-by-year — series sit on
 * top of each other and read like geological strata: each band's thickness
 * encodes that series' share of the year. Earlier (largest) series sit
 * at the bottom of the stack and are most visible, which is the order
 * `densifyBySeries` preserves from the upstream JSON.
 *
 * The chart cycles through `--chart-1` … `--chart-5` for series colors;
 * series 6+ re-use the same tokens but read as the same hue family
 * because we only have 5 sage tokens. Visual ambiguity for the tail is
 * acceptable since the largest series (which we color first) carry the
 * bulk of the signal.
 */
export function StackedAreaOverTime({
  keys,
  rows,
}: {
  keys: string[];
  rows: Array<Record<string, number> & { year: number }>;
}) {
  const config: ChartConfig = {};
  keys.forEach((k, i) => {
    const slot = (i % 5) + 1; // 1..5
    config[k] = {
      label: humanizeTag(k),
      colorVar: `chart-${slot}`,
    };
  });

  return (
    <ChartContainer
      config={config}
      className="h-full w-full [&_.recharts-default-legend]:!flex [&_.recharts-default-legend]:!flex-wrap [&_.recharts-default-legend]:!justify-start [&_.recharts-default-legend]:!gap-x-3 [&_.recharts-default-legend]:!gap-y-1 [&_.recharts-default-legend-item-text]:!text-muted-foreground [&_.recharts-default-legend-item-text]:!text-[11px]"
    >
      <AreaChart data={rows} margin={{ top: 4, right: 12, bottom: 24, left: 4 }}>
        <CartesianGrid vertical={false} strokeDasharray="2 3" />
        <XAxis
          dataKey="year"
          tickLine={false}
          axisLine={false}
          tickMargin={6}
          fontSize={11}
        />
        <YAxis tickLine={false} axisLine={false} width={32} fontSize={11} />
        <ChartTooltip content={<ChartTooltipContent labelKey="year" />} />
        {keys.map((k) => (
          <Area
            key={k}
            type="monotone"
            dataKey={k}
            stackId="1"
            stroke={`var(--color-${k})`}
            fill={`var(--color-${k})`}
            fillOpacity={0.7}
            strokeWidth={0.5}
          />
        ))}
        <Legend
          iconType="square"
          iconSize={8}
          wrapperStyle={{ paddingTop: 8 }}
        />
      </AreaChart>
    </ChartContainer>
  );
}
