import nameCatalogue from "../../../collections/finite-group-cohomology/sources/standard-names-through-32.json";

interface StandardNameRow {
  lmfdb_label: string;
  standard_name: string;
}

const expectedContract = "finite-group-standard-names/gap-structure-description-v1";
if (nameCatalogue.contract !== expectedContract) {
  throw new Error("The finite-group standard-name catalogue has an unsupported contract");
}

const names = new Map(
  (nameCatalogue.names as StandardNameRow[]).map((row) => [row.lmfdb_label, row.standard_name]),
);

export function standardGroupName(label: unknown, supplied?: unknown): string {
  const explicit = String(supplied ?? "").trim();
  if (explicit) return explicit;
  return names.get(String(label)) ?? String(label);
}

export function displayGroupName(name: unknown): string {
  return String(name).replaceAll(" x ", " × ");
}
