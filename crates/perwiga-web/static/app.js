import { api } from "/assets/api.js";
import {
  filterAndSortEntities,
  renderEntityDetail,
  renderEntityList,
  renderEventTimeline,
  renderInspectorEmpty,
  renderTypeNavigation,
} from "/assets/ui.js";
import { renderLoreEventDetail, renderLoreMap, renderLoreReview } from "/assets/lore.js";

const state = {
  works: [],
  modules: [],
  work: null,
  types: [],
  facets: [],
  allEntities: [],
  entities: [],
  selectedType: "",
  selectedId: "",
  detail: null,
  selectedRarity: "",
  selectedFacets: {},
  sort: "name",
  events: [],
  eventStatus: "all",
  loreSchema: null,
  loreGraph: null,
  loreSubjects: [],
  loreCandidates: [],
  selectedLoreEventId: "",
  loreSubjectId: "",
  loreSubjectType: "",
  view: "wiki",
  switchSequence: 0,
};

const dom = {
  gameSwitcher: document.querySelector("#game-switcher"),
  gameSwitcherShell: document.querySelector(".game-switcher-shell"),
  gameSwitcherValue: document.querySelector("#game-switcher-value"),
  gameSwitcherMenu: document.querySelector("#game-switcher-menu"),
  openSearch: document.querySelector("#open-search"),
  addGame: document.querySelector("#add-game"),
  pageTitle: document.querySelector("#page-title"),
  moduleKicker: document.querySelector("#module-kicker"),
  moduleId: document.querySelector("#module-id"),
  viewButtons: Array.from(document.querySelectorAll("[data-view]")),
  wikiView: document.querySelector("#wiki-view"),
  timelineView: document.querySelector("#timeline-view"),
  loreView: document.querySelector("#lore-view"),
  loreViewButton: document.querySelector('[data-view="lore"]'),
  eventStatusButtons: Array.from(document.querySelectorAll("[data-event-status]")),
  timelineStatus: document.querySelector("#timeline-status"),
  timeline: document.querySelector("#event-timeline"),
  loreStatus: document.querySelector("#lore-status"),
  loreMap: document.querySelector("#lore-map"),
  loreDetail: document.querySelector("#lore-detail"),
  loreReview: document.querySelector("#lore-review"),
  loreSubjectTypeFilter: document.querySelector("#lore-subject-type-filter"),
  loreSubjectFilter: document.querySelector("#lore-subject-filter"),
  workspace: document.querySelector(".workspace"),
  nav: document.querySelector("#type-nav"),
  heading: document.querySelector("#collection-heading"),
  search: document.querySelector("#entity-search"),
  controls: document.querySelector("#entity-list-controls"),
  rarityField: document.querySelector("#rarity-filter-field"),
  rarityFilter: document.querySelector("#rarity-filter"),
  facetFilters: document.querySelector("#entity-facet-filters"),
  sortField: document.querySelector("#entity-sort-field"),
  sort: document.querySelector("#entity-sort"),
  status: document.querySelector("#list-status"),
  list: document.querySelector("#entity-list"),
  inspector: document.querySelector("#inspector-content"),
  newButton: document.querySelector("#new-entity"),
  dialog: document.querySelector("#entity-dialog"),
  dialogTitle: document.querySelector("#entity-dialog-title"),
  form: document.querySelector("#entity-form"),
  id: document.querySelector("#entity-id"),
  type: document.querySelector("#entity-type"),
  english: document.querySelector("#english-name"),
  original: document.querySelector("#original-name"),
  officialVietnamese: document.querySelector("#official-vietnamese"),
  automaticVietnamese: document.querySelector("#automatic-vietnamese"),
  description: document.querySelector("#description"),
  other: document.querySelector("#other-information"),
  save: document.querySelector("#save-entity"),
  toast: document.querySelector("#toast"),
  gameDialog: document.querySelector("#game-dialog"),
  gameForm: document.querySelector("#game-form"),
  gameModule: document.querySelector("#game-module"),
  gameName: document.querySelector("#game-name"),
  saveGame: document.querySelector("#save-game"),
};

let toastTimer;
let searchTimer;
let gameMenuFocusIndex = -1;

