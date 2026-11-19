import { steinerDiagram } from "./steiner-diagram";
import { initializePresentation, mountPresentationControls } from "./presentation";
import { posetPath, routeLabel, steinerPath } from "./routes";
import type {
  BasepointRecord,
  CellRecord,
  CategoryDetail,
  CategorySearchRow,
  GrayRow,
  HomologyGroup,
  MappingRecord,
  CategoricalHomotopyRecord,
  SteinerManifest,
} from "./steiner-model";

const PAGE_SIZE = 24;

initializePresentation();

function required<T extends HTMLElement>(id: string): T {
  const found = document.getElementById(id);
  if (!found) throw new Error("Missing browser element #" + id);
  return found as T;
}

function escape(value: unknown): string {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

function plural(count: number, singular: string, many = singular + "s"): string {
  return count.toLocaleString() + " " + (count === 1 ? singular : many);
}

function parseHomology(signature: string): HomologyGroup[] {
  return signature
    .split(";")
    .filter(Boolean)
    .map((group) => {
      const [degree, freeRank, torsion = ""] = group.split(":");
      return {
        degree: Number(degree),
        free_rank: Number(freeRank),
        torsion: torsion ? torsion.split(",") : [],
      };
    });
}

function groupText(group: HomologyGroup): string {
  const parts: string[] = [];
  if (group.free_rank === 1) parts.push("ℤ");
  if (group.free_rank > 1) parts.push("ℤ^" + group.free_rank);
  for (const factor of group.torsion) parts.push("ℤ/" + factor);
  return "H" + group.degree + " = " + (parts.join(" ⊕ ") || "0");
}

function compactHomology(signature: string): string {
  const groups = parseHomology(signature);
  const ranks = groups.map((group) => group.free_rank).join(", ");
  const torsion = groups
    .filter((group) => group.torsion.length > 0)
    .map((group) => "Tor H" + group.degree + " = " + group.torsion.join(","))
    .join("; ");
  return "β = (" + ranks + ")" + (torsion ? "; " + torsion : "");
}

const elements = {
  cataloguePage: required<HTMLElement>("catalogue-page"),
  recordPage: required<HTMLElement>("record-page"),
  form: required<HTMLFormElement>("filters"),
  label: required<HTMLInputElement>("label-filter"),
  size: required<HTMLSelectElement>("size-filter"),
  dimension: required<HTMLSelectElement>("dimension-filter"),
  family: required<HTMLSelectElement>("family-filter"),
  connected: required<HTMLInputElement>("connected-filter"),
  selfDual: required<HTMLInputElement>("self-dual-filter"),
  interesting: required<HTMLInputElement>("interesting-filter"),
  interestingHomotopy: required<HTMLInputElement>("interesting-homotopy-filter"),
  homologyDegree: required<HTMLSelectElement>("homology-degree-filter"),
  minimumBetti: required<HTMLInputElement>("minimum-betti-filter"),
  torsion: required<HTMLSelectElement>("torsion-filter"),
  homotopyOrder: required<HTMLInputElement>("homotopy-order-filter"),
  count: required<HTMLElement>("result-count"),
  note: required<HTMLElement>("query-note"),
  list: required<HTMLElement>("object-list"),
  pagination: required<HTMLElement>("pagination"),
  previous: required<HTMLButtonElement>("previous-page"),
  next: required<HTMLButtonElement>("next-page"),
  page: required<HTMLElement>("page-position"),
  record: required<HTMLElement>("record-content"),
  skipLink: required<HTMLAnchorElement>("skip-link"),
  gray: required<HTMLElement>("gray-table"),
};

let manifest: SteinerManifest;
let catalogue: CategorySearchRow[] = [];
let interesting = new Set<string>();
let offset = 0;
let currentResults: CategorySearchRow[] = [];

function selectedRows(): CategorySearchRow[] {
  const label = elements.label.value.trim();
  if (label && !/^\d+(?:\.-?\d+)?(?:\.\d+)?$/.test(label)) {
    throw new Error("Labels contain two or three numbers separated by periods");
  }
  const size = elements.size.value === "" ? null : Number(elements.size.value);
  const dimension =
    elements.dimension.value === "" ? null : Number(elements.dimension.value);
  const family = elements.family.value;
  const homologyDegree =
    elements.homologyDegree.value === "" ? null : Number(elements.homologyDegree.value);
  const minimumRank = Math.max(0, Number(elements.minimumBetti.value) || 0);
  const minimumHomotopyOrder = Math.max(0, Number(elements.homotopyOrder.value) || 0);
  return catalogue.filter((row) => {
    if (label && !row.label.startsWith(label)) return false;
    if (size !== null && row.atomic_size !== size) return false;
    if (dimension !== null && row.dimension !== dimension) return false;
    if (family && !row.recognized_families.includes(family)) {
      return false;
    }
    if (elements.connected.checked && !row.is_connected) return false;
    if (elements.selfDual.checked && !row.is_self_dual) return false;
    if (elements.interesting.checked && !interesting.has(row.label)) return false;
    if (elements.interestingHomotopy.checked && !row.has_interesting_homotopy_poset) {
      return false;
    }
    const groups = parseHomology(row.polygraphic_homology_signature);
    if (
      homologyDegree !== null &&
      (groups.find((group) => group.degree === homologyDegree)?.free_rank ?? 0) < minimumRank
    ) {
      return false;
    }
    const hasTorsion =
      homologyDegree === null
        ? row.has_polygraphic_torsion
        : (groups.find((group) => group.degree === homologyDegree)?.torsion.length ?? 0) > 0;
    if (elements.torsion.value === "present" && !hasTorsion) return false;
    if (elements.torsion.value === "absent" && hasTorsion) return false;
    return row.maximum_homotopy_poset_order >= minimumHomotopyOrder;
  });
}

function renderRows(rows: CategorySearchRow[]): void {
  if (rows.length === 0) {
    elements.list.innerHTML =
      '<div class="empty-result"><p class="eyebrow">No label found</p>' +
      "<h3>Try opening the arrangement.</h3>" +
      "<p>Remove a quality or broaden the label, size, dimension, or homology range.</p></div>";
    return;
  }
  elements.list.innerHTML = rows
    .map((row) => {
      const marks = [
        row.is_discrete ? "<span>discrete</span>" : "",
        row.is_self_dual ? "<span>self-dual</span>" : "",
        row.has_polygraphic_torsion ? "<span>torsion</span>" : "",
        row.has_interesting_homotopy_poset ? "<span>nontrivial hPos</span>" : "",
        interesting.has(row.label) ? "<span>interesting</span>" : "",
      ].join("");
      return (
        '<article class="object-entry">' +
        '<a class="object-entry__link" href="' +
        steinerPath(row.label) +
        '">' +
        '<span class="accession">' +
        escape(row.label) +
        "</span>" +
        '<span class="object-entry__arrow" aria-hidden="true">↗</span></a>' +
        '<dl class="object-entry__facts">' +
        "<div><dt>Atoms</dt><dd>(" +
        row.atom_counts.join(", ") +
        ")</dd></div>" +
        "<div><dt>Cells</dt><dd>" +
        row.total_nonidentity_cells +
        "</dd></div>" +
        "<div><dt>Basepoints</dt><dd>" +
        row.basepoint_orbit_count +
        "</dd></div>" +
        '<div class="object-entry__homology"><dt>Polygraphic homology</dt><dd>' +
        escape(compactHomology(row.polygraphic_homology_signature)) +
        "</dd></div>" +
        "<div><dt>Symmetry</dt><dd>|Aut| = " +
        escape(row.automorphism_order) +
        "</dd></div></dl>" +
        '<div class="object-entry__marks">' +
        marks +
        "</div></article>"
      );
    })
    .join("");
}

function search(reset = false): void {
  try {
    if (reset) offset = 0;
    currentResults = selectedRows();
    const page = currentResults.slice(offset, offset + PAGE_SIZE);
    renderRows(page);
    elements.count.textContent = plural(currentResults.length, "object");
    elements.note.textContent =
      currentResults.length > PAGE_SIZE
        ? "Showing " +
          (offset + 1) +
          "–" +
          Math.min(offset + PAGE_SIZE, currentResults.length) +
          " in canonical order."
        : "Shown in canonical order.";
    elements.pagination.hidden = currentResults.length <= PAGE_SIZE;
    elements.previous.disabled = offset === 0;
    elements.next.disabled = offset + PAGE_SIZE >= currentResults.length;
    elements.page.textContent =
      Math.floor(offset / PAGE_SIZE) +
      1 +
      " / " +
      Math.max(1, Math.ceil(currentResults.length / PAGE_SIZE));
  } catch (error) {
    showError(error);
  }
}

function objectOf(cell: { positive_supports: number[][] }): number | undefined {
  return cell.positive_supports[0]?.[0];
}

function cellName(cell: { dimension: number; positive_supports: number[][] }): string {
  const top = cell.positive_supports[cell.dimension] ?? [];
  if (top.length === 0) return "identity";
  if (cell.dimension === 1) return "path [" + top.map((atom) => "a" + atom).join(", ") + "]";
  return "support [" + top.join(", ") + "] in degree " + cell.dimension;
}

function cellKey(cell: CellRecord): string {
  return JSON.stringify([
    cell.dimension,
    cell.negative_supports,
    cell.positive_supports,
  ]);
}

function boundaryCell(cell: CellRecord, positive: boolean): CellRecord | undefined {
  if (cell.dimension === 0) return undefined;
  const dimension = cell.dimension - 1;
  const negativeSupports = cell.negative_supports.slice(0, cell.dimension).map((row) => [...row]);
  const positiveSupports = cell.positive_supports.slice(0, cell.dimension).map((row) => [...row]);
  const support = [
    ...(positive ? cell.positive_supports[dimension] : cell.negative_supports[dimension]) ?? [],
  ];
  negativeSupports[dimension] = support;
  positiveSupports[dimension] = [...support];
  return {
    dimension,
    negative_supports: negativeSupports,
    positive_supports: positiveSupports,
  };
}

function ambientCells(detail: CategoryDetail): CellRecord[][] {
  const byDimension = Array.from(
    { length: Math.max(0, detail.category.invariants.dimension + 1) },
    () => new Map<string, CellRecord>(),
  );
  const add = (cell: CellRecord) => {
    if ((cell.negative_supports[cell.dimension] ?? []).length === 0) return;
    const bucket = byDimension[cell.dimension];
    if (bucket) bucket.set(cellKey(cell), cell);
  };
  for (const mapping of detail.mappings) {
    add(mapping.source);
    add(mapping.target);
    for (const cells of mapping.cells) for (const cell of cells) add(cell);
  }
  return byDimension.map((bucket) =>
    [...bucket.values()].sort((left, right) => cellKey(left).localeCompare(cellKey(right))),
  );
}

interface NamedCellRegister {
  cells: CellRecord[][];
  names: Map<string, string>;
}

function namedCells(detail: CategoryDetail): NamedCellRegister {
  const cells = ambientCells(detail);
  const names = new Map<string, string>();
  cells.forEach((degreeCells, dimension) => {
    degreeCells.forEach((cell, index) => {
      const name = dimension === 0 ? `v${objectOf(cell) ?? index}` : `c${dimension}.${index + 1}`;
      names.set(cellKey(cell), name);
    });
  });
  return { cells, names };
}

function cellBoundaryText(cell: CellRecord, names: Map<string, string>): string {
  if (cell.dimension === 0) return "object";
  const source = boundaryCell(cell, false);
  const target = boundaryCell(cell, true);
  const sourceName = source ? names.get(cellKey(source)) ?? cellName(source) : "?";
  const targetName = target ? names.get(cellKey(target)) ?? cellName(target) : "?";
  return `${sourceName} ${cell.dimension === 1 ? "→" : "⇒"} ${targetName}`;
}

function atomicSupportText(cell: CellRecord): string {
  const support = cell.negative_supports[cell.dimension] ?? [];
  if (cell.dimension === 0) return `atom ${cell.positive_supports[0]?.[0] ?? "?"}`;
  if (support.length === 0) return "identity";
  return `top atomic support {${support.join(", ")}}`;
}

function cellRegisterHtml(register: NamedCellRegister): string {
  const rows = register.cells
    .map((cells, dimension) => {
      const body = cells
        .map((cell) => {
          const name = register.names.get(cellKey(cell)) ?? "?";
          return `<tr><th>${escape(name)}</th><td>${escape(cellBoundaryText(cell, register.names))}</td><td>${escape(atomicSupportText(cell))}</td></tr>`;
        })
        .join("");
      return `<details ${dimension <= 2 ? "open" : ""}><summary>Dimension ${dimension} <span>${plural(cells.length, "cell")}</span></summary><table><tbody>${body}</tbody></table></details>`;
    })
    .join("");
  return `<div class="cell-register">${rows || '<p class="muted">No cells.</p>'}</div>`;
}

function mappingFor(detail: CategoryDetail, source: number, target: number): MappingRecord | undefined {
  return detail.mappings.find(
    (mapping) => objectOf(mapping.source) === source && objectOf(mapping.target) === target,
  );
}

function mappingPanel(
  mapping: MappingRecord | undefined,
  source: number,
  target: number,
  names: Map<string, string>,
): string {
  if (!mapping) return "<p class=\"muted\">No stored mapping category for this pair.</p>";
  const label = mapping.mapping_label
    ? "<div><dt>Label</dt><dd>" + escape(mapping.mapping_label) + "</dd></div>"
    : "<div><dt>Label</dt><dd>derived only</dd></div>";
  const link = mapping.mapping_label
    ? '<a class="button mapping-panel__link" href="' +
      steinerPath(mapping.mapping_label) +
      '">Open Mor(' +
      source +
      ", " +
      target +
      ") as " +
      escape(mapping.mapping_label) +
      " →</a>"
    : mapping.has_strong_steiner_model
      ? '<p class="muted">This mapping category has a strong-Steiner model with ' +
        (mapping.mapping_atomic_size ?? "more") +
        " atoms, outside this published label range.</p>"
      : '<p class="muted">No strong-Steiner presentation is attached to this mapping category, so it has no label in the present strong-Steiner collection. Its exact Steiner mapping-category data are still shown below.</p>';
  const cells = mapping.cells
    .map((degreeCells, relativeDimension) =>
      `<div><strong>Relative dimension ${relativeDimension}</strong><span>${degreeCells.length ? degreeCells.map((cell) => escape(names.get(cellKey(cell)) ?? cellName(cell))).join(", ") : "none"}</span></div>`,
    )
    .join("");
  return (
    '<div class="mapping-panel__heading"><p class="eyebrow">Selected mapping category</p>' +
    "<h4>Mor(" +
    source +
    ", " +
    target +
    ")</h4></div>" +
    '<dl class="record__primary-facts">' +
    "<div><dt>Dimension</dt><dd>" +
    mapping.relative_dimension +
    "</dd></div>" +
    "<div><dt>Exact cells</dt><dd>(" +
    mapping.exact_cell_counts.join(", ") +
    ")</dd></div>" +
    label +
    "</dl>" +
    link +
    '<div class="mapping-panel__cells">' +
    cells +
    "</div>"
  );
}

function homologyHtml(groups: HomologyGroup[]): string {
  return groups.length
    ? groups.map((group) => escape(groupText(group))).join("<br>")
    : "No nonzero groups";
}

function basepointRows(basepoints: BasepointRecord[]): string {
  if (basepoints.length === 0) return '<p class="muted">No monomorphic oriented basepoints.</p>';
  return (
    '<div class="basepoint-register">' +
    basepoints
      .map((basepoint) => {
        const summary = basepoint.homotopy_poset_summary;
        const reducedGroups = summary.integral_homology.filter(
          (group) =>
            group.degree > 0 && (group.free_rank > 0 || group.torsion.length > 0),
        );
        const homology = reducedGroups.length
          ? " · " + reducedGroups.map(groupText).join(", ")
          : summary.component_count > 1
            ? " · H̃0 = ℤ^" + (summary.component_count - 1)
            : "";
        const poset = basepoint.homotopy_poset_label
          ? '<a href="' +
            posetPath(basepoint.homotopy_poset_label) +
            '">Poset ' +
            escape(basepoint.homotopy_poset_label) +
            " ↗</a>"
          : '<span class="muted">order ' +
            basepoint.homotopy_poset_order +
            " · outside Poset edition</span>";
        return (
          '<div class="' +
          (summary.is_interesting ? "is-homotopically-interesting" : "") +
          '"><span class="basepoint-register__boundary"><strong>hPos<sub>' +
          basepoint.homotopy_degree +
          "</sub></strong> at " +
          escape(basepoint.description) +
          '<small>local label ' +
          basepoint.homotopy_degree +
          "." +
          basepoint.local_index +
          "</small></span><span>" +
          escape(summary.shape) +
          homology +
          "</span><span>orbit " +
          escape(basepoint.orbit_size) +
          " · core " +
          summary.core_order +
          "</span>" +
          poset +
          "</div>"
        );
      })
      .join("") +
    "</div>"
  );
}

function categoricalComponentRows(homotopy: CategoricalHomotopyRecord): string {
  const labels = new Map(
    homotopy.component_poset_links.map((link) => [link.particle_number, link.poset_label]),
  );
  return (
    '<div class="basepoint-register categorical-slices">' +
    homotopy.component_poset_slices
      .map((slice) => {
        const label = labels.get(slice.particle_number);
        const destination = label
          ? '<a href="' +
            posetPath(label) +
            '">Poset ' +
            escape(label) +
            " ↗</a>"
          : '<span class="muted">outside finite-Poset edition</span>';
        return (
          "<div><span>weight " +
          slice.particle_number +
          "</span><span>hPos<sub>0</sub></span><span>|hPos| = " +
          slice.poset_order +
          "</span>" +
          destination +
          "</div>"
        );
      })
      .join("") +
    "</div>"
  );
}

function groupLink(detail: CategoryDetail): string {
  const label = detail.automorphism_lmfdb_label;
  if (!label) {
    return '<span class="muted">Abstract group identification is not yet attached.</span>';
  }
  return (
    '<a href="https://www.lmfdb.org/Groups/Abstract/' +
    encodeURIComponent(label) +
    '" target="_blank" rel="noreferrer">LMFDB ' +
    escape(label) +
    " ↗</a>"
  );
}

async function openRecord(label: string): Promise<void> {
  elements.cataloguePage.hidden = true;
  elements.recordPage.hidden = false;
  elements.skipLink.href = "#record-page";
  elements.skipLink.textContent = "Skip to Steiner category";
  elements.record.innerHTML =
    '<div class="record__loading"><p class="eyebrow">Label ' +
    escape(label) +
    "</p><p>Opening the record…</p></div>";
  try {
    const response = await fetch(new URL("details/" + label + ".json", dataRoot));
    if (!response.ok) throw new Error(response.status === 404 ? "No Steiner category has label " + label : `Category data returned HTTP ${response.status}`);
    const detail = (await response.json()) as CategoryDetail;
    if (detail.label !== label) throw new Error(`The detail projection omits ${label}`);
    const category = detail.category;
    const cells = namedCells(detail);
    const families = category.recognized_families
      .map((family) => family.family + " " + family.parameter)
      .join(" · ");
    elements.record.innerHTML =
      '<header class="record__header"><div><p class="eyebrow">TypeBase label</p>' +
      '<h2 id="record-title">' +
      escape(label) +
      '</h2></div><a class="record__back" href="./steiner.html">← Steiner Categories</a></header>' +
      '<div class="record__presentation"><div data-presentation-controls="site" aria-label="Website presentation proof"></div><div data-presentation-controls="diagram" aria-label="Diagram plate"></div></div>' +
      '<nav class="record__reference" aria-label="About the Steiner-category database"><a href="/Steiner/Source">Source</a><a href="/Steiner/Completeness">Completeness</a><a href="/Glossary">Glossary</a></nav>' +
      '<div class="record__body"><figure class="record__figure" id="record-figure">' +
      "<figcaption>This is a display of ι<sub>2</sub>(C), the 2-dimensional view of C. Every object and generating arrow appears once; each colored transformation mark is a generating 2-cell between its displayed composite boundaries. Dense or coincident faces use separate lanes and an overpass at unrelated 1-cells. Click two objects to inspect Mor(x,y).</figcaption></figure>" +
      '<aside class="record__identity"><p class="accession">Strong Steiner category</p><h3>' +
      escape(families || category.invariants.atomic_size + " atoms") +
      '</h3><dl class="record__primary-facts">' +
      "<div><dt>Dimension</dt><dd>" +
      category.invariants.dimension +
      "</dd></div><div><dt>Atoms</dt><dd>(" +
      category.atom_counts.join(", ") +
      ")</dd></div><div><dt>Objects</dt><dd>" +
      category.invariants.object_count +
      "</dd></div><div><dt>Exact cells</dt><dd>(" +
      category.invariants.exact_cell_counts.join(", ") +
      ")</dd></div></dl>" +
      '<div class="record__marks">' +
      (category.invariants.is_connected ? "<span>Connected</span>" : "") +
      (detail.dualities.some((duality) => duality.is_self_dual)
        ? "<span>Self-dual</span>"
        : "") +
      "</div></aside></div>" +
      '<section class="mapping-panel" id="mapping-panel" aria-live="polite"><p class="muted">Select a source object, then a target object, in the drawing.</p></section>' +
      '<div class="record__sections"><section class="record__technical record__cells"><p class="eyebrow">All exact nonidentity cells</p><p>Cells are grouped by ambient dimension. A row in dimension <i>q</i> gives its globular source and target in dimension <i>q</i> − 1; thus the higher rows include maps between 2-cells and beyond. Iterated identity cells are determined uniquely and are not repeated in every higher dimension.</p>' +
      cellRegisterHtml(cells) +
      '</section><section><p class="eyebrow">Integral polygraphic homology</p><p class="math-line">' +
      homologyHtml(category.invariants.integral_polygraphic_homology) +
      "</p><p class=\"muted\">χ = " +
      category.invariants.polygraphic_euler_characteristic +
      " from the atomic chain complex.</p></section>" +
      '<section><p class="eyebrow">Categorical spectrum · a concrete reading</p>' +
      '<p><strong>First, the local construction.</strong> Choose an oriented boundary Z: a pair of objects, then a parallel pair of arrows, and so on. Form the iterated mapping category Mor<sup>q</sup><sub>C</sub>(Z). Its connected components are ordered when a one-dimension-higher cell runs from one component to another. That finite order is hPos<sub>q,Z</sub>(C), and the one-particle cases are listed below.</p>' +
      '<p><strong>Then stabilize.</strong> Categorical homology permits finite unordered collections of those cells—think “particles.” Weight 1 is C itself. Weight r combines r one-particle problems; after fixing a matching of their boundaries, the resulting hPos is their Cartesian product, and the different matchings form disjoint permutation sectors.</p>' +
      '<p class="math-line">Ω<sup>∞</sup>(Hℕ ∧ Σ<sup>∞</sup><sub>cat</sub>C<sub>+</sub>)</p>' +
      '<p class="muted">The rows here are only hPos<sub>0</sub>, split by particle number. They are not ordinary homology groups and they are not the higher basepoint Posets in the register below.</p>' +
      categoricalComponentRows(detail.categorical_homotopy) +
      '<details><summary>Stored computational presentation</summary><p class="muted">The certified Street model ℕ[N<sub>Street</sub>(C)] is retained through simplicial degree ' +
      detail.categorical_homology.complete_through_degree +
      ". Simplex counts: (" +
      detail.categorical_homology.simplex_counts.join(", ") +
      "); thin: (" +
      detail.categorical_homology.thin_simplex_counts.join(", ") +
      "). It is source evidence, not a list of homology groups.</p></details></section>" +
      '<section><p class="eyebrow">Automorphisms</p><p class="math-line">|Aut(C)| = ' +
      escape(category.automorphism_group.order) +
      "</p><p>" +
      groupLink(detail) +
      '</p><p class="muted">Natural action on ' +
      category.automorphism_group.degree +
      " atoms in degree blocks.</p></section>" +
      '<section><p class="eyebrow">Dualities</p><div class="relation-list">' +
      detail.dualities
        .map((duality) => {
          const operation = "Reverse dimensions {" + duality.dimensions.join(",") + "}";
          if (!duality.target_label) {
            return (
              '<div class="relation-link relation-link--unavailable"><span>' +
              operation +
              "</span><strong>outside the strong-Steiner collection</strong><span>—</span></div>"
            );
          }
          return (
            '<a class="relation-link" href="' +
            steinerPath(duality.target_label) +
            '"><span>' +
            operation +
            "</span><strong>" +
            escape(duality.target_label) +
            "</strong><span>→</span></a>"
          );
        })
        .join("") +
      "</div></section>" +
      '<section class="record__technical record__basepoints"><p class="eyebrow">Monomorphic basepoints and local homotopy Posets</p>' +
      '<p>A basepoint is an embedded oriented boundary: its negative and positive face remain distinct at every level. The boundary description—not its local label—is the mathematical name shown first.</p>' +
      basepointRows(detail.basepoints) +
      "</section>" +
      '<section class="record__technical"><details><summary>Canonical atomic data</summary><dl>' +
      "<div><dt>Steiner key</dt><dd>[" +
      category.key.join(", ") +
      "]</dd></div><div><dt>Boundaries</dt><dd>" +
      escape(JSON.stringify(category.boundaries)) +
      "</dd></div></dl></details></section></div>";

    const figure = required<HTMLElement>("record-figure");
    const panel = required<HTMLElement>("mapping-panel");
    mountPresentationControls(elements.record);
    let selected: number[] = [];
    const draw = () => {
      const old = figure.querySelector("svg");
      if (old) old.remove();
      figure.append(
        steinerDiagram(category, selected, (object) => {
          if (selected.length === 0 || selected.length === 2) selected = [object];
          else selected.push(object);
          const keyboardFocus = figure.contains(document.activeElement);
          draw();
          if (keyboardFocus) figure.querySelector<SVGElement>(`[data-object="${object}"]`)?.focus();
          if (selected.length === 2) {
            panel.innerHTML = mappingPanel(
              mappingFor(detail, selected[0] ?? 0, selected[1] ?? 0),
              selected[0] ?? 0,
              selected[1] ?? 0,
              cells.names,
            );
          } else {
            panel.innerHTML =
              '<p class="muted">Source object ' +
              object +
              " selected. Now choose the target.</p>";
          }
        }),
      );
    };
    draw();
    document.title = `Steiner Category ${label} — TypeBase Volume I`;
  } catch (error) {
    elements.record.innerHTML =
      '<div class="record__error"><p class="eyebrow">Unable to open label</p><h2>' +
      escape(error instanceof Error ? error.message : error) +
      '</h2><a class="button" href="./steiner.html">Back to Steiner Categories</a></div>';
  }
}

function renderGray(rows: GrayRow[]): void {
  const names = [...new Set(rows.map((row) => row.left_name))];
  const header =
    "<tr><th>⊗</th>" + names.map((name) => "<th>" + escape(name) + "</th>").join("") + "</tr>";
  const body = names
    .map(
      (left) =>
        "<tr><th>" +
        escape(left) +
        "</th>" +
        names
          .map((right) => {
            const row = rows.find(
              (candidate) => candidate.left_name === left && candidate.right_name === right,
            );
            if (!row) return "<td>—</td>";
            if (row.product_label) {
              return (
                '<td><a href="' +
                steinerPath(row.product_label) +
                '">' +
                escape(row.product_label) +
                "</a></td>"
              );
            }
            return (
              '<td class="gray-table__outside" title="Constructed outside complete range">(' +
              row.product_atom_counts.join(",") +
              ")</td>"
            );
          })
          .join("") +
        "</tr>",
    )
    .join("");
  elements.gray.innerHTML = "<table>" + header + body + "</table>";
}

function populateFilters(): void {
  for (let size = 0; size <= manifest.maximum_atomic_size; size += 1) {
    elements.size.add(new Option(String(size), String(size)));
  }
  const dimensions = [...new Set(catalogue.map((row) => row.dimension))].sort(
    (left, right) => left - right,
  );
  for (const dimension of dimensions) {
    elements.dimension.add(new Option(String(dimension), String(dimension)));
    elements.homologyDegree.add(new Option(String(dimension), String(dimension)));
  }
  const families = [...new Set(catalogue.flatMap((row) => row.recognized_families))].sort();
  for (const family of families) {
    const split = family.lastIndexOf(" ");
    const display = split < 0 ? family : `${family.slice(0, split)}(${family.slice(split + 1)})`;
    elements.family.add(new Option(display, family));
  }
}

function attachEvents(): void {
  mountPresentationControls();
  elements.form.addEventListener("submit", (event) => {
    event.preventDefault();
    const exact = /^\d+\.-?\d+\.\d+$/.test(elements.label.value.trim());
    if (exact) {
      window.location.assign(steinerPath(elements.label.value.trim()));
      return;
    }
    search(true);
  });
  elements.homologyDegree.addEventListener("change", () => {
    elements.minimumBetti.disabled = elements.homologyDegree.value === "";
  });
  elements.previous.addEventListener("click", () => {
    offset = Math.max(0, offset - PAGE_SIZE);
    search();
  });
  elements.next.addEventListener("click", () => {
    offset += PAGE_SIZE;
    search();
  });
}

function showError(error: unknown): void {
  const message = error instanceof Error ? error.message : String(error);
  elements.count.textContent = "The catalogue could not be read.";
  elements.note.textContent = message;
  elements.list.innerHTML =
    '<div class="empty-result"><h3>' +
    escape(message) +
    "</h3><p>Check the immutable category edition beside the site.</p></div>";
}

const dataRoot = new URL(
  import.meta.env.VITE_TYPEBASE_STEINER_DATA_URL ?? "data/strong-steiner-categories/",
  document.baseURI,
);

async function start(): Promise<void> {
  try {
    const legacyLabel = new URL(window.location.href).searchParams.get("label");
    const routedLabel = routeLabel("Steiner");
    if (legacyLabel && !routedLabel) {
      window.location.replace(steinerPath(legacyLabel));
      return;
    }
    const manifestResponse = await fetch(new URL("manifest.json", dataRoot));
    if (!manifestResponse.ok) throw new Error("Category manifest is unavailable");
    manifest = (await manifestResponse.json()) as SteinerManifest;
    if (manifest.status !== "passed" || manifest.stores_labeled_categories !== false) {
      throw new Error("This is not a passed, unlabeled Steiner-category edition");
    }
    if (routedLabel) {
      mountPresentationControls();
      await openRecord(routedLabel);
      return;
    }
    const [catalogueResponse, interestingResponse, grayResponse] = await Promise.all([
      fetch(new URL(manifest.catalogue_path, dataRoot)),
      fetch(new URL(manifest.interesting_path, dataRoot)),
      fetch(new URL(manifest.gray_table_path, dataRoot)),
    ]);
    if (!catalogueResponse.ok || !interestingResponse.ok || !grayResponse.ok) {
      throw new Error("One or more category projections are unavailable");
    }
    catalogue = (await catalogueResponse.json()) as CategorySearchRow[];
    interesting = new Set((await interestingResponse.json()) as string[]);
    renderGray((await grayResponse.json()) as GrayRow[]);
    populateFilters();
    attachEvents();
    search();
  } catch (error) {
    showError(error);
  }
}

void start();
