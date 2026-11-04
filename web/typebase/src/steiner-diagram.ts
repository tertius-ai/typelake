import type { BoundaryRecord, CategoryRecord } from "./steiner-model";

export interface SteinerDiagramRecord {
  boundaries: BoundaryRecord[][];
  invariants: Pick<
    CategoryRecord["invariants"],
    "object_count" | "generating_arrow_count" | "is_empty"
  >;
}

export interface SteinerDiagramOptions {
  thumbnail?: boolean;
}

interface Point {
  x: number;
  y: number;
}

interface ArrowLayout {
  arrow: number;
  source: number;
  target: number;
  start: Point;
  control: Point;
  finish: Point;
  color: string;
}

const SVG_NS = "http://www.w3.org/2000/svg";
const WIDTH = 960;
const SCENE_HEIGHT = 500;
const OBJECT_RADIUS = 16;
const BYRNE = ["#e53f28", "#17658e", "#e5aa17", "#1b1a16"] as const;

function element<K extends keyof SVGElementTagNameMap>(
  name: K,
  attributes: Record<string, string>,
): SVGElementTagNameMap[K] {
  const node = document.createElementNS(SVG_NS, name);
  for (const [key, value] of Object.entries(attributes)) node.setAttribute(key, value);
  return node;
}

function arrowEndpoints(category: SteinerDiagramRecord): Array<[number, number]> {
  return (category.boundaries[1] ?? []).map((boundary) => [
    boundary.negative[0] ?? 0,
    boundary.positive[0] ?? 0,
  ]);
}

function topologicalObjects(category: SteinerDiagramRecord): number[] {
  const count = category.invariants.object_count;
  const outgoing = Array.from({ length: count }, () => [] as number[]);
  const indegree = new Array<number>(count).fill(0);
  for (const [source, target] of arrowEndpoints(category)) {
    if (!outgoing[source]?.includes(target)) {
      outgoing[source]?.push(target);
      indegree[target] = (indegree[target] ?? 0) + 1;
    }
  }
  const ready = indegree
    .map((degree, object) => ({ degree, object }))
    .filter(({ degree }) => degree === 0)
    .map(({ object }) => object);
  const order: number[] = [];
  while (ready.length > 0) {
    ready.sort((left, right) => left - right);
    const object = ready.shift();
    if (object === undefined) break;
    order.push(object);
    for (const target of outgoing[object] ?? []) {
      indegree[target] = (indegree[target] ?? 1) - 1;
      if (indegree[target] === 0) ready.push(target);
    }
  }
  return order.length === count ? order : Array.from({ length: count }, (_, object) => object);
}

function orderedPath(arrows: number[], endpoints: Array<[number, number]>): number[] {
  if (arrows.length < 2) return [...arrows];
  const remaining = new Set(arrows);
  const targets = new Set(arrows.map((arrow) => endpoints[arrow]?.[1]));
  let current = arrows.find((arrow) => !targets.has(endpoints[arrow]?.[0])) ?? arrows[0];
  const ordered: number[] = [];
  while (current !== undefined && remaining.delete(current)) {
    ordered.push(current);
    const target = endpoints[current]?.[1];
    current = arrows.find(
      (arrow) => remaining.has(arrow) && endpoints[arrow]?.[0] === target,
    );
  }
  ordered.push(...arrows.filter((arrow) => remaining.has(arrow)).sort((a, b) => a - b));
  return ordered;
}

function pathObjects(path: number[], endpoints: Array<[number, number]>): number[] {
  if (path.length === 0) return [];
  const objects = [endpoints[path[0] ?? 0]?.[0] ?? 0];
  for (const arrow of path) objects.push(endpoints[arrow]?.[1] ?? 0);
  return objects;
}

function boundaryWord(arrows: number[], endpoints: Array<[number, number]>): string {
  const path = orderedPath(arrows, endpoints);
  return path.length === 0
    ? "id"
    : path
        .reverse()
        .map((arrow) => `a${arrow}`)
        .join(" ∘ ");
}

