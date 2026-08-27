function element(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

function typeName(types, key) {
  return types.find((type) => type.key === key)?.display_name || key;
}

const ENTITY_PLACEHOLDER_BY_TYPE = Object.freeze({
  npc: "character",
  character: "character",
  operator: "character",
  place: "place",
  region: "place",
  location: "place",
  concept: "concept",
  lore: "concept",
  weapon: "weapon",
  enemy: "enemy",
  mission: "mission",
  quest: "mission",
  event: "event",
  item: "item",
  faction: "faction",
  organization: "faction",
});

function optionalText(value) {
  return value || "Not recorded";
}

function wholeDaysSince(value, now = new Date()) {
  const endedAt = new Date(value);
  if (Number.isNaN(endedAt.getTime())) return null;
  const elapsed = now.getTime() - endedAt.getTime();
  if (elapsed < 0) return null;
  return Math.floor(elapsed / 86_400_000);
}

function eventRecencyPanel(recency) {
  const days = wholeDaysSince(recency?.ended_at);
  if (!Number.isInteger(days)) return null;
  const section = element("section", "entity-event-recency");
  section.setAttribute(
    "aria-label",
    `${days} ${days === 1 ? "day" : "days"} since ${recency.event_title} ended`,
  );
  const metric = element("div", "event-recency-metric");
  metric.append(
    element("strong", "", String(days)),
    element("span", "", days === 1 ? "day" : "days"),
  );
  const ended = element("time", "event-recency-date");
  ended.dateTime = recency.ended_at;
  ended.textContent = `Ended ${new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
  }).format(new Date(recency.ended_at))}`;
  section.append(
    element("p", "event-recency-heading", recency.heading),
    metric,
    element("p", "event-recency-context", `since ${recency.event_title} ended`),
    ended,
  );
  return section;
}

function entityPlaceholder(entityType) {
  const asset = ENTITY_PLACEHOLDER_BY_TYPE[entityType] || "generic";
  const placeholder = element("span", "entity-avatar-placeholder");
  placeholder.style.setProperty(
    "--entity-placeholder-image",
    `url("/assets/placeholders/${asset}.svg")`,
  );
  return placeholder;
}

function showEntityPlaceholder(avatar, entityType) {
  avatar.classList.remove("has-thumbnail");
  avatar.style.removeProperty("--entity-accent");
  avatar.replaceChildren(entityPlaceholder(entityType));
}

function entityAvatar(entity) {
  const presentation = entity.presentation;
  const avatar = element("span", "entity-avatar");
  avatar.setAttribute("aria-hidden", "true");
  if (!presentation?.thumbnail_url) {
    showEntityPlaceholder(avatar, entity.entity_type);
    return avatar;
  }

  avatar.classList.add("has-thumbnail");
  avatar.style.setProperty("--entity-accent", presentation.accent_color);
  const image = element("img", "entity-avatar-image");
  image.src = presentation.thumbnail_url;
  image.alt = "";
  image.width = 160;
  image.height = 160;
  image.loading = "lazy";
  image.decoding = "async";
  const rarity = element("span", "entity-rarity", presentation.label);
  image.addEventListener(
    "error",
    () => {
      showEntityPlaceholder(avatar, entity.entity_type);
    },
    { once: true },
  );
  avatar.append(image, rarity);
  return avatar;
}

function entityKind(entityType, presentation) {
  const kind = element("span", "entity-kind");
  const contextLabel = typeof presentation?.context_label === "string"
    ? presentation.context_label.trim()
    : "";
  const contextIconUrl = typeof presentation?.context_icon_url === "string"
    ? presentation.context_icon_url.trim()
    : "";

  if (contextLabel && contextIconUrl) {
    const marker = element("span", "entity-context-marker");
    const accessibleLabel = element("span", "visually-hidden", contextLabel);
    const contextIcon = element("img", "entity-context-icon");
    contextIcon.src = contextIconUrl;
    contextIcon.alt = "";
    contextIcon.width = 40;
    contextIcon.height = 40;
    contextIcon.loading = "lazy";
    contextIcon.decoding = "async";
    const fallback = element("span", "entity-context-fallback", contextLabel);
    fallback.hidden = true;
    fallback.setAttribute("aria-hidden", "true");
    contextIcon.addEventListener(
      "error",
      () => {
        contextIcon.remove();
        fallback.hidden = false;
        marker.classList.add("is-fallback");
      },
      { once: true },
    );
    marker.append(accessibleLabel, contextIcon, fallback);
    kind.append(marker);
  } else if (contextLabel) {
    kind.append(
      element("span", "entity-context-fallback", contextLabel),
      element("span", "entity-context-separator", "·"),
    );
  }

  kind.append(element("span", "entity-kind-label", entityType));
  return kind;
}

const nameCollator = new Intl.Collator(undefined, { sensitivity: "base" });

export function filterAndSortEntities(entities, { rarity = "", facets = {}, sort = "name" } = {}) {
  const selectedRarity = rarity === "" ? null : Number(rarity);
  const selectedFacets = Object.entries(facets).filter(([, value]) => value);
  const filtered = entities.filter((entity) => {
    if (selectedRarity !== null && entity.presentation?.rarity !== selectedRarity) return false;
    return selectedFacets.every(([key, value]) => {
      if (entity.presentation?.facets?.[key] === value) return true;
      return entity.presentation?.facet_values?.[key]?.includes(value) === true;
    });
  });

  const byName = (left, right) =>
    nameCollator.compare(
      left.catalog_label || left.official_english_name,
      right.catalog_label || right.official_english_name,
    );
  if (sort === "name") return filtered.sort(byName);

  const direction = sort === "rarity-asc" ? 1 : -1;
  return filtered.sort((left, right) => {
    const leftRarity = left.presentation?.rarity;
    const rightRarity = right.presentation?.rarity;
    const leftHasRarity = Number.isInteger(leftRarity);
    const rightHasRarity = Number.isInteger(rightRarity);
    if (leftHasRarity !== rightHasRarity) return leftHasRarity ? -1 : 1;
    if (leftHasRarity && leftRarity !== rightRarity) {
      return (leftRarity - rightRarity) * direction;
    }
    return byName(left, right);
  });
}

export function renderTypeNavigation(nav, types, entities, selected, onSelect) {
  const counts = entities.reduce((result, entity) => {
    result[entity.entity_type] = (result[entity.entity_type] || 0) + 1;
    return result;
  }, {});
  const items = [{ key: "", display_name: "All records", description: "Every entity" }, ...types];

  nav.replaceChildren();
  nav.removeAttribute("aria-busy");
  for (const item of items) {
    const button = element("button", "type-button");
    button.type = "button";
    button.dataset.type = item.key;
    button.setAttribute("aria-pressed", String(item.key === selected));
    if (item.key === selected) button.classList.add("is-active");

    const label = element("span", "type-label", item.display_name);
    const count = element(
      "span",
      "type-count",
      String(item.key ? counts[item.key] || 0 : entities.length),
    );
    button.append(label, count);
    button.title = item.description;
    button.addEventListener("click", () => onSelect(item.key));
    nav.append(button);
  }
}

export function renderEntityList(list, entities, types, selectedId, onSelect) {
  list.replaceChildren();
  if (!entities.length) {
    const item = element("li", "list-empty");
    item.append(
      element("span", "empty-symbol", "⌁"),
      element("h3", "", "No matching records"),
      element("p", "", "Create the first entry or adjust the current search and type filter."),
    );
    list.append(item);
    return;
  }

  for (const entity of entities) {
    const item = element("li", "entity-list-item");
    const button = element("button", "entity-row");
    button.type = "button";
    if (entity.id === selectedId) {
      button.classList.add("is-selected");
      button.setAttribute("aria-current", "true");
    }

    const avatar = entityAvatar(entity);
    const copy = element("span", "entity-copy");
    copy.append(
      element("strong", "entity-name", entity.catalog_label || entity.official_english_name),
      element("span", "entity-original", entity.official_original_name),
    );
    const kind = entityKind(typeName(types, entity.entity_type), entity.presentation);
    button.append(avatar, copy, kind);
    button.addEventListener("click", () => onSelect(entity.id));
    item.append(button);
    list.append(item);
  }
}

function detailRow(term, value, marker) {
  const wrapper = element("div", "detail-row");
  const title = element("dt", "");
  title.append(document.createTextNode(term));
  if (marker) title.append(element("span", `provenance ${marker.className}`, marker.label));
  wrapper.append(title, element("dd", value ? "" : "is-empty", optionalText(value)));
  return wrapper;
}

function inputField(labelText, control) {
  const label = element("label", "alias-field");
  label.append(element("span", "", labelText), control);
  return label;
}

function aliasForm(onAlias, onClose) {
  const form = element("form", "alias-form");
  const heading = element("h3", "subheading", "Add a known name");
  heading.id = "alias-editor-heading";
  form.setAttribute("aria-labelledby", heading.id);
  const grid = element("div", "alias-grid");
  const value = element("input");
  value.name = "value";
  value.required = true;
  value.maxLength = 240;
  const language = element("input");
  language.name = "language";
  language.placeholder = "e.g. vi, zh, ja";
  language.maxLength = 32;
  const kind = element("select");
  kind.name = "kind";
  for (const [key, label] of [
    ["alternative", "Alternative name"],
    ["romanization", "Romanization"],
    ["han-viet", "Hán Việt"],
    ["former", "Former name"],
    ["user-note", "User note"],
  ]) {
    const option = element("option", "", label);
    option.value = key;
    kind.append(option);
  }
  const label = element("input");
  label.name = "label";
  label.maxLength = 120;
  label.placeholder = "Optional context";
  const notes = element("textarea");
  notes.name = "notes";
  notes.rows = 2;
  const submit = element("button", "button button-secondary", "Add alias");
  submit.type = "submit";
  const cancel = element("button", "button button-secondary", "Cancel");
  cancel.type = "button";
  const actions = element("div", "alias-form-actions");
  actions.append(submit, cancel);

  grid.append(
    inputField("Alias or other name", value),
    inputField("Language code", language),
    inputField("Name kind", kind),
    inputField("Label", label),
  );
  form.append(heading, grid, inputField("Notes", notes), actions);
  cancel.addEventListener("click", () => {
    form.reset();
    onClose();
  });
  form.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      event.preventDefault();
      form.reset();
      onClose();
    }
  });
  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    submit.disabled = true;
    submit.textContent = "Adding…";
    try {
      await onAlias(Object.fromEntries(new FormData(form)));
      form.reset();
      onClose();
    } catch {
      // The application controller reports the API error in its live toast.
    } finally {
      submit.disabled = false;
      submit.textContent = "Add alias";
    }
  });
  return form;
}