const themeProperties = {
  background: "--bg",
  surface: "--surface",
  surface_raised: "--surface-raised",
  surface_active: "--surface-active",
  border: "--border",
  border_strong: "--border-strong",
  text: "--text",
  text_muted: "--text-muted",
  text_subtle: "--text-subtle",
  accent: "--accent",
  accent_ink: "--accent-ink",
};

function applyTheme(theme) {
  for (const [key, property] of Object.entries(themeProperties)) {
    document.documentElement.style.setProperty(property, theme[key]);
  }
  document.documentElement.dataset.module = state.work.module_id;
}

function gameOptions() {
  return Array.from(dom.gameSwitcherMenu.querySelectorAll('[role="option"]'));
}

function closeGameMenu(restoreFocus = false) {
  dom.gameSwitcherMenu.hidden = true;
  dom.gameSwitcher.setAttribute("aria-expanded", "false");
  gameMenuFocusIndex = -1;
  if (restoreFocus) dom.gameSwitcher.focus();
}

function focusGameOption(index) {
  const options = gameOptions();
  if (options.length === 0) return;
  gameMenuFocusIndex = (index + options.length) % options.length;
  options[gameMenuFocusIndex].focus();
}

function openGameMenu() {
  if (dom.gameSwitcher.disabled) return;
  const options = gameOptions();
  if (options.length === 0) return;
  dom.gameSwitcherMenu.hidden = false;
  dom.gameSwitcher.setAttribute("aria-expanded", "true");
  const selectedIndex = options.findIndex(
    (option) => option.getAttribute("aria-selected") === "true",
  );
  focusGameOption(selectedIndex >= 0 ? selectedIndex : 0);
}

async function chooseGame(workId) {
  closeGameMenu(true);
  if (workId !== state.work?.id) await activateWork(workId);
}

function handleGameOptionKeydown(event) {
  const options = gameOptions();
  const currentIndex = options.indexOf(event.currentTarget);
  if (event.key === "ArrowDown") {
    event.preventDefault();
    focusGameOption(currentIndex + 1);
  } else if (event.key === "ArrowUp") {
    event.preventDefault();
    focusGameOption(currentIndex - 1);
  } else if (event.key === "Home") {
    event.preventDefault();
    focusGameOption(0);
  } else if (event.key === "End") {
    event.preventDefault();
    focusGameOption(options.length - 1);
  } else if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    event.currentTarget.click();
  } else if (event.key === "Escape") {
    event.preventDefault();
    closeGameMenu(true);
  } else if (event.key === "Tab") {
    closeGameMenu();
  }
}

function renderLibrary() {
  const activeWork =
    state.works.find((work) => work.id === state.work?.id) || state.works[0] || null;
  dom.gameSwitcherValue.textContent = activeWork?.display_name || "No games in library";
  dom.gameSwitcherMenu.replaceChildren();
  for (const work of state.works) {
    const module = state.modules.find(
      (candidate) => candidate.kind === work.kind && candidate.id === work.module_id,
    );
    const option = document.createElement("button");
    option.type = "button";
    option.id = `game-option-${work.id}`;
    option.className = "game-picker-option";
    option.setAttribute("role", "option");
    option.setAttribute("aria-selected", String(work.id === activeWork?.id));
    option.tabIndex = -1;
    option.dataset.workId = work.id;
    if (work.id === activeWork?.id) option.classList.add("is-selected");

    const mark = document.createElement("span");
    mark.className = "game-picker-option-mark";
    mark.setAttribute("aria-hidden", "true");
    mark.textContent = "✓";

    const copy = document.createElement("span");
    copy.className = "game-picker-option-copy";
    const title = document.createElement("strong");
    title.textContent = work.display_name;
    const support = document.createElement("small");
    support.textContent = module?.display_name || work.module_id;
    copy.append(title, support);
    option.append(mark, copy);
    option.addEventListener("click", () => chooseGame(work.id));
    option.addEventListener("keydown", handleGameOptionKeydown);
    dom.gameSwitcherMenu.append(option);
  }
  dom.gameSwitcher.disabled = state.works.length === 0;

  dom.gameModule.replaceChildren();
  for (const module of state.modules) {
    const option = document.createElement("option");
    option.value = module.id;
    option.textContent = module.display_name;
    dom.gameModule.append(option);
  }
}