function boundaryFormula(
  boundary: BoundaryRecord,
  atom: number,
  endpoints: Array<[number, number]>,
): string {
  return `α${atom}: ${boundaryWord(boundary.negative, endpoints)} ⇒ ${boundaryWord(boundary.positive, endpoints)}`;
}

function objectRanks(category: SteinerDiagramRecord, order: number[]): number[] {
  const ranks = new Array<number>(category.invariants.object_count).fill(0);
  const outgoing = Array.from(
    { length: category.invariants.object_count },
    () => [] as number[],
  );
  for (const [source, target] of arrowEndpoints(category)) {
    if (!outgoing[source]?.includes(target)) outgoing[source]?.push(target);
  }
  for (const source of order) {
    for (const target of outgoing[source] ?? []) {
      ranks[target] = Math.max(ranks[target] ?? 0, (ranks[source] ?? 0) + 1);
    }
  }
  return ranks;
}

function layoutObjects(category: SteinerDiagramRecord): Map<number, Point> {
  const count = category.invariants.object_count;
  const endpoints = arrowEndpoints(category);
  const order = topologicalObjects(category);
  const ranks = objectRanks(category, order);
  const maximumRank = Math.max(0, ...ranks);
  const influence = new Array<number>(count).fill(0);
  const appearances = new Array<number>(count).fill(0);
  for (const boundary of category.boundaries[2] ?? []) {
    for (const [arrows, direction] of [
      [boundary.negative, -1],
      [boundary.positive, 1],
    ] as Array<[number[], number]>) {
      const objects = pathObjects(orderedPath(arrows, endpoints), endpoints);
      for (const object of objects.slice(1, -1)) {
        influence[object] = (influence[object] ?? 0) + direction;
        appearances[object] = (appearances[object] ?? 0) + 1;
      }
    }
  }
  if (maximumRank === 0) {
    return new Map(
      order.map((object, index) => [
        object,
        {
          x: count === 1 ? WIDTH / 2 : 90 + (780 * index) / Math.max(1, count - 1),
          y: SCENE_HEIGHT / 2,
        },
      ]),
    );
  }
  const byRank = new Map<number, number[]>();
  for (const object of order) {
    const rank = ranks[object] ?? 0;
    const members = byRank.get(rank) ?? [];
    members.push(object);
    byRank.set(rank, members);
  }
  const points = new Map<number, Point>();
  for (const [rank, objects] of byRank) {
    objects.forEach((object, index) => {
      const spread = (index - (objects.length - 1) / 2) * 82;
      const facePull =
        appearances[object] === 0
          ? 0
          : (95 * (influence[object] ?? 0)) / (appearances[object] ?? 1);
      points.set(object, {
        x: 88 + (784 * rank) / maximumRank,
        y: SCENE_HEIGHT / 2 + spread + facePull,
      });
    });
  }
  return points;
}

function arrowScores(category: SteinerDiagramRecord): number[] {
  const count = category.boundaries[1]?.length ?? 0;
  const scores = new Array<number>(count).fill(0);
  const appearances = new Array<number>(count).fill(0);
  for (const boundary of category.boundaries[2] ?? []) {
    for (const arrow of boundary.negative) {
      scores[arrow] = (scores[arrow] ?? 0) - 1;
      appearances[arrow] = (appearances[arrow] ?? 0) + 1;
    }
    for (const arrow of boundary.positive) {
      scores[arrow] = (scores[arrow] ?? 0) + 1;
      appearances[arrow] = (appearances[arrow] ?? 0) + 1;
    }
  }
  return scores.map((score, arrow) => score / Math.max(1, appearances[arrow] ?? 0));
}

