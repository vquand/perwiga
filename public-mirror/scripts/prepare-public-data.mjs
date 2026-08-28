import { execFileSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const projectDir = resolve(scriptDir, "..");
const repositoryDir = resolve(projectDir, "..");
const databasePath = join(repositoryDir, "perwiga.sqlite");
const catalogPath = join(projectDir, "public", "data", "catalog.json");
const assetDir = join(projectDir, "public", "assets");

if (!existsSync(databasePath)) {
  throw new Error(`canonical SQLite database not found: ${databasePath}`);
}

mkdirSync(dirname(catalogPath), { recursive: true });
execFileSync(
  "cargo",
  [
    "run",
    "-p",
    "perwiga",
    "--",
    "--database",
    databasePath,
    "export-public",
    "--output",
    catalogPath,
  ],
  { cwd: repositoryDir, stdio: "inherit" },
);

const catalog = JSON.parse(readFileSync(catalogPath, "utf8"));
const assetUrls = new Set();
for (const entity of catalog.entities) {
  if (entity.presentation?.thumbnail_url) assetUrls.add(entity.presentation.thumbnail_url);
  if (entity.presentation?.context_icon_url) assetUrls.add(entity.presentation.context_icon_url);
}
for (const event of catalog.events) {
  for (const featured of event.presentation?.featured_entities || []) {
    if (featured.thumbnail_url) assetUrls.add(featured.thumbnail_url);
  }
}

rmSync(join(assetDir, "modules"), { recursive: true, force: true });
rmSync(join(assetDir, "placeholders"), { recursive: true, force: true });

for (const assetUrl of assetUrls) {
  const source = sourcePathFor(assetUrl);
  if (!source) continue;
  if (!existsSync(source)) throw new Error(`public asset source not found: ${source}`);
  const destination = join(projectDir, "public", assetUrl.replace(/^\//, ""));
  mkdirSync(dirname(destination), { recursive: true });
  cpSync(source, destination);
}

console.log(
  `Prepared public catalog: ${catalog.works.length} works, ${catalog.entities.length} entities, ${catalog.events.length} events, ${assetUrls.size} assets.`,
);

function sourcePathFor(assetUrl) {
  const moduleMatch = assetUrl.match(/^\/assets\/modules\/([^/]+)\/(.+)$/);
  if (moduleMatch) {
    const [, moduleId, assetPath] = moduleMatch;
    const moduleAssets = join(repositoryDir, "games", moduleId, "assets");
    return join(moduleAssets, assetPath);
  }

  const placeholderMatch = assetUrl.match(/^\/assets\/placeholders\/(.+)$/);
  if (placeholderMatch) {
    return join(repositoryDir, "crates", "perwiga-web", "static", "placeholders", placeholderMatch[1]);
  }

  return null;
}