function renderActiveWork() {
  const module = state.modules.find(
    (candidate) => candidate.kind === state.work.kind && candidate.id === state.work.module_id,
  );
  dom.pageTitle.textContent = state.work.display_name;
  dom.moduleKicker.textContent = `Game library / ${module?.display_name || state.work.module_id}`;
  dom.moduleId.textContent = state.work.module_id;
  dom.workspace.setAttribute("aria-label", `${state.work.display_name} wiki workspace`);
  document.title = `Perwiga · ${state.work.display_name}`;
  dom.loreViewButton.hidden = !state.loreSchema;
  if (!state.loreSchema && state.view === "lore") showView("wiki");
  renderLibrary();

  dom.type.replaceChildren();
  for (const type of state.types) {
    const option = document.createElement("option");
    option.value = type.key;
    option.textContent = type.display_name;
    dom.type.append(option);
  }
}

function showToast(message, isError = false) {
  clearTimeout(toastTimer);
  dom.toast.textContent = message;
  dom.toast.classList.toggle("is-error", isError);
  dom.toast.hidden = false;
  toastTimer = setTimeout(() => {
    dom.toast.hidden = true;
  }, 4200);
}

function setBusy(message) {
  dom.status.textContent = message;
  dom.list.setAttribute("aria-busy", "true");
}

function showView(view) {
  state.view = view === "timeline" || (view === "lore" && state.loreSchema) ? view : "wiki";
  dom.wikiView.hidden = state.view !== "wiki";
  dom.timelineView.hidden = state.view !== "timeline";
  dom.loreView.hidden = state.view !== "lore";
  for (const button of dom.viewButtons) {
    const active = button.dataset.view === state.view;
    button.classList.toggle("is-active", active);
    button.setAttribute("aria-pressed", String(active));
  }
  if (state.view === "timeline") {
    requestAnimationFrame(() => dom.timeline.focus({ preventScroll: true }));
  } else if (state.view === "lore") {
    requestAnimationFrame(() => dom.loreMap.focus({ preventScroll: true }));
  }
}

function currentTypeName() {
  return state.types.find((type) => type.key === state.selectedType)?.display_name || "All records";
}

function resetEntityControls() {
  state.selectedRarity = "";
  state.selectedFacets = {};
  state.sort = "name";
  dom.rarityFilter.value = "";
  dom.sort.value = "name";
}

function applicableFacets() {
  if (!state.selectedType) return [];
  return state.facets.filter((facet) => facet.entity_types.includes(state.selectedType));
}

function renderEntityControls() {
  const rarities = Array.from(
    new Set(
      state.entities
        .map((entity) => entity.presentation?.rarity)
        .filter(Number.isInteger),
    ),
  ).sort((left, right) => right - left);
  const hasRarity = rarities.length > 0;
  if (state.selectedRarity && !rarities.includes(Number(state.selectedRarity))) {
    state.selectedRarity = "";
  }

  dom.rarityFilter.replaceChildren();
  const allRarities = document.createElement("option");
  allRarities.value = "";
  allRarities.textContent = "All rarities";
  dom.rarityFilter.append(allRarities);
  for (const rarity of rarities) {
    const option = document.createElement("option");
    option.value = String(rarity);
    option.textContent = `${rarity}★`;
    dom.rarityFilter.append(option);
  }
  dom.rarityFilter.value = state.selectedRarity;
  dom.rarityField.hidden = !hasRarity;
  dom.sortField.hidden = !hasRarity;
  if (!hasRarity) state.sort = "name";
  dom.sort.value = state.sort;

  const facets = applicableFacets();
  dom.facetFilters.replaceChildren();
  for (const facet of facets) {
    const field = document.createElement("label");
    field.className = "list-control";
    const label = document.createElement("span");
    label.textContent = facet.display_name;
    const select = document.createElement("select");
    select.setAttribute("aria-label", `Filter by ${facet.display_name.toLowerCase()}`);
    const all = document.createElement("option");
    all.value = "";
    all.textContent = `All ${facet.display_name.toLowerCase()}s`;
    select.append(all);
    for (const facetOption of facet.options) {
      const option = document.createElement("option");
      option.value = facetOption.value;
      option.textContent = facetOption.display_name;
      select.append(option);
    }
    select.value = state.selectedFacets[facet.key] || "";
    select.addEventListener("change", () => {
      state.selectedFacets[facet.key] = select.value;
      renderWorkspace();
    });
    field.append(label, select);
    dom.facetFilters.append(field);
  }
  dom.controls.hidden = !hasRarity && facets.length === 0;
}

