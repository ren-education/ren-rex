import type { Metadata, Viewport } from "next";
import "./globals.css";
import { Geist, Source_Serif_4, JetBrains_Mono } from "next/font/google";
import { PostHogProvider } from "@/components/posthog-provider";
import { cn } from "@/lib/utils";
import { ALL_KEYWORDS, BRAND, SITE_URL, structuredData } from "@/lib/seo";

const geist = Geist({ subsets: ["latin"], variable: "--font-sans" });
const serif = Source_Serif_4({
  subsets: ["latin"],
  variable: "--font-serif",
  display: "swap",
});
const mono = JetBrains_Mono({
  subsets: ["latin"],
  variable: "--font-mono",
  display: "swap",
});

export const metadata: Metadata = {
  metadataBase: new URL(SITE_URL),
  title: {
    default: `${BRAND.fullName} — ${BRAND.tagline}`,
    // Children can `export const metadata = { title: "..." }` and it slots
    // into the `%s` in front of the brand for instant breadcrumb context.
    template: `%s — ${BRAND.fullName}`,
  },
  description: BRAND.description,
  applicationName: BRAND.fullName,
  keywords: [...ALL_KEYWORDS],
  authors: [{ name: "ren education" }],
  creator: "ren education",
  publisher: "ren education",
  category: "education",
  alternates: {
    canonical: "/",
  },
  openGraph: {
    type: "website",
    url: SITE_URL,
    siteName: BRAND.fullName,
    title: `${BRAND.fullName} — ${BRAND.tagline}`,
    description: BRAND.description,
    locale: BRAND.locale,
    // opengraph-image.tsx is auto-picked up by Next; no need to list here.
  },
  twitter: {
    card: "summary_large_image",
    title: `${BRAND.fullName} — ${BRAND.tagline}`,
    description: BRAND.shortDescription,
    ...(BRAND.twitterHandle ? { creator: BRAND.twitterHandle } : {}),
  },
  robots: {
    index: true,
    follow: true,
    googleBot: {
      index: true,
      follow: true,
      "max-image-preview": "large",
      "max-snippet": -1,
      "max-video-preview": -1,
    },
  },
  formatDetection: {
    email: false,
    address: false,
    telephone: false,
  },
};

export const viewport: Viewport = {
  // Theme color matches the sage-linen active theme. Update if ACTIVE_THEME
  // below changes — they're intentionally kept in sync but not linked, since
  // theme tokens live in CSS and meta needs a literal hex.
  themeColor: [
    { media: "(prefers-color-scheme: light)", color: "#f4f1ea" },
    { media: "(prefers-color-scheme: dark)", color: "#1a1d18" },
  ],
  width: "device-width",
  initialScale: 1,
};

/**
 * Active theme is set by the `data-theme` attribute on <html>. Available
 * values: "sage-linen" (A4, default), "warm-paper" (A1). See `app/themes/`
 * for the full list and DESIGN.md for the architecture.
 */
const ACTIVE_THEME = "sage-linen";

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <html
      lang="en"
      data-theme={ACTIVE_THEME}
      className={cn("font-sans", geist.variable, serif.variable, mono.variable)}
    >
      <body className="min-h-screen antialiased">
        <PostHogProvider />
        {children}
        {/* JSON-LD structured data. Emitted as a single script tag with an
         * array of three @type entries (WebSite / EducationalOrganization /
         * SoftwareApplication) — google.com/structured-data accepts arrays
         * directly, and it's lighter than three separate <script>s. */}
        <script
          type="application/ld+json"
          // eslint-disable-next-line react/no-danger -- JSON-LD payload is
          // built from a typed constant and contains no user input.
          dangerouslySetInnerHTML={{ __html: JSON.stringify(structuredData()) }}
        />
      </body>
    </html>
  );
}
