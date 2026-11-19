import { complexPlate, simplicialSetPlate } from "./collection-plates";
import { displayGroupName, standardGroupName } from "./group-names";
import { hasseDiagram } from "./hasse";
import type { Artifact, DetailRow, Manifest, PosetRecord } from "./model";
import { initializePresentation } from "./presentation";
import { posetPath, steinerPath } from "./routes";
import { steinerDiagram } from "./steiner-diagram";
import type { BoundaryRecord } from "./steiner-model";

type JsonObject = Record<string, unknown>;

interface SteinerGalleryPart {
  path: string;
  row_count: number;
  first_label: string;
  last_label: string;
}

interface SteinerGalleryIndex {
  record_version: number;
  category_count: number;
  rows_per_part: number;
  parts: SteinerGalleryPart[];
}

interface SteinerGalleryRow {
  label: string;
  atomic_size: number;
  dimension: number;
  bucket_index: number;
  boundaries: BoundaryRecord[][];
  object_count: number;
  generating_arrow_count: number;
  is_empty: boolean;
}

interface PosetGalleryPart {
  artifact: Artifact;
  order: number;
  dimension: number;
  part: number;
}

interface IndexedGalleryRow extends JsonObject {
  label: string;
  path: string;
  detail_path?: string;
  detail_row?: number;
}

type GalleryCollection =
  | "steiner"
  | "poset"
  | "complex"
  | "simplicial-set"
  | "group-cohomology"
  | "eilenberg-maclane";

interface GalleryState {
  ready: boolean;
  loading: boolean;
  fixed: boolean;
  loaded: number;
  total: number;
  error: string;
}

const GALLERY_BATCH_SIZE = 48;
const galleryOrder: GalleryCollection[] = [
  "steiner",
  "poset",
  "complex",
  "simplicial-set",
  "group-cohomology",
  "eilenberg-maclane",
];
const browsePaths: Record<GalleryCollection, string> = {
  steiner: "/typebase/steiner.html",
  poset: "/typebase/posets.html",
  complex: "/SimplicialComplex/",
  "simplicial-set": "/SimplicialSet/",
  "group-cohomology": "/FiniteGroupCohomology/",
  "eilenberg-maclane": "/EilenbergMacLane/",
};

function required<T extends HTMLElement>(id: string): T {
  const found = document.getElementById(id);
  if (!found) throw new Error(`Missing browser element #${id}`);
  return found as T;
}

function asObject(value: unknown): JsonObject {
  return typeof value === "object" && value !== null ? (value as JsonObject) : {};
}

function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : [];
}

function number(value: unknown): number {
  return typeof value === "number" ? value : Number(value) || 0;
}

function strings(value: unknown): string[] {
  return asArray(value).map(String);
}

function numberMatrix(value: unknown): number[][] {
  return asArray(value).map((row) => asArray(row).map(number));
}

const legacyLabel = new URL(window.location.href).searchParams.get("label");
if (legacyLabel && /^\d+\.-?\d+\.\d+$/.test(legacyLabel)) {
  window.location.replace(posetPath(legacyLabel));
}

initializePresentation();

const panels = new Map<GalleryCollection, HTMLElement>();
const tabs = new Map<GalleryCollection, HTMLButtonElement>();
for (const collection of galleryOrder) {
  panels.set(collection, required<HTMLElement>(`${collection}-gallery`));
  tabs.set(collection, required<HTMLButtonElement>(`gallery-${collection}-tab`));
}
const browse = required<HTMLAnchorElement>("gallery-browse");
const stream = required<HTMLElement>("collection-gallery-stream");
const more = required<HTMLButtonElement>("collection-gallery-more");
const status = required<HTMLElement>("collection-gallery-status");

const states = new Map<GalleryCollection, GalleryState>(
  galleryOrder.map((collection) => [
    collection,
    {
      ready: false,
      loading: false,
      fixed: !["steiner", "poset"].includes(collection),
      loaded: 0,
      total: 0,
      error: "",
    },
  ]),
);