function layoutArrows(category: SteinerDiagramRecord, points: Map<number, Point>): ArrowLayout[] {
  const endpoints = arrowEndpoints(category);
  const scores = arrowScores(category);
  const groups = new Map<string, number[]>();
  endpoints.forEach(([source, target], arrow) => {
    const key = `${source}:${target}`;
    const group = groups.get(key) ?? [];
    group.push(arrow);
    groups.set(key, group);
  });
  const offsets = new Array<number>(endpoints.length).fill(0);
  for (const arrows of groups.values()) {
    arrows.sort((left, right) => (scores[left] ?? 0) - (scores[right] ?? 0) || left - right);
    if (arrows.length === 1) {
      const arrow = arrows[0] ?? 0;
      offsets[arrow] = 48 * (scores[arrow] ?? 0);
      continue;
    }
    const spacing = Math.min(200, 450 / Math.max(1, arrows.length - 1));
    arrows.forEach((arrow, index) => {
      offsets[arrow] = (index - (arrows.length - 1) / 2) * spacing;
    });
  }
  return endpoints.map(([source, target], arrow) => {
    const start = points.get(source) ?? { x: 70, y: SCENE_HEIGHT / 2 };
    const finish = points.get(target) ?? { x: WIDTH - 70, y: SCENE_HEIGHT / 2 };
    return {
      arrow,
      source,
      target,
      start,
      finish,
      control: {
        x: (start.x + finish.x) / 2,
        y: (start.y + finish.y) / 2 + (offsets[arrow] ?? 0),
      },
      color: BYRNE[arrow % BYRNE.length] ?? BYRNE[3],
    };
  });
}

function quadraticPoint(layout: ArrowLayout, t: number): Point {
  const oneMinus = 1 - t;
  return {
    x:
      oneMinus * oneMinus * layout.start.x +
      2 * oneMinus * t * layout.control.x +
      t * t * layout.finish.x,
    y:
      oneMinus * oneMinus * layout.start.y +
      2 * oneMinus * t * layout.control.y +
      t * t * layout.finish.y,
  };
}

function samplePath(
  arrows: number[],
  t: number,
  layouts: ArrowLayout[],
  fallback: Point,
): Point {
  if (arrows.length === 0) return fallback;
  const scaled = Math.min(0.999999, Math.max(0, t)) * arrows.length;
  const index = Math.min(arrows.length - 1, Math.floor(scaled));
  const local = scaled - index;
  const layout = layouts[arrows[index] ?? 0];
  return layout ? quadraticPoint(layout, local) : fallback;
}

function distance(left: Point, right: Point): number {
  return Math.hypot(right.x - left.x, right.y - left.y);
}

function midpoint(left: Point, right: Point): Point {
  return { x: (left.x + right.x) / 2, y: (left.y + right.y) / 2 };
}

function toward(point: Point, target: Point, amount: number): Point {
  const length = distance(point, target);
  if (length === 0) return point;
  return {
    x: point.x + ((target.x - point.x) * amount) / length,
    y: point.y + ((target.y - point.y) * amount) / length,
  };
}

function pointList(points: Point[]): string {
  return points.map((point) => `${point.x},${point.y}`).join(" ");
}

function rightDoubleArrowPath(start: Point, finish: Point): string {
  const length = distance(start, finish);
  if (length === 0) return "";
  const unit = {
    x: (finish.x - start.x) / length,
    y: (finish.y - start.y) / length,
  };
  const normal = { x: -unit.y, y: unit.x };
  const shaftOffset = Math.min(2.4, length / 10);
  const headLength = Math.min(9, length / 3);
  const headWidth = Math.min(6, length / 4);
  const head = {
    x: finish.x - unit.x * headLength,
    y: finish.y - unit.y * headLength,
  };
  const upperStart = {
    x: start.x + normal.x * shaftOffset,
    y: start.y + normal.y * shaftOffset,
  };
  const lowerStart = {
    x: start.x - normal.x * shaftOffset,
    y: start.y - normal.y * shaftOffset,
  };
  const shaftHeadDistance = headLength * (1 - shaftOffset / headWidth);
  const upperFinish = {
    x: head.x + unit.x * shaftHeadDistance + normal.x * shaftOffset,
    y: head.y + unit.y * shaftHeadDistance + normal.y * shaftOffset,
  };
  const lowerFinish = {
    x: head.x + unit.x * shaftHeadDistance - normal.x * shaftOffset,
    y: head.y + unit.y * shaftHeadDistance - normal.y * shaftOffset,
  };
  const upperHead = {
    x: head.x + normal.x * headWidth,
    y: head.y + normal.y * headWidth,
  };
  const lowerHead = {
    x: head.x - normal.x * headWidth,
    y: head.y - normal.y * headWidth,
  };
  return [
    `M ${upperStart.x} ${upperStart.y} L ${upperFinish.x} ${upperFinish.y}`,
    `M ${lowerStart.x} ${lowerStart.y} L ${lowerFinish.x} ${lowerFinish.y}`,
    `M ${upperHead.x} ${upperHead.y} L ${finish.x} ${finish.y} L ${lowerHead.x} ${lowerHead.y}`,
  ].join(" ");
}

