import { Catalogue } from "./catalogue";
import { hasseDiagram } from "./hasse";
import { initializePresentation, mountPresentationControls } from "./presentation";
import { posetPath, routeLabel } from "./routes";
import type {
  ComplexRecord,
  DetailRow,
  Filters,
  Manifest,
  PosetRecord,
  SearchRow,
} from "./model";

const PAGE_SIZE = 30;
const elements = {
  cataloguePage: required<HTMLElement>("catalogue-page"),
  recordPage: required<HTMLElement>("record-page"),
  form: required<HTMLFormElement>("filters"),
  scope: required<HTMLSelectElement>("scope-filter"),
  label: required<HTMLInputElement>("label-filter"),
  order: required<HTMLSelectElement>("order-filter"),
  dimension: required<HTMLSelectElement>("dimension-filter"),
  connected: required<HTMLInputElement>("connected-filter"),
  selfDual: required<HTMLInputElement>("self-dual-filter"),
  lattice: required<HTMLInputElement>("lattice-filter"),
  homologyDegree: required<HTMLSelectElement>("homology-degree-filter"),
  minimumBetti: required<HTMLInputElement>("minimum-betti-filter"),
  torsion: required<HTMLSelectElement>("torsion-filter"),
  reducedHomology: required<HTMLInputElement>("reduced-homology-filter"),
  count: required<HTMLElement>("result-count"),
  note: required<HTMLElement>("query-note"),
  list: required<HTMLElement>("object-list"),
  pagination: required<HTMLElement>("pagination"),
  previous: required<HTMLButtonElement>("previous-page"),
  next: required<HTMLButtonElement>("next-page"),
  pagePosition: required<HTMLElement>("page-position"),
  record: required<HTMLElement>("record-content"),
  skipLink: required<HTMLAnchorElement>("skip-link"),
};

let catalogue: Catalogue;
let manifest: Manifest;
let offset = 0;
let resultCount = 0;

initializePresentation();