function state(collection: GalleryCollection): GalleryState {
  const value = states.get(collection);
  if (!value) throw new Error(`Missing gallery state for ${collection}`);
  return value;
}

const steinerDataRoot = new URL(
  import.meta.env.VITE_TYPEBASE_STEINER_DATA_URL ?? "data/strong-steiner-categories/",
  document.baseURI,
);
const posetDataRoot = new URL(
  import.meta.env.VITE_TYPEBASE_DATA_URL ?? "data/finite-posets/",
  document.baseURI,
);
const complexDataRoot = new URL(
  import.meta.env.VITE_TYPEBASE_SIMPLICIAL_COMPLEX_DATA_URL ??
    "data/finite-simplicial-complexes/",
  document.baseURI,
);
const simplicialSetDataRoot = new URL(
  import.meta.env.VITE_TYPEBASE_SIMPLICIAL_SET_DATA_URL ?? "data/finite-simplicial-sets/",
  document.baseURI,
);
const groupDataRoot = new URL(
  // The documented name, as read by volume-one.ts; the undocumented alias is a fallback.
  import.meta.env.VITE_TYPEBASE_FINITE_GROUP_COHOMOLOGY_DATA_URL ??
    import.meta.env.VITE_TYPEBASE_GROUP_COHOMOLOGY_DATA_URL ??
    "data/finite-group-cohomology/",
  document.baseURI,
);
const eilenbergMacLaneDataRoot = new URL(
  import.meta.env.VITE_TYPEBASE_EILENBERG_MACLANE_DATA_URL ?? "data/eilenberg-maclane/",
  document.baseURI,
);

let activeCollection: GalleryCollection = "steiner";
let steinerIndex: SteinerGalleryIndex | null = null;
let steinerNextPart = 0;
let posetManifest: Manifest | null = null;
let posetParts: PosetGalleryPart[] = [];
let posetNextPart = 0;
let posetRows: DetailRow[] = [];
let posetNextRow = 0;
const initializations = new Map<GalleryCollection, Promise<void>>();
const detailCache = new Map<string, Promise<unknown[]>>();

function galleryCard(
  href: string,
  labelText: string,
  factsText: string,
  visual: Element | string,
  collection: GalleryCollection,
): HTMLAnchorElement {
  const link = document.createElement("a");
  link.className = `collection-gallery__item collection-gallery__item--${collection}`;
  link.href = href;
  link.setAttribute("aria-label", `Open ${labelText}`);

  const figure = document.createElement("figure");
  const image = document.createElement("div");
  image.className = "collection-gallery__image";
  if (typeof visual === "string") image.innerHTML = visual;
  else image.append(visual);

  const caption = document.createElement("figcaption");
  const label = document.createElement("strong");
  label.textContent = labelText;
  const facts = document.createElement("span");
  facts.textContent = factsText;
  caption.append(label, facts);
  figure.append(image, caption);
  link.append(figure);
  return link;
}

function steinerGalleryCard(row: SteinerGalleryRow): HTMLAnchorElement {
  const diagram = steinerDiagram(
    {
      boundaries: row.boundaries,
      invariants: {
        object_count: row.object_count,
        generating_arrow_count: row.generating_arrow_count,
        is_empty: row.is_empty,
      },
    },
    [],
    () => undefined,
    { thumbnail: true },
  );
  return galleryCard(
    steinerPath(row.label),
    row.label,
    `${row.atomic_size.toLocaleString()} ${row.atomic_size === 1 ? "atom" : "atoms"} · dimension ${row.dimension}`,
    diagram,
    "steiner",
  );
}

function posetGalleryCard(row: DetailRow): HTMLAnchorElement {
  const poset = JSON.parse(row.poset_json) as PosetRecord;
  const dimension = Number(row.label.split(".")[1]);
  const diagram = hasseDiagram(poset);
  diagram.classList.add("hasse--thumbnail");
  return galleryCard(
    posetPath(row.label),
    row.label,
    `${poset.order.toLocaleString()} ${poset.order === 1 ? "point" : "points"} · dimension ${dimension}`,
    diagram,
    "poset",
  );
}

