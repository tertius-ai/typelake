import type { PosetRecord } from "./model";

interface Point {
  x: number;
  y: number;
}

interface CubicCurve {
  start: Point;
  controlStart: Point;
  controlFinish: Point;
  finish: Point;
}

const SVG_NS = "http://www.w3.org/2000/svg";

function element<K extends keyof SVGElementTagNameMap>(
  name: K,
  attributes: Record<string, string>,
): SVGElementTagNameMap[K] {
  const node = document.createElementNS(SVG_NS, name);
  for (const [key, value] of Object.entries(attributes)) node.setAttribute(key, value);
  return node;
}

function heightColumns(poset: PosetRecord): number[][] {
  const grouped = new Map<number, number[]>();
  poset.invariants.strict_lower_counts.forEach((count, vertex) => {
    const column = grouped.get(count) ?? [];
    column.push(vertex);
    grouped.set(count, column);
  });
  const maximumHeight = Math.max(0, ...poset.invariants.strict_lower_counts);
  // Keep empty heights: horizontal position records the actual number of
  // strict predecessors rather than merely the order of occupied buckets.
  const columns = Array.from(
    { length: maximumHeight + 1 },
    (_, height) => grouped.get(height) ?? [],
  );

  const position = new Map<number, number>();
  const refresh = () =>
    columns.forEach((column) =>
      column.forEach((vertex, index) => position.set(vertex, index / Math.max(column.length - 1, 1))),
    );
  const average = (neighbors: number[], fallback: number) =>
    neighbors.length === 0
      ? fallback
      : neighbors.reduce((sum, vertex) => sum + (position.get(vertex) ?? 0), 0) /
        neighbors.length;

  refresh();
  for (let sweep = 0; sweep < 4; sweep += 1) {
    const leftToRight = sweep % 2 === 0;
    const ordered = leftToRight ? columns : [...columns].reverse();
    for (const column of ordered) {
      column.sort((left, right) => {
        const neighbors = (vertex: number) =>
          poset.covers
            .filter((edge) => (leftToRight ? edge[1] === vertex : edge[0] === vertex))
            .map((edge) => (leftToRight ? edge[0] : edge[1]));
        const comparison =
          average(neighbors(left), position.get(left) ?? 0) -
          average(neighbors(right), position.get(right) ?? 0);
        return comparison || left - right;
      });
      refresh();
    }
  }
  return columns;
}

function cubicPoint(curve: CubicCurve, time: number): Point {
  const remaining = 1 - time;
  return {
    x:
      remaining ** 3 * curve.start.x +
      3 * remaining ** 2 * time * curve.controlStart.x +
      3 * remaining * time ** 2 * curve.controlFinish.x +
      time ** 3 * curve.finish.x,
    y:
      remaining ** 3 * curve.start.y +
      3 * remaining ** 2 * time * curve.controlStart.y +
      3 * remaining * time ** 2 * curve.controlFinish.y +
      time ** 3 * curve.finish.y,
  };
}

function routedCurve(
  start: Point,
  finish: Point,
  lower: number,
  upper: number,
  points: Map<number, Point>,
): CubicCurve {
  const midpoint = (start.y + finish.y) / 2;
  const candidates = [midpoint, start.y, finish.y];
  for (let y = 30; y <= 490; y += 20) candidates.push(y);
  for (const [vertex, point] of points) {
    if (vertex !== lower && vertex !== upper) {
      candidates.push(Math.max(24, point.y - 52), Math.min(496, point.y + 52));
    }
  }
  const unique = [...new Set(candidates.map((candidate) => Math.round(candidate * 10) / 10))];
  const horizontal = finish.x - start.x;
  let best: { curve: CubicCurve; score: number } | undefined;
  for (const lane of unique) {
    const curve: CubicCurve = {
      start,
      controlStart: { x: start.x + horizontal * 0.34, y: lane },
      controlFinish: { x: finish.x - horizontal * 0.34, y: lane },
      finish,
    };
    let collisions = 0;
    let closest = Number.POSITIVE_INFINITY;
    for (let step = 2; step < 59; step += 1) {
      const sample = cubicPoint(curve, step / 60);
      for (const [vertex, point] of points) {
        if (vertex === lower || vertex === upper) continue;
        const distance = Math.hypot(sample.x - point.x, sample.y - point.y);
        closest = Math.min(closest, distance);
        if (distance < 31) collisions += 1;
      }
    }
    const score =
      collisions * 1_000_000 +
      Math.abs(lane - midpoint) * 2 +
      Math.abs(lane - start.y) * 0.18 +
      Math.abs(lane - finish.y) * 0.18 -
      Math.min(closest, 80) * 0.03;
    if (!best || score < best.score) best = { curve, score };
  }
  return best?.curve ?? {
    start,
    controlStart: { x: start.x + horizontal * 0.34, y: midpoint },
    controlFinish: { x: finish.x - horizontal * 0.34, y: midpoint },
    finish,
  };
}

export function hasseDiagram(poset: PosetRecord): SVGSVGElement {
  const svg = element("svg", {
    viewBox: "0 0 860 520",
    role: "img",
    "aria-label": `Hasse diagram of a Poset on ${poset.order} points`,
    class: "hasse",
  });
  if (poset.order === 0) {
    const empty = element("text", { x: "430", y: "272", "text-anchor": "middle", class: "hasse__empty" });
    empty.textContent = "∅";
    svg.append(empty);
    return svg;
  }

  const columns = heightColumns(poset);
  const points = new Map<number, Point>();
  columns.forEach((column, columnIndex) => {
    const x = columns.length === 1 ? 430 : 75 + (710 * columnIndex) / (columns.length - 1);
    column.forEach((vertex, rowIndex) => {
      const y = column.length === 1 ? 260 : 70 + (380 * rowIndex) / (column.length - 1);
      points.set(vertex, { x, y });
    });
  });

  const edges = element("g", { class: "hasse__edges" });
  for (const [lower, upper] of poset.covers) {
    const start = points.get(lower);
    const finish = points.get(upper);
    if (!start || !finish) continue;
    const curve = routedCurve(start, finish, lower, upper, points);
    edges.append(
      element("path", {
        d: `M ${curve.start.x} ${curve.start.y} C ${curve.controlStart.x} ${curve.controlStart.y}, ${curve.controlFinish.x} ${curve.controlFinish.y}, ${curve.finish.x} ${curve.finish.y}`,
      }),
    );
  }
  svg.append(edges);

  const vertices = element("g", { class: "hasse__vertices" });
  for (let vertex = 0; vertex < poset.order; vertex += 1) {
    const point = points.get(vertex);
    if (!point) continue;
    const group = element("g", {
      transform: `translate(${point.x} ${point.y})`,
      "data-vertex": String(vertex),
      "data-height": String(poset.invariants.strict_lower_counts[vertex] ?? 0),
    });
    group.append(element("circle", { r: "20" }));
    const label = element("text", { y: "5", "text-anchor": "middle" });
    label.textContent = String(vertex);
    group.append(label);
    vertices.append(group);
  }
  svg.append(vertices);
  return svg;
}