function required<T extends HTMLElement>(id: string): T {
  const found = document.getElementById(id);
  if (!found) throw new Error(`Missing browser element #${id}`);
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

function plural(count: number, singular: string, pluralForm = `${singular}s`): string {
  return `${count.toLocaleString()} ${count === 1 ? singular : pluralForm}`;
}

function getFilters(): Filters {
  return {
    scope: elements.scope.value === "cores" ? "cores" : "posets",
    label: elements.label.value.trim(),
    order: elements.order.value === "" ? null : Number(elements.order.value),
    dimension:
      elements.dimension.value === "" ? null : Number(elements.dimension.value),
    connected: elements.connected.checked,
    selfDual: elements.selfDual.checked,
    lattice: elements.lattice.checked,
    homologyDegree:
      elements.homologyDegree.value === "" ? null : Number(elements.homologyDegree.value),
    // No fixed cap: the K_{5,5} Poset of order 10 has beta_1 = 16. The bound
    // only keeps the value an exact integer for the SQL filter.
    minimumBetti: Math.min(
      Number.MAX_SAFE_INTEGER,
      Math.max(0, Math.trunc(Number(elements.minimumBetti.value) || 0)),
    ),
    torsion:
      elements.torsion.value === "present" || elements.torsion.value === "absent"
        ? elements.torsion.value
        : "any",
    nonzeroReducedHomology: elements.reducedHomology.checked,
  };
}

function compactHomology(signature: string): string {
  const groups = signature.split(";").filter(Boolean).map((group) => group.split(":"));
  const betti = groups.map((group) => Number(group[1] ?? 0));
  const torsion = groups
    .filter((group) => group[2])
    .map((group) => `Tor H${group[0]} = ${group[2]}`)
    .join("; ");
  return `β = (${betti.join(", ")})${torsion ? `; ${torsion}` : ""}`;
}

function renderRows(records: SearchRow[]): void {
  if (records.length === 0) {
    elements.list.innerHTML = `
      <div class="empty-result">
        <p class="eyebrow">No label found</p>
        <h3>Try opening the arrangement.</h3>
        <p>Remove a quality or broaden the label, order, or dimension.</p>
      </div>`;
    return;
  }
  elements.list.innerHTML = records
    .map(
      (record) => `
        <article class="object-entry">
          <a class="object-entry__link" href="${posetPath(record.label)}">
            <span class="accession">${escape(record.label)}</span>
            <span class="object-entry__arrow" aria-hidden="true">↗</span>
          </a>
          <dl class="object-entry__facts">
            <div><dt>Dimension</dt><dd>${record.dimension}</dd></div>
            <div><dt>Width</dt><dd>${record.width}</dd></div>
            <div><dt>${record.fiber_count === undefined ? "Covers" : "Homotopy fiber"}</dt><dd>${record.fiber_count === undefined ? record.cover_count : record.fiber_count.toLocaleString()}</dd></div>
            <div class="object-entry__homology"><dt>Homology</dt><dd>${escape(compactHomology(record.homology_signature))}</dd></div>
            <div><dt>Symmetry</dt><dd>|Aut| = ${escape(record.automorphism_order)}</dd></div>
          </dl>
          <div class="object-entry__marks">
            ${record.is_core ? "<span>core</span>" : ""}
            ${record.is_self_dual ? "<span>self-dual</span>" : ""}
            ${record.is_lattice ? "<span>lattice</span>" : ""}
            ${record.has_torsion ? "<span>torsion</span>" : ""}
          </div>
        </article>`,
    )
    .join("");
}

async function search(reset = false): Promise<void> {
  if (reset) offset = 0;
  elements.count.textContent = "Reading the catalogue…";
  elements.note.textContent = "Querying the compressed columnar edition in your browser.";
  elements.list.setAttribute("aria-busy", "true");
  try {
    const filters = getFilters();
    const exact = /^\d+\.-?\d+\.\d+$/.test(filters.label);
    if (exact) {
      window.location.assign(posetPath(filters.label));
      return;
    }
    const result = await catalogue.search(filters, offset, PAGE_SIZE);
    resultCount = result.count;
    renderRows(result.records);
    elements.count.textContent = plural(
      result.count,
      filters.scope === "cores" ? "homotopy core" : "object",
    );
    const pageNote =
      result.count > PAGE_SIZE
        ? `Showing ${offset + 1}–${Math.min(offset + PAGE_SIZE, result.count)} in canonical order.`
        : "Shown in canonical order.";
    elements.note.textContent =
      filters.scope === "cores"
        ? `${pageNote} Fiber counts include every isomorphism class through order ${manifest.maximum_order}.`
        : pageNote;
    renderPagination();
  } catch (error) {
    showError(error);
  } finally {
    elements.list.removeAttribute("aria-busy");
  }
}

function renderPagination(): void {
  elements.pagination.hidden = resultCount <= PAGE_SIZE;
  elements.previous.disabled = offset === 0;
  elements.next.disabled = offset + PAGE_SIZE >= resultCount;
  const page = Math.floor(offset / PAGE_SIZE) + 1;
  const pages = Math.ceil(resultCount / PAGE_SIZE);
  elements.pagePosition.textContent = `${page} / ${pages.toLocaleString()}`;
}

function homology(complex: ComplexRecord): string {
  if (complex.integral_homology.length === 0) return "No nonzero groups";
  return complex.integral_homology
    .map((group) => {
      const summands: string[] = [];
      if (group.free_rank === 1) summands.push("ℤ");
      if (group.free_rank > 1) summands.push(`ℤ${superscript(group.free_rank)}`);
      for (const factor of group.invariant_factors) summands.push(`ℤ/${escape(factor)}`);
      return `H<sub>${group.degree}</sub> = ${summands.join(" ⊕ ") || "0"}`;
    })
    .join("<br>");
}

function superscript(value: number): string {
  return String(value).replace(/[0-9]/g, (digit) => "⁰¹²³⁴⁵⁶⁷⁸⁹"[Number(digit)] ?? digit);
}

function relationButton(label: string, description: string): string {
  return `<a class="relation-link" href="${posetPath(label)}">
    <span>${escape(description)}</span><strong>${escape(label)}</strong><span aria-hidden="true">→</span>
  </a>`;
}

function groupLink(detail: DetailRow, poset: PosetRecord): string {
  if (!detail.automorphism_lmfdb_label) {
    return `<span class="muted">Abstract group not yet externally identified</span>`;
  }
  const label = detail.automorphism_lmfdb_label;
  return `<a href="https://www.lmfdb.org/Groups/Abstract/${encodeURIComponent(label)}" target="_blank" rel="noreferrer">
    LMFDB ${escape(label)} <span aria-hidden="true">↗</span>
  </a><span class="muted"> · natural action of degree ${poset.automorphism_group.degree}</span>`;
}

async function openRecord(label: string): Promise<void> {
  elements.cataloguePage.hidden = true;
  elements.recordPage.hidden = false;
  elements.skipLink.href = "#record-page";
  elements.skipLink.textContent = "Skip to Poset";
  elements.record.innerHTML = `<div class="record__loading"><p class="eyebrow">Label ${escape(label)}</p><p>Opening the record…</p></div>`;
  try {
    const detail = await catalogue.detail(label);
    const poset = JSON.parse(detail.poset_json) as PosetRecord;
    const complex = JSON.parse(detail.order_complex_json) as ComplexRecord;
    elements.record.innerHTML = `
      <header class="record__header">
        <div><p class="eyebrow">TypeBase label</p><h2 id="record-title">${escape(label)}</h2></div>
        <a class="record__back" href="./posets.html">← Posets</a>
      </header>
      <div class="record__presentation">
        <div data-presentation-controls="site" aria-label="Website presentation proof"></div>
        <div data-presentation-controls="diagram" aria-label="Diagram plate"></div>
      </div>
      <nav class="record__reference" aria-label="About the Poset database">
        <a href="/Posets/Source">Source</a>
        <a href="/Posets/Completeness">Completeness</a>
        <a href="/Glossary">Glossary</a>
      </nav>
      <div class="record__body">
        <figure class="record__figure" id="record-figure">
          <figcaption>Canonical Hasse diagram · column h contains exactly the vertices with h strict predecessors</figcaption>
        </figure>
        <aside class="record__identity">
          <p class="accession">Finite Poset</p>
          <h3>${plural(poset.order, "point")}, dimension ${complex.dimension}</h3>
          <dl class="record__primary-facts">
            <div><dt>Longest chain</dt><dd>${poset.invariants.height}</dd></div>
            <div><dt>Width</dt><dd>${poset.invariants.width}</dd></div>
            <div><dt>Cover relations</dt><dd>${poset.invariants.cover_count}</dd></div>
            <div><dt>Components</dt><dd>${poset.invariants.component_count}</dd></div>
          </dl>
          <div class="record__marks">
            <span>${poset.relations.has_core.is_core ? "Core" : `Core size ${poset.relations.has_core.core_size}`}</span>
            <span>${poset.relations.opposite.is_self_dual ? "Self-dual" : "Paired dual"}</span>
            ${poset.invariants.is_lattice ? "<span>Lattice</span>" : ""}
          </div>
        </aside>
      </div>
      <div class="record__sections">
        <section>
          <p class="eyebrow">Relations</p>
          <div class="relation-list">
            ${relationButton(detail.core_label, "Minimal homotopy core")}
            ${relationButton(detail.opposite_label, "Opposite Poset")}
          </div>
        </section>
        <section>
          <p class="eyebrow">Integral homology</p>
          <p class="math-line">${homology(complex)}</p>
          <p class="muted">Order complex: f = (${complex.f_vector.join(", ")}); χ = ${complex.euler_characteristic}.</p>
        </section>
        <section>
          <p class="eyebrow">Automorphisms</p>
          <p class="math-line">|Aut(P)| = ${escape(poset.automorphism_group.order)}</p>
          <p>${groupLink(detail, poset)}</p>
          <p class="muted">${plural(poset.automorphism_group.generators.length, "verified generator")}; orbits ${poset.automorphism_group.orbits.map((orbit) => `{${orbit.join(",")}}`).join(" ")}.</p>
        </section>
        <section class="record__technical">
          <details>
            <summary>Canonical data</summary>
            <dl>
              <div><dt>Poset key</dt><dd>[${poset.key.join(", ")}]</dd></div>
              <div><dt>Covers</dt><dd>${poset.covers.length ? poset.covers.map(([a, b]) => `${a}≺${b}`).join(", ") : "none"}</dd></div>
              <div><dt>Order-complex key</dt><dd>${escape(complex.key)}</dd></div>
            </dl>
          </details>
        </section>
      </div>`;
    const figure = required<HTMLElement>("record-figure");
    figure.prepend(hasseDiagram(poset));
    mountPresentationControls(elements.record);
    document.title = `Poset ${label} — TypeBase Volume I`;
  } catch (error) {
    elements.record.innerHTML = `<div class="record__error"><p class="eyebrow">Unable to open label</p><h2>${escape(error instanceof Error ? error.message : error)}</h2><a class="button" href="./posets.html">Back to Posets</a></div>`;
  }
}

function showError(error: unknown): void {
  const message = error instanceof Error ? error.message : String(error);
  elements.count.textContent = "The catalogue could not be read.";
  elements.note.textContent = message;
  elements.list.innerHTML = `<div class="empty-result"><h3>${escape(message)}</h3><p>Check that the immutable data edition is available beside the site.</p></div>`;
}

function populateFilters(): void {
  for (let order = 0; order <= manifest.maximum_order; order += 1) {
    elements.order.add(new Option(String(order), String(order)));
  }
  const dimensions = [...new Set(manifest.label_buckets.map((bucket) => bucket.dimension))].sort(
    (left, right) => left - right,
  );
  for (const dimension of dimensions) {
    elements.dimension.add(new Option(String(dimension), String(dimension)));
  }
  for (let degree = 0; degree < manifest.maximum_order; degree += 1) {
    elements.homologyDegree.add(new Option(String(degree), String(degree)));
  }
}

function attachEvents(): void {
  mountPresentationControls();
  elements.form.addEventListener("submit", (event) => {
    event.preventDefault();
    void search(true);
  });
  elements.homologyDegree.addEventListener("change", () => {
    elements.minimumBetti.disabled = elements.homologyDegree.value === "";
  });
  elements.previous.addEventListener("click", () => {
    offset = Math.max(0, offset - PAGE_SIZE);
    void search();
    elements.count.scrollIntoView({ behavior: "smooth", block: "center" });
  });
  elements.next.addEventListener("click", () => {
    offset += PAGE_SIZE;
    void search();
    elements.count.scrollIntoView({ behavior: "smooth", block: "center" });
  });
}

async function start(): Promise<void> {
  try {
    const configuredRoot = import.meta.env.VITE_TYPEBASE_DATA_URL;
    const dataRoot = new URL(configuredRoot ?? "data/finite-posets/", document.baseURI);
    const response = await fetch(new URL("manifest.json", dataRoot));
    if (!response.ok) throw new Error(`Data manifest returned HTTP ${response.status}`);
    manifest = (await response.json()) as Manifest;
    if (manifest.status !== "passed" || manifest.stores_labeled_posets !== false) {
      throw new Error("This is not a passed, unlabeled TypeBase Poset edition");
    }
    populateFilters();
    catalogue = new Catalogue(manifest, dataRoot);
    attachEvents();
    const legacyLabel = new URL(window.location.href).searchParams.get("label");
    const label = routeLabel("Poset") ?? legacyLabel;
    if (label) {
      if (legacyLabel && !routeLabel("Poset")) {
        window.location.replace(posetPath(legacyLabel));
        return;
      }
      await openRecord(label);
    }
    else await search();
  } catch (error) {
    showError(error);
  }
}

void start();