function algebraPlate(title: string, degrees: number[], operations: string[]): HTMLElement {
  const plate = document.createElement("div");
  plate.className = "algebra-gallery-plate";
  const symbol = document.createElement("strong");
  symbol.textContent = title;
  const degreeLine = document.createElement("p");
  degreeLine.className = "algebra-gallery-plate__degrees";
  degreeLine.textContent = degrees.length
    ? `generator degrees  ${degrees.join(" · ")}`
    : "unit presentation";
  const operationLine = document.createElement("p");
  operationLine.textContent = operations.length
    ? operations.slice(0, 4).join("  /  ")
    : "complete graded-commutative presentation";
  plate.append(symbol, degreeLine, operationLine);
  return plate;
}

async function fetchJson<T>(url: URL): Promise<T> {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`${url.pathname} returned HTTP ${response.status}`);
  if (!url.pathname.endsWith(".zst")) return (await response.json()) as T;
  const [{ decompress }, compressed] = await Promise.all([
    import("fzstd"),
    response.arrayBuffer(),
  ]);
  return JSON.parse(new TextDecoder().decode(decompress(new Uint8Array(compressed)))) as T;
}

function complete(collection: GalleryCollection): boolean {
  const value = state(collection);
  return value.ready && value.loaded >= value.total;
}

function updateStatus(): void {
  const value = state(activeCollection);
  if (value.error) status.textContent = value.error;
  else if (!value.ready) status.textContent = "Preparing the gallery…";
  else if (value.fixed) status.textContent = `${value.loaded.toLocaleString()} selected records`;
  else status.textContent = `${value.loaded.toLocaleString()} of ${value.total.toLocaleString()}`;
  more.hidden = value.fixed || complete(activeCollection) || Boolean(value.error);
  more.disabled = value.loading;
  stream.classList.toggle("is-complete", complete(activeCollection));
}

async function loadNextSteinerPart(): Promise<void> {
  const value = state("steiner");
  if (value.loading || !steinerIndex || steinerNextPart >= steinerIndex.parts.length) return;
  const part = steinerIndex.parts[steinerNextPart];
  if (!part) return;
  value.loading = true;
  updateStatus();
  try {
    const rows = await fetchJson<SteinerGalleryRow[]>(new URL(part.path, steinerDataRoot));
    if (
      rows.length !== part.row_count ||
      rows[0]?.label !== part.first_label ||
      rows.at(-1)?.label !== part.last_label
    ) {
      throw new Error("The Steiner gallery part does not match its index");
    }
    const fragment = document.createDocumentFragment();
    for (const row of rows) fragment.append(steinerGalleryCard(row));
    panels.get("steiner")?.append(fragment);
    value.loaded += rows.length;
    steinerNextPart += 1;
  } catch (error) {
    value.error = error instanceof Error ? error.message : String(error);
  } finally {
    value.loading = false;
    updateStatus();
  }
}

function parsePosetPart(artifact: Artifact): PosetGalleryPart | null {
  if (artifact.kind !== "details") return null;
  const match = /^details\/order=(\d+)\/dimension=(-?\d+)\/part-(\d+)\.parquet$/.exec(
    artifact.path,
  );
  if (!match) return null;
  return {
    artifact,
    order: Number(match[1]),
    dimension: Number(match[2]),
    part: Number(match[3]),
  };
}

async function readNextPosetPart(): Promise<boolean> {
  const part = posetParts[posetNextPart];
  if (!part) return false;
  const [{ parquetReadObjects }, { decompress }, response] = await Promise.all([
    import("hyparquet"),
    import("fzstd"),
    fetch(new URL(part.artifact.path, posetDataRoot)),
  ]);
  if (!response.ok) throw new Error(`Poset gallery data returned HTTP ${response.status}`);
  const rows = (await parquetReadObjects({
    file: await response.arrayBuffer(),
    compressors: { ZSTD: (input) => decompress(input) },
    columns: ["label", "poset_json"],
  })) as DetailRow[];
  if (rows.length !== part.artifact.row_count) {
    throw new Error("A Poset gallery part does not match its manifest");
  }
  posetRows = rows;
  posetNextRow = 0;
  posetNextPart += 1;
  return true;
}

