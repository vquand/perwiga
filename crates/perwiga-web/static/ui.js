function element(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

function typeName(types, key) {
  return types.find((type) => type.key === key)?.display_name || key;
}

function initials(value) {
  return value
    .split(/\s+/u)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0])
    .join("")
    .toUpperCase();
}

function optionalText(value) {
  return value || "Not recorded";
}

function entityAvatar(entity) {
  const presentation = entity.presentation;
  const avatar = element("span", "entity-avatar");
  if (!presentation?.thumbnail_url) {
    avatar.textContent = initials(entity.official_english_name);
    avatar.setAttribute("aria-hidden", "true");
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
      avatar.classList.remove("has-thumbnail");
      avatar.style.removeProperty("--entity-accent");
      avatar.textContent = initials(entity.official_english_name);
      avatar.setAttribute("aria-hidden", "true");
    },
    { once: true },
  );
  avatar.append(image, rarity);
  return avatar;
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
    const meta = element(
      "span",
      "type-meta",
      item.key ? item.key : "module index",
    );
    const count = element(
      "span",
      "type-count",
      String(item.key ? counts[item.key] || 0 : entities.length),
    );
    button.append(label, meta, count);
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
      element("strong", "entity-name", entity.official_english_name),
      element("span", "entity-original", entity.official_original_name),
    );
    const kind = element("span", "entity-kind", typeName(types, entity.entity_type));
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

export function renderEntityDetail(container, detail, types, { onEdit, onAlias }) {
  const { entity, aliases } = detail;
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

  container.append(heading, names, aliasSection, aliasEditor(onAlias));
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
