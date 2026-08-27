import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Perwiga Public Atlas",
  description: "A multilingual atlas for the worlds you play and read.",
  icons: { icon: "/favicon.svg", shortcut: "/favicon.svg" },
};

export default function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