function visibleEntities() {
  return filterAndSortEntities(state.entities, {
    rarity: state.selectedRarity,
    facets: state.selectedFacets,
    sort: state.sort,
  });
}

function renderWorkspace() {
  renderEntityControls();
  const entities = visibleEntities();
  renderTypeNavigation(
    dom.nav,
    state.types,
    state.allEntities,
    state.selectedType,
    selectType,
  );
  renderEntityList(dom.list, entities, state.types, state.selectedId, selectEntity);
  dom.list.removeAttribute("aria-busy");
  dom.heading.textContent = currentTypeName();
  const noun = state.entities.length === 1 ? "record" : "records";
  dom.status.textContent = entities.length === state.entities.length
    ? `${entities.length} ${noun} shown`
    : `${entities.length} of ${state.entities.length} ${noun} shown`;
}

function renderTimeline() {
  const summary = renderEventTimeline(dom.timeline, state.events, state.eventStatus);
  const noun = summary.shown === 1 ? "event" : "events";
  const typeNoun = summary.lanes === 1 ? "type" : "types";
  const filterLabel = state.eventStatus === "all" ? "scheduled" : state.eventStatus;
  dom.timelineStatus.textContent = summary.shown
    ? `${summary.shown} ${filterLabel} ${noun} across ${summary.lanes} ${typeNoun}${summary.timeZone ? ` · ${summary.timeZone}` : ""}`
    : state.events.length
      ? `No ${filterLabel} events in this schedule`
      : "No event schedule has been imported for this title";
}

function renderLore() {
  const graph = state.loreGraph || { periods: [], events: [], relations: [], subjects: [], involvements: [] };
  dom.loreMap.removeAttribute("aria-busy");
  renderLoreMap(dom.loreMap, graph, {
    onEvent: selectLoreEvent,
    subjectType: state.loreSubjectType,
  });

  const currentSubjectType = state.loreSubjectType;
  dom.loreSubjectTypeFilter.replaceChildren(new Option("All subjects", ""));
  for (const subjectType of ["operator", "npc", "region"]) {
    const definition = state.loreSchema?.subject_types?.find((item) => item.key === subjectType);
    if (!definition) continue;
    dom.loreSubjectTypeFilter.append(new Option(definition.display_name, subjectType));
  }
  state.loreSubjectType = ["", "operator", "npc", "region"].includes(currentSubjectType)
    ? currentSubjectType
    : "";
  dom.loreSubjectTypeFilter.value = state.loreSubjectType;

  const currentSubject = state.loreSubjectId;
  dom.loreSubjectFilter.replaceChildren();
  dom.loreSubjectFilter.append(new Option("All subjects", ""));
  for (const subject of state.loreSubjects) {
    dom.loreSubjectFilter.append(new Option(
      `${subject.attested_name} · ${subject.proposed_type}`,
      subject.id,
    ));
  }
  state.loreSubjectId = state.loreSubjects.some((subject) => subject.id === currentSubject)
    ? currentSubject
    : "";
  dom.loreSubjectFilter.value = state.loreSubjectId;
  renderLoreReview(dom.loreReview, state.loreCandidates, {
    onDecision: reviewLoreCandidate,
  });
  const eventCount = graph.events.length;
  const candidateCount = state.loreCandidates.length;
  const characterSubjects = graph.subjects.filter((subject) =>
    subject.entity_type === "operator" || subject.entity_type === "npc"
  );
  const involvedIds = new Set(graph.involvements.map((item) => item.subject_id));
  const coverageSubjects = ["operator", "npc"].includes(state.loreSubjectType)
    ? characterSubjects.filter((subject) => subject.entity_type === state.loreSubjectType)
    : state.loreSubjectType === "region" ? [] : characterSubjects;
  const placedCharacters = coverageSubjects.filter((subject) => involvedIds.has(subject.id)).length;
  const characterCoverage = coverageSubjects.length
    ? ` · ${placedCharacters} placed / ${coverageSubjects.length - placedCharacters} awaiting placement`
    : "";
  dom.loreStatus.textContent = eventCount
    ? `${eventCount} reviewed ${eventCount === 1 ? "event" : "events"}${characterCoverage} · ${candidateCount} candidate${candidateCount === 1 ? "" : "s"} awaiting review`
    : candidateCount
      ? `${candidateCount} candidate${candidateCount === 1 ? "" : "s"} awaiting review · no events approved yet`
      : "No reviewed lore events yet";
}

