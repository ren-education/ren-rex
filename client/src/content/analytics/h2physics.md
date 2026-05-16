# H2 Physics Question Bank — Analysis

Companion to `dashboard.html`. The dashboard shows the numbers; this file
explains what they mean and what jumped out while running queries.

> Source: `ren-rex/server/rex.db` (subject_id = `h2physics`) — 10,305
> questions and 3,061 notes from 24 junior colleges, tagged across 858
> topics. Year-parseable for **12,940 / 13,366** documents (97%) — the
> trend signal here is much stronger than for GP.

## Headline numbers

| | Count |
|---|---|
| Questions | 10,305 |
| With a mark assigned | 8,941 (87%) |
| With a model answer | 5,209 (51%) |
| Source-year parseable | 12,940 (97%) |
| Distinct topics tagged | 858 |
| Distinct schools | 24 |
| Year range | 2011 – 2025 |

The corpus is well-distributed across years (2018, 2022, 2023, 2025 each
have 1.4k–1.8k docs). Trend signals here are reliable.

## What the data is telling you

### 1. Mechanics is rising; quantum/nuclear is fading
Comparing 2018-and-earlier vs 2023+ papers:

```
+1.4pp  dynamics                   (4.0% → 5.4%)
+1.2pp  forces                     (6.0% → 7.2%)
+1.1pp  measurement                (3.6% → 4.7%)
+1.1pp  circular-motion            (3.9% → 5.0%)
+1.0pp  alternating-current        (1.1% → 2.1%)
+0.9pp  momentum                   (2.0% → 2.9%)
+0.9pp  kinetic-theory             (1.0% → 1.9%)
```

```
-2.5pp  quantum-physics            (7.3% → 4.7%)
-1.7pp  nuclear-physics            (7.6% → 5.9%)
-1.5pp  thermal-physics            (4.4% → 2.9%)
-1.2pp  dc-circuits                (4.3% → 3.1%)
-1.1pp  photoelectric-effect       (3.1% → 2.0%)
-0.9pp  semiconductors             (1.1% → 0.2%)
```

Quantum and nuclear are still the most-tagged topics overall, but their
share of recent papers has dropped meaningfully — likely a syllabus or
emphasis shift toward classical/applied mechanics. **Don't over-prep on
photoelectric or semiconductors based on TYS frequency** — the recent
papers feature them less.

`measurement` rising is a quiet signal worth taking seriously: more
recent papers test experimental design and uncertainty analysis.

### 2. The "study these together" topic clusters
Top co-occurring topic pairs (questions tagged with both):

```
340  oscillations + simple-harmonic-motion       (basically the same topic)
247  photoelectric-effect + quantum-physics
242  interference + superposition
229  kinematics + projectile-motion
226  equilibrium + forces
166  superposition + waves
151  electromagnetism + magnetic-fields
142  dynamics + forces
138  equilibrium + moments
130  dynamics + momentum
```

Practical use: when you build a problem set, group these — solving one
half usually requires the other. The `oscillations`/`SHM` pair being the
strongest co-occurrence (×4 the next highest) suggests the tagger is
double-tagging redundant labels; you can fold them in your prep.

### 3. The mark distribution reveals what kind of test this is
```
1 mark:   4,082 (mostly MCQ)
2 marks:  2,663 (short structured)
3 marks:    896
4-12 marks:  ~1,300 (long structured)
no mark:  4,425 (notes + un-marked items)
```

Roughly **75% of marked questions are 1-3 marks**. This is a high-volume,
quick-recall + short-calculation discipline; the prep strategy is
"hundreds of small problems," not "deep essays." It's the inverse of the
GP corpus, where 50-mark essays dominate.

### 4. School signatures are weaker than in GP
Top over-represented topic per school (≥8 questions):

| School | Signature topic | Lift |
|---|---|---|
| VJC | general-physics | ×21.7 |
| HCI | microwaves | ×10.9 |
| NJC | work-energy | ×8.4 |
| EJC | uncertainty-analysis | ×6.9 |
| RI | elastic-collision | ×6.8 |
| DHS | centre-of-gravity | ×7.8 |

Lifts are smaller (×3–10) than GP's (×15–25). Physics syllabus is more
homogenised — every school covers the same content list, so school
"style" lives in problem framing, not topic selection. Probably not
worth optimising prep by school here.

## Data-quality flags

- **`general-physics`** appears as a topic with 30 hits at VJC alone, ×22
  base-rate lift. Looks like a fallback tag for content the tagger
  couldn't categorise — worth filtering or re-tagging upstream.
- **`physics`** also appears as a topic (NYJC's signature) — same kind of
  meta-tag noise. Real topics in this domain should be sub-discipline
  level (mechanics, electromagnetism, quantum, etc.).
- **`oscillations` and `simple-harmonic-motion`** are tagged together on
  340 questions — the second-highest co-occurrence in the entire corpus.
  These are the same topic; consider deduping the tag vocabulary.

## How to extend

Re-run with: `python3 analytics/gp/build_dashboard.py h2physics physics`
