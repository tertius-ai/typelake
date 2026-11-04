import { initializePresentation, mountPresentationControls } from "./presentation";
import { complexPlate, simplicialSetPlate } from "./collection-plates";
import { displayGroupName, standardGroupName } from "./group-names";
import "./style.css";
import "./volume-one.css";

type CollectionKind =
  | "SimplicialComplex"
  | "SimplicialSet"
  | "FiniteGroupCohomology"
  | "EilenbergMacLane";

type JsonObject = Record<string, unknown>;

interface Artifact {
  kind: string;
  path: string;
  row_count: number;
  byte_count: number;
  sha256: string;
}

interface LabelBucket {
  vertices?: number;
  total_nondegenerate?: number;
  dimension: number;
  row_count: number;
}

interface Manifest extends JsonObject {
  component: string;
  status: string;
  collection: string;
  outputs: Artifact[];
  inputs: Artifact[];
  checks: Record<string, boolean>;
  label_buckets?: LabelBucket[];
  rows_per_detail_part?: number;
  complex_count?: number;
  simplicial_set_count?: number;
  group_count?: number;
  space_count?: number;
  coefficient_rows?: number;
}

interface SearchRow extends JsonObject {
  label: string;
  path: string;
  dimension?: number;
  coefficient?: string;
  standard_name?: string;
  named_operations?: string[];
  has_torsion?: boolean;
}

interface CollectionConfig {
  kind: CollectionKind;
  title: string;
  singular: string;
  dataRoot: URL;
  sourceDirectory: string;
  compressedSearch: boolean;
  searchFilename: string;
}

const main = required<HTMLElement>("volume-one-main");
const PAGE_SIZE = 30;
const bucketCache = new Map<string, Promise<SearchRow[]>>();
let manifest: Manifest;
let config: CollectionConfig;
let catalogue: SearchRow[] = [];
let filtered: SearchRow[] = [];
let offset = 0;
let completeCatalogueLoaded = false;

initializePresentation();
document.documentElement.dataset.volumeOneReady = "true";

function required<T extends HTMLElement>(id: string): T {
  const element = document.getElementById(id);
  if (!element) throw new Error(`Missing browser element #${id}`);
  return element as T;
}