function aliasEditor(onAlias) {
  const editor = element("section", "alias-editor");
  const toggle = element("button", "button button-secondary", "Add");
  const panel = element("div", "alias-editor-panel");
  panel.id = "alias-editor-panel";
  panel.hidden = true;
  toggle.type = "button";
  toggle.setAttribute("aria-controls", panel.id);
  toggle.setAttribute("aria-expanded", "false");

  const close = () => {
    panel.hidden = true;
    toggle.hidden = false;
    toggle.setAttribute("aria-expanded", "false");
    toggle.focus();
  };
  const form = aliasForm(onAlias, close);
  const value = form.elements.namedItem("value");
  toggle.addEventListener("click", () => {
    toggle.setAttribute("aria-expanded", "true");
    toggle.hidden = true;
    panel.hidden = false;
    requestAnimationFrame(() => value.focus());
  });

  panel.append(form);
  editor.append(toggle, panel);
  return editor;
}

function entityAppearanceSection(appearances) {
  const section = element("section", "entity-appearances");
  section.append(element("h3", "subheading", `Appearances · ${appearances.length}`));
  if (!appearances.length) {
    section.append(element("p", "empty-copy", "No source-backed quest, event, or action records have been recorded."));
    return section;
  }

  const list = element("ul", "appearance-list");
  for (const appearance of appearances) {
    const item = element("li", "appearance-list-item");
    const heading = element("div", "appearance-heading");
    const title = appearance.related_source_url
      ? element("a", "appearance-title", appearance.related_title)
      : element("strong", "appearance-title", appearance.related_title);
    if (appearance.related_source_url) {
      title.href = appearance.related_source_url;
      title.target = "_blank";
      title.rel = "noreferrer noopener";
    }
    heading.append(title, element("span", "appearance-kind", appearance.relation_kind));
    item.append(heading);
    if (appearance.locations?.length) {
      const locations = appearance.locations.map((location) =>
        location.region_name && location.region_name !== location.location_name
          ? `${location.location_name} · ${location.region_name}`
          : location.location_name,
      );
      item.append(element("small", "appearance-locations", locations.join(", ")));
    }
    if (appearance.source_notes) item.append(element("small", "appearance-notes", appearance.source_notes));
    list.append(item);
  }
  section.append(list);
  return section;
}

