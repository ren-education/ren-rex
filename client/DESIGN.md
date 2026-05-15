# rex client · Design System

> **Status:** v0.1 (Sage & Linen baseline)
> **Owner:** rex-client
> **Last updated:** 2026-05-15

This document specifies the rex client's design system: tokens, typography,
component conventions, and the theming architecture that lets us swap
aesthetic directions without touching component code.

---

## 1. Architecture: three-tier design tokens

The system follows the W3C Design Tokens / Adobe Spectrum / Material 3
convention of layering tokens by purpose.

```
┌──────────────────────────────────────────────────────────────────┐
│  Layer 1 · Reference tokens   (raw colors, scoped per theme)     │
│    --sage-500, --linen-100, --forest-700, ...                    │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼ mapped by theme
┌──────────────────────────────────────────────────────────────────┐
│  Layer 2 · Semantic tokens   (the API for components)            │
│    --background, --foreground, --primary, --accent, --border, ...│
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼ consumed by
┌──────────────────────────────────────────────────────────────────┐
│  Components (shadcn ui/* + app code)                             │
│    bg-background, text-foreground, ring-primary, border-border... │
└──────────────────────────────────────────────────────────────────┘
```

**Key invariant: components NEVER reference Layer 1.** They consume only
semantic tokens (Layer 2). A new theme is a new Layer 1 → Layer 2 mapping;
not a single component changes.

This means:

- **Swapping themes is one attribute change.** Change `<html data-theme="…">`
  in `app/layout.tsx` and the entire UI repaints. No rebuild needed.
- **Adding a theme is one new file.** Drop a `themes/<name>.css` that
  scopes its tokens under `[data-theme="<name>"]`. Import it from
  `globals.css`. Done.
- **shadcn components stay vanilla.** We never modify files in
  `components/ui/*` — they already use the semantic token names. Theming
  flows through CSS variables, not component edits.

---

## 2. File layout

```
src/app/
├── globals.css                ← plumbing only: tailwind, theme-inline mapping, base
└── themes/
    ├── _design-system.css     ← universal rules (typography, .num, em.match, .paper, .leadin)
    ├── sage-linen.css         ← A4 — the default theme
    └── warm-paper.css         ← A1 — alternative, kept for theme-switcher demo
```

To activate a different theme, edit `ACTIVE_THEME` in `app/layout.tsx`.

---

## 3. Active theme: Sage & Linen (A4)

### 3.1 Palette (reference)

| Role | Token | OKLCH | Approx hex |
|---|---|---|---|
| linen paper | `--linen-100` | `oklch(0.945 0.018 92)` | `#f1ede0` |
| paper card | `--linen-50`  | `oklch(0.965 0.016 90)` | `#f5f1e6` |
| paper-2     | `--linen-200` | `oklch(0.905 0.020 90)` | `#e6e0cd` |
| hairline    | `--linen-300` | `oklch(0.820 0.022 95)` | `#cdc7b3` |
| muted-fg    | `--linen-500` | `oklch(0.500 0.018 115)` | `#7c7d6b` |
| forest ink  | `--forest-700` | `oklch(0.245 0.025 142)` | `#1f2a1c` |
| ink-2       | `--forest-500` | `oklch(0.300 0.025 140)` | `#293a23` |
| sage accent (primary) | `--sage-500` | `oklch(0.500 0.078 135)` | `#4a6b3d` |
| sage soft (highlight bg) | `--sage-100` | `oklch(0.900 0.032 125)` | `#dde5d0` |
| oxblood (destructive) | `--oxblood-500` | `oklch(0.50 0.14 28)` | `#a23320` |

### 3.2 Semantic mappings (Layer 2 — what components see)

