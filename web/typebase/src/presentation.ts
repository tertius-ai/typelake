export interface PresentationChoice {
  id: string;
  name: string;
  note: string;
}

export const SITE_STYLES: readonly PresentationChoice[] = [
  { id: "report", name: "Technical memorandum", note: "Monospaced release copy" },
  { id: "registrar", name: "Card catalogue", note: "Buff label cards" },
  { id: "monograph", name: "Mathematical folio", note: "Quiet scholarly page" },
  { id: "field-notes", name: "Field notes", note: "Ruled survey notebook" },
  { id: "swiss-index", name: "Swiss index", note: "International typographic grid" },
  { id: "blueprint", name: "Blueprint office", note: "White line on Prussian blue" },
  { id: "tabulator", name: "Tabulator", note: "Green-bar machine listing" },
  { id: "biblioteca", name: "Biblioteca", note: "Italian reading room" },
  { id: "constructivist", name: "Constructivist", note: "Red, black, and diagonal" },
  { id: "hydrographic", name: "Hydrographic chart", note: "Soundings and coastal ink" },
  { id: "laboratory", name: "Laboratory sheet", note: "Measured ruled record" },
  { id: "microfilm", name: "Microfilm reader", note: "Silver image and frame marks" },
  { id: "concrete", name: "Concrete index", note: "Brutal modular register" },
  { id: "herbarium", name: "Herbarium", note: "Natural-history specimen sheet" },
  { id: "bureau", name: "Bureau form", note: "Civil-service typescript" },
  { id: "blackboard", name: "Blackboard", note: "Chalk lecture room" },
  { id: "terminal", name: "Phosphor terminal", note: "Amber command-line catalogue" },
  { id: "risograph", name: "Risograph", note: "Two-ink duplicated pamphlet" },
  { id: "theorem", name: "Theorem journal", note: "Dense numbered mathematics" },
  { id: "punchcard", name: "Punch card", note: "Machine-readable deck" },
  { id: "cartographic", name: "Cartographic index", note: "Gridded geographic sheet" },
  { id: "specimen", name: "Type specimen", note: "Display type and fine rules" },
  { id: "nocturne", name: "Nocturne", note: "Midnight blue reading room" },
  { id: "codex", name: "Illuminated codex", note: "Pigment, rubric, and vellum" },
  { id: "archive", name: "Archival box", note: "Acid-free finding aid" },
] as const;

export const DIAGRAM_STYLES: readonly PresentationChoice[] = [
  { id: "byrne", name: "Byrne plate", note: "Euclidean primary pigments" },
  { id: "cyanotype", name: "Cyanotype", note: "Blueprint contact print" },
  { id: "copperplate", name: "Copperplate", note: "Fine engraved line" },
  { id: "bauhaus", name: "Bauhaus", note: "Primary form and heavy rule" },
  { id: "survey", name: "Survey sheet", note: "Measured grid and station marks" },
  { id: "manuscript", name: "Manuscript", note: "Rubricated mathematical hand" },
  { id: "oscilloscope", name: "Oscilloscope", note: "Green trace on glass" },
  { id: "tabular", name: "Tabular ink", note: "Black machine diagram" },
  { id: "cartographic", name: "Cartographic", note: "Navy route and vermilion station" },
  { id: "chalk", name: "Chalk study", note: "Lecture-board construction" },
] as const;

const STYLE_STORAGE = "typebase-site-proof";
const DIAGRAM_STORAGE = "typebase-diagram-plate";
const DEFAULT_STYLE = "report";
const DEFAULT_DIAGRAM = "byrne";

function accepted(
  choices: readonly PresentationChoice[],
  value: string | null | undefined,
): value is string {
  return Boolean(value && choices.some((choice) => choice.id === value));
}

function stored(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function remember(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // The query string remains the shareable source of truth when storage is unavailable.
  }
}

function current(kind: "site" | "diagram"): string {
  return kind === "site"
    ? document.documentElement.dataset.style ?? DEFAULT_STYLE
    : document.documentElement.dataset.plate ?? DEFAULT_DIAGRAM;
}

function setCurrent(kind: "site" | "diagram", value: string, updateAddress = true): void {
  const choices = kind === "site" ? SITE_STYLES : DIAGRAM_STYLES;
  const fallback = choices[0]?.id ?? "";
  const selected = accepted(choices, value) ? value : fallback;
  const parameter = kind === "site" ? "style" : "plate";
  const storage = kind === "site" ? STYLE_STORAGE : DIAGRAM_STORAGE;
  if (kind === "site") document.documentElement.dataset.style = selected;
  else document.documentElement.dataset.plate = selected;
  remember(storage, selected);
  if (updateAddress) {
    const url = new URL(window.location.href);
    url.searchParams.set(parameter, selected);
    history.replaceState(history.state, "", url);
  }
  synchronizeControls(kind);
  window.dispatchEvent(
    new CustomEvent("typebase:presentationchange", { detail: { kind, value: selected } }),
  );
}

function move(kind: "site" | "diagram", amount: number): void {
  const choices = kind === "site" ? SITE_STYLES : DIAGRAM_STYLES;
  const index = Math.max(0, choices.findIndex((choice) => choice.id === current(kind)));
  const next = (index + amount + choices.length) % choices.length;
  setCurrent(kind, choices[next]?.id ?? choices[0]?.id ?? "");
}