export function renderEntityDetail(container, detail, types, { onEdit, onAlias }) {
  const {
    entity,
    aliases,
    appearances = [],
    event_recency: eventRecency,
  } = detail;
  container.className = "inspector-content";
  container.replaceChildren();

  const heading = element("div", "detail-heading");
  const titleBlock = element("div", "");
  titleBlock.append(
    element("p", "entity-type-tag", typeName(types, entity.entity_type)),
    element("h3", "detail-title", entity.official_english_name),
    element("p", "detail-original", entity.official_original_name),
  );
  const edit = element("button", "button button-secondary", "Edit");
  edit.type = "button";
  edit.addEventListener("click", onEdit);
  heading.append(titleBlock, edit);

  const names = element("dl", "detail-list");
  names.append(
    detailRow("Official Vietnamese", entity.official_vietnamese_name, {
      label: "Official",
      className: "is-official",
    }),
    detailRow("Automatic Vietnamese", entity.automatic_vietnamese_translation, {
      label: "Automatic",
      className: "is-automatic",
    }),
    detailRow("English description", entity.english_description),
    detailRow("Other information", entity.other_information),
  );

  const aliasSection = element("section", "aliases");
  aliasSection.append(element("h3", "subheading", `Known names · ${aliases.length}`));
  if (aliases.length) {
    const aliasList = element("ul", "alias-list");
    for (const alias of aliases) {
      const item = element("li", "");
      const copy = element("span", "");
      copy.append(
        element("strong", "", alias.value),
        element("small", "", alias.label || alias.kind),
      );
      item.append(copy, element("code", "", alias.language || "—"));
      aliasList.append(item);
    }
    aliasSection.append(aliasList);
  } else {
    aliasSection.append(element("p", "empty-copy", "No aliases have been recorded."));
  }

  const recency = eventRecencyPanel(eventRecency);
  container.append(heading);
  if (recency) container.append(recency);
  container.append(names, entityAppearanceSection(appearances), aliasSection, aliasEditor(onAlias));
}

