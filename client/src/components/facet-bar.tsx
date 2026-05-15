"use client";

import { ChevronDown, X } from "lucide-react";
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";
import { FACET_FIELDS, type FacetField, type FacetMap } from "@/lib/use-facets";
import type { DocumentKind, Filters } from "@/lib/types";

/** Human-readable name for each facet field. */
const FIELD_LABEL: Record<FacetField, string> = {
  topics:         "Topics",
  schools:        "Schools",
  paper_types:    "Paper",
  source_types:   "Source",
  question_types: "Question",
};

/** Pretty-print a tag value: replace dashes with spaces, title-case. */
function prettyValue(s: string): string {
  return s
    .split("-")
    .map((w) => (w.length <= 3 ? w.toUpperCase() : w[0].toUpperCase() + w.slice(1)))
    .join(" ");
}

interface FacetBarProps {
  filters: Filters;
  facets: FacetMap;
  onToggle: (field: FacetField, value: string) => void;
  onClear: (field: FacetField) => void;
  onClearAll: () => void;
  onKindChange: (kind: DocumentKind | null) => void;
}

export function FacetBar({
  filters,
  facets,
  onToggle,
  onClear,
  onClearAll,
  onKindChange,
}: FacetBarProps) {
  const anySelected =
    FACET_FIELDS.some((f) => selected(filters, f).length > 0) || !!filters.kind;
  const currentKind: string = filters.kind ?? "All";

  return (
    <div className="flex flex-col gap-2">
      {/* Facet dropdowns */}
      <div className="flex flex-wrap items-center gap-1.5">
        {/* Question / Note / All — three-state segmented control. Distinct
            from the tag facets since `kind` is a single-select on the
            Filters object (not a multi-select tag array). */}
        <Tabs
          value={currentKind}
          onValueChange={(v) => onKindChange(v === "All" ? null : (v as DocumentKind))}
        >
          <TabsList>
            <TabsTrigger value="All">All</TabsTrigger>
            <TabsTrigger value="Question">Questions</TabsTrigger>
            <TabsTrigger value="Note">Notes</TabsTrigger>
          </TabsList>
        </Tabs>

        {FACET_FIELDS.map((field) => {
          const sel = selected(filters, field);
          const values = facets[field] ?? [];
          if (values.length === 0 && sel.length === 0) return null;
          return (
            <FacetDropdown
              key={field}
              field={field}
              values={values}
              selected={sel}
              onToggle={(v) => onToggle(field, v)}
            />
          );
        })}
        {anySelected && (
          <Button
            type="button"
            size="xs"
            variant="ghost"
            className="ml-auto text-muted-foreground"
            onClick={onClearAll}
          >
            Clear all
          </Button>
        )}
      </div>

      {/* Applied-filter chips. Same info as the dropdown counts, but
          rendered as removable inline chips so it's always visible
          which filters are constraining the result set. */}
      {anySelected && (
        <div className="flex flex-wrap items-center gap-1.5">
          {FACET_FIELDS.map((field) =>
            selected(filters, field).map((v) => (
              <button
                key={`${field}:${v}`}
                type="button"
                onClick={() => onToggle(field, v)}
                className={cn(
                  "inline-flex items-center gap-1.5 rounded-full border border-border",
                  "bg-accent/40 px-2.5 py-0.5 text-xs text-foreground",
                  "transition-colors hover:bg-accent",
                )}
              >
                <span className="text-muted-foreground">
                  {FIELD_LABEL[field]}:
                </span>
                {prettyValue(v)}
                <X className="size-3 text-muted-foreground" />
              </button>
            )),
          )}
          {FACET_FIELDS.map((field) =>
            selected(filters, field).length > 1 ? (
              <button
                key={`clear:${field}`}
                type="button"
                onClick={() => onClear(field)}
                className="text-xs italic text-muted-foreground underline-offset-2 hover:underline"
              >
                clear {FIELD_LABEL[field].toLowerCase()}
              </button>
            ) : null,
          )}
        </div>
      )}
    </div>
  );
}

interface FacetDropdownProps {
  field: FacetField;
  values: Array<{ value: string; count: number }>;
  selected: string[];
  onToggle: (value: string) => void;
}

function FacetDropdown({ field, values, selected, onToggle }: FacetDropdownProps) {
  const label = FIELD_LABEL[field];
  const sortedValues = [...values].sort((a, b) => b.count - a.count);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button
            type="button"
            variant={selected.length ? "secondary" : "outline"}
            size="sm"
          >
            <span>{label}</span>
            {selected.length > 0 && (
              <span className="num ml-1 rounded-md bg-primary/15 px-1.5 text-primary">
                {selected.length}
              </span>
            )}
            <ChevronDown className="size-3.5 opacity-60" />
          </Button>
        }
      />
      <DropdownMenuContent
        align="start"
        className="max-h-[360px] w-60 overflow-y-auto"
      >
        {sortedValues.length === 0 && (
          <div className="px-2 py-3 text-xs italic text-muted-foreground">
            No values match current filters.
          </div>
        )}
        {sortedValues.map((v) => (
          <DropdownMenuCheckboxItem
            key={v.value}
            checked={selected.includes(v.value)}
            onCheckedChange={() => onToggle(v.value)}
            closeOnClick={false}
            className="text-xs"
          >
            <span className="flex-1 truncate">{prettyValue(v.value)}</span>
            <span className="num ml-2 text-muted-foreground">{v.count}</span>
          </DropdownMenuCheckboxItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** Read the selected values for a given field out of the Filters object. */
function selected(filters: Filters, field: FacetField): string[] {
  // Filters' field names match the FacetField string values exactly.
  return (filters as Record<string, unknown>)[field] as string[] | undefined ?? [];
}