async function loadNextPosetBatch(): Promise<void> {
  const value = state("poset");
  if (value.loading || !posetManifest || complete("poset")) return;
  value.loading = true;
  updateStatus();
  const fragment = document.createDocumentFragment();
  let appended = 0;
  try {
    while (appended < GALLERY_BATCH_SIZE) {
      if (posetNextRow >= posetRows.length && !(await readNextPosetPart())) break;
      const row = posetRows[posetNextRow];
      if (!row) break;
      fragment.append(posetGalleryCard(row));
      posetNextRow += 1;
      value.loaded += 1;
      appended += 1;
    }
    panels.get("poset")?.append(fragment);
  } catch (error) {
    value.error = error instanceof Error ? error.message : String(error);
  } finally {
    value.loading = false;
    updateStatus();
  }
}

async function initializeSteinerGallery(): Promise<void> {
  const value = state("steiner");
  try {
    steinerIndex = await fetchJson<SteinerGalleryIndex>(
      new URL("gallery/index.json", steinerDataRoot),
    );
    if (
      steinerIndex.record_version !== 1 ||
      steinerIndex.category_count < 1 ||
      steinerIndex.parts.reduce((count, part) => count + part.row_count, 0) !==
        steinerIndex.category_count
    ) {
      throw new Error("The Steiner gallery index is inconsistent");
    }
    value.ready = true;
    value.total = steinerIndex.category_count;
    await loadNextSteinerPart();
  } catch (error) {
    value.error = error instanceof Error ? error.message : String(error);
    updateStatus();
  }
}

async function initializePosetGallery(): Promise<void> {
  const value = state("poset");
  try {
    posetManifest = await fetchJson<Manifest>(new URL("manifest.json", posetDataRoot));
    posetParts = posetManifest.outputs
      .map(parsePosetPart)
      .filter((part): part is PosetGalleryPart => part !== null)
      .sort((left, right) => {
        const leftElementary = left.dimension <= 0 ? 1 : 0;
        const rightElementary = right.dimension <= 0 ? 1 : 0;
        return (
          leftElementary - rightElementary ||
          left.order - right.order ||
          left.dimension - right.dimension ||
          left.part - right.part
        );
      });
    if (
      posetManifest.poset_count < 1 ||
      posetParts.reduce((count, part) => count + part.artifact.row_count, 0) !==
        posetManifest.poset_count
    ) {
      throw new Error("The Poset gallery manifest is inconsistent");
    }
    value.ready = true;
    value.total = posetManifest.poset_count;
    await loadNextPosetBatch();
  } catch (error) {
    value.error = error instanceof Error ? error.message : String(error);
    updateStatus();
  }
}

async function indexedDetails(row: IndexedGalleryRow, root: URL): Promise<JsonObject> {
  if (!row.detail_path) throw new Error(`No detail path for ${row.label}`);
  const url = new URL(row.detail_path, root);
  let pending = detailCache.get(url.href);
  if (!pending) {
    pending = fetchJson<unknown[]>(url);
    detailCache.set(url.href, pending);
  }
  const rows = await pending;
  return asObject(rows[number(row.detail_row)]);
}

function chooseRows(
  rows: IndexedGalleryRow[],
  wanted: string[],
  maximum: number,
): IndexedGalleryRow[] {
  const byLabel = new Map(rows.map((row) => [row.label, row]));
  const chosen = wanted.map((label) => byLabel.get(label)).filter(Boolean) as IndexedGalleryRow[];
  if (chosen.length >= maximum) return chosen.slice(0, maximum);
  const remaining = rows.filter((row) => !chosen.some((entry) => entry.path === row.path));
  const needed = maximum - chosen.length;
  for (let index = 0; index < needed && remaining.length; index += 1) {
    const position = Math.min(
      remaining.length - 1,
      Math.floor(((index + 0.5) * remaining.length) / needed),
    );
    const row = remaining[position];
    if (row) chosen.push(row);
  }
  return chosen;
}

