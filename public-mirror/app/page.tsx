import type { Metadata } from "next";
import { PublicMirror } from "./public-mirror";

export const metadata: Metadata = {
  title: "Perwiga Public Atlas",
  description: "A multilingual atlas for the worlds you play and read.",
  icons: { icon: "/favicon.svg", shortcut: "/favicon.svg" },
};

export default function Home() {
  return <PublicMirror />;
}