| Semantic | Light → | Dark → |
|---|---|---|
| `--background`         | `--linen-100` | dark forest `oklch(0.22 …)` |
| `--foreground`         | `--forest-700` | warm cream `oklch(0.93 …)` |
| `--card`               | `--linen-50` (one notch warmer than bg) | slightly lighter dark forest |
| `--primary`            | `--sage-500` | bright sage `oklch(0.68 …)` |
| `--primary-foreground` | `--linen-50` | dark forest |
| `--secondary`          | `--linen-200` | dark forest-2 |
| `--muted`              | `--linen-200` | dark forest-2 |
| `--muted-foreground`   | `--linen-500` | faded cream |
| `--accent` (hover/match-bg) | `--sage-100` | dim sage |
| `--accent-foreground` (match text) | `--sage-700` | bright sage |
| `--border`             | `--linen-300` | translucent white 10% |
| `--ring` (focus)       | `--sage-500` | bright sage |
| `--destructive`        | `--oxblood-500` | brighter oxblood |
| `--radius`             | `0.5rem` (book-spine corners) | same |

### 3.3 Voice

Sage & Linen is **calm, scholarly, considered**. Read it as: a botanical
herbarium or university reading room. Not flashy, not minimal-to-the-point-
of-coldness. Generous spacing, hairline rules instead of shadows, serif
headlines that feel printed.

---

## 4. Typography

| Role | Family | Size | Weight | Notes |
|---|---|---|---|---|
| Page headline (H1) | Source Serif 4 | 36px / 2.25rem | 400-500 | Italic e in brand mark |
| Section headline (H2) | Source Serif 4 | 28px / 1.75rem | 400-500 | |
| Result-card title  | Source Serif 4 | 19px | 500 | Always renders match highlights inline |
| Body text          | Geist (sans) | 16px | 400 | |
| UI label           | Geist (sans) | 13-14px | 400-500 | |
| Smallcaps meta     | Geist (sans) | 11px | 400 | `text-transform: uppercase`, `letter-spacing: 0.08em` |
| Italic context (leadin) | Source Serif 4 italic | 14-16px | 400 | `.leadin` utility |
| Numerics (scores, ms, marks) | JetBrains Mono | inherits | inherits | `.num` utility, `tabular-nums` |

Fonts are loaded via `next/font/google` in `app/layout.tsx` and exposed
as CSS variables (`--font-sans`, `--font-serif`, `--font-mono`). The
shorthand `--font-heading` is aliased to `--font-serif`.

---

## 5. Universal utility classes (from `_design-system.css`)

| Class | What it does | Use case |
|---|---|---|
| `.font-heading` | Force the serif family on any element. | Headings outside `<h1-h3>` (e.g., big numbers in dashboards). |
| `.smallcaps` | Smallcaps meta label. Uppercase, letter-spaced, `--muted-foreground`. | Per-hit meta row, sidebar section titles. |
| `.num` | Mono tabular numerals. | Scores, marks, page numbers, counts, timings. |
| `.leadin` | Italic serif, muted color. | Question context strings ("In a thought experiment…"). |
| `.paper` | Removes card background/shadow/ring, leaves only a bottom hairline. | Apply to `<Card>` in the result list to get the calm-book stack. |
| `em.match`, `[data-slot="hit"] em` | Sage-soft highlight on matched terms. | The server returns highlight HTML with `<em>` tags — they're styled automatically. |

These exist so a feature dev can compose styles using semantic class names
instead of repeating Tailwind atoms. They're not strict — Tailwind atoms
are still fine for one-off needs.

---

## 6. Component conventions

### 6.1 Use shadcn primitives as-is

We treat `components/ui/*` as a sealed library. **Never edit them in
place** — they're our shadcn baseline and we want to be able to update
them by pulling fresh shadcn copies. Theming flows through CSS variables.

If a component genuinely needs a variant we can't express via classes
(e.g., a completely new style of Card), copy the file to
`components/ui-rex/` and modify the copy. Don't touch the original.

### 6.2 Spacing & layout

