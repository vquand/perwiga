import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Perwiga Public Atlas",
  description: "A read-only public mirror of selected Perwiga game and novel wiki data.",
  icons: { icon: "/favicon.svg", shortcut: "/favicon.svg" },
};

export default function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
