import assert from "node:assert/strict";
import { access, readFile } from "node:fs/promises";
import test from "node:test";

async function render() {
  const workerUrl = new URL("../dist/server/index.js", import.meta.url);
  workerUrl.searchParams.set("test", `${process.pid}-${Date.now()}`);
  const { default: worker } = await import(workerUrl.href);

  return worker.fetch(
    new Request("http://localhost/", { headers: { accept: "text/html" } }),
    {
      ASSETS: {
        fetch: async () => new Response("Not found", { status: 404 }),
      },
    },
    {
      waitUntil() {},
      passThroughOnException() {},
    },
  );
}

test("server-renders the Perwiga atlas", async () => {
  const response = await render();
  assert.equal(response.status, 200);
  assert.match(response.headers.get("content-type") ?? "", /^text\/html\b/i);
  assert.equal(response.headers.get("x-content-type-options"), "nosniff");
  assert.equal(response.headers.get("x-frame-options"), "DENY");
  assert.match(response.headers.get("content-security-policy") ?? "", /frame-ancestors 'none'/);

  const html = await response.text();
  assert.match(html, /<title>Perwiga Public Atlas<\/title>/i);
  assert.match(html, /Loading atlas/);
  assert.doesNotMatch(html, /Genshin Impact|Arknights: Endfield/);
  assert.equal((html.match(/class="entity-row/g) ?? []).length, 0);
  assert.doesNotMatch(html, /Continue browsing/);
  assert.doesNotMatch(html, /codex-preview|SkeletonPreview|react-loading-skeleton/i);
  assert.doesNotMatch(html, /\/api\//i);
});

test("does not retain the starter preview or editing affordances", async () => {
  const [page, client, timeline] = await Promise.all([
    readFile(new URL("../app/page.tsx", import.meta.url), "utf8"),
    readFile(new URL("../app/public-mirror.tsx", import.meta.url), "utf8"),
    readFile(new URL("../app/components/timeline.tsx", import.meta.url), "utf8"),
  ]);
  await assert.rejects(access(new URL("../app/_sites-preview", import.meta.url)));

  assert.match(page, /Perwiga Public Atlas/);
  assert.match(client, /Worlds, names, and stories/);
  assert.match(client, /PAGE_SIZE = 10/);
  assert.match(client, /IntersectionObserver/);
  assert.match(client, /fetch\("\/data\/catalog\.json"/);
  assert.match(timeline, /timeline-axis/);
  assert.match(timeline, /timeline-bar/);
  assert.match(timeline, /timeline-lane-track/);
  assert.match(timeline, /timeline-event-tooltip/);
  assert.match(timeline, /aria-describedby/);
  assert.doesNotMatch(timeline, /className="eyebrow">Swimlane/);
  assert.doesNotMatch(timeline, /className="timeline-events"/);
  assert.doesNotMatch(timeline, /timeline-event-list|timeline-event-card/);
  assert.doesNotMatch(client, /slice\(0, 350\)/);
  assert.doesNotMatch(client, /new-entity|save-entity|create_work|PATCH|POST/);
});