function synchronizeControls(kind?: "site" | "diagram"): void {
  for (const control of document.querySelectorAll<HTMLElement>("[data-presentation-controls]")) {
    const controlKind = control.dataset.presentationControls === "diagram" ? "diagram" : "site";
    if (kind && controlKind !== kind) continue;
    const choices = controlKind === "site" ? SITE_STYLES : DIAGRAM_STYLES;
    const selected = current(controlKind);
    const index = Math.max(0, choices.findIndex((choice) => choice.id === selected));
    const choice = choices[index] ?? choices[0];
    const select = control.querySelector<HTMLSelectElement>("select");
    const position = control.querySelector<HTMLElement>("[data-presentation-position]");
    const note = control.querySelector<HTMLElement>("[data-presentation-note]");
    if (select) select.value = choice?.id ?? "";
    if (position) position.textContent = `${String(index + 1).padStart(2, "0")}/${choices.length}`;
    if (note) note.textContent = choice?.note ?? "";
  }
}

function mountControl(control: HTMLElement): void {
  if (control.dataset.presentationMounted === "true") return;
  const kind = control.dataset.presentationControls === "diagram" ? "diagram" : "site";
  const choices = kind === "site" ? SITE_STYLES : DIAGRAM_STYLES;
  const noun = kind === "site" ? "Proof" : "Plate";
  control.classList.add("presentation-stepper");
  control.innerHTML = `
    <span class="presentation-stepper__kind">${noun}</span>
    <button class="presentation-stepper__step" type="button" data-presentation-previous aria-label="Previous ${noun.toLowerCase()}">←</button>
    <label class="presentation-stepper__choice">
      <span class="sr-only">${noun} style</span>
      <select aria-label="${noun} style">
        ${choices
          .map(
            (choice, index) =>
              `<option value="${choice.id}">${String(index + 1).padStart(2, "0")} · ${choice.name}</option>`,
          )
          .join("")}
      </select>
      <small data-presentation-note></small>
    </label>
    <span class="presentation-stepper__position" data-presentation-position></span>
    <button class="presentation-stepper__step" type="button" data-presentation-next aria-label="Next ${noun.toLowerCase()}">→</button>`;
  control.querySelector("[data-presentation-previous]")?.addEventListener("click", () =>
    move(kind, -1),
  );
  control.querySelector("[data-presentation-next]")?.addEventListener("click", () =>
    move(kind, 1),
  );
  control.querySelector("select")?.addEventListener("change", (event) => {
    if (event.currentTarget instanceof HTMLSelectElement) {
      setCurrent(kind, event.currentTarget.value);
    }
  });
  control.dataset.presentationMounted = "true";
}

function currentCollection(): string | null {
  const path = window.location.pathname;
  if (/\/(?:Poset\/|typebase\/posets\.html)/.test(path)) return "Poset";
  if (/\/(?:Steiner\/|typebase\/steiner\.html)/.test(path)) return "Steiner";
  return [
    "SimplicialComplex",
    "SimplicialSet",
    "FiniteGroupCohomology",
    "EilenbergMacLane",
  ].find((collection) => path.split("/").includes(collection)) ?? null;
}

function mountCollectionMenu(): void {
  const selected = currentCollection();
  for (const link of document.querySelectorAll<HTMLAnchorElement>("[data-typebase-collection]")) {
    if (link.dataset.typebaseCollection === selected) link.setAttribute("aria-current", "page");
    else link.removeAttribute("aria-current");
  }
  for (const menu of document.querySelectorAll<HTMLDetailsElement>(".collection-menu")) {
    if (menu.dataset.collectionMenuMounted === "true") continue;
    document.addEventListener("click", (event) => {
      if (menu.open && event.target instanceof Node && !menu.contains(event.target)) {
        menu.open = false;
      }
    });
    menu.addEventListener("keydown", (event) => {
      if (event.key === "Escape") {
        menu.open = false;
        menu.querySelector("summary")?.focus();
      }
    });
    menu.dataset.collectionMenuMounted = "true";
  }
}

export function initializePresentation(): void {
  const url = new URL(window.location.href);
  const requestedStyle = url.searchParams.get("style");
  const requestedDiagram = url.searchParams.get("plate");
  const storedStyle = stored(STYLE_STORAGE);
  const storedDiagram = stored(DIAGRAM_STORAGE);
  const site = accepted(SITE_STYLES, requestedStyle)
    ? requestedStyle
    : accepted(SITE_STYLES, storedStyle)
      ? storedStyle
      : DEFAULT_STYLE;
  const diagram = accepted(DIAGRAM_STYLES, requestedDiagram)
    ? requestedDiagram
    : accepted(DIAGRAM_STYLES, storedDiagram)
      ? storedDiagram
      : DEFAULT_DIAGRAM;
  setCurrent("site", site, false);
  setCurrent("diagram", diagram, false);
  mountCollectionMenu();
  mountPresentationControls();
}

export function mountPresentationControls(root: ParentNode = document): void {
  for (const control of root.querySelectorAll<HTMLElement>("[data-presentation-controls]")) {
    mountControl(control);
  }
  synchronizeControls();
}
