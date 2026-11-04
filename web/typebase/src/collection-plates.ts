type JsonObject = Record<string, unknown>;

function asObject(value: unknown): JsonObject {
  return typeof value === "object" && value !== null ? (value as JsonObject) : {};
}

function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : [];
}

function number(value: unknown): number {
  return typeof value === "number" ? value : Number(value) || 0;
}

export function complexPlate(vertices: number, facets: number[][]): string {
  if (vertices === 0) {
    return '<svg class="incidence-plate" viewBox="0 0 520 300" role="img" aria-label="Empty complex"><text x="260" y="150">∅</text></svg>';
  }
  if (facets.some((facet) => facet.length > 3) || facets.length > 8) {
    return complexFacetIncidencePlate(vertices, facets);
  }
  const center = { x: 260, y: 150 };
  const radius = Math.min(115, 38 + vertices * 13);
  const points = Array.from({ length: vertices }, (_, index) => {
    const angle = -Math.PI / 2 + (2 * Math.PI * index) / vertices;
    return {
      x: center.x + radius * Math.cos(angle),
      y: center.y + radius * Math.sin(angle),
    };
  });
  const palette = ["#d94b32", "#196d84", "#e3a51a", "#7c5ca7", "#4d8a51", "#cc6b96"];
  const faces = facets
    .slice()
    .sort((left, right) => right.length - left.length)
    .map((facet, index) => {
      const selected = facet.map((vertex) => points[vertex]).filter(Boolean);
      const color = palette[index % palette.length];
      if (selected.length >= 3) {
        return `<polygon points="${selected.map((point) => `${point?.x},${point?.y}`).join(" ")}" fill="${color}" stroke="${color}" />`;
      }
      if (selected.length === 2) {
        return `<line x1="${selected[0]?.x}" y1="${selected[0]?.y}" x2="${selected[1]?.x}" y2="${selected[1]?.y}" stroke="${color}" />`;
      }
      return "";
    })
    .join("");
  const nodes = points
    .map(
      (point, index) =>
        `<g><circle cx="${point.x}" cy="${point.y}" r="10"/><text x="${point.x}" y="${point.y + 4}">${index}</text></g>`,
    )
    .join("");
  return `<svg class="incidence-plate" viewBox="0 0 520 300" role="img" aria-label="Canonical facet incidence">${faces}${nodes}</svg>`;
}

function complexFacetIncidencePlate(vertices: number, facets: number[][]): string {
  const height = Math.max(300, 52 + Math.max(vertices, facets.length) * 22);
  const vertexPoint = (vertex: number) => ({
    x: 105,
    y: 36 + ((vertex + 1) * (height - 72)) / (vertices + 1),
  });
  const facetPoint = (facet: number) => ({
    x: 515,
    y: 36 + ((facet + 1) * (height - 72)) / (facets.length + 1),
  });
  const lines = facets
    .flatMap((facet, facetIndex) =>
      facet.map((vertex) => {
        const source = vertexPoint(vertex);
        const target = facetPoint(facetIndex);
        return `<path d="M ${source.x + 11} ${source.y} C 240 ${source.y}, 380 ${target.y}, ${target.x - 23} ${target.y}"/>`;
      }),
    )
    .join("");
  const vertexNodes = Array.from({ length: vertices }, (_, vertex) => {
    const point = vertexPoint(vertex);
    return `<g><circle cx="${point.x}" cy="${point.y}" r="10"/><text x="${point.x}" y="${point.y + 4}">${vertex}</text></g>`;
  }).join("");
  const facetNodes = facets
    .map((facet, index) => {
      const point = facetPoint(index);
      return `<g class="facet-node"><rect x="${point.x - 22}" y="${point.y - 10}" width="44" height="20"/><text x="${point.x}" y="${point.y + 4}">F${index}</text><title>F${index} = {${facet.join(", ")}}</title></g>`;
    })
    .join("");
  return `<svg class="incidence-plate facet-incidence-plate" viewBox="0 0 620 ${height}" role="img" aria-label="Canonical vertex-to-facet incidence"><text class="plate-column-label" x="105" y="19">vertices</text><text class="plate-column-label" x="515" y="19">facets</text><g class="incidence-lines">${lines}</g>${vertexNodes}${facetNodes}</svg>`;
}

export function simplicialSetPlate(dimensions: unknown[]): string {
  const cells = dimensions.map(asArray);
  const maximum = Math.max(1, cells.length - 1);
  const width = 620;
  const height = 330;
  const positions = new Map<string, { x: number; y: number }>();
  cells.forEach((level, degree) => {
    level.forEach((_cell, index) => {
      positions.set(`${degree}:${index}`, {
        x: 60 + (degree * (width - 120)) / maximum,
        y: 48 + ((index + 1) * (height - 96)) / (level.length + 1),
      });
    });
  });
  const lines: string[] = [];
  cells.forEach((level, degree) => {
    if (degree === 0) return;
    level.forEach((cell, index) => {
      const source = positions.get(`${degree}:${index}`);
      for (const face of asArray(cell)) {
        const descriptor = asObject(face);
        const targetDegree =
          descriptor.kind === "cell" ? degree - 1 : number(descriptor.simplex_dimension);
        const target = positions.get(`${targetDegree}:${number(descriptor.simplex)}`);
        if (source && target) {
          lines.push(
            `<path d="M ${source.x - 13} ${source.y} C ${(source.x + target.x) / 2} ${source.y}, ${(source.x + target.x) / 2} ${target.y}, ${target.x + 11} ${target.y}" class="${descriptor.kind === "cell" ? "" : "degenerate"}"/>`,
          );
        }
      }
    });
  });
  const nodes = [...positions]
    .map(([key, point]) => {
      const [degree, index] = key.split(":").map(Number);
      const shape =
        degree === 0
          ? `<circle cx="${point.x}" cy="${point.y}" r="12"/>`
          : degree === 1
            ? `<rect x="${point.x - 17}" y="${point.y - 10}" width="34" height="20" rx="10"/>`
            : degree === 2
              ? `<polygon points="${point.x},${point.y - 17} ${point.x - 17},${point.y + 13} ${point.x + 17},${point.y + 13}"/>`
              : `<rect x="${point.x - 14}" y="${point.y - 14}" width="28" height="28" transform="rotate(45 ${point.x} ${point.y})"/>`;
      return `<g class="cell cell-${degree}">${shape}<text x="${point.x}" y="${point.y + 4}">${index}</text></g>`;
    })
    .join("");
  const labels = cells
    .map(
      (_level, degree) =>
        `<text class="degree-label" x="${60 + (degree * (width - 120)) / maximum}" y="315">dimension ${degree}</text>`,
    )
    .join("");
  return `<svg class="incidence-plate simplicial-set-plate" viewBox="0 0 ${width} ${height}" role="img" aria-label="Nondegenerate cells and faces"><g class="face-lines">${lines.join("")}</g>${nodes}${labels}</svg>`;
}
