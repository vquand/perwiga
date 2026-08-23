import { api } from "/assets/api.js";
import {
  renderEntityDetail,
  renderEntityList,
  renderInspectorEmpty,
  renderTypeNavigation,
} from "/assets/ui.js";

const state = {
  works: [],
  modules: [],
  work: null,
  types: [],
  allEntities: [],
  entities: [],
  selectedType: "",
  selectedId: "",
  detail: null,
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
  workspace: document.querySelector(".workspace"),
  nav: document.querySelector("#type-nav"),
  heading: document.querySelector("#collection-heading"),
  search: document.querySelector("#entity-search"),
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

function currentTypeName() {
  return state.types.find((type) => type.key === state.selectedType)?.display_name || "All records";
}

function renderWorkspace() {
  renderTypeNavigation(
    dom.nav,
    state.types,
    state.allEntities,
    state.selectedType,
    selectType,
  );
  renderEntityList(dom.list, state.entities, state.types, state.selectedId, selectEntity);
  dom.list.removeAttribute("aria-busy");
  dom.heading.textContent = currentTypeName();
  const noun = state.entities.length === 1 ? "record" : "records";
  dom.status.textContent = `${state.entities.length} ${noun} shown`;
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
    state.selectedType = "";
    state.selectedId = "";
    state.detail = null;
    dom.search.value = "";
    applyTheme(workspace.theme);
    renderActiveWork();
    renderInspectorEmpty(
      dom.inspector,
      "Select a record",
      `Choose an entry belonging to ${state.work.display_name}.`,
    );
    await refreshEntities();
  } catch (error) {
    showToast(error.message, true);
  } finally {
    if (sequence === state.switchSequence) dom.gameSwitcher.disabled = false;
  }
}

function selectType(type) {
  state.selectedType = type;
  refreshEntities();
}

async function selectEntity(entityId) {
  state.selectedId = entityId;
  renderEntityList(dom.list, state.entities, state.types, state.selectedId, selectEntity);
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
    const setup = await api.setupEndfield();
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
for (const button of document.querySelectorAll("[data-close-dialog]")) {
  button.addEventListener("click", () => dom.dialog.close());
}
for (const button of document.querySelectorAll("[data-close-game-dialog]")) {
  button.addEventListener("click", () => dom.gameDialog.close());
}

initialize();
