async function request(path, options = {}) {
  const response = await fetch(path, {
    ...options,
    headers: options.body
      ? { "content-type": "application/json", ...options.headers }
      : options.headers,
  });

  if (!response.ok) {
    let message = `Request failed (${response.status})`;
    try {
      const body = await response.json();
      message = body.error || message;
    } catch {
      // Keep the status-based fallback when an intermediary returns non-JSON.
    }
    throw new Error(message);
  }

  return response.json();
}

function compact(values) {
  return Object.fromEntries(
    Object.entries(values).filter(([, value]) => value !== "" && value != null),
  );
}

export const api = {
  setupEndfield() {
    return request("/api/uat/endfield", { method: "POST" });
  },

  setupGenshin() {
    return request("/api/uat/genshin", { method: "POST" });
  },

  listWorks() {
    return request("/api/works?kind=game");
  },

  listModules() {
    return request("/api/modules?kind=game");
  },

  getWorkspace(workId) {
    return request(`/api/works/${encodeURIComponent(workId)}/workspace`);
  },

  listCalendarEvents(workId) {
    return request(`/api/works/${encodeURIComponent(workId)}/calendar-events`);
  },

  getLoreGraph(workId, { subjectId = "", cursor = "", limit = 100 } = {}) {
    const search = new URLSearchParams();
    if (subjectId) search.set("subject_id", subjectId);
    if (cursor) search.set("cursor", cursor);
    if (limit) search.set("limit", String(limit));
    const suffix = search.size ? `?${search}` : "";
    return request(`/api/works/${encodeURIComponent(workId)}/lore-graph${suffix}`);
  },

  listLoreSubjects(workId) {
    return request(`/api/works/${encodeURIComponent(workId)}/lore-subjects`);
  },

  listLoreCandidates(workId, status = "pending") {
    const suffix = status ? `?status=${encodeURIComponent(status)}` : "";
    return request(`/api/works/${encodeURIComponent(workId)}/lore-candidates${suffix}`);
  },

  getLoreEvent(eventId) {
    return request(`/api/lore-events/${encodeURIComponent(eventId)}`);
  },

  reviewLoreCandidate(candidateId, values) {
    return request(`/api/lore-candidates/${encodeURIComponent(candidateId)}/decisions`, {
      method: "POST",
      body: JSON.stringify(compact(values)),
    });
  },

  createWork(values) {
    return request("/api/works", {
      method: "POST",
      body: JSON.stringify(compact(values)),
    });
  },

  listEntities(workId, { query = "", entityType = "" } = {}) {
    const search = new URLSearchParams();
    if (query.trim()) search.set("query", query.trim());
    if (entityType) search.set("entity_type", entityType);
    const suffix = search.size ? `?${search}` : "";
    return request(`/api/works/${encodeURIComponent(workId)}/entities${suffix}`);
  },

  getEntity(entityId) {
    return request(`/api/entities/${encodeURIComponent(entityId)}`);
  },

  createEntity(workId, values) {
    return request(`/api/works/${encodeURIComponent(workId)}/entities`, {
      method: "POST",
      body: JSON.stringify(compact(values)),
    });
  },

  updateEntity(entityId, values) {
    return request(`/api/entities/${encodeURIComponent(entityId)}`, {
      method: "PATCH",
      body: JSON.stringify(compact(values)),
    });
  },

  addAlias(entityId, values) {
    return request(`/api/entities/${encodeURIComponent(entityId)}/aliases`, {
      method: "POST",
      body: JSON.stringify(compact(values)),
    });
  },
};