async function initializeFixedGallery(
  collection: GalleryCollection,
  loader: () => Promise<HTMLAnchorElement[]>,
): Promise<void> {
  const value = state(collection);
  value.loading = true;
  updateStatus();
  try {
    const cards = await loader();
    const fragment = document.createDocumentFragment();
    for (const card of cards) fragment.append(card);
    panels.get(collection)?.append(fragment);
    value.loaded = cards.length;
    value.total = cards.length;
    value.ready = true;
  } catch (error) {
    value.error = error instanceof Error ? error.message : String(error);
  } finally {
    value.loading = false;
    updateStatus();
  }
}

async function complexCards(): Promise<HTMLAnchorElement[]> {
  const buckets = await Promise.all(
    [
      "search/vertices=3/dimension=1.json.zst",
      "search/vertices=4/dimension=2.json.zst",
      "search/vertices=5/dimension=2.json.zst",
    ].map((path) => fetchJson<IndexedGalleryRow[]>(new URL(path, complexDataRoot))),
  );
  const rows = chooseRows(
    buckets.flat(),
    ["3.1.2", "4.2.2", "4.2.6", "5.2.1", "5.2.15", "5.2.37", "5.2.72", "5.2.117"],
    12,
  );
  return Promise.all(
    rows.map(async (row) => {
      const detail = await indexedDetails(row, complexDataRoot);
      const identity = asObject(detail.identity);
      const facets = numberMatrix(identity.facets);
      return galleryCard(
        row.path,
        row.label,
        `${number(row.vertices)} vertices · ${number(row.facet_count)} facets`,
        complexPlate(number(row.vertices), facets),
        "complex",
      );
    }),
  );
}

async function simplicialSetCards(): Promise<HTMLAnchorElement[]> {
  const buckets = await Promise.all(
    [
      "search/total=2/dimension=2.json.zst",
      "search/total=3/dimension=2.json.zst",
      "search/total=3/dimension=3.json.zst",
      "search/total=4/dimension=2.json.zst",
      "search/total=4/dimension=3.json.zst",
    ].map((path) => fetchJson<IndexedGalleryRow[]>(new URL(path, simplicialSetDataRoot))),
  );
  const rows = chooseRows(
    buckets.flat(),
    ["2.2.1", "3.2.1", "3.2.4", "3.3.7", "4.2.1", "4.2.20", "4.2.40", "4.3.1"],
    12,
  );
  return Promise.all(
    rows.map(async (row) => {
      const detail = await indexedDetails(row, simplicialSetDataRoot);
      const faceData = asArray(asObject(asObject(detail.identity).key).nondegenerate_faces);
      return galleryCard(
        row.path,
        row.label,
        `${number(row.total_nondegenerate)} cells · dimension ${number(row.dimension)}`,
        simplicialSetPlate(faceData),
        "simplicial-set",
      );
    }),
  );
}

function evenlySample(rows: IndexedGalleryRow[], count: number): IndexedGalleryRow[] {
  if (rows.length <= count) return rows;
  return Array.from({ length: count }, (_, index) => {
    const position = Math.min(rows.length - 1, Math.floor(((index + 0.5) * rows.length) / count));
    return rows[position];
  }).filter((row): row is IndexedGalleryRow => Boolean(row));
}