function faceFallback(
  boundary: BoundaryRecord,
  endpoints: Array<[number, number]>,
  points: Map<number, Point>,
): Point {
  const path = orderedPath(
    boundary.negative.length > 0 ? boundary.negative : boundary.positive,
    endpoints,
  );
  const object = endpoints[path[0] ?? 0]?.[0] ?? 0;
  return points.get(object) ?? { x: WIDTH / 2, y: SCENE_HEIGHT / 2 };
}

function facePaths(
  boundary: BoundaryRecord,
  endpoints: Array<[number, number]>,
): [number[], number[]] {
  return [
    orderedPath(boundary.negative, endpoints),
    orderedPath(boundary.positive, endpoints),
  ];
}

function useFaceLanes(category: SteinerDiagramRecord, thumbnail: boolean): boolean {
  const boundaries = category.boundaries[2] ?? [];
  if (thumbnail || boundaries.length > 2) return true;
  const endpoints = arrowEndpoints(category);
  const seen = new Set<string>();
  for (const boundary of boundaries) {
    const [negative, positive] = facePaths(boundary, endpoints);
    const signature = `${negative.join(",")}|${positive.join(",")}`;
    if (seen.has(signature)) return true;
    seen.add(signature);
  }
  return false;
}

function bestRibbonParameter(
  atom: number,
  faceCount: number,
  negative: number[],
  positive: number[],
  layouts: ArrowLayout[],
  fallback: Point,
  objectPoints: Map<number, Point>,
): number {
  const margin = faceCount <= 2 ? 0.36 : 0.18;
  const preferred =
    faceCount === 1 ? 0.5 : margin + ((1 - 2 * margin) * atom) / (faceCount - 1);
  const candidates = [
    preferred,
    ...Array.from({ length: 31 }, (_, index) => 0.125 + index * 0.025),
  ];
  return candidates.reduce((best, candidate) => {
    const negativePoint = samplePath(negative, candidate, layouts, fallback);
    const positivePoint = samplePath(positive, candidate, layouts, fallback);
    const separation = distance(negativePoint, positivePoint);
    const vertexClearance = Math.min(
      ...[...objectPoints.values()].flatMap((point) => [
        distance(negativePoint, point),
        distance(positivePoint, point),
      ]),
    );
    // A composite path passes through intermediate objects. Favor an interior
    // cross-section with visible clearance so the 2-cell glyph cannot be read
    // as an arrow entering or leaving one of those objects.
    const score =
      separation +
      2.4 * Math.min(90, vertexClearance) -
      120 * Math.abs(candidate - preferred);
    return score > best.score ? { value: candidate, score } : best;
  }, { value: preferred, score: Number.NEGATIVE_INFINITY }).value;
}

function appendDefinitions(svg: SVGSVGElement, layouts: ArrowLayout[]): void {
  const definitions = element("defs", {});
  for (const layout of layouts) {
    const marker = element("marker", {
      id: `steiner-arrow-${layout.arrow}`,
      markerWidth: "8",
      markerHeight: "8",
      refX: "7",
      refY: "4",
      orient: "auto",
      markerUnits: "userSpaceOnUse",
    });
    marker.append(element("path", { d: "M 0 0 L 8 4 L 0 8 z", fill: layout.color }));
    definitions.append(marker);
  }
  svg.append(definitions);
}

