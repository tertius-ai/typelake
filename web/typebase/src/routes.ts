const LABEL = /^\d+\.-?\d+\.\d+$/;

function objectLabel(segment: string | undefined): string | null {
  if (!segment) return null;
  const decoded = decodeURIComponent(segment);
  return LABEL.test(decoded) ? decoded : null;
}

export function routeLabel(kind: "Poset" | "Steiner"): string | null {
  const segments = window.location.pathname.split("/").filter(Boolean);
  const index = segments.lastIndexOf(kind);
  return index >= 0 ? objectLabel(segments[index + 1]) : null;
}

export function posetPath(label: string): string {
  return `/Poset/${encodeURIComponent(label)}`;
}

export function steinerPath(label: string): string {
  return `/Steiner/${encodeURIComponent(label)}`;
}

export function referencePath(
  collection: "Posets" | "Steiner",
  page: "Source" | "Completeness",
): string {
  return `/${collection}/${page}`;
}

export const glossaryPath = "/Glossary";
