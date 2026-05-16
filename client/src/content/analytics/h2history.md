# H2 History Question Bank — Analysis

Companion to `dashboard.html`. The dashboard shows the numbers; this file
explains what they mean and what jumped out while running queries.

> Source: `ren-rex/server/rex.db` (subject_id = `h2history`) — 1,342
> questions and 515 notes from 24 junior colleges, tagged across 1,429
> topics. Year range 2000-2025 but the early sample is small (~880 docs
> before 2017 vs ~340 from 2023+). Treat the trend signal as directional,
> not statistical.

## Headline numbers

| | Count |
|---|---|
| Questions | 1,342 |
| With a mark assigned | 778 (58%) |
| With a model answer | n/a (not set on most rows) |
| Year-parseable docs | most (small unknown-year tail) |
| Distinct topics tagged | 1,429 (high-cardinality, granular) |
| Distinct schools | 24 |
| Year range | 2000 – 2025 |

History is a **smaller and more granular corpus** than physics. 1,429
distinct topic tags across 1,857 docs means most topics appear once or
twice — the long tail is dominant. This is normal for an essay-driven
subject but worth knowing when you build any clustering view.

## What the data is telling you

### 1. The biggest story: a syllabus shift toward country case-studies
Comparing pre-2017 papers vs 2023+ papers (caveat: small late sample):

```
+6.2pp  indonesia                       (0.3% → 6.5%)
+5.4pp  china-economic-transformation   (1.1% → 6.5%)
+3.5pp  vietnam                         (1.6% → 5.1%)
+3.3pp  political-stability             (0.6% → 3.9%)
+2.9pp  nation-building                 (1.3% → 4.2%)
+2.9pp  authoritarianism                (2.5% → 5.4%)
+2.8pp  japan-economic-transformation   (1.9% → 4.8%)
```

```
-11.6pp  united-nations          (11.6% → 0.0%)
-8.1pp   regional-conflicts      (12.5% → 4.5%)
-5.7pp   peacekeeping            (7.5% → 1.8%)
-5.4pp   cold-war-end            (5.4% → 0.0%)
-5.1pp   superpowers             (6.3% → 1.2%)
-4.7pp   pre-war-nationalism     (4.7% → 0.0%)
-4.6pp   forming-nation-states   (12.9% → 8.3%)
-4.0pp   nationalism             (5.5% → 1.5%)
```

This isn't drift — it's a **regime change**. The pre-2017 paper was
organised around macro-level Cold War / UN / regional-conflict themes;
recent papers shift toward specific country case-studies (Indonesia,
Vietnam, Japan, China). United Nations dropping from 11.6% to literal
zero is the strongest single signal in any of the three subjects.

If your prep notes are inherited from older years, the macro-themes are
still useful for context but are *not* what you'll be tested on. Build
your essay banks around country case-studies, not Cold-War-as-a-theme.

### 2. Southeast Asia is the gravity centre
Top topics overall:

| Rank | Topic | Count |
|---|---|---|
| 1 | southeast-asia | 478 |
| 2 | cold-war | 366 |
| 3 | asean | 274 |
| 4 | source-analysis | 231 |
| 5 | forming-nation-states | 177 |
| 6 | united-nations | 168 |
| 7 | economic-development | 159 |

`southeast-asia` is tagged on **36% of all questions** and co-occurs with
nearly every other major topic. It's the primary lens; everything else
(authoritarianism, economic development, regional conflicts, ASEAN,
forming-nation-states) is a sub-frame.

### 3. Top co-occurring pairs map the essay vocabulary
```
170  forming-nation-states + southeast-asia
 96  peacekeeping + united-nations
 90  asean + regional-conflicts
 89  asean + regional-cooperation
 85  asean + source-analysis           (SBCS Qs about ASEAN)
 84  asean + southeast-asia
 65  provenance + source-analysis      (SBCS evaluation)
 59  cold-war + superpowers
```

The pairs split cleanly into two essay families:
1. **SE Asia themes** — nation-building, ASEAN, regional cooperation
2. **Cold War / international** — superpowers, peacekeeping, UN

Recent trend (point 1 above) means family (1) is growing and family (2)
is shrinking.

### 4. Sample essay prompts — the question shapes
- "To what extent do you agree that Communism was a threat to the
  newly-independent governments of Southeast Asia?"
- "How effective were the industrialisation policies to the achievement
  of economic development in Southeast Asian states?"
- "Critically examine whether regional organisations were successful in
  Southeast Asia, in the period between 1945 and 1997."
- "'There was no unity in diversity.' How far is this true of inter-state
  tensions in Southeast Asia from 1960 to 1997?"
- "How significant was the explosion of the Atomic Bomb in the genesis
  of the Cold War?"

Three recurring shapes:
1. **"To what extent do you agree…"** — agree/disagree on a causal claim
2. **"How effective/significant…"** — weighing factors
3. **"'<quote>.' Discuss / How far is this true…"** — defending or
   challenging an interpretation

These three templates cover the vast majority of essays. Practising them
explicitly is more useful than topic-by-topic drilling.

### 5. Mark distribution confirms the structure
```
30:  763  (essay)
25:  293  (SBCS Q-a, source-based case study part)
40:  144  (combined SBCS or extended)
10:   83
None: 564
```

Essay (30) and SBCS Q-a (25) account for **80% of marked items**. Build
your prep around exactly two artefact types: an essay plan and an SBCS
mini-evaluation.

### 6. School signatures are mid-strength
| School | Signature topic | Lift |
|---|---|---|
| St Andrew's JC | china-economic-transformation | ×5.8 |
| RI | us-foreign-policy | ×4.9 |
| NYJC | china | ×7.5 |
| HCI | stalin | ×3.8 |
| CJC | southeast-asia-economic-development | ×3.5 |
| ACJC | southeast-asia | ×1.6 |
| RVHS | regional-conflicts | ×1.9 |

Lifts are smaller than GP (×3–7 vs ×15–25). H2 History is more
syllabus-bound, but you can still see SAJC and NYJC leaning into the
China case-study, RI into US foreign policy, etc.

## Caveats

- The early-vs-late comparison has a thin late sample (336 docs from
  2023+). Treat the rising/falling deltas as directional. The
  *direction* (case-study over macro-theme) is corroborated by the
  syllabus change announcement, but exact magnitudes will shift as more
  recent papers are ingested.
- 1,429 distinct topic tags is high — consider trimming the tag
  vocabulary if you want cleaner trend signal. Many one-off tags fragment
  what should be the same theme.
- 564 docs have no mark assigned — these are notes + un-marked items.
  Filter them out before any per-mark analysis.

## How to extend

Re-run with: `python3 analytics/gp/build_dashboard.py h2history history`