async function selectLoreEvent(eventId) {
  state.selectedLoreEventId = eventId;
  dom.loreDetail.setAttribute("aria-busy", "true");
  try {
    const detail = await api.getLoreEvent(eventId);
    if (state.selectedLoreEventId !== eventId) return;
    renderLoreEventDetail(dom.loreDetail, detail, {
      onBack: () => {
        state.selectedLoreEventId = "";
        renderLoreEventDetail(dom.loreDetail, null);
      },
    });
  } catch (error) {
    renderLoreEventDetail(dom.loreDetail, null);
    showToast(error.message, true);
  } finally {
    dom.loreDetail.removeAttribute("aria-busy");
  }
}

async function reviewLoreCandidate(candidate, decision) {
  try {
    await api.reviewLoreCandidate(candidate.id, { decision });
    showToast(`Lore candidate ${decision === "approve" ? "approved" : "rejected"}`);
    await refreshLore();
  } catch (error) {
    showToast(error.message, true);
  }
}

async function refreshLore() {
  if (!state.work || !state.loreSchema) return;
  const workId = state.work.id;
  dom.loreMap.setAttribute("aria-busy", "true");
  dom.loreStatus.textContent = "Loading lore map…";
  try {
    const [graph, subjects, candidates] = await Promise.all([
      api.getLoreGraph(workId, {
        subjectId: state.loreSubjectId,
        subjectType: state.loreSubjectType,
      }),
      api.listLoreSubjects(workId),
      api.listLoreCandidates(workId),
    ]);
    if (state.work?.id !== workId) return;
    state.loreGraph = graph;
    state.loreSubjects = subjects;
    state.loreCandidates = candidates;
    renderLore();
  } catch (error) {
    if (state.work?.id !== workId) return;
    state.loreGraph = null;
    state.loreSubjects = [];
    state.loreCandidates = [];
    dom.loreMap.removeAttribute("aria-busy");
    dom.loreStatus.textContent = "The lore map could not be loaded.";
    showToast(error.message, true);
  }
}

async function refreshTimeline() {
  if (!state.work) return;
  const workId = state.work.id;
  dom.timeline.setAttribute("aria-busy", "true");
  dom.timelineStatus.textContent = "Loading event schedule…";
  try {
    const events = await api.listCalendarEvents(workId);
    if (state.work?.id !== workId) return;
    state.events = events;
    renderTimeline();
  } catch (error) {
    if (state.work?.id !== workId) return;
    state.events = [];
    dom.timeline.removeAttribute("aria-busy");
    dom.timelineStatus.textContent = "The event schedule could not be loaded.";
    showToast(error.message, true);
  }
}

async function refreshEntities() {
  if (!state.work) return;
  setBusy("Refreshing local records…");
  try {
    const filters = {
      query: dom.search.value,
      entityType: state.selectedType,
    };
    [state.allEntities, state.entities] = await Promise.all([
      api.listEntities(state.work.id),
      api.listEntities(state.work.id, filters),
    ]);
    renderWorkspace();
  } catch (error) {
    dom.list.removeAttribute("aria-busy");
    dom.status.textContent = "Records could not be loaded.";
    showToast(error.message, true);
  }
}

