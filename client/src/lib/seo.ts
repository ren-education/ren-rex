/**
 * Centralised SEO constants. Anything that names the product, describes it,
 * or asserts where it lives in the world goes here so layout.tsx, sitemap.ts,
 * robots.ts, manifest.ts, and the OG image stay consistent.
 *
 * Override the canonical URL at build time with NEXT_PUBLIC_SITE_URL so
 * preview deployments don't claim the production canonical and confuse
 * Google's deduplication.
 */
export const SITE_URL =
  process.env.NEXT_PUBLIC_SITE_URL ?? "https://rex.reneducation.com";

export const BRAND = {
  name: "rex",
  fullName: "rex by ren",
  tagline: "PDF search & navigator for A-Level study",
  description:
    "Search 25,000+ A-Level questions, notes, and marking schemes from Singapore JC prelims. Find the exact page in the source PDF instantly. H2 Physics, H2 History, General Paper, and more.",
  shortDescription:
    "Search Singapore JC prelim questions, notes, and marking schemes with PDF page anchors.",
  locale: "en_SG",
  twitterHandle: undefined as string | undefined, // set when account exists
} as const;

/**
 * Long-tail keyword strategy. Google ignores the meta keywords tag, but
 * Bing, DuckDuckGo, and many internal search engines still use it — and it
 * also serves as a single-source-of-truth list for content review: the
 * landing page copy should naturally contain each of these phrases (or
 * close paraphrases) for on-page SEO to actually do anything.
 *
 * Bucketed by intent so future page-level keyword targeting can pull from
 * specific buckets. Add subject-specific buckets here as you add deep pages.
 */
export const KEYWORDS = {
  brand: ["rex", "rex search", "ren education", "tippytop"],
  product: [
    "a level practice",
    "a level singapore",
    "jc study tool",
    "jc revision",
    "prelim papers",
    "past year papers",
    "marking scheme",
    "exam practice",
    "study questions",
  ],
  subjects: [
    "h2 physics",
    "h2 history",
    "h2 economics",
    "h2 chemistry",
    "general paper",
    "gp essays",
    "h2 maths",
    "english language",
  ],
  schools: [
    "hwa chong institution",
    "raffles institution",
    "ri prelims",
    "hci prelims",
    "nyjc prelims",
    "vjc prelims",
    "ajc prelims",
    "cjc prelims",
    "tjc prelims",
    "dhs prelims",
    "sajc prelims",
    "acjc prelims",
  ],
  longTail: [
    "h2 physics prelim papers",
    "h2 history essay questions",
    "gp essay topics",
    "a level marking schemes",
    "jc prelim past papers",
    "h2 physics mcq practice",
    "h2 history source based questions",
  ],
} as const;

export const ALL_KEYWORDS: readonly string[] = [
  ...KEYWORDS.brand,
  ...KEYWORDS.product,
  ...KEYWORDS.subjects,
  ...KEYWORDS.schools,
  ...KEYWORDS.longTail,
];

/**
 * JSON-LD structured data. Three blocks are emitted on the landing page:
 *
 *  1. WebSite — claims the site is the canonical for the URL and declares a
 *     SearchAction. Google may use the latter to render a sitelinks
 *     searchbox in SERP results (one query directly from Google).
 *
 *  2. EducationalOrganization — positions rex as study content for the
 *     Singapore A-Level audience. Helps disambiguate "rex" from unrelated
 *     "rex" entities (animals, software unrelated to education, etc.).
 *
 *  3. SoftwareApplication — describes the tool itself so it can surface in
 *     "tools/apps" SERP categories.
 */
export function structuredData() {
  return [
    {
      "@context": "https://schema.org",
      "@type": "WebSite",
      name: BRAND.fullName,
      alternateName: BRAND.name,
      url: SITE_URL,
      description: BRAND.description,
      inLanguage: "en-SG",
      potentialAction: {
        "@type": "SearchAction",
        target: {
          "@type": "EntryPoint",
          urlTemplate: `${SITE_URL}/?q={search_term_string}`,
        },
        "query-input": "required name=search_term_string",
      },
    },
    {
      "@context": "https://schema.org",
      "@type": "EducationalOrganization",
      name: BRAND.fullName,
      url: SITE_URL,
      description: BRAND.description,
      areaServed: {
        "@type": "Country",
        name: "Singapore",
      },
      audience: {
        "@type": "EducationalAudience",
        educationalRole: "student",
      },
    },
    {
      "@context": "https://schema.org",
      "@type": "SoftwareApplication",
      name: BRAND.fullName,
      applicationCategory: "EducationalApplication",
      operatingSystem: "Any (browser-based)",
      description: BRAND.description,
      offers: {
        "@type": "Offer",
        price: "0",
        priceCurrency: "SGD",
      },
    },
  ];
}