export function renderInspectorEmpty(container, title = "Select a record", message) {
  container.className = "empty-inspector";
  container.replaceChildren(
    element("div", "empty-symbol", "⌁"),
    element("h3", "", title),
    element(
      "p",
      "",
      message || "Choose an entry to inspect every known name, provenance field, description, and alias.",
    ),
  );
}

const TIMELINE_TYPE_ORDER = [
  "Version",
  "Headhunting",
  "Sign-in",
  "Celebration",
  "Narrative",
  "Guide",
  "Supply",
  "Production",
  "Camera",
  "Fun & Games",
  "Challenge",
  "Puzzle",
  "Permanent",
];

const TIMELINE_LANE_COLORS = [
  "#d7f75b",
  "#f08d9d",
  "#f0ca72",
  "#82d8c9",
  "#aeb7ff",
  "#81bdf3",
  "#d7a6f4",
  "#ef9f74",
];

const MS_PER_DAY = 86_400_000;

function timelineDate(value, allDay = false) {
  if (!value) return null;
  const parsed = allDay && /^\d{4}-\d{2}-\d{2}$/.test(value)
    ? new Date(`${value}T00:00:00Z`)
    : new Date(value);
  return Number.isNaN(parsed.getTime()) ? null : parsed;
}

function eventScheduleStatus(event, now) {
  const start = timelineDate(event.starts_at, event.is_all_day);
  const end = timelineDate(event.ends_at, event.is_all_day);
  if (!start) return "unknown";
  if (start > now) return "upcoming";
  if (end && end < now) return "past";
  return "ongoing";
}