async function activateWork(workId) {
  const sequence = ++state.switchSequence;
  dom.gameSwitcher.disabled = true;
  setBusy("Switching game workspace…");
  try {
    const workspace = await api.getWorkspace(workId);
    if (sequence !== state.switchSequence) return;
    state.work = workspace.work;
    state.types = workspace.entity_types;
    state.facets = workspace.entity_facets || [];
    state.loreSchema = workspace.lore_schema || null;
    state.loreGraph = null;
    state.loreSubjects = [];
    state.loreCandidates = [];
    state.selectedLoreEventId = "";
    state.loreSubjectId = "";
    state.loreSubjectType = "";
    state.selectedType = "";
    state.selectedId = "";
    state.detail = null;
    state.events = [];
    state.eventStatus = "all";
    dom.search.value = "";
    resetEntityControls();
    for (const button of dom.eventStatusButtons) {
      const active = button.dataset.eventStatus === "all";
      button.classList.toggle("is-active", active);
      button.setAttribute("aria-pressed", String(active));
    }
    applyTheme(workspace.theme);
    renderActiveWork();
    renderInspectorEmpty(
      dom.inspector,
      "Select a record",
      `Choose an entry belonging to ${state.work.display_name}.`,
    );
    renderLoreEventDetail(dom.loreDetail, null);
    await Promise.all([refreshEntities(), refreshTimeline(), refreshLore()]);
  } catch (error) {
    showToast(error.message, true);
  } finally {
    if (sequence === state.switchSequence) dom.gameSwitcher.disabled = false;
  }
}

function selectType(type) {
  state.selectedType = type;
  resetEntityControls();
  refreshEntities();
}

async function selectEntity(entityId) {
  state.selectedId = entityId;
  renderEntityList(dom.list, visibleEntities(), state.types, state.selectedId, selectEntity);
  dom.inspector.setAttribute("aria-busy", "true");
  try {
    state.detail = await api.getEntity(entityId);
    renderEntityDetail(dom.inspector, state.detail, state.types, {
      onEdit: () => openEntityDialog(state.detail.entity),
      onAlias: async (values) => {
        try {
          await api.addAlias(entityId, values);
          showToast("Alias added");
          await selectEntity(entityId);
          await refreshEntities();
        } catch (error) {
          showToast(error.message, true);
          throw error;
        }
      },
    });
  } catch (error) {
    renderInspectorEmpty(dom.inspector, "Record unavailable", error.message);
    showToast(error.message, true);
  } finally {
    dom.inspector.removeAttribute("aria-busy");
  }
}

function fillForm(entity) {
  dom.form.reset();
  dom.id.value = entity?.id || "";
  dom.type.value = entity?.entity_type || state.selectedType || state.types[0]?.key || "";
  dom.type.disabled = Boolean(entity);
  dom.english.value = entity?.official_english_name || "";
  dom.original.value = entity?.official_original_name || "";
  dom.officialVietnamese.value = entity?.official_vietnamese_name || "";
  dom.automaticVietnamese.value = entity?.automatic_vietnamese_translation || "";
  dom.description.value = entity?.english_description || "";
  dom.other.value = entity?.other_information || "";
}

function openEntityDialog(entity = null) {
  fillForm(entity);
  dom.dialogTitle.textContent = entity ? "Edit record" : "New record";
  dom.save.textContent = entity ? "Save changes" : "Create record";
  dom.dialog.showModal();
  requestAnimationFrame(() => (entity ? dom.english : dom.type).focus());
}

function formValues() {
  return {
    entity_type: dom.type.value,
    official_english_name: dom.english.value.trim(),
    official_original_name: dom.original.value.trim(),
    official_vietnamese_name: dom.officialVietnamese.value.trim(),
    automatic_vietnamese_translation: dom.automaticVietnamese.value.trim(),
    english_description: dom.description.value.trim(),
    other_information: dom.other.value.trim(),
  };
}

async function saveEntity(event) {
  event.preventDefault();
  if (!dom.form.reportValidity()) return;
  dom.save.disabled = true;
  const editingId = dom.id.value;
  try {
    const values = formValues();
    const saved = editingId
      ? await api.updateEntity(editingId, values)
      : await api.createEntity(state.work.id, values);
    dom.dialog.close();
    showToast(editingId ? "Record updated" : "Record created");
    await refreshEntities();
    await selectEntity(saved.id);
  } catch (error) {
    showToast(error.message, true);
  } finally {
    dom.save.disabled = false;
  }
}