- **Page gutter:** `px-6` on small screens, `px-8` to `px-12` on large. Max content width `max-w-5xl`.
- **Vertical rhythm:** `gap-6` between sibling sections; `gap-10` to `gap-12` between major chunks (header / search / results).
- **Hairline rules:** prefer `border-b border-border` over shadows. Cards should feel like pages stacked, not boxes floated.
- **Padding inside cards (when not `.paper`):** `p-4` to `p-6` depending on density.

### 6.3 The match highlight

The rex API returns `highlights: { field, text }[]`. `text` contains
HTML with `<em>` tags around matched substrings. **Render it via
`dangerouslySetInnerHTML`** — the design system styles `em` globally
inside `[data-slot="hit"]` and `.hit` scopes, so wrapping isn't needed.

### 6.4 PDF anchors and fallback reasons

When a `pdf_anchor.fallback_reason` is set, surface it as italicized
text in the destructive/70 color next to the link. Possible values:
`LowConfidence`, `PdfReadFailed`, `PdfNotFound`. This is an honesty
signal — students need to know if the "Open PDF" link will take them
to the exact page or just the file.

---

## 7. Adding or switching themes

### To switch to an existing theme

1. Open `src/app/layout.tsx`.
2. Change the `ACTIVE_THEME` constant to the new name (e.g. `"warm-paper"`).
3. Save. Next.js HMR repaints; no rebuild needed.

### To add a new theme (e.g. "pearl-indigo")

1. Create `src/app/themes/pearl-indigo.css`.
2. Define `[data-theme="pearl-indigo"] { … }` with reference + semantic
   tokens following the same shape as `sage-linen.css`.
3. Add a `[data-theme="pearl-indigo"].dark { … }` block for dark mode.
4. Add `@import "./themes/pearl-indigo.css";` to `globals.css`.
5. Activate by changing `ACTIVE_THEME` in `layout.tsx`.

### To add a runtime theme switcher

The architecture is ready — `data-theme` is just an attribute. Wire a
client-side picker that calls
`document.documentElement.setAttribute("data-theme", value)` and persists
the choice to `localStorage`. Hydration safety: pass the value through
`<html>` server-side so the first paint matches.

---

## 8. Accessibility checklist

| Item | Status |
|---|---|
| Focus rings visible on every interactive element | ✅ `--ring` is sage-500, contrasts on both bg and card |
| Color is never the only state signal | ✅ active filter chips have a "·" or fill change, not just color |
| Contrast of body text on background | AA at 16px (forest-700 on linen-100 ≈ 11:1) |
| Contrast of muted text | AA at 14px (linen-500 on linen-100 ≈ 4.6:1) — verify per platform |
| Contrast of primary action button | AA (linen-50 on sage-500 ≈ 8.3:1) |
| Dark mode mirrors all of the above | ✅ same tier of ratios |
| Reduced motion respected | Tailwind v4 + `tw-animate-css` honor `prefers-reduced-motion`; no JS-driven motion |

---

## 9. What this system is NOT (yet)

- **Not a marketing brand system.** No logos, no marketing typography
  stack, no illustrative palette. Just enough to make the product feel
  considered.
- **Not a component-token system.** We stop at semantic tokens (Layer 2).
  If a future component needs a per-component override (Layer 3) we'll
  add it in a focused way, not globally.
- **Not internationalized.** Type metrics assume Latin script. RTL is
  disabled in `components.json`.
- **Not motion-rich.** Animations are limited to subtle hover/transition
  on borders and `tw-animate-css` enter/leave; no scroll-driven motion,
  no springs.

---

## 10. References

- W3C Design Tokens spec — <https://www.designtokens.org/>
- shadcn theming guide — <https://ui.shadcn.com/docs/theming>
- Tailwind CSS v4 `@theme` directive — <https://tailwindcss.com/docs/theme>
- Source Serif 4 (Adobe via Google Fonts) — <https://fonts.google.com/specimen/Source+Serif+4>
- Direction A color exploration — `client/design-explorations/`