function renderFaceMembrane(
  group: SVGGElement,
  atom: number,
  boundary: BoundaryRecord,
  endpoints: Array<[number, number]>,
  layouts: ArrowLayout[],
  points: Map<number, Point>,
): void {
  const [negative, positive] = facePaths(boundary, endpoints);
  const fallback = faceFallback(boundary, endpoints, points);
  const samples = Array.from({ length: 17 }, (_, index) => index / 16);
  const upper = samples.map((t) => samplePath(negative, t, layouts, fallback));
  const lower = samples.map((t) => samplePath(positive, t, layouts, fallback));
  const color = BYRNE[(atom + 1) % BYRNE.length] ?? BYRNE[0];
  group.append(
    element("polygon", {
      points: pointList([...upper, ...lower.slice().reverse()]),
      fill: color,
      "fill-opacity": "0.095",
      stroke: color,
      "stroke-opacity": "0.42",
      "stroke-width": "1.4",
      class: "steiner-diagram__membrane",
    }),
  );
  for (const t of [0.2, 0.4, 0.6, 0.8]) {
    const from = samplePath(negative, t, layouts, fallback);
    const to = samplePath(positive, t, layouts, fallback);
    group.append(
      element("path", {
        d: `M ${from.x} ${from.y} L ${to.x} ${to.y}`,
        stroke: color,
        "stroke-opacity": "0.24",
        "stroke-width": "1",
        class: "steiner-diagram__membrane-rule",
      }),
    );
  }
}

function renderFaceRibbon(
  group: SVGGElement,
  atom: number,
  faceCount: number,
  laneMode: boolean,
  boundary: BoundaryRecord,
  endpoints: Array<[number, number]>,
  layouts: ArrowLayout[],
  points: Map<number, Point>,
): void {
  const [negative, positive] = facePaths(boundary, endpoints);
  const fallback = faceFallback(boundary, endpoints, points);
  const dense = laneMode;
  // Lane plates need a hard non-collision invariant. A clearance optimizer
  // may otherwise place several faces on the same attractive cross-section.
  // Reserve a strictly ordered lane for every generating 2-cell instead.
  const parameter = dense
    ? 0.14 + (0.72 * atom) / Math.max(1, faceCount - 1)
    : bestRibbonParameter(
        atom,
        faceCount,
        negative,
        positive,
        layouts,
        fallback,
        points,
      );
  const halfWidth = Math.min(0.055, 0.17 / Math.max(1, faceCount));
  const left = Math.max(0.08, parameter - halfWidth);
  const right = Math.min(0.92, parameter + halfWidth);
  const negativeLeft = samplePath(negative, left, layouts, fallback);
  const negativeRight = samplePath(negative, right, layouts, fallback);
  const positiveLeft = samplePath(positive, left, layouts, fallback);
  const positiveRight = samplePath(positive, right, layouts, fallback);
  const negativeMiddle = samplePath(negative, parameter, layouts, fallback);
  const positiveMiddle = samplePath(positive, parameter, layouts, fallback);
  const center = midpoint(negativeMiddle, positiveMiddle);
  const color = BYRNE[(atom + 1) % BYRNE.length] ?? BYRNE[0];
  const ribbon = element("g", {
    class: dense
      ? "steiner-diagram__ribbon steiner-diagram__ribbon--lane"
      : "steiner-diagram__ribbon steiner-diagram__ribbon--surface",
    style: `--face-color: ${color}`,
  });
  const title = element("title", {});
  title.textContent = boundaryFormula(boundary, atom, endpoints);
  ribbon.append(title);
  if (!dense) {
    ribbon.append(
      element("polygon", {
        points: pointList([negativeLeft, negativeRight, positiveRight, positiveLeft]),
        fill: color,
        "fill-opacity": "0.5",
        stroke: BYRNE[3],
        "stroke-width": "1.25",
        class: "steiner-diagram__face-tile",
      }),
    );
  }
  if (distance(negativeMiddle, positiveMiddle) > 18) {
    const inset = Math.min(dense ? 8 : 14, distance(negativeMiddle, positiveMiddle) / 3);
    const from = toward(negativeMiddle, positiveMiddle, inset);
    const to = toward(positiveMiddle, negativeMiddle, inset);
    if (dense) {
      // In a dense mapping category, a transformation may pass across
      // unrelated parallel 1-cells.  A paper-colored clearance stroke gives
      // that crossing an explicit overpass convention instead of allowing a
      // filled face to obscure the strand.
      ribbon.append(
        element("path", {
          d: `M ${from.x} ${from.y} L ${to.x} ${to.y}`,
          class: "steiner-diagram__face-clearance",
        }),
      );
    }
    ribbon.append(
      element("path", {
        d: rightDoubleArrowPath(from, to),
        class: "steiner-diagram__face-direction",
      }),
    );
  }
  const label = element("text", {
    x: String(center.x + (dense ? 10 : 8)),
    y: String(center.y - (dense ? 9 : 7)),
    class: "steiner-diagram__face-label",
  });
  label.textContent = `α${atom}`;
  ribbon.append(label);
  group.append(ribbon);
}