function escape(value: unknown): string {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
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

function collectionFromRoute(): CollectionKind {
  const segment = window.location.pathname
    .split("/")
    .filter(Boolean)
    .find((part) =>
      [
        "SimplicialComplex",
        "SimplicialSet",
        "FiniteGroupCohomology",
        "EilenbergMacLane",
      ].includes(part),
    );
  return (segment as CollectionKind | undefined) ?? "SimplicialComplex";
}

function configured(kind: CollectionKind): CollectionConfig {
  const definitions: Record<CollectionKind, Omit<CollectionConfig, "dataRoot">> = {
    SimplicialComplex: {
      kind,
      title: "Simplicial complexes",
      singular: "simplicial complex",
      sourceDirectory: "finite-simplicial-complexes",
      compressedSearch: true,
      searchFilename: "search.json.zst",
    },
    SimplicialSet: {
      kind,
      title: "Simplicial Sets",
      singular: "simplicial set",
      sourceDirectory: "finite-simplicial-sets",
      compressedSearch: true,
      searchFilename: "search.json.zst",
    },
    FiniteGroupCohomology: {
      kind,
      title: "Finite-group Cohomology",
      singular: "coefficient ring",
      sourceDirectory: "finite-group-cohomology",
      compressedSearch: false,
      searchFilename: "search.json",
    },
    EilenbergMacLane: {
      kind,
      title: "Eilenberg–Mac Lane Anima",
      singular: "coefficient calculation",
      sourceDirectory: "eilenberg-maclane",
      compressedSearch: false,
      searchFilename: "search.json.zst",
    },
  };
  const roots: Record<CollectionKind, string | undefined> = {
    SimplicialComplex: import.meta.env.VITE_TYPEBASE_SIMPLICIAL_COMPLEX_DATA_URL,
    SimplicialSet: import.meta.env.VITE_TYPEBASE_SIMPLICIAL_SET_DATA_URL,
    FiniteGroupCohomology: import.meta.env.VITE_TYPEBASE_FINITE_GROUP_COHOMOLOGY_DATA_URL,
    EilenbergMacLane: import.meta.env.VITE_TYPEBASE_EILENBERG_MACLANE_DATA_URL,
  };
  const definition = definitions[kind];
  return {
    ...definition,
    dataRoot: new URL(roots[kind] ?? `data/${definition.sourceDirectory}/`, document.baseURI),
  };
}

async function loadJson<T>(path: string, compressed = path.endsWith(".zst")): Promise<T> {
  const response = await fetch(new URL(path, config.dataRoot));
  if (!response.ok) throw new Error(`Data returned HTTP ${response.status}: ${path}`);
  if (!compressed) return (await response.json()) as T;
  const [{ decompress }, bytes] = await Promise.all([
    import("fzstd"),
    response.arrayBuffer(),
  ]);
  return JSON.parse(new TextDecoder().decode(decompress(new Uint8Array(bytes)))) as T;
}

function routeTail(): string[] {
  const segments = window.location.pathname.split("/").filter(Boolean);
  const index = segments.indexOf(config.kind);
  return index < 0 ? [] : segments.slice(index + 1).map(decodeURIComponent);
}

function updateNavigation(): void {
  document.title = `${config.title} — TypeBase Volume I`;
  for (const anchor of document.querySelectorAll<HTMLAnchorElement>("[data-volume-one-nav] a")) {
    const active = new URL(anchor.href).pathname.split("/").filter(Boolean)[0] === config.kind;
    if (active) anchor.setAttribute("aria-current", "page");
    else anchor.removeAttribute("aria-current");
  }
}

function collectionCount(): number {
  if (config.kind === "SimplicialComplex") return manifest.complex_count ?? 0;
  if (config.kind === "SimplicialSet") return manifest.simplicial_set_count ?? 0;
  if (config.kind === "FiniteGroupCohomology") return manifest.coefficient_rows ?? 0;
  return manifest.coefficient_rows ?? 0;
}

function collectionIntro(): string {
  if (config.kind === "SimplicialComplex") {
    return "Finite abstract simplicial complexes up to simplicial isomorphism, with strong cores, collapse witnesses, integral homology, and finite-field cohomology rings.";
  }
  if (config.kind === "SimplicialSet") {
    return "Finite simplicial sets up to isomorphism, with normalized chains, exact homology, cohomology operations, and witnessed horn-collapse relations.";
  }
  if (config.kind === "FiniteGroupCohomology") {
    return "Complete modular presentations and exact integral extents for finite groups, linked to their LMFDB abstract-group records.";
  }
  return "Homology and cohomology of symbolic Eilenberg–Mac Lane anima, with complete presentations where a family theorem is implemented.";
}

function renderCatalogueShell(): void {
  const groupCatalogue = config.kind === "FiniteGroupCohomology";
  const sizeLabel =
    config.kind === "SimplicialComplex"
      ? "Vertices"
      : config.kind === "SimplicialSet"
        ? "Nondegenerate simplices"
        : config.kind === "FiniteGroupCohomology"
          ? "Group order"
          : "Group order";
  const dimensions = [...new Set((manifest.label_buckets ?? []).map((bucket) => bucket.dimension))]
    .sort((left, right) => left - right)
    .map((value) => `<option value="${value}">${value}</option>`)
    .join("");
  const sizes = sizeValues()
    .map((value) => `<option value="${value}">${value}</option>`)
    .join("");
  const primaryFilters =
    config.kind === "FiniteGroupCohomology" || config.kind === "EilenbergMacLane"
      ? `<label><span>Coefficients</span><select name="coefficient"><option value="">All</option><option value="Z">ℤ</option><option value="F2">F₂</option><option value="F3">F₃</option><option value="F5">F₅</option></select></label>
         <label><span>Named operation</span><select name="operation"><option value="">Any</option><option value="Bockstein">Bockstein</option><option value="Tor">Integral Tor</option><option value="Sq">Steenrod square</option><option value="P^">Reduced power</option></select></label>`
      : `<label><span>Integral torsion</span><select name="torsion"><option value="">Either</option><option value="yes">Present</option><option value="no">Absent</option></select></label>`;
  const dimensionFilter =
    config.kind === "FiniteGroupCohomology"
      ? ""
      : `<label><span>${config.kind === "EilenbergMacLane" ? "Delooping" : "Dimension"}</span><select name="dimension"><option value="">All</option>${dimensions || [1, 2, 3, 4].map((value) => `<option value="${value}">${value}</option>`).join("")}</select></label>`;
  main.innerHTML = `
    <section class="volume-one-heading section-heading">
      <div><p class="eyebrow">TypeBase · Volume I</p><h1>${config.title}</h1><p>${collectionIntro()}</p></div>
      <p class="volume-one-count">${collectionCount().toLocaleString()}<span>${escape(config.singular)} records</span></p>
    </section>
    <nav class="database-links" aria-label="About this collection">
      <a href="/${config.kind}/Source">Source</a>
      <a href="/${config.kind}/Completeness">Completeness</a>
      <a href="/${config.kind}/Glossary">Glossary</a>
    </nav>
    <form class="volume-one-filters filters" id="volume-one-filters" role="search">
      <label class="volume-one-label"><span>${groupCatalogue ? "Label or group" : "Label"}</span><input name="label" autocomplete="off" placeholder="${groupCatalogue ? "8.3 or D8" : "Exact or initial label"}" /></label>
      <label><span>${sizeLabel}</span><select name="size"><option value="">All</option>${sizes}</select></label>
      ${dimensionFilter}
      ${primaryFilters}
      <button type="submit">Search</button>
      <details class="volume-one-more">
        <summary>More restrictions</summary>
        <div class="volume-one-more__fields">${advancedFilters()}</div>
      </details>
    </form>
    <section class="volume-one-results" aria-labelledby="volume-one-result-count">
      <div class="volume-one-results__head"><p id="volume-one-result-count"></p><p id="volume-one-result-note" class="muted"></p></div>
      <div id="volume-one-result-list" class="volume-one-list"></div>
      <nav class="volume-one-pages" id="volume-one-pages" aria-label="Search pages">
        <button type="button" id="volume-one-previous">Previous</button><span id="volume-one-page"></span><button type="button" id="volume-one-next">Next</button>
      </nav>
    </section>`;
  required<HTMLFormElement>("volume-one-filters").addEventListener("submit", (event) => {
    event.preventDefault();
    void search(true);
  });
  required<HTMLButtonElement>("volume-one-previous").addEventListener("click", () => {
    offset = Math.max(0, offset - PAGE_SIZE);
    renderResults();
  });
  required<HTMLButtonElement>("volume-one-next").addEventListener("click", () => {
    void nextPage();
  });
}

function advancedFilters(): string {
  if (config.kind === "SimplicialComplex") {
    return `${numberFilter("facets", "Facets")}${numberFilter("euler", "Euler characteristic")}${numberFilter("components", "Components")}
      ${textFilter("face_vector", "Face vector", "6, 9, 4")}${textFilter("core", "Strong-core label", "4.1.2")}${textFilter("automorphisms", "Automorphism order", "12")}
      ${numberFilter("homology_degree", "Homology degree")}${numberFilter("free_rank", "Free rank")}${textFilter("homology_torsion", "Torsion order", "2")}
      <label><span>Core status</span><select name="is_core"><option value="">Either</option><option value="yes">Is a core</option><option value="no">Has dominated vertices</option></select></label>`;
  }
  if (config.kind === "SimplicialSet") {
    return `${textFilter("cell_vector", "Cell vector", "2, 3, 1")}${numberFilter("euler", "Euler characteristic")}${textFilter("automorphisms", "Automorphism order", "6")}
      ${numberFilter("homology_degree", "Homology degree")}${numberFilter("free_rank", "Free rank")}${textFilter("homology_torsion", "Torsion order", "3")}${numberFilter("morse_cells", "Morse-reduced cells")}
      <label><span>Horn collapse</span><select name="horn_collapse"><option value="">Either</option><option value="yes">At least one</option><option value="no">None</option></select></label>
      ${textFilter("component", "Witnessed component", "6.2.17")}`;
  }
  if (config.kind === "FiniteGroupCohomology") {
    return `${numberFilter("small_group_index", "SmallGroups index")}${numberFilter("generator_degree", "Generator degree")}${numberFilter("relation_degree", "Relation degree")}${numberFilter("proof_degree", "Proof through degree")}
      ${numberFilter("homology_degree", "Integral cohomology degree")}${numberFilter("free_rank", "Integral free rank")}${textFilter("homology_torsion", "Integral torsion order", "2")}
      <label><span>Extent</span><select name="extent"><option value="">Complete or bounded</option><option value="complete">Complete</option><option value="bounded">Exact truncation</option></select></label>
      <label><span>Completion method</span><select name="completion_method"><option value="">Any</option><option value="cyclic_periodic_resolution">Cyclic periodic</option><option value="finite_abelian_classifying_space">Finite abelian</option><option value="symonds_parameter_criterion">Symonds parameters</option><option value="sparse_normalized_bar_with_universal_unit_dmt">Sparse normalized bar</option><option value="universal_low_degree_via_abelianization_ext">Universal degree 2</option></select></label>`;
  }
  return `<label><span>Coefficient group</span><select name="group_kind"><option value="">Finite or infinite</option><option value="finite">Finite</option><option value="infinite">Infinite</option></select></label>
    ${textFilter("group", "Group label or form", "8.2 or C4 × C2")}${numberFilter("generator_degree", "Generator degree")}${numberFilter("relation_degree", "Relation degree")}
    <label><span>Presentation extent</span><select name="extent"><option value="">Any</option><option value="complete_closed_form">Complete closed form</option><option value="exact_through_degree">Exact through degree</option><option value="additive_exact_through_degree">Additive only</option><option value="pending">Pending</option></select></label>
    <label><span>Integral torsion</span><select name="integral_torsion"><option value="">Either</option><option value="yes">Present</option><option value="no">Absent</option></select></label>
    <label><span>Additive theory</span><select name="additive_theory"><option value="homology">Homology</option><option value="cohomology">Cohomology</option></select></label>
    ${numberFilter("homology_degree", "Additive degree")}${numberFilter("free_rank", "Free rank")}${textFilter("homology_torsion", "Torsion order", "2")}`;
}

function numberFilter(name: string, label: string): string {
  return `<label><span>${label}</span><input name="${name}" type="number" inputmode="numeric" /></label>`;
}

function textFilter(name: string, label: string, placeholder: string): string {
  return `<label><span>${label}</span><input name="${name}" autocomplete="off" placeholder="${placeholder}" /></label>`;
}

function sizeValues(): number[] {
  if (manifest.label_buckets) {
    return [
      ...new Set(
        manifest.label_buckets.map((bucket) => bucket.vertices ?? bucket.total_nondegenerate ?? 0),
      ),
    ].sort((left, right) => left - right);
  }
  const key = config.kind === "FiniteGroupCohomology" ? "group_order" : "group_order";
  return [...new Set(catalogue.map((row) => number(row[key])).filter((value) => value > 0))].sort(
    (left, right) => left - right,
  );
}

async function loadInitialCatalogue(): Promise<void> {
  catalogue = config.compressedSearch
    ? await loadJson<SearchRow[]>("landing.json", false)
    : await loadJson<SearchRow[]>(config.searchFilename);
  completeCatalogueLoaded = !config.compressedSearch;
  filtered = catalogue;
}

async function search(reset: boolean): Promise<void> {
  if (reset) offset = 0;
  const form = new FormData(required<HTMLFormElement>("volume-one-filters"));
  const exact = String(form.get("label") ?? "").trim();
  if (exact && exactLabel(exact)) {
    const coefficient = String(form.get("coefficient") ?? "");
    window.location.assign(objectPath(exact, coefficient || undefined));
    return;
  }
  if (config.compressedSearch) catalogue = await loadBuckets(form);
  filtered = catalogue.filter((row) => matches(row, form));
  renderResults();
}

function exactLabel(label: string): boolean {
  if (config.kind === "FiniteGroupCohomology") return /^\d+\.\d+$/.test(label);
  if (config.kind === "EilenbergMacLane") return /^(?:Z|\d+(?:\.\d+)+)\.\d+$/.test(label);
  return /^\d+\.-?\d+\.\d+$/.test(label);
}

function objectPath(label: string, coefficient?: string): string {
  const suffix = coefficient ? `/${encodeURIComponent(coefficient)}` : "";
  return `/${config.kind}/${encodeURIComponent(label)}${suffix}`;
}

async function loadBuckets(form: FormData): Promise<SearchRow[]> {
  const label = formString(form, "label");
  const coordinates = /^(\d+)(?:\.(-?\d+))?/.exec(label);
  const size = formString(form, "size") || coordinates?.[1] || "";
  const dimension = formString(form, "dimension") || coordinates?.[2] || "";
  const buckets = (manifest.label_buckets ?? []).filter(
    (bucket) =>
      (!size || String(bucket.vertices ?? bucket.total_nondegenerate) === size) &&
      (!dimension || String(bucket.dimension) === dimension),
  );
  const rows = await Promise.all(
    buckets.map((bucket) => {
      const path =
        config.kind === "SimplicialComplex"
          ? `search/vertices=${bucket.vertices}/dimension=${bucket.dimension}.json.zst`
          : `search/total=${bucket.total_nondegenerate}/dimension=${bucket.dimension}.json.zst`;
      let request = bucketCache.get(path);
      if (!request) {
        request = loadJson<SearchRow[]>(path);
        bucketCache.set(path, request);
      }
      return request;
    }),
  );
  completeCatalogueLoaded = true;
  return rows.flat();
}

function matches(row: SearchRow, form: FormData): boolean {
  const label = formString(form, "label");
  if (label) {
    const matchesLabel = row.label.startsWith(label);
    const normalizedName = displayGroupName(standardGroupName(row.label, row.standard_name))
      .toLowerCase()
      .replaceAll("×", "x");
    const matchesGroupName =
      config.kind === "FiniteGroupCohomology" &&
      normalizedName.includes(label.toLowerCase().replaceAll("×", "x"));
    if (!matchesLabel && !matchesGroupName) return false;
  }
  const size = formString(form, "size");
  const sizeValue = row.vertices ?? row.total_nondegenerate ?? row.group_order;
  if (size && String(sizeValue) !== size) return false;
  const dimension = formString(form, "dimension");
  const dimensionValue = row.dimension ?? row.delooping_degree;
  if (dimension && String(dimensionValue) !== dimension) return false;
  const coefficient = formString(form, "coefficient");
  if (coefficient && row.coefficient !== coefficient) return false;
  const torsion = formString(form, "torsion");
  if (torsion === "yes" && row.has_torsion !== true) return false;
  if (torsion === "no" && row.has_torsion === true) return false;
  const operation = formString(form, "operation");
  if (operation && !(row.named_operations ?? []).some((name) => name.includes(operation))) {
    return false;
  }
  if (!matchesHomology(row, form)) return false;
  if (config.kind === "SimplicialComplex") return matchesComplex(row, form);
  if (config.kind === "SimplicialSet") return matchesSimplicialSet(row, form);
  if (config.kind === "FiniteGroupCohomology") return matchesGroup(row, form);
  return matchesEilenbergMacLane(row, form);
}

function formString(form: FormData, name: string): string {
  return String(form.get(name) ?? "").trim();
}

function exactValue(form: FormData, name: string, value: unknown): boolean {
  const expected = formString(form, name);
  return !expected || String(value) === expected;
}

function vectorValue(value: unknown): string {
  return asArray(value).map(String).join(",");
}

function matchesVector(form: FormData, name: string, value: unknown): boolean {
  const expected = formString(form, name).replaceAll(/[\s()[\]]/g, "");
  return !expected || vectorValue(value) === expected;
}

function matchesDegree(form: FormData, name: string, value: unknown): boolean {
  const expected = formString(form, name);
  return !expected || asArray(value).some((degree) => String(degree) === expected);
}

function matchesHomology(row: SearchRow, form: FormData): boolean {
  const degree = formString(form, "homology_degree");
  const freeRank = formString(form, "free_rank");
  const torsion = formString(form, "homology_torsion");
  if (!degree && !freeRank && !torsion) return true;
  const additiveTheory = formString(form, "additive_theory") || "homology";
  const signature =
    config.kind === "EilenbergMacLane"
      ? additiveTheory === "cohomology"
        ? row.integral_cohomology_signature
        : row.integral_homology_signature
      : config.kind === "FiniteGroupCohomology"
        ? row.integral_cohomology_signature
      : row.homology_signature;
  const groups = String(signature ?? "")
    .split(";")
    .filter(Boolean)
    .map((entry) => entry.split(":"));
  return groups.some(
    ([entryDegree, entryRank, entryTorsion]) =>
      (!degree || entryDegree === degree) &&
      (!freeRank || entryRank === freeRank) &&
      (!torsion || (entryTorsion ?? "").split(",").includes(torsion)),
  );
}

function matchesComplex(row: SearchRow, form: FormData): boolean {
  const coreStatus = formString(form, "is_core");
  return (
    exactValue(form, "facets", row.facet_count) &&
    exactValue(form, "euler", row.euler_characteristic) &&
    exactValue(form, "components", row.component_count) &&
    exactValue(form, "automorphisms", row.automorphism_order) &&
    matchesVector(form, "face_vector", row.f_vector) &&
    (!formString(form, "core") || String(row.core_label).startsWith(formString(form, "core"))) &&
    (!coreStatus || (row.is_core === (coreStatus === "yes")))
  );
}

function matchesSimplicialSet(row: SearchRow, form: FormData): boolean {
  const hornCollapse = formString(form, "horn_collapse");
  const hornCount = number(row.immediate_horn_collapse_count);
  return (
    exactValue(form, "euler", row.euler_characteristic) &&
    exactValue(form, "automorphisms", row.automorphism_order) &&
    exactValue(form, "morse_cells", row.normalized_reduced_cells) &&
    matchesVector(form, "cell_vector", row.nondegenerate_counts) &&
    (!formString(form, "component") ||
      String(row.witnessed_component_label).startsWith(formString(form, "component"))) &&
    (!hornCollapse || (hornCollapse === "yes" ? hornCount > 0 : hornCount === 0))
  );
}

function matchesGroup(row: SearchRow, form: FormData): boolean {
  const extent = formString(form, "extent");
  return (
    exactValue(form, "small_group_index", row.small_groups_index) &&
    exactValue(form, "proof_degree", row.proof_degree) &&
    matchesDegree(form, "generator_degree", row.generator_degrees) &&
    matchesDegree(form, "relation_degree", row.relation_degrees) &&
    (!extent || row.complete === (extent === "complete")) &&
    (!formString(form, "completion_method") ||
      row.completion_method === formString(form, "completion_method"))
  );
}

function matchesEilenbergMacLane(row: SearchRow, form: FormData): boolean {
  const groupKind = formString(form, "group_kind");
  const group = formString(form, "group").toLowerCase();
  const torsion = formString(form, "integral_torsion");
  return (
    (!groupKind || row.finite_group === (groupKind === "finite")) &&
    (!group ||
      String(row.group_label ?? "").toLowerCase().startsWith(group) ||
      String(row.group_description ?? "").toLowerCase().includes(group)) &&
    (!formString(form, "extent") || row.presentation_extent === formString(form, "extent")) &&
    (!torsion || row.has_integral_torsion === (torsion === "yes")) &&
    matchesDegree(form, "generator_degree", row.generator_degrees) &&
    matchesDegree(form, "relation_degree", row.relation_degrees)
  );
}

async function nextPage(): Promise<void> {
  if (!completeCatalogueLoaded && offset + PAGE_SIZE >= filtered.length) {
    const form = new FormData(required<HTMLFormElement>("volume-one-filters"));
    catalogue = await loadBuckets(form);
    filtered = catalogue.filter((row) => matches(row, form));
  }
  offset += PAGE_SIZE;
  renderResults();
}

function renderResults(): void {
  const rows = filtered.slice(offset, offset + PAGE_SIZE);
  const total = completeCatalogueLoaded ? filtered.length : collectionCount();
  required<HTMLElement>("volume-one-result-count").textContent = `${total.toLocaleString()} ${config.singular}${total === 1 ? "" : "s"}`;
  required<HTMLElement>("volume-one-result-note").textContent =
    completeCatalogueLoaded || offset > 0
      ? "Canonical order."
      : `Showing the first ${rows.length}; search shards load only when needed.`;
  required<HTMLElement>("volume-one-result-list").innerHTML = rows.length
    ? rows.map(renderRow).join("")
    : '<p class="volume-one-empty">No records satisfy these restrictions.</p>';
  const pages = required<HTMLElement>("volume-one-pages");
  pages.hidden = total <= PAGE_SIZE;
  required<HTMLButtonElement>("volume-one-previous").disabled = offset === 0;
  required<HTMLButtonElement>("volume-one-next").disabled = offset + PAGE_SIZE >= total;
  required<HTMLElement>("volume-one-page").textContent = `${Math.floor(offset / PAGE_SIZE) + 1} / ${Math.max(1, Math.ceil(total / PAGE_SIZE)).toLocaleString()}`;
}

function renderRow(row: SearchRow): string {
  let facts: Array<[string, unknown]>;
  if (config.kind === "SimplicialComplex") {
    facts = [
      ["dim", row.dimension],
      ["facets", row.facet_count],
      ["f", `(${asArray(row.f_vector).join(", ")})`],
      ["core", row.core_label],
      ["Aut", row.automorphism_order],
    ];
  } else if (config.kind === "SimplicialSet") {
    facts = [
      ["dim", row.dimension],
      ["cells", `(${asArray(row.nondegenerate_counts).join(", ")})`],
      ["Morse", `${row.normalized_source_cells}→${row.normalized_reduced_cells}`],
      ["horns", row.immediate_horn_collapse_count],
      ["component", row.witnessed_component_size],
    ];
  } else if (config.kind === "FiniteGroupCohomology") {
    facts = [
      ["group", displayGroupName(standardGroupName(row.label, row.standard_name))],
      ["coeff", row.coefficient],
      ["generators", row.generator_count],
      ["relations", row.relation_count],
      ["method", row.completion_method],
      ["extent", row.complete ? "complete" : `exact ≤ ${row.proof_degree ?? "bound"}`],
    ];
  } else {
    facts = [
      ["coeff", row.coefficient],
      ["group", row.group_description],
      ["delooping", row.delooping_degree],
      ["generators", row.generator_count],
      ["extent", row.presentation_extent],
    ];
  }
  return `<article class="volume-one-row">
    <a href="${escape(row.path)}" class="volume-one-row__label">${escape(row.label)}<span>↗</span></a>
    <dl>${facts.map(([name, value]) => `<div><dt>${escape(name)}</dt><dd>${escape(value ?? "—")}</dd></div>`).join("")}</dl>
  </article>`;
}

async function openObject(tail: string[]): Promise<void> {
  const label = tail[0];
  if (!label || !exactLabel(label)) throw new Error("That is not a label in this collection");
  const coefficient = tail[1];
  const detail = await loadDetail(label);
  document.title = `${label} — ${config.title} — TypeBase`;
  main.innerHTML = renderDetail(label, coefficient, detail);
}

async function loadDetail(label: string): Promise<JsonObject> {
  if (config.kind === "FiniteGroupCohomology" || config.kind === "EilenbergMacLane") {
    return loadJson<JsonObject>(`details/${label}.json.zst`);
  }
  const match = /^(\d+)\.(-?\d+)\.(\d+)$/.exec(label);
  if (!match) throw new Error("Malformed catalogue label");
  const size = Number(match[1]);
  const dimension = Number(match[2]);
  const index = Number(match[3]);
  const part = Math.floor((index - 1) / number(manifest.rows_per_detail_part));
  const prefix = config.kind === "SimplicialComplex" ? `vertices=${size}` : `total=${size}`;
  const path = `details/${prefix}/dimension=${dimension}/part-${String(part).padStart(5, "0")}.json.zst`;
  const details = await loadJson<JsonObject[]>(path);
  const row = details[(index - 1) % number(manifest.rows_per_detail_part)];
  if (!row || row.label !== label) throw new Error(`The detail projection omits ${label}`);
  return row;
}

function detailHeading(label: string, subtitle: string): string {
  return `<header class="volume-one-record__head"><div><p class="eyebrow">${escape(config.title)}</p><h1 class="record-title">${escape(label)}</h1><p>${subtitle}</p></div><a href="/${config.kind}/">Back to catalogue</a></header>`;
}

function renderDetail(label: string, coefficient: string | undefined, detail: JsonObject): string {
  if (config.kind === "SimplicialComplex") return renderComplexDetail(label, detail);
  if (config.kind === "SimplicialSet") return renderSimplicialSetDetail(label, detail);
  if (config.kind === "FiniteGroupCohomology") return renderGroupDetail(label, coefficient, detail);
  return renderEilenbergMacLaneDetail(label, coefficient, detail);
}

function renderComplexDetail(label: string, detail: JsonObject): string {
  const identity = asObject(detail.identity);
  const topology = asObject(detail.topology);
  const collapseSteps = asArray(asObject(topology.greedy_collapse).steps).length;
  const facets = asArray(identity.facets).map((facet) => asArray(facet).map(number));
  return `<article class="volume-one-record">
    ${detailHeading(label, `${number(identity.vertices)} vertices · dimension ${number(identity.dimension)}`)}
    <div class="volume-one-record__lead"><figure>${complexPlate(number(identity.vertices), facets)}<figcaption>Canonical facet-incidence plate</figcaption></figure>
    ${factTable([
      ["Facets", facets.length], ["f-vector", `(${asArray(topology.f_vector).join(", ")})`],
      ["Euler characteristic", topology.euler_characteristic], ["Components", asArray(topology.component_sizes).length],
      ["Automorphisms", asObject(identity.automorphism_group).order], ["Strong core", detail.core_label],
      ["Greedy collapse", detail.collapse_target_label],
    ])}</div>
    ${section("Facets", `<p class="math-list">${facets.map((facet) => `{${facet.join(", ")}}`).join(" · ") || "∅"}</p>`)}
    ${section("Certified reductions", `<div class="relation-register"><a href="/SimplicialComplex/${escape(detail.core_label)}">${escape(detail.core_label)} <span>strong core</span></a><a href="/SimplicialComplex/${escape(detail.collapse_target_label)}">${escape(detail.collapse_target_label)} <span>${collapseSteps} witnessed collapse steps</span></a></div>`)}
    ${section("Integral homology", homologyTable(asArray(topology.integral_homology)))}
    ${cohomologySection("F₂", asObject(detail.cohomology_f2))}
    ${cohomologySection("F₃", asObject(detail.cohomology_f3))}
    ${rawEvidence(detail)}
  </article>`;
}

function renderSimplicialSetDetail(label: string, detail: JsonObject): string {
  const identity = asObject(detail.identity);
  const topology = asObject(detail.topology);
  const key = asObject(identity.key);
  const faceData = asArray(key.nondegenerate_faces);
  const relations = asArray(detail.elementary_horn_relations);
  return `<article class="volume-one-record">
    ${detailHeading(label, `${number(identity.total_nondegenerate)} nondegenerate simplices · dimension ${number(identity.dimension)}`)}
    <div class="volume-one-record__lead"><figure>${simplicialSetPlate(faceData)}<figcaption>Nondegenerate cells and face incidence</figcaption></figure>
    ${factTable([
      ["Cells by degree", `(${asArray(identity.nondegenerate_counts).join(", ")})`],
      ["Euler characteristic", topology.euler_characteristic],
      ["Automorphisms", asObject(identity.automorphism_group).order],
      ["Normalized cells", asObject(topology.normalized_chain_reduction).source_cells],
      ["Morse-reduced cells", asObject(topology.normalized_chain_reduction).reduced_cells],
      ["Immediate horn collapses", relations.length],
      ["Greedy target", detail.greedy_target_label],
    ])}</div>
    ${section("Integral homology", homologyTable(asArray(topology.integral_homology)))}
    ${section("Faces and degeneracies", faceRegister(faceData))}
    ${section("Horn-collapse relations", `<div class="relation-register"><a href="/SimplicialSet/${escape(detail.greedy_target_label)}">${escape(detail.greedy_target_label)} <span>witnessed greedy endpoint</span></a>${relations.map((entry) => { const record = asObject(entry); return `<a href="/SimplicialSet/${escape(record.target_label)}">${escape(record.target_label)} <span>elementary horn collapse</span></a>`; }).join("")}</div>${relations.length ? "" : '<p class="muted">No immediate elementary horn collapse.</p>'}`)}
    ${cohomologySection("F₂", asObject(detail.cohomology_f2))}
    ${cohomologySection("F₃", asObject(detail.cohomology_f3))}
    ${operationsSection(asObject(detail.steenrod_operations_f3), asObject(detail.cohomology_f3))}
    ${rawEvidence(detail)}
  </article>`;
}

function renderGroupDetail(label: string, coefficient: string | undefined, detail: JsonObject): string {
  const modular = asArray(detail.modular_cohomology).map(asObject);
  const integralCoverage = asObject(detail.integral_cohomology);
  const coefficients = [
    ...(integralCoverage.status === "available" ? ["Z"] : []),
    ...modular.map((record) => `F${asObject(record.coefficient_context).characteristic}`),
  ];
  const selected = coefficient ?? coefficients[0];
  const tabs = [...new Set([selected, ...coefficients].filter(Boolean))];
  const source = asObject(detail.source);
  const groupName = displayGroupName(
    standardGroupName(label, detail.standard_name ?? source.standard_name),
  );
  let content: string;
  if (selected === "Z") {
    content =
      integralCoverage.status === "available"
        ? integralRing(asObject(integralCoverage.record))
        : pendingPanel(
            String(
              integralCoverage.detail ??
                "No theorem-complete integral presentation is available for this group.",
            ),
          );
  } else {
    const prime = selected?.slice(1);
    const record = modular.find(
      (candidate) => String(asObject(candidate.coefficient_context).characteristic) === prime,
    );
    content = record ? modularRing(record) : '<p class="muted">No selected coefficient record.</p>';
  }
  return `<article class="volume-one-record">
    ${detailHeading(label, `${escape(groupName)} · SmallGroup(${escape(source.small_groups_order)}, ${escape(source.small_groups_index)})`)}
    <div class="coefficient-tabs">${tabs.map((item) => `<a ${item === selected ? 'aria-current="page"' : ""} href="/FiniteGroupCohomology/${escape(label)}/${item}">${item === "Z" ? "ℤ" : escape(item)}</a>`).join("")}</div>
    <div class="volume-one-record__lead volume-one-record__lead--algebra">${factTable([
      ["Standard name", groupName], ["Order", source.small_groups_order], ["SmallGroups index", source.small_groups_index],
      ["Permutation degree", source.permutation_degree], ["Canonical SHA-256", shortHash(detail.canonical_group_sha256)],
    ])}<p><a href="${escape(detail.lmfdb_url)}" target="_blank" rel="noreferrer">LMFDB ${escape(label)} ↗</a></p></div>
    ${content}
    ${rawEvidence(detail)}
  </article>`;
}

function renderEilenbergMacLaneDetail(
  label: string,
  coefficient: string | undefined,
  detail: JsonObject,
): string {
  const record = asObject(detail.record);
  const group = asObject(record.group);
  const groupName = displayGroupName(group.structure_description);
  const modular = asArray(record.modular_cohomology).map(asObject);
  const coefficients = ["Z", ...modular.map((item) => `F${item.prime}`)];
  const selected = coefficient ?? "Z";
  const tabs = [...new Set([selected, ...coefficients])];
  let content: string;
  if (selected === "Z") {
    const ring = asObject(record.integral_cohomology_ring);
    content =
      ring.status === "available"
        ? integralPresentation(asObject(ring.value))
        : pendingPanel("No complete integral ring presentation is currently implemented for this family.");
    content += coveragePanel("Integral homology", asObject(record.integral_homology));
    content += coveragePanel("Integral cohomology", asObject(record.integral_cohomology));
  } else {
    const row = modular.find((item) => `F${item.prime}` === selected);
    const coverage = asObject(row?.coverage);
    content = coverage.status === "available" ? emlModularRing(asObject(coverage.value)) : pendingPanel("This modular calculation is pending.");
  }
  return `<article class="volume-one-record">
    ${detailHeading(label, `K(${escape(groupName)}, ${escape(record.delooping_degree)})`)}
    <div class="coefficient-tabs">${tabs.map((item) => `<a ${item === selected ? 'aria-current="page"' : ""} href="/EilenbergMacLane/${escape(label)}/${item}">${item === "Z" ? "ℤ" : escape(item)}</a>`).join("")}</div>
    <div class="volume-one-record__lead volume-one-record__lead--algebra">${factTable([
      ["Coefficient group", groupName], ["Delooping", record.delooping_degree],
      ["Finite order", group.order ?? "infinite"], ["Canonical SHA-256", shortHash(record.space_canonical_sha256)],
    ])}${group.lmfdb_url ? `<p><a href="${escape(group.lmfdb_url)}" target="_blank" rel="noreferrer">LMFDB ${escape(group.lmfdb_label)} ↗</a></p>` : ""}</div>
    ${content}
    ${rawEvidence(detail)}
  </article>`;
}

function integralRing(record: JsonObject): string {
  const extent = asObject(record.extent);
  if (extent.kind === "exact_through_degree") {
    const certificate = asObject(extent.certificate);
    const sparseBar = certificate.kind === "sparse_normalized_bar";
    return `${section(`Exact integral cohomology through degree ${escape(extent.degree)}`, factTable([
      ["Method", certificate.method],
      [
        sparseBar ? "Census sparse-work budget" : "Abelianization",
        sparseBar
          ? certificate.census_sparse_entry_limit
          : asArray(certificate.abelianization_invariant_factors).join(" × ") || "trivial",
      ],
      ["No claim above degree", extent.no_claim_above_degree === true ? "yes" : extent.no_claim_above_degree],
      ["Verified", certificate.verified === true ? "yes" : certificate.verified],
      ["Result SHA-256", shortHash(certificate.noether_result_sha256)],
    ]))}${section("Additive groups", homologyTable(asArray(record.additive_groups_through_degree), "H^"))}${sparseBar ? integralTruncation(asObject(record.truncation)) : ""}`;
  }
  return integralPresentation({
    ...asObject(record.presentation),
    additive_groups: record.additive_groups_through_degree,
    certificate: asObject(asObject(record.extent).certificate),
  });
}

function integralTruncation(truncation: JsonObject): string {
  const basis = asArray(truncation.basis).map(asObject);
  const products = asArray(truncation.products).map(asObject);
  const basisRows = basis.length
    ? basis.map((entry) => {
        const cocycle = asArray(entry.canonical_model_cocycle).map(asObject);
        const support = cocycle
          .map((term) => `${escape(term.coefficient)}·e<sub>${escape(term.coordinate)}</sub>`)
          .join(" + ") || "0";
        return `<tr><td>${mathematicalName(entry.id)}</td><td>${escape(entry.degree)}</td><td>${entry.order === null || entry.order === undefined ? "∞" : escape(entry.order)}</td><td><details><summary>${cocycle.length.toLocaleString()} nonzero coordinates</summary><span class="math-list">${support}</span></details></td></tr>`;
      }).join("")
    : '<tr><td colspan="4">No basis classes</td></tr>';
  const productRows = products.length
    ? products.map((entry) => {
        const terms = asArray(entry.target_terms).map(asObject);
        const target = terms.length
          ? terms.map((term) => `${escape(term.coefficient)}·${mathematicalName(term.basis_id)}`).join(" + ")
          : "0";
        return `<tr><td>${mathematicalName(entry.left_id)}</td><td>${mathematicalName(entry.right_id)}</td><td>${target}</td></tr>`;
      }).join("")
    : '<tr><td colspan="3">No products in the selected range</td></tr>';
  return `${section("Canonical normalized-bar basis", `<p>${basis.length.toLocaleString()} invariant-factor basis classes. Cocycle coordinates use the normalized bar basis fixed by the record contract.</p><div class="artifact-table"><table class="volume-one-table"><thead><tr><th>Class</th><th>Degree</th><th>Additive order</th><th>Cocycle</th></tr></thead><tbody>${basisRows}</tbody></table></div>`)}${section("Cup products in range", `<p>Every ordered pair whose degree sum is in range is listed; an empty target is the zero class.</p><div class="artifact-table"><table class="volume-one-table"><thead><tr><th>Left</th><th>Right</th><th>Product</th></tr></thead><tbody>${productRows}</tbody></table></div>`)}`;
}

function integralPresentation(presentation: JsonObject): string {
  const generators = asArray(presentation.generators).map(asObject);
  const relations = asArray(presentation.relations).map(asObject);
  return `${section("Complete graded-commutative ring", presentationBlock("ℤ", generators, relations))}
    ${section("Additive groups", homologyTable(asArray(presentation.additive_groups), "H"))}
    ${certificatePanel(asObject(presentation.certificate), presentation)}`;
}

function modularRing(record: JsonObject): string {
  const computation = asObject(record.computation);
  if (computation.kind === "finite_abelian_classifying_space") {
    const certificate = asObject(computation.certificate);
    const presentation = asObject(certificate.presentation);
    const generators = asArray(certificate.generators).map(asObject);
    return `${section("Complete graded-commutative ring", presentationBlock(`F${asObject(record.coefficient_context).characteristic}`, generators, asArray(presentation.relations).map(asObject)))}${fieldDimensions(asArray(certificate.hilbert_function))}${operationGenealogy(generators, asArray(certificate.operations))}${certificatePanel(certificate)}`;
  }
  const algebra = asObject(computation.algebra);
  const generators = asArray(algebra.generators).map(asObject);
  return `${section("Complete graded-commutative ring", presentationBlock(`F${asObject(record.coefficient_context).characteristic}`, generators, asArray(algebra.relations).map(asObject)))}${fieldDimensions(cohomologyDimensions(asArray(computation.basis)))}${operationGenealogy(generators, generatorWitnessOperations(generators))}${certificatePanel(asObject(asObject(computation.extent).certificate), asObject(computation.resolution))}`;
}

function emlModularRing(value: JsonObject): string {
  const generators = asArray(value.generators).map(asObject);
  return `${section("Graded-commutative ring", presentationBlock(String(value.coefficient), generators, asArray(asObject(value.presentation).relations).map(asObject)))}${fieldDimensions(asArray(value.hilbert_function))}${operationGenealogy(generators, asArray(value.operations))}${certificatePanel(value)}`;
}

function mathematicalName(raw: unknown): string {
  return escape(raw)
    .replaceAll(/\biota_(\d+)(?:(?:\.(\d+))|\^\((\d+)\))?/g, (
      _match,
      degree: string,
      dottedComponent?: string,
      indexedComponent?: string,
    ) =>
      `ι<sub>${degree}${dottedComponent || indexedComponent ? `,${dottedComponent || indexedComponent}` : ""}</sub>`,
    )
    .replaceAll(/\bSq\^(\d+)/g, "Sq<sup>$1</sup>")
    .replaceAll(/\bP\^(\d+)/g, "P<sup>$1</sup>")
    .replaceAll(/\bbeta_(\d+)/g, "β<sub>$1</sub>")
    .replaceAll(/\bbeta\b/g, "β")
    .replaceAll(/\bTor_Z\b/g, "Tor<sub>ℤ</sub>")
    .replaceAll(/\bx_(\d+)(?:\.(\d+))?/g, (_match, degree: string, index?: string) =>
      `x<sub>${degree}${index ? `,${index}` : ""}</sub>`,
    )
    .replaceAll(/\bh(\d+)[_.](\d+)\b/g, "h<sub>$1,$2</sub>");
}

function coefficientMarkup(coefficient: string): string {
  const finiteField = /^F(\d+)$/.exec(coefficient);
  return finiteField ? `𝔽<sub>${finiteField[1]}</sub>` : escape(coefficient);
}

function generatorOrigin(generator: JsonObject): string {
  if (generator.display_origin) return String(generator.display_origin);
  const witness = asObject(generator.operation_witness);
  const operation = asObject(witness.operation);
  if (operation.display_name) {
    return `${String(operation.display_name)} of generator ${number(witness.source_generator_index) + 1}, modulo decomposables`;
  }
  const origin = asObject(generator.origin);
  const originOperation = asObject(origin.operation);
  if (originOperation.display_name) {
    return `${String(originOperation.display_name)} on component ${escape(origin.component)}`;
  }
  const kind = String(origin.kind ?? "");
  const squares = asArray(origin.sequence).map(number);
  if (squares.length) return `admissible Sq sequence (${squares.join(", ")})`;
  const powers = asArray(origin.reduced_powers).map(number);
  const bocksteins = asArray(origin.bocksteins).map(Boolean);
  if (powers.length || bocksteins.some(Boolean)) {
    const word = powers.map((power, index) => `${bocksteins[index] ? "β " : ""}P^${power}`);
    if (bocksteins.at(-1)) word.push("β");
    return `admissible odd-primary sequence (${word.join(" ")})`;
  }
  return kind.replaceAll("_", " ");
}

function generatorWitnessOperations(generators: JsonObject[]): JsonObject[] {
  return generators.flatMap((generator, targetIndex) => {
    const witness = asObject(generator.operation_witness);
    const operation = asObject(witness.operation);
    if (!operation.display_name) return [];
    return [{
      operation: operation.display_name,
      source_generator: witness.source_generator_index,
      target_generator: generator.display_name ?? generator.name ?? `g${targetIndex + 1}`,
    }];
  });
}

function presentationBlock(
  coefficient: string,
  generators: JsonObject[],
  relations: JsonObject[],
): string {
  const canonicalNames = generators.map((generator, index) =>
    String(generator.id ?? generator.name ?? `g${index + 1}`),
  );
  const displayNames = generators.map((generator, index) =>
    String(generator.display_name ?? generator.basis_id ?? canonicalNames[index]),
  );
  const generatorRows = generators.length
    ? generators
        .map(
          (generator, index) =>
            `<tr><td>${escape(canonicalNames[index])}</td><td>${escape(generator.degree)}</td><td>${mathematicalName(displayNames[index])}</td><td>${mathematicalName(generatorOrigin(generator))}</td></tr>`,
        )
        .join("")
    : '<tr><td colspan="4">No algebra generators</td></tr>';
  const polynomialRing = displayNames.length
    ? `${coefficientMarkup(coefficient)}[${displayNames.map(mathematicalName).join(", ")}]`
    : coefficientMarkup(coefficient);
  const quotient = relations.length
    ? ` / (${relations.map((relation) => polynomialRelation(relation, displayNames)).join("<br>")})`
    : "";
  return `<p class="ring-display">${polynomialRing}${quotient}</p>
    <table class="volume-one-table"><thead><tr><th>Record ID</th><th>Degree</th><th>Mathematical name</th><th>Origin</th></tr></thead><tbody>${generatorRows}</tbody></table>`;
}

function polynomialRelation(relation: JsonObject, names: string[]): string {
  const terms = asArray(relation.terms).map(asObject);
  if (!terms.length && relation.degree !== undefined) return `relation in degree ${escape(relation.degree)}`;
  return polynomialExpression(relation, names);
}

function polynomialExpression(polynomial: JsonObject, names: string[]): string {
  const terms = asArray(polynomial.terms).map(asObject);
  return terms
    .map((term, termIndex) => {
      const coefficient = coefficientText(term.coefficient);
      const monomial = asArray(term.generator_exponents ?? term.exponents)
        .map(number)
        .map((exponent, index) => {
          if (exponent === 0) return "";
          return `${mathematicalName(names[index] ?? `g${index + 1}`)}${exponent === 1 ? "" : `<sup>${exponent}</sup>`}`;
        })
        .join("");
      const sign = coefficient.startsWith("-") || termIndex === 0 ? "" : "+";
      return `${sign}${escape(coefficient === "1" && monomial ? "" : coefficient)}${monomial || "1"}`;
    })
    .join(" ") || "0";
}

function coefficientText(raw: unknown): string {
  if (Array.isArray(raw)) return raw.join(",");
  const typedCoefficients = asArray(asObject(raw).coefficients);
  return typedCoefficients.length ? typedCoefficients.join(",") : String(raw ?? "1");
}

function operationGenealogy(generators: JsonObject[], operations: unknown[]): string {
  if (!generators.length && !operations.length) return "";
  const canonicalNames = generators.map((generator, index) =>
    String(generator.id ?? generator.name ?? `g${index + 1}`),
  );
  const displayNames = generators.map((generator, index) =>
    String(generator.display_name ?? generator.basis_id ?? canonicalNames[index]),
  );
  const nodes = generators
    .map(
      (generator, index) =>
        `<div class="genealogy-node"><strong>${mathematicalName(displayNames[index])}</strong><span>${escape(canonicalNames[index])} · degree ${escape(generator.degree)} · ${escape(generatorOrigin(generator))}</span></div>`,
    )
    .join("");
  const arrows = operations.length
    ? `<ol class="genealogy-arrows">${operations.map((value) => {
        const operation = asObject(value);
        const rawSource = operation.source_generator ?? operation.source_id;
        const source = typeof rawSource === "number" ? displayNames[rawSource] : String(rawSource ?? "—");
        const target = operationTarget(operation, displayNames);
        return `<li><span>${mathematicalName(source)}</span><strong>${mathematicalName(cohomologyOperationName(operation.operation))}</strong><span>${target}</span></li>`;
      }).join("")}</ol>`
    : "";
  return section(
    "Steenrod-operation genealogy",
    `<div class="operation-genealogy"><div class="genealogy-nodes">${nodes}</div>${arrows}</div>`,
  );
}

function operationTarget(operation: JsonObject, names: string[]): string {
  const target = asObject(operation.target ?? operation.target_normal_form);
  if (asArray(target.terms).length) return polynomialExpression(target, names);
  const terms = asArray(operation.target_terms).map(asObject);
  if (terms.length) {
    return terms
      .map((term) => `${escape(coefficientText(term.coefficient))}${mathematicalName(term.basis_id ?? "")}`)
      .join(" + ");
  }
  return mathematicalName(operation.target_generator ?? operation.target_id ?? "0");
}

function cohomologyDimensions(basis: unknown[]): unknown[] {
  const dimensions = new Map<number, number>();
  for (const value of basis) {
    const degree = number(asObject(value).degree);
    dimensions.set(degree, (dimensions.get(degree) ?? 0) + 1);
  }
  const maximum = Math.max(-1, ...dimensions.keys());
  return Array.from({ length: maximum + 1 }, (_, degree) => dimensions.get(degree) ?? 0);
}

function fieldDimensions(dimensions: unknown[]): string {
  if (!dimensions.length) return "";
  return section(
    "Additive dimensions",
    `<table class="volume-one-table dimension-table"><thead><tr><th>Degree</th>${dimensions.map((_value, degree) => `<th>${degree}</th>`).join("")}</tr></thead><tbody><tr><th>Dimension</th>${dimensions.map((value) => `<td>${escape(value)}</td>`).join("")}</tr></tbody></table>`,
  );
}

function operationList(operations: unknown[], names = new Map<string, string>()): string {
  const meaningful = operations
    .map(asObject)
    .filter((operation) => !isIdentityOperation(operation.operation));
  if (!meaningful.length) return "";
  return `<ul class="operation-list">${meaningful.map((operation) => {
      const sourceId = String(operation.source_generator ?? operation.source_id ?? "");
      const source = names.get(sourceId) ?? sourceId;
      const target = asArray(operation.target_terms).length
        ? asArray(operation.target_terms).map(asObject).map((term) => {
            const coefficient = coefficientText(term.coefficient);
            const basisId = String(term.basis_id ?? "");
            return `${coefficient === "1" ? "" : escape(coefficient)}${mathematicalName(names.get(basisId) ?? basisId)}`;
          }).join(" + ") || "0"
        : operationTarget(operation, [...names.values()]);
      return `<li><span>${mathematicalName(source)}</span><strong>${mathematicalName(cohomologyOperationName(operation.operation))}</strong><span>${target}</span></li>`;
    }).join("")}</ul>`;
}

function certificatePanel(certificate: JsonObject, fallback: JsonObject = {}): string {
  const source = Object.keys(certificate).length ? certificate : fallback;
  if (!Object.keys(source).length) return "";
  return section(
    "Completion certificate",
    factTable([
      ["Method", source.method ?? source.kind ?? source.strategy],
      ["Invariant factors", asArray(source.invariant_factors).join(" × ") || undefined],
      ["Theorem", source.theorem_reference],
      ["Verified", source.verified === true ? "yes" : source.verified],
      ["Result SHA-256", shortHash(source.noether_result_sha256)],
    ].filter((entry) => entry[1] !== undefined) as Array<[string, unknown]>),
  );
}

function coveragePanel(title: string, coverage: JsonObject): string {
  if (coverage.status !== "available") return pendingPanel(`${title} is outside the implemented family.`);
  const value = asObject(coverage.value);
  return section(title, `${homologyTable(asArray(value.groups), title.startsWith("Integral homology") ? "H" : "H^")}${certificatePanel(value)}`);
}

function pendingPanel(text: string): string {
  return `<section class="volume-one-section volume-one-pending"><p class="eyebrow">Explicitly pending</p><p>${escape(text)}</p></section>`;
}

function isIdentityOperation(value: unknown): boolean {
  const name = cohomologyOperationName(value);
  return name === "Sq^0" || name === "P^0";
}

function cohomologyOperationName(value: unknown): string {
  const operation = asObject(value);
  return String(operation.display_name ?? value ?? "");
}

function unitCoefficient(raw: unknown): boolean {
  const values = Array.isArray(raw) ? raw : asArray(asObject(raw).coefficients);
  return values.length === 1 && String(values[0]) === "1";
}

function singleOperationTarget(operation: JsonObject): string | null {
  const terms = asArray(operation.target_terms).map(asObject);
  if (terms.length !== 1 || !unitCoefficient(terms[0]?.coefficient)) return null;
  return String(terms[0]?.basis_id ?? "") || null;
}

function namedCohomologyGenerators(
  generators: JsonObject[],
  operations: JsonObject[],
): JsonObject[] {
  const named = generators.map((generator) => ({ ...generator }));
  const byId = new Map<string, JsonObject>();
  for (const generator of named) {
    for (const value of [generator.id, generator.name, generator.basis_id]) {
      if (value !== undefined) byId.set(String(value), generator);
    }
  }
  const meaningful = operations.filter(
    (operation) => !isIdentityOperation(operation.operation) && singleOperationTarget(operation),
  );
  const targets = new Set(meaningful.map(singleOperationTarget).filter(Boolean));
  const degreeIndices = new Map<number, number>();
  for (const generator of named) {
    const id = String(generator.id ?? generator.name ?? generator.basis_id ?? "");
    if (targets.has(id)) continue;
    const degree = number(generator.degree);
    const index = (degreeIndices.get(degree) ?? 0) + 1;
    degreeIndices.set(degree, index);
    generator.display_name = `x_${degree}.${index}`;
    generator.display_origin = "chosen algebra generator";
  }
  for (const operation of meaningful) {
    const source = byId.get(String(operation.source_id ?? ""));
    const target = byId.get(singleOperationTarget(operation) ?? "");
    if (!source?.display_name || !target || target.display_name) continue;
    const operationName = cohomologyOperationName(operation.operation);
    target.display_name = `${operationName}(${String(source.display_name)})`;
    target.display_origin = `admissible ${operationName} of ${String(source.display_name)}`;
  }
  for (const generator of named) {
    if (generator.display_name) continue;
    const degree = number(generator.degree);
    const index = (degreeIndices.get(degree) ?? 0) + 1;
    degreeIndices.set(degree, index);
    generator.display_name = `x_${degree}.${index}`;
    generator.display_origin = "chosen algebra generator";
  }
  return named;
}

function cohomologyNameMap(generators: JsonObject[]): Map<string, string> {
  const names = new Map<string, string>();
  for (const generator of generators) {
    const display = String(generator.display_name ?? generator.name ?? generator.id ?? "");
    for (const value of [generator.id, generator.name, generator.basis_id]) {
      if (value !== undefined) names.set(String(value), display);
    }
  }
  return names;
}

function cohomologySection(field: string, record: JsonObject): string {
  const basis = asArray(record.basis).map(asObject);
  const operations = asArray(record.operations).map(asObject);
  const meaningfulOperations = operations.filter(
    (operation) => !isIdentityOperation(operation.operation),
  );
  let presentation = "";
  let names = new Map<string, string>();
  if (typeof record.connected_presentation_json === "string") {
    try {
      const parsed = asObject(JSON.parse(record.connected_presentation_json));
      const ring = asObject(parsed.ring);
      const generators = namedCohomologyGenerators(
        asArray(ring.generators).map(asObject),
        operations,
      );
      names = cohomologyNameMap(generators);
      presentation = presentationBlock(
        field,
        generators,
        asArray(parsed.relations).map(asObject),
      );
    } catch {
      presentation = '<p class="muted">The stored connected presentation could not be rendered.</p>';
    }
  }
  return section(
    `Cohomology over ${field}`,
    `${presentation}<p>${basis.length} canonical basis elements · ${asArray(record.products).length} stored products · ${meaningfulOperations.length} nonidentity Steenrod values</p>${operationList(meaningfulOperations, names)}`,
  );
}

function operationsSection(record: JsonObject, cohomology: JsonObject): string {
  let generators: JsonObject[] = [];
  if (typeof cohomology.connected_presentation_json === "string") {
    try {
      const parsed = asObject(JSON.parse(cohomology.connected_presentation_json));
      generators = namedCohomologyGenerators(
        asArray(asObject(parsed.ring).generators).map(asObject),
        asArray(record.operations).map(asObject),
      );
    } catch {
      generators = [];
    }
  }
  const values = operationList(asArray(record.operations), cohomologyNameMap(generators));
  return values ? section("Odd-primary Steenrod-operation values", values) : "";
}

function homologyTable(groups: unknown[], symbol = "H"): string {
  if (!groups.length) return '<p class="muted">No nonzero groups in the recorded range.</p>';
  return `<table class="volume-one-table"><thead><tr><th>Degree</th><th>Group</th></tr></thead><tbody>${groups
    .map((value) => {
      const group = asObject(value);
      const parts: string[] = [];
      const rank = number(group.free_rank);
      if (rank === 1) parts.push("ℤ");
      if (rank > 1) parts.push(`ℤ<sup>${rank}</sup>`);
      for (const factor of asArray(group.torsion_coefficients)) parts.push(`ℤ/${escape(factor)}`);
      return `<tr><td>${escape(symbol)}<sub>${escape(group.degree)}</sub></td><td>${parts.join(" ⊕ ") || "0"}</td></tr>`;
    })
    .join("")}</tbody></table>`;
}

function faceRegister(faceData: unknown[]): string {
  const rows: string[] = [];
  faceData.map(asArray).forEach((simplices, degree) => {
    if (degree === 0) return;
    simplices.forEach((faces, simplex) => {
      rows.push(`<tr><th>σ<sub>${degree},${simplex}</sub></th><td>${asArray(faces)
        .map(
          (value, face) =>
            `<span>d<sub>${face}</sub> = ${faceDescriptor(asObject(value), degree - 1)}</span>`,
        )
        .join("")}</td></tr>`);
    });
  });
  return rows.length
    ? `<table class="volume-one-table face-register"><thead><tr><th>Simplex</th><th>Canonical faces</th></tr></thead><tbody>${rows.join("")}</tbody></table>`
    : '<p class="muted">There are no positive-dimensional nondegenerate simplices.</p>';
}

function faceDescriptor(face: JsonObject, faceDegree: number): string {
  if (face.kind === "cell") {
    return `σ<sub>${faceDegree},${escape(face.simplex)}</sub>`;
  }
  const surjection = asArray(face.surjection).map(number);
  return `<span class="degeneracy-word" title="canonical monotone surjection [${surjection.join(", ")}]">s<sub>[${surjection.join("")}]</sub>(σ<sub>${escape(face.simplex_dimension)},${escape(face.simplex)}</sub>)</span>`;
}

function factTable(entries: Array<[string, unknown]>): string {
  return `<dl class="volume-one-facts">${entries.map(([name, value]) => `<div><dt>${escape(name)}</dt><dd>${escape(value ?? "—")}</dd></div>`).join("")}</dl>`;
}

function section(title: string, content: string): string {
  return `<section class="volume-one-section"><h2>${escape(title)}</h2>${content}</section>`;
}

function rawEvidence(detail: JsonObject): string {
  return `<details class="volume-one-raw"><summary>Exact record</summary><pre>${escape(JSON.stringify(detail, null, 2))}</pre></details>`;
}

function shortHash(value: unknown): string {
  const hash = String(value ?? "");
  return hash.length > 16 ? `${hash.slice(0, 12)}…${hash.slice(-6)}` : hash;
}

function renderReference(page: "Source" | "Completeness"): void {
  const source = page === "Source";
  const artifacts = source ? manifest.inputs : manifest.outputs;
  const checks = Object.entries(manifest.checks ?? {});
  main.innerHTML = `<article class="volume-one-reference">
    ${detailHeading(page, config.title)}
    <section class="volume-one-section"><h2>${source ? "Immutable inputs" : "Published extent"}</h2>
      <p>${source ? "Every displayed value is rebuilt from the pinned TypeLake artifacts below. Labels and external catalogue names are not mathematical identity." : completenessText()}</p>
      ${source ? '<p><a href="https://github.com/tertius-ai/typelake/tree/main" target="_blank" rel="noreferrer">TypeLake source ↗</a> · <a href="https://github.com/tertius-ai/noether/tree/v0.3.2" target="_blank" rel="noreferrer">Noether 0.3.2 ↗</a></p>' : ""}
    </section>
    ${source ? "" : section("Coverage", coverageTable())}
    ${section(source ? "Source artifacts" : "Published artifacts", artifactTable(artifacts))}
    ${section("Machine checks", `<ul class="check-register">${checks.map(([name, passed]) => `<li><span>${passed ? "✓" : "×"}</span>${escape(name.replaceAll("_", " "))}</li>`).join("")}</ul>`)}
    ${section("Selected manifest", `<pre class="manifest-preview">${escape(JSON.stringify(manifestSummary(), null, 2))}</pre>`)}
  </article>`;
}

function completenessText(): string {
  if (config.kind === "SimplicialComplex") {
    return `Complete up to simplicial isomorphism through ${number(manifest.maximum_vertices)} vertices: ${number(manifest.complex_count).toLocaleString()} classes. Strong cores and the selected cohomology sidecars are resolved throughout that shell.`;
  }
  if (config.kind === "SimplicialSet") {
    return `The manifest certifies ${number(manifest.simplicial_set_count).toLocaleString()} isomorphism classes in the frontier tabulated below. Horn relations are witnessed, not promoted to a theorem-complete homotopy classification.`;
  }
  if (config.kind === "FiniteGroupCohomology") {
    const groups = number(manifest.group_count);
    const integral = asObject(manifest.integral_type_lake_manifest);
    const complete = number(integral.complete_presentations);
    const universal = number(integral.exact_low_degree_truncations);
    const sparse = number(integral.exact_sparse_bar_truncations);
    return `All ${number(manifest.complete_modular_presentations).toLocaleString()} required modular rows for the ${groups.toLocaleString()} groups through order ${number(manifest.maximum_order)} are complete. Integral presentations are complete for ${complete.toLocaleString()} groups; ${sparse.toLocaleString()} further groups have budgeted sparse-bar ring truncations, and ${universal.toLocaleString()} retain the universal exact range through degree 2.`;
  }
  return `All ${number(manifest.modular_cohomology_available).toLocaleString()} modular coefficient rows in the selected parameter box are available. Integral homology and additive cohomology are available for ${number(manifest.integral_homology_available).toLocaleString()} of ${number(manifest.space_count).toLocaleString()} spaces; complete integral rings are available for ${number(manifest.integral_cohomology_ring_available).toLocaleString()}, with every other cell explicitly pending.`;
}

function coverageTable(): string {
  if (config.kind === "SimplicialComplex") {
    const rows = (manifest.label_buckets ?? []).map((bucket) => [
      bucket.vertices,
      bucket.dimension,
      bucket.row_count,
    ]);
    return coverageRows(["Vertices", "Dimension", "Classes"], rows);
  }
  if (config.kind === "SimplicialSet") {
    const rows = asArray(manifest.frontier).map((value) => {
      const shell = asObject(value);
      return [shell.total_nondegenerate, shell.maximum_dimension, shell.row_count];
    });
    return coverageRows(["Total nondegenerate", "Dimension bound", "Classes"], rows);
  }
  if (config.kind === "FiniteGroupCohomology") {
    const integral = asObject(manifest.integral_type_lake_manifest);
    const exactDegrees = asObject(integral.exact_degrees);
    const integralRows: unknown[][] = Object.entries(exactDegrees)
      .sort(([left], [right]) => number(left) - number(right))
      .map(([degree, count]) => [`Integral exact through degree ${degree}`, count, 0]);
    return coverageRows(
      ["Layer", "Available", "Pending"],
      [
        ["Modular complete presentations", manifest.complete_modular_presentations, 0],
        ["Integral complete presentations", integral.complete_presentations, 0],
        ...integralRows,
      ],
    );
  }
  return coverageRows(
    ["Layer", "Available", "Pending"],
    [
      ["Modular cohomology rings", manifest.modular_cohomology_available, manifest.modular_cohomology_pending],
      ["Integral homology", manifest.integral_homology_available, manifest.integral_homology_pending],
      ["Integral additive cohomology", manifest.integral_cohomology_available, manifest.integral_cohomology_pending],
      ["Integral cohomology rings", manifest.integral_cohomology_ring_available, manifest.integral_cohomology_ring_pending],
    ],
  );
}

function coverageRows(headings: string[], rows: unknown[][]): string {
  return `<div class="artifact-table"><table class="volume-one-table"><thead><tr>${headings.map((heading) => `<th>${escape(heading)}</th>`).join("")}</tr></thead><tbody>${rows.map((row) => `<tr>${row.map((value) => `<td>${typeof value === "number" ? value.toLocaleString() : escape(value ?? "—")}</td>`).join("")}</tr>`).join("")}</tbody></table></div>`;
}

function artifactTable(artifacts: Artifact[]): string {
  const visible = artifacts.slice(0, 80);
  const omitted = artifacts.length - visible.length;
  return `<div class="artifact-table"><table class="volume-one-table"><thead><tr><th>Artifact</th><th>Rows</th><th>Bytes</th><th>SHA-256</th></tr></thead><tbody>${visible.map((artifact) => `<tr><td>${escape(artifact.path)}</td><td>${artifact.row_count.toLocaleString()}</td><td>${artifact.byte_count.toLocaleString()}</td><td><code>${escape(shortHash(artifact.sha256))}</code></td></tr>`).join("")}</tbody></table>${omitted > 0 ? `<p class="muted">${omitted.toLocaleString()} additional immutable detail shards are inventoried in manifest.json.</p>` : ""}</div>`;
}

function renderGlossary(): void {
  const common: Array<[string, string]> = [
    ["Canonical index", "The one-based position of the canonical encoding inside the complete bucket fixed by the preceding label coordinates."],
    ["Complete", "A claim discharged by the theorem or finite census boundary named in the retained certificate; it is not inferred from a quiet computation."],
    ["Pending", "No mathematical value is asserted for this cell. Pending is different from zero or from an empty presentation."],
  ];
  const terms: Record<CollectionKind, Array<[string, string]>> = {
    SimplicialComplex: [
      ["Label a.b.c", "a is the number of vertices, b is the maximum simplex dimension (−1 for the empty complex), and c is the canonical index."],
      ["Facet", "A simplex maximal under inclusion. The facet antichain is the stored abstract simplicial complex."],
      ["Strong core", "The unique-up-to-isomorphism result of deleting dominated vertices until none remain."],
      ["Elementary collapse", "Deletion of a free face together with its unique coface; the record retains a replayable sequence."],
      ["Facet-incidence plate", "A derived drawing of canonical vertices and maximal faces. It is not part of mathematical identity."],
    ],
    SimplicialSet: [
      ["Label a.b.c", "a is the total number of nondegenerate simplices, b is the maximum dimension, and c is the canonical index."],
      ["Nondegenerate simplex", "A simplex not in the image of a degeneracy map. Degenerate simplices are determined and are not counted in the label."],
      ["Degeneracy word", "The display s_[q](sigma) records the unique canonical monotone surjection q from a degenerate face to its underlying nondegenerate simplex."],
      ["Normalized chains", "The chain complex generated only by nondegenerate simplices, with the alternating face boundary."],
      ["Horn collapse", "A witnessed elementary removal inverse to a horn attachment. Stored paths do not claim a complete homotopy classification."],
      ["Witnessed component", "The connected component in the finite graph of stored horn-collapse and reverse horn-attachment moves."],
    ],
    FiniteGroupCohomology: [
      ["Label n.k", "The LMFDB/SmallGroups label: n is the group order and k is its catalogue index. It links to the corresponding LMFDB abstract group."],
      ["Standard group name", "GAP's StructureDescription for the labelled group, used for display and search but not as mathematical identity."],
      ["Coefficient row", "One cohomology ring H*(BG; R) for R = ℤ or a prime field whose characteristic divides the group order."],
      ["Graded-commutative", "Homogeneous elements satisfy xy = (−1)^(|x||y|)yx; integral odd squares can therefore be nonzero 2-torsion."],
      ["Completion certificate", "Replayable evidence that the displayed generators and relations present the full ring, not merely a bounded truncation."],
      ["Integral Tor class", "The degree-three class coupling two cyclic invariant factors in integral classifying-space cohomology."],
    ],
    EilenbergMacLane: [
      ["Label A.n", "A is the LMFDB finite-abelian-group label, or Z for the infinite cyclic group; n is the delooping degree."],
      ["K(A,n)", "An anima with homotopy group A in degree n and all other homotopy groups zero."],
      ["Delooping degree", "The integer n in K(A,n). It is the degree of the sole nonzero homotopy group."],
      ["Presentation extent", "Either a complete family-theorem presentation or an exact-through-degree calculation with no claim above its bound."],
      ["Operation name", "A reader-facing admissible Steenrod word on a fundamental class: Sq sequences at the prime 2, and Bockstein/reduced-power sequences at odd primes. The canonical generator id remains available as record identity."],
    ],
  };
  main.innerHTML = `<article class="volume-one-reference">${detailHeading("Glossary", config.title)}<dl class="glossary-register">${[...terms[config.kind], ...common].map(([term, definition]) => `<div id="${term.toLowerCase().replaceAll(/[^a-z0-9]+/g, "-")}"><dt>${escape(term)}</dt><dd>${escape(definition)}</dd></div>`).join("")}</dl></article>`;
}

function manifestSummary(): JsonObject {
  const omitted = new Set(["inputs", "outputs", "type_lake_manifest", "integral_type_lake_manifest"]);
  return Object.fromEntries(Object.entries(manifest).filter(([key]) => !omitted.has(key)));
}

function showError(error: unknown): void {
  const message = error instanceof Error ? error.message : String(error);
  main.innerHTML = `<section class="volume-one-error"><p class="eyebrow">Catalogue unavailable</p><h1>${escape(config?.title ?? "TypeBase")}</h1><p>${escape(message)}</p></section>`;
}

async function start(): Promise<void> {
  try {
    config = configured(collectionFromRoute());
    updateNavigation();
    mountPresentationControls();
    const tail = routeTail();
    const needsReleaseManifest = tail[0] === "Source" || tail[0] === "Completeness";
    const manifestPath =
      config.compressedSearch && !needsReleaseManifest ? "catalogue.json" : "manifest.json";
    manifest = await loadJson<Manifest>(manifestPath, false);
    if (manifest.status !== "passed" && !manifest.status.includes("partial")) {
      throw new Error("The selected TypeBase component has not passed validation");
    }
    if (tail[0] === "Source" || tail[0] === "Completeness") {
      renderReference(tail[0]);
      return;
    }
    if (tail[0] === "Glossary") {
      renderGlossary();
      return;
    }
    if (tail.length > 0) {
      await openObject(tail);
      return;
    }
    await loadInitialCatalogue();
    renderCatalogueShell();
    renderResults();
  } catch (error) {
    showError(error);
  }
}

void start();
