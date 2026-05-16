# GP Question Bank — Analysis

Companion to `dashboard.html`. The dashboard shows the numbers; this file
explains what they mean and what jumped out while running queries.

> Source: `ren-rex/server/rex.db` (subject_id = `gp`) — 5,899 questions and
> 843 notes, drawn from 26 junior colleges and tagged across 314 topics.
> Source-year is parsed from the filename and is available for ~3.8k of the
> 6.7k documents (the rest are from files without a year stamp).
>
> **Updated after school re-indexing.** School count dropped 31 → 26 as
> aliases were collapsed; signatures and lifts below reflect the cleaned
> taxonomy. Some pre-merger historical names (Anderson JC, Pioneer JC,
> Tampines JC, Innova JC, Yishun JC, Raffles JC) are still kept distinct
> from their post-merger successors (ASJC, JPJC, TMJC, YIJC, RI) — that
> may be intentional, but worth confirming.

## Headline numbers

| | Count |
|---|---|
| Questions | 5,899 |
| With a mark assigned | 4,854 (82%) |
| With a model answer | 1,737 (29%) |
| Source-year parseable | 3,825 (57%) |
| Distinct topics tagged | 314 |
| Distinct schools | 26 (was 31 pre-reindex) |
| Year range | 2008 – 2025 |

The papers cluster heavily in **2013** (778 questions, mostly SRJC's solutions
bundle) and **2022–2025** (~1.9k questions). Treat anything before 2019 as a
historical sample, not a trend baseline.

## What the data is actually telling you

### 1. Climate change is the standout riser
Comparing pre-2020 papers to 2022-and-newer:

```
+4.2pp  climate-change                (1.5% → 5.7% of all topic tags)
+3.4pp  change-and-adaptation         (1.9% → 5.3%)
+3.0pp  city-life                     (0.3% → 3.3%)
+2.9pp  architecture-and-design       (0% → 2.9%)
+2.5pp  humour                        (0.1% → 2.6%)
```

Climate's growth is real and not a single-school artefact — it shows up
across paper-1 essay prompts at multiple JCs. If you're picking essay topics
to prep, climate is the one with the strongest tailwind. **In 2024 alone it
was tagged on ~11% of GP questions**, more than triple its 2020 share.

### 2. The decliners tell a story about the syllabus shifting away from "lifestyle" themes

```
-3.5pp  education
-2.9pp  identity-and-individuality
-2.5pp  science-and-technology       (note: distinct from technology-and-society)
-2.1pp  favouritism-and-fairness
-2.0pp  cosmetics-and-fashion
```

The pattern: examiners have moved away from soft "lifestyle" essays
(fashion, leisure-and-busyness, identity) toward systemic topics (climate,
cities, governance). `science-and-technology` shrinking *while*
`technology-and-society` stays flat suggests the framing has shifted from
"is technology good?" to "what does technology do to people?". Worth
matching your prep angle accordingly.

### 3. Each school has a discernible "house style"
The signature topics (over-represented vs the corpus base rate, ≥8 questions):

| School | Signature topic | Lift | n |
|---|---|---|---|
| Jurong-Pioneer JC | success-and-materialism | ×28.6 | 16 |
| Millennia | fairness-and-favouritism | ×25.6 | 10 |
| Dunman High | rudeness-and-civility | ×22.8 | 16 |
| Temasek JC | online-shopping-and-ecommerce | ×22.5 | 13 |
| Anderson JC | rules-and-order | ×21.8 | 16 |
| St Andrew's JC | urban-farming | ×18.7 | 16 |
| Jurong JC | minority-languages | ×18.1 | 24 |
| National JC | collecting | ×17.3 | 8 |
| Catholic JC | mechanics | ×17.2 | 54 |
| Hwa Chong | resilience-and-coping | ×16.9 | 9 |
| Nanyang JC | rules-and-anarchy | ×16.7 | 11 |
| VJC | noise-and-society | ×16.1 | 14 |
| ACJC | robotics | ×15.3 | 17 |
| RI | money-and-happiness | ×15.3 | 11 |
| River Valley | advertising-and-consumerism | ×15.0 | 12 |