function renderFaces(
  svg: SVGSVGElement,
  category: SteinerDiagramRecord,
  layouts: ArrowLayout[],
  points: Map<number, Point>,
  laneMode: boolean,
): void {
  const endpoints = arrowEndpoints(category);
  const boundaries = category.boundaries[2] ?? [];
  const surfaces = element("g", { class: "steiner-diagram__surfaces" });
  if (!laneMode) {
    boundaries.forEach((boundary, atom) => {
      renderFaceMembrane(surfaces, atom, boundary, endpoints, layouts, points);
    });
  }
  boundaries.forEach((boundary, atom) => {
    renderFaceRibbon(
      surfaces,
      atom,
      boundaries.length,
      laneMode,
      boundary,
      endpoints,
      layouts,
      points,
    );
  });
  svg.append(surfaces);
}

function renderArrows(svg: SVGSVGElement, layouts: ArrowLayout[]): void {
  const arrows = element("g", { class: "steiner-diagram__arrows" });
  const parallelCounts = new Map<string, number>();
  for (const layout of layouts) {
    const key = `${layout.source}:${layout.target}`;
    parallelCounts.set(key, (parallelCounts.get(key) ?? 0) + 1);
  }
  for (const layout of layouts) {
    const start = quadraticPoint(layout, 0.035);
    const finish = quadraticPoint(layout, 0.955);
    const path = element("path", {
      d: `M ${start.x} ${start.y} Q ${layout.control.x} ${layout.control.y} ${finish.x} ${finish.y}`,
      stroke: layout.color,
      "marker-end": `url(#steiner-arrow-${layout.arrow})`,
    });
    const title = element("title", {});
    title.textContent = `a${layout.arrow}: v${layout.source} → v${layout.target}`;
    path.append(title);
    const labelParameter =
      (parallelCounts.get(`${layout.source}:${layout.target}`) ?? 0) > 1 ? 0.1 : 0.5;
    const labelPoint = quadraticPoint(layout, labelParameter);
    const label = element("text", {
      x: String(labelPoint.x),
      y: String(labelPoint.y - 10),
      fill: layout.color,
      "text-anchor": "middle",
    });
    label.textContent = `a${layout.arrow}`;
    arrows.append(path, label);
  }
  svg.append(arrows);
}

function interactiveObject(
  object: number,
  point: Point,
  selected: number[],
  selectObject: (object: number) => void,
  interactive: boolean,
): SVGGElement {
  const attributes: Record<string, string> = {
    transform: `translate(${point.x} ${point.y})`,
    "data-object": String(object),
    class: selected.includes(object) ? "is-selected" : "",
  };
  if (interactive) {
    attributes.tabindex = "0";
    attributes.role = "button";
    attributes["aria-label"] = `Select object v${object}`;
  }
  const group = element("g", attributes);
  group.append(element("circle", { r: String(OBJECT_RADIUS) }));
  const label = element("text", { y: "4", "text-anchor": "middle" });
  label.textContent = `v${object}`;
  group.append(label);
  if (interactive) {
    group.addEventListener("click", () => selectObject(object));
    group.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        selectObject(object);
      }
    });
  }
  return group;
}