async function groupCohomologyCards(): Promise<HTMLAnchorElement[]> {
  const rows = await fetchJson<IndexedGalleryRow[]>(new URL("search.json", groupDataRoot));
  const selected = evenlySample(
    rows.filter((row) => Boolean(row.complete) && strings(row.named_operations).length > 0),
    12,
  );
  return selected.map((row) => {
    const coefficient = String(row.coefficient ?? "Z");
    const degrees = asArray(row.generator_degrees).map(number);
    const operations = strings(row.named_operations);
    const groupName = displayGroupName(standardGroupName(row.label, row.standard_name));
    return galleryCard(
      row.path,
      `${row.label} / ${coefficient}`,
      `${groupName} · ${degrees.length} generators`,
      algebraPlate(`H*(B${groupName}; ${coefficient})`, degrees, operations),
      "group-cohomology",
    );
  });
}

async function eilenbergMacLaneCards(): Promise<HTMLAnchorElement[]> {
  const rows = await fetchJson<IndexedGalleryRow[]>(
    new URL("search.json.zst", eilenbergMacLaneDataRoot),
  );
  const selected = evenlySample(
    rows.filter((row) => Boolean(row.ring_available) && strings(row.named_operations).length > 0),
    12,
  );
  return selected.map((row) => {
    const coefficient = String(row.coefficient ?? "Z");
    const group = displayGroupName(row.group_description ?? row.group_label ?? "A");
    const delooping = number(row.delooping_degree);
    const degrees = asArray(row.generator_degrees).map(number);
    return galleryCard(
      row.path,
      `${row.label} / ${coefficient}`,
      `K(${group}, ${delooping}) · ${degrees.length} generators`,
      algebraPlate(
        `H*(K(${group}, ${delooping}); ${coefficient})`,
        degrees,
        strings(row.named_operations),
      ),
      "eilenberg-maclane",
    );
  });
}

function ensureInitialized(collection: GalleryCollection): Promise<void> {
  const existing = initializations.get(collection);
  if (existing) return existing;
  const initializer: Record<GalleryCollection, () => Promise<void>> = {
    steiner: initializeSteinerGallery,
    poset: initializePosetGallery,
    complex: () => initializeFixedGallery("complex", complexCards),
    "simplicial-set": () => initializeFixedGallery("simplicial-set", simplicialSetCards),
    "group-cohomology": () =>
      initializeFixedGallery("group-cohomology", groupCohomologyCards),
    "eilenberg-maclane": () =>
      initializeFixedGallery("eilenberg-maclane", eilenbergMacLaneCards),
  };
  const pending = initializer[collection]();
  initializations.set(collection, pending);
  return pending;
}

async function loadActiveGallery(): Promise<void> {
  await ensureInitialized(activeCollection);
  if (activeCollection === "steiner") await loadNextSteinerPart();
  else if (activeCollection === "poset") await loadNextPosetBatch();
}

function selectCollection(collection: GalleryCollection): void {
  activeCollection = collection;
  for (const key of galleryOrder) {
    const selected = key === collection;
    const panel = panels.get(key);
    const tab = tabs.get(key);
    if (panel) panel.hidden = !selected;
    if (tab) {
      tab.setAttribute("aria-selected", String(selected));
      tab.tabIndex = selected ? 0 : -1;
    }
  }
  browse.href = browsePaths[collection];
  updateStatus();
  void ensureInitialized(collection);
}

for (const collection of galleryOrder) {
  const tab = tabs.get(collection);
  tab?.addEventListener("click", () => selectCollection(collection));
  tab?.addEventListener("keydown", (event) => {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    event.preventDefault();
    const current = galleryOrder.indexOf(activeCollection);
    const amount = event.key === "ArrowRight" ? 1 : -1;
    const next = galleryOrder[(current + amount + galleryOrder.length) % galleryOrder.length];
    if (!next) return;
    selectCollection(next);
    tabs.get(next)?.focus();
  });
}

more.addEventListener("click", () => void loadActiveGallery());
if ("IntersectionObserver" in window) {
  const observer = new IntersectionObserver(
    (entries) => {
      if (entries.some((entry) => entry.isIntersecting)) void loadActiveGallery();
    },
    { rootMargin: "1200px 0px" },
  );
  observer.observe(stream);
}

void ensureInitialized("steiner");