function sourceDateLabel(value, allDay = false) {
  if (!value) return "End not announced";
  const match = value.match(/^(\d{4})-(\d{2})-(\d{2})(?:T(\d{2}):(\d{2}))?/);
  if (!match) return value;
  const [, year, month, day, hour, minute] = match;
  const calendarDate = new Intl.DateTimeFormat(undefined, {
    day: "numeric",
    month: "short",
    year: "numeric",
    timeZone: "UTC",
  }).format(new Date(Date.UTC(Number(year), Number(month) - 1, Number(day))));
  return allDay || !hour ? calendarDate : `${calendarDate} · ${hour}:${minute}`;
}

function monthFloor(date) {
  return new Date(Date.UTC(date.getUTCFullYear(), date.getUTCMonth(), 1));
}

function nextMonth(date) {
  return new Date(Date.UTC(date.getUTCFullYear(), date.getUTCMonth() + 1, 1));
}

function timelineRatio(value, start, duration) {
  return Math.max(0, Math.min(1, (value - start) / duration));
}

function assignTimelineTracks(events, domainEnd) {
  const trackEnds = [];
  return events.map((event) => {
    const start = timelineDate(event.starts_at, event.is_all_day);
    const end = timelineDate(event.ends_at, event.is_all_day) || domainEnd;
    let track = trackEnds.findIndex((trackEnd) => trackEnd <= start);
    if (track === -1) {
      track = trackEnds.length;
      trackEnds.push(end);
    } else {
      trackEnds[track] = end;
    }
    return { event, start, end, track };
  });
}

function timelineEmpty(container, message) {
  const empty = element("div", "timeline-empty");
  empty.append(
    element("span", "empty-symbol", "⌁"),
    element("h3", "", "No events in this view"),
    element("p", "", message),
  );
  container.replaceChildren(empty);
  container.removeAttribute("aria-busy");
}