function renderObjects(
  svg: SVGSVGElement,
  points: Map<number, Point>,
  selected: number[],
  selectObject: (object: number) => void,
  interactive: boolean,
): void {
  const objects = element("g", { class: "steiner-diagram__objects" });
  for (const [object, point] of points) {
    objects.append(interactiveObject(object, point, selected, selectObject, interactive));
  }
  svg.append(objects);
}

function renderFaceRegister(svg: SVGSVGElement, category: SteinerDiagramRecord): void {
  const boundaries = category.boundaries[2] ?? [];
  if (boundaries.length === 0) return;
  const endpoints = arrowEndpoints(category);
  const columns = boundaries.length <= 4 ? 2 : 3;
  const rows = Math.ceil(boundaries.length / columns);
  const top = SCENE_HEIGHT + 20;
  svg.append(
    element("path", {
      d: `M 62 ${top - 14} L ${WIDTH - 62} ${top - 14}`,
      class: "steiner-diagram__register-rule",
    }),
  );
  boundaries.forEach((boundary, atom) => {
    const column = atom % columns;
    const row = Math.floor(atom / columns);
    const columnWidth = (WIDTH - 124) / columns;
    const x = 62 + column * columnWidth;
    const y = top + row * 28;
    const mark = element("rect", {
      x: String(x),
      y: String(y - 10),
      width: "18",
      height: "8",
      fill: BYRNE[(atom + 1) % BYRNE.length] ?? BYRNE[0],
    });
    const formula = element("text", {
      x: String(x + 28),
      y: String(y - 2),
      class: "steiner-diagram__formula",
    });
    formula.textContent = boundaryFormula(boundary, atom, endpoints);
    svg.append(mark, formula);
  });
  const caption = element("text", {
    x: String(WIDTH - 62),
    y: String(top + rows * 28 + 1),
    "text-anchor": "end",
    class: "steiner-diagram__plate-note",
  });
  caption.textContent = "ι₂(C) · GENERATORS THROUGH DIMENSION TWO · AFTER BYRNE";
  svg.append(caption);
}

export function steinerDiagram(
  category: SteinerDiagramRecord,
  selected: number[],
  selectObject: (object: number) => void,
  options: SteinerDiagramOptions = {},
): SVGSVGElement {
  const faceCount = category.boundaries[2]?.length ?? 0;
  const laneMode = useFaceLanes(category, options.thumbnail ?? false);
  const columns = faceCount <= 4 ? 2 : 3;
  const registerRows = faceCount === 0 ? 0 : Math.ceil(faceCount / columns);
  const height = options.thumbnail
    ? SCENE_HEIGHT
    : SCENE_HEIGHT + (faceCount === 0 ? 0 : 44 + registerRows * 28);
  const svg = element("svg", {
    viewBox: `0 0 ${WIDTH} ${height}`,
    role: "img",
    "aria-label": `Display of the 2-dimensional view iota 2 of a strong Steiner category, with ${category.invariants.object_count} objects, ${category.invariants.generating_arrow_count} generating arrows, and ${faceCount} generating 2-cells`,
    class:
      "steiner-diagram steiner-diagram--byrne" +
      (options.thumbnail ? " steiner-diagram--thumbnail" : ""),
  });
  if (category.invariants.is_empty) {
    const empty = element("text", {
      x: String(WIDTH / 2),
      y: String(height / 2),
      "text-anchor": "middle",
      class: "hasse__empty",
    });
    empty.textContent = "∅";
    svg.append(empty);
    return svg;
  }
  const points = layoutObjects(category);
  const layouts = layoutArrows(category, points);
  appendDefinitions(svg, layouts);
  if (laneMode) {
    // Dense 2-cell lanes use a crossing overpass, so they must be painted
    // above the 1-skeleton. Objects remain above both layers.
    renderArrows(svg, layouts);
    renderFaces(svg, category, layouts, points, laneMode);
  } else {
    renderFaces(svg, category, layouts, points, laneMode);
    renderArrows(svg, layouts);
  }
  renderObjects(svg, points, selected, selectObject, !options.thumbnail);
  if (!options.thumbnail) renderFaceRegister(svg, category);
  return svg;
}
