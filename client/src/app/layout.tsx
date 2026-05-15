import type { Metadata } from "next";
import "./globals.css";
import { Geist, Source_Serif_4, JetBrains_Mono } from "next/font/google";
import { cn } from "@/lib/utils";

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
  title: "rex — PDF Search & Navigator",
  description: "Search questions, notes, and PDF pages across subjects.",
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
      <body className="min-h-screen antialiased">{children}</body>
    </html>
  );
}
