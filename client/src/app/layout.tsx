import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "rex — PDF Search & Navigator",
  description: "Search questions, notes, and PDF pages across subjects.",
};

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en">
      <body className="min-h-screen antialiased">{children}</body>
    </html>
  );
}
