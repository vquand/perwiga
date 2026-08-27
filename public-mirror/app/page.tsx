import type { Metadata } from "next";
import catalogData from "../public/data/catalog.json";
import { PublicMirror } from "./public-mirror";
import type { Catalog } from "./types";

export const metadata: Metadata = {
  title: "Perwiga Public Atlas",
  description: "A multilingual atlas for the worlds you play and read.",
  icons: { icon: "/favicon.svg", shortcut: "/favicon.svg" },
};

export default function Home() {
  return <PublicMirror initialCatalog={catalogData as unknown as Catalog} />;
}