export function renderEventTimeline(container, events, statusFilter = "all", now = new Date()) {
  const datedEvents = events
    .map((event) => ({
      ...event,
      schedule_status: eventScheduleStatus(event, now),
      parsed_start: timelineDate(event.starts_at, event.is_all_day),
      parsed_end: timelineDate(event.ends_at, event.is_all_day),
    }))
    .filter((event) => event.parsed_start);
  const statusCounts = datedEvents.reduce(
    (counts, event) => {
      if (counts[event.schedule_status] !== undefined) counts[event.schedule_status] += 1;
      return counts;
    },
    { past: 0, ongoing: 0, upcoming: 0 },
  );
  const visible = statusFilter === "all"
    ? datedEvents
    : datedEvents.filter((event) => event.schedule_status === statusFilter);

  if (!visible.length) {
    timelineEmpty(
      container,
      events.length
        ? "Choose another schedule status to see its events."
        : "This title module has not imported any dated events yet.",
    );
    return { shown: 0, total: datedEvents.length, statusCounts, lanes: 0, timeZone: "" };
  }

  const earliest = new Date(Math.min(...visible.map((event) => event.parsed_start.getTime())));
  const latestKnown = new Date(
    Math.max(
      now.getTime(),
      ...visible.map((event) => (event.parsed_end || event.parsed_start).getTime()),
    ),
  );
  const domainStart = monthFloor(earliest);
  let domainEnd = nextMonth(latestKnown);
  if (visible.some((event) => !event.parsed_end && event.parsed_start <= now)) {
    domainEnd = new Date(Math.max(domainEnd.getTime(), now.getTime() + 28 * MS_PER_DAY));
  }
  const duration = Math.max(MS_PER_DAY, domainEnd - domainStart);
  const timelineWidth = Math.max(72 * 16, Math.ceil(duration / MS_PER_DAY) * 5.5);
  const grouped = Map.groupBy
    ? Map.groupBy(visible, (event) => event.event_type || "Other")
    : visible.reduce((groups, event) => {
      const key = event.event_type || "Other";
      if (!groups.has(key)) groups.set(key, []);
      groups.get(key).push(event);
      return groups;
    }, new Map());
  const typeRank = (type) => {
    const rank = TIMELINE_TYPE_ORDER.indexOf(type);
    return rank === -1 ? TIMELINE_TYPE_ORDER.length : rank;
  };
  const types = Array.from(grouped.keys()).sort((left, right) =>
    typeRank(left) - typeRank(right) || left.localeCompare(right),
  );

  const grid = element("div", "timeline-grid");
  grid.style.setProperty("--timeline-width", `${timelineWidth}px`);
  const axis = element("div", "timeline-axis");
  axis.append(element("div", "timeline-axis-label", "Schedule"));
  const axisScale = element("div", "timeline-axis-scale");
  axisScale.style.width = `${timelineWidth}px`;
  for (let cursor = monthFloor(domainStart); cursor <= domainEnd; cursor = nextMonth(cursor)) {
    const tick = element(
      "span",
      "timeline-month",
      new Intl.DateTimeFormat(undefined, { month: "short", year: "numeric", timeZone: "UTC" })
        .format(cursor),
    );
    tick.style.left = `${timelineRatio(cursor, domainStart, duration) * timelineWidth}px`;
    axisScale.append(tick);
  }
  if (now >= domainStart && now <= domainEnd) {
    const today = element("span", "timeline-today", "Today");
    today.style.left = `${timelineRatio(now, domainStart, duration) * timelineWidth}px`;
    axisScale.append(today);
  }
  axis.append(axisScale);
  grid.append(axis);

  for (const [typeIndex, type] of types.entries()) {
    const lane = element("section", "timeline-lane");
    lane.style.setProperty(
      "--lane-color",
      TIMELINE_LANE_COLORS[typeIndex % TIMELINE_LANE_COLORS.length],
    );
    const laneEvents = grouped
      .get(type)
      .slice()
      .sort((left, right) => left.parsed_start - right.parsed_start || left.title.localeCompare(right.title));
    const assigned = assignTimelineTracks(laneEvents, domainEnd);
    const trackCount = Math.max(...assigned.map((item) => item.track)) + 1;
    const label = element("div", "timeline-lane-label");
    label.append(
      element("strong", "", type),
      element("span", "", `${laneEvents.length} ${laneEvents.length === 1 ? "event" : "events"}`),
    );
    const body = element("div", "timeline-lane-body");
    body.style.width = `${timelineWidth}px`;
    body.style.height = `${Math.max(3.5, trackCount * 2 + 1.4)}rem`;
    if (now >= domainStart && now <= domainEnd) {
      const nowLine = element("span", "timeline-now-line");
      nowLine.style.left = `${timelineRatio(now, domainStart, duration) * timelineWidth}px`;
      body.append(nowLine);
    }
    for (const item of assigned) {
      const event = item.event;
      const startX = timelineRatio(item.start, domainStart, duration) * timelineWidth;
      const endX = timelineRatio(item.end, domainStart, duration) * timelineWidth;
      const bar = event.source_url
        ? element("a", `timeline-event is-${event.schedule_status}`)
        : element("div", `timeline-event is-${event.schedule_status}`);
      if (event.source_url) {
        bar.href = event.source_url;
        bar.target = "_blank";
        bar.rel = "noreferrer";
      }
      if (!event.ends_at) bar.classList.add("is-open-ended");
      bar.style.left = `${startX}px`;
      bar.style.top = `${item.track * 2 + 0.7}rem`;
      bar.style.width = `${Math.max(18, endX - startX)}px`;
      const startLabel = sourceDateLabel(event.starts_at, event.is_all_day);
      const endLabel = sourceDateLabel(event.ends_at, event.is_all_day);
      const patch = event.patch_start
        ? `Patch ${event.patch_start}${event.patch_end && event.patch_end !== event.patch_start ? `–${event.patch_end}` : ""}`
        : "Patch not recorded";
      const featured = Array.isArray(event.presentation?.featured_entities)
        ? event.presentation.featured_entities.filter((entity) => entity.thumbnail_url)
        : [];
      const featuredDescription = featured.length
        ? ` ${event.presentation.heading}: ${featured.map((entity) => {
          const gap = entity.previous_event_gap;
          return gap
            ? `${entity.display_name} (${gap.days} days since ${gap.event_title} ended)`
            : entity.display_name;
        }).join(", ")}.`
        : "";
      const description = `${event.title}. ${startLabel} — ${endLabel}. ${patch}. ${event.time_zone || "Timezone not recorded"}.${featuredDescription}`;
      bar.setAttribute("aria-label", description);
      bar.append(element("span", "timeline-event-title", event.title));
      if (featured.length) {
        if (!event.source_url) bar.tabIndex = 0;
        const preview = element("span", "event-feature-preview");
        preview.setAttribute("aria-hidden", "true");
        preview.append(
          element("span", "event-feature-heading", event.presentation.heading || "Featured"),
        );
        const roster = element("span", "event-feature-roster");
        for (const featuredEntity of featured) {
          const card = element("span", "event-feature-card");
          card.style.setProperty("--feature-accent", featuredEntity.accent_color);
          const image = element("img", "event-feature-image");
          image.src = featuredEntity.thumbnail_url;
          image.alt = "";
          image.width = 48;
          image.height = 48;
          image.loading = "lazy";
          image.decoding = "async";
          const identity = element("span", "event-feature-identity");
          identity.append(
            element("strong", "", featuredEntity.display_name),
            element("span", "", featuredEntity.label),
          );
          card.append(image, identity);
          if (featuredEntity.previous_event_gap) {
            const gap = element("span", "event-feature-gap");
            gap.append(
              element(
                "strong",
                "",
                `${featuredEntity.previous_event_gap.days} ${featuredEntity.previous_event_gap.days === 1 ? "day" : "days"}`,
              ),
              element(
                "small",
                "",
                `since ${featuredEntity.previous_event_gap.event_title}`,
              ),
            );
            card.append(gap);
          }
          roster.append(card);
        }
        preview.append(roster);
        bar.append(preview);
      }
      body.append(bar);
    }
    lane.append(label, body);
    grid.append(lane);
  }

  container.replaceChildren(grid);
  container.removeAttribute("aria-busy");
  return {
    shown: visible.length,
    total: datedEvents.length,
    statusCounts,
    lanes: types.length,
    timeZone: visible.find((event) => event.time_zone)?.time_zone || "",
  };
}