function openGameDialog() {
  dom.gameForm.reset();
  dom.gameDialog.showModal();
  requestAnimationFrame(() => dom.gameModule.focus());
}

function focusRecordSearch() {
  showView("wiki");
  const behavior = window.matchMedia("(prefers-reduced-motion: reduce)").matches
    ? "auto"
    : "smooth";
  dom.search.scrollIntoView({ behavior, block: "center" });
  dom.search.focus({ preventScroll: true });
}

async function saveGame(event) {
  event.preventDefault();
  if (!dom.gameForm.reportValidity()) return;
  dom.saveGame.disabled = true;
  try {
    const work = await api.createWork({
      kind: "game",
      module_id: dom.gameModule.value,
      display_name: dom.gameName.value.trim(),
    });
    state.works = await api.listWorks();
    renderLibrary();
    dom.gameDialog.close();
    showToast(`${work.display_name} added to the library`);
    await activateWork(work.id);
  } catch (error) {
    showToast(error.message, true);
  } finally {
    dom.saveGame.disabled = false;
  }
}

async function initialize() {
  try {
    const [setup] = await Promise.all([api.setupEndfield(), api.setupGenshin()]);
    [state.works, state.modules] = await Promise.all([api.listWorks(), api.listModules()]);
    renderLibrary();
    await activateWork(setup.work.id);
  } catch (error) {
    dom.status.textContent = "The local workspace could not be initialized.";
    showToast(error.message, true);
  }
}

dom.newButton.addEventListener("click", () => openEntityDialog());
dom.form.addEventListener("submit", saveEntity);
dom.addGame.addEventListener("click", openGameDialog);
dom.openSearch.addEventListener("click", focusRecordSearch);
dom.gameForm.addEventListener("submit", saveGame);
dom.gameSwitcher.addEventListener("click", () => {
  if (dom.gameSwitcher.getAttribute("aria-expanded") === "true") closeGameMenu();
  else openGameMenu();
});
dom.gameSwitcher.addEventListener("keydown", (event) => {
  if (event.key === "ArrowDown" || event.key === "ArrowUp") {
    event.preventDefault();
    openGameMenu();
  } else if (event.key === "Escape") {
    closeGameMenu();
  }
});
document.addEventListener("pointerdown", (event) => {
  if (!dom.gameSwitcherMenu.hidden && !dom.gameSwitcherShell.contains(event.target)) {
    closeGameMenu();
  }
});
dom.search.addEventListener("input", () => {
  clearTimeout(searchTimer);
  searchTimer = setTimeout(refreshEntities, 220);
});
dom.rarityFilter.addEventListener("change", () => {
  state.selectedRarity = dom.rarityFilter.value;
  renderWorkspace();
});
dom.sort.addEventListener("change", () => {
  state.sort = dom.sort.value;
  renderWorkspace();
});
dom.loreSubjectFilter.addEventListener("change", () => {
  state.loreSubjectId = dom.loreSubjectFilter.value;
  refreshLore();
});
dom.loreSubjectTypeFilter.addEventListener("change", () => {
  state.loreSubjectType = dom.loreSubjectTypeFilter.value;
  const focusedSubject = state.loreSubjects.find((subject) => subject.id === state.loreSubjectId);
  if (focusedSubject && state.loreSubjectType && focusedSubject.proposed_type !== state.loreSubjectType) {
    state.loreSubjectId = "";
  }
  refreshLore();
});
for (const button of dom.viewButtons) {
  button.addEventListener("click", () => showView(button.dataset.view));
}
for (const button of dom.eventStatusButtons) {
  button.addEventListener("click", () => {
    state.eventStatus = button.dataset.eventStatus;
    for (const candidate of dom.eventStatusButtons) {
      const active = candidate === button;
      candidate.classList.toggle("is-active", active);
      candidate.setAttribute("aria-pressed", String(active));
    }
    renderTimeline();
  });
}
for (const button of document.querySelectorAll("[data-close-dialog]")) {
  button.addEventListener("click", () => dom.dialog.close());
}
for (const button of document.querySelectorAll("[data-close-game-dialog]")) {
  button.addEventListener("click", () => dom.gameDialog.close());
}

initialize();