These aren't the schools' *most-asked* topics — they're the topics where
each school disproportionately leans in. If you're training on a specific
school's papers, expect those angles to recur. Note that JJC's lean on
`minority-languages` (24 questions, ×18) is one of the strongest
school-topic signals in the corpus by absolute volume.

### 4. The `essay` slice is the right thing to prep against
Of the 1,980 essays, the top 5 essay topics — **governance-and-leadership
(232), singapore-society (185), arts-and-culture (162),
technology-and-society (152), values-and-ethics (134)** — together make up
about half of all essays. Sample prompts from each:

- *governance-and-leadership* — "Bad government results from too much
  government. Do you agree?"
- *singapore-society* — "'Singapore for Singaporeans.' Discuss."
- *arts-and-culture* — "'A society that tells its artists what they cannot
  do short-changes itself.' Discuss."
- *values-and-ethics* — "'Sacrificing for the greater good is always worth
  it.' Comment."
- *technology-and-society* — "The most divisive force in the modern world
  is new media. Do you agree?"

These cluster into two essay-question shapes: (1) "X is the most/best Y —
agree?" and (2) "Discuss the impact of X on Y." Practising those two
templates covers most of the surface.

### 5. Topic co-occurrence reveals natural argument bundles
The top co-tagged pairs are the topic clusters worth memorising as
*evidence buckets* — examples that work for one almost always work for the
other:

```
70  city-life + urbanisation
55  cultural-identity + globalisation
42  governance-and-leadership + politics-and-public-rhetoric
41  media-and-public + technology-and-society
37  city-life + liveability
36  ageing + elderly
36  climate-change + environment-and-sustainability
33  conformity + individualism
```

Practical use: when you build an essay example bank, file each example
under the *cluster*, not the individual topic.

## Data-quality flags worth raising

**1. Physics papers still mis-routed into GP** (unchanged from previous
re-index). `mechanics` (54 hits, all from CJC) and `physics` appear in
the GP topic list. Source files `J2.H2.PRELIM.2022.P3_Question Paper`
and the matching solutions are H2 Physics papers asking about momentum
and spring constants — they should be under `h2physics`, not `gp`. This
is why CJC's signature topic is `mechanics` ×17.2. Fix upstream in the
ingest pipeline (subject-aware routing).

**2. Pre/post-merger school aliases still split.** The re-index dropped
the school count from 31 to 26, but several historical-name pairs still
exist as separate schools:

| Successor (post-merger) | Historical name(s) still in DB |
|---|---|
| Anderson Serangoon JC (`anderson-serangoon-jc`, 187) | `anderson-junior-college` (228), `serangoon-junior-college` (208) |
| Jurong-Pioneer JC (`jurong-pioneer-junior-college`, 225) | `jurong-junior-college` (242), `pioneer-junior-college` (165) |
| Tampines-Meridian JC (`tampines-meridian-junior-college`, 191) | `tampines-junior-college` (66), `meridian-junior-college` (203) |
| Yishun Innova JC (`yishun-innova-junior-college`, 109) | `yishun-junior-college` (168), `innova-junior-college` (193) |
| Raffles Institution (`raffles-institution`, 416) | `raffles-junior-college` (30) |

This may be deliberate — pre-2019 papers really were set by different
faculties — but it inflates the apparent number of schools and dilutes
school-level signal. If you want to compare *current* JCs head-to-head,
collapse each row above into a single bucket. If you want to track how
each school's question style evolved across the merger, keep them
separate but tag them as alias-pairs in the schema.

## How to extend this

- **`build_dashboard.py`** is the only thing you need to re-run. It reads
  the SQLite DB directly, so it picks up new ingests automatically.
- Output lands in `dashboard.html` (interactive) and `summary.json` (the
  numbers behind the KPIs and topic counts, for downstream scripts).
- To add a chart: write a function that returns a `plotly.graph_objects.Figure`
  and append `(heading, fig)` to the list in `main()`.
- To analyse a different subject: change `subject_id='gp'` in the two
  queries in `load()` and the keyword query in `chart_keyword_topics()`.

## Open via
```sh
open analytics/gp/dashboard.html
```
