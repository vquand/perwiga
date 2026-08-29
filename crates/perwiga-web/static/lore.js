function element(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

function certaintyLabel(event) {
  return event.time_precision === "exact"
    ? "Exact"
    : event.time_precision === "relative"
      ? "Relative"
      : event.time_precision;
}

export function renderLoreMap(container, graph, { onEvent } = {}) {
  container.replaceChildren();
  if (!graph?.events?.length) {
    container.append(
      element("div", "lore-empty", "No reviewed lore events yet. Import an official-corpus candidate batch, then approve it in the review queue."),
    );
    return;
  }

  const periods = [...(graph.periods || [])].sort((left, right) =>
    left.display_order - right.display_order || left.id.localeCompare(right.id),
  );
  const periodById = new Map(periods.map((period) => [period.id, period]));
  const grouped = new Map();
  for (const event of graph.events) {
    const key = event.start_period_id || "__unplaced";
    if (!grouped.has(key)) grouped.set(key, []);
    grouped.get(key).push(event);
  }

  const map = element("div", "lore-map");
  map.setAttribute("role", "list");
  for (const [key, events] of [
    ...periods.map((period) => [period.id, grouped.get(period.id) || []]),
    ["__unplaced", grouped.get("__unplaced") || []],
  ]) {
    if (!events.length && key === "__unplaced") continue;
    const period = periodById.get(key);
    const column = element("section", "lore-period");
    column.setAttribute("role", "listitem");
    column.append(
      element("p", "lore-period-order", period ? `Period ${period.display_order}` : "Unplaced"),
      element("h3", "lore-period-title", period?.name_en || "Relative / unplaced time"),
    );
    if (period?.description_en) column.append(element("p", "lore-period-description", period.description_en));
    const stack = element("div", "lore-event-stack");
    for (const event of events) {
      const card = element("article", "lore-event-card");
      card.dataset.eventId = event.id;
      card.tabIndex = 0;
      card.setAttribute("role", "button");
      card.setAttribute("aria-label", `Open lore event ${event.title_en}`);
      const title = element("h4", "lore-event-title", event.title_en);
      const time = element("p", "lore-event-time", event.time_label);
      const summary = event.summary_en ? element("p", "lore-event-summary", event.summary_en) : null;
      const meta = element("div", "lore-event-meta");
      meta.append(
        element("span", "lore-event-badge", certaintyLabel(event)),
        element("span", "lore-event-badge", `Revision ${event.revision}`),
      );
      card.append(title, time, ...(summary ? [summary] : []), meta);
      const open = () => onEvent?.(event.id);
      card.addEventListener("click", open);
      card.addEventListener("keydown", (eventKey) => {
        if (eventKey.key === "Enter" || eventKey.key === " ") {
          eventKey.preventDefault();
          open();
        }
      });
      stack.append(card);
    }
    column.append(stack);
    map.append(column);
  }
  container.append(map);
}

export function renderLoreEventDetail(container, detail, { onBack } = {}) {
  container.replaceChildren();
  if (!detail) {
    container.append(element("div", "empty-inspector", "Select an event on the lore map to inspect its claims and evidence."));
    return;
  }
  const event = detail.event;
  const header = element("div", "lore-detail-header");
  header.append(element("p", "section-index", "Lore event"), element("h3", "lore-detail-title", event.title_en));
  if (onBack) {
    const back = element("button", "button button-secondary", "Back to map");
    back.type = "button";
    back.addEventListener("click", onBack);
    header.append(back);
  }
  container.append(header, element("p", "lore-detail-time", `${event.time_label} · ${certaintyLabel(event)}`));
  if (event.summary_en) container.append(element("p", "lore-detail-summary", event.summary_en));

  const subjects = new Map((detail.subjects || []).map((subject) => [subject.id, subject]));
  const involvementSection = element("section", "lore-detail-section");
  involvementSection.append(element("h4", "lore-detail-section-title", "Involvement"));
  const involvementList = element("ul", "lore-subject-list");
  for (const involvement of detail.involvements || []) {
    const subject = subjects.get(involvement.subject_id);
    involvementList.append(element("li", "lore-subject-item", `${subject?.attested_name || "Unknown subject"} · ${involvement.role}`));
  }
  if (!involvementList.children.length) involvementList.append(element("li", "lore-muted", "No subjects recorded."));
  involvementSection.append(involvementList);
  container.append(involvementSection);

  const evidenceByClaim = new Map();
  for (const item of detail.evidence || []) {
    if (!evidenceByClaim.has(item.claim_id)) evidenceByClaim.set(item.claim_id, []);
    evidenceByClaim.get(item.claim_id).push(item);
  }
  const claimSection = element("section", "lore-detail-section");
  claimSection.append(element("h4", "lore-detail-section-title", "Claims and evidence"));
  for (const claim of detail.claims || []) {
    const article = element("article", "lore-claim");
    article.append(
      element("p", "lore-claim-text", claim.text_en),
      element("p", "lore-claim-meta", `${claim.assertion_kind} · ${claim.certainty}`),
    );
    for (const item of evidenceByClaim.get(claim.id) || []) {
      const evidence = element("blockquote", "lore-evidence", item.evidence.excerpt);
      evidence.append(element("cite", "lore-evidence-locator", `${item.evidence.locator} · ${item.stance}`));
      article.append(evidence);
    }
    claimSection.append(article);
  }
  if (!detail.claims?.length) claimSection.append(element("p", "lore-muted", "No claims recorded."));
  container.append(claimSection);
}

export function renderLoreReview(container, candidates, { onDecision } = {}) {
  container.replaceChildren();
  if (!candidates?.length) {
    container.append(element("p", "lore-muted", "The review queue is clear."));
    return;
  }
  for (const candidate of candidates) {
    const item = element("article", "lore-review-item");
    item.append(
      element("div", "lore-review-heading", `${candidate.candidate_kind} · ${candidate.candidate_key}`),
      element("p", "lore-review-payload", candidate.payload_json),
    );
    const actions = element("div", "lore-review-actions");
    for (const decision of ["approve", "reject"]) {
      const button = element("button", decision === "approve" ? "button button-primary" : "button button-secondary", decision[0].toUpperCase() + decision.slice(1));
      button.type = "button";
      button.addEventListener("click", () => onDecision?.(candidate, decision));
      actions.append(button);
    }
    item.append(actions);
    container.append(item);
  }
}
