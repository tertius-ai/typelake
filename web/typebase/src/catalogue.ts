import type {
  AsyncDuckDB,
  AsyncDuckDBConnection,
} from "@duckdb/duckdb-wasm";

import type { DetailRow, Filters, Manifest, SearchRow } from "./model";

const FAST_SEARCH_BYTES = 4_000_000;

function quote(value: string): string {
  return `'${value.replaceAll("'", "''")}'`;
}

function rows<T>(table: { toArray(): Array<{ toJSON(): unknown }> }): T[] {
  return table.toArray().map((row) => row.toJSON() as T);
}

export class Catalogue {
  private database?: AsyncDuckDB;
  private connection?: AsyncDuckDBConnection;
  private connectionPromise?: Promise<AsyncDuckDBConnection>;
  private landingPromise?: Promise<SearchRow[]>;
  private orderRows = new Map<number, Promise<SearchRow[]>>();
  private searchViewReady = false;
  private coreTableReady = false;

  constructor(
    readonly manifest: Manifest,
    readonly dataRoot: URL,
  ) {}

  private async ensureSearchView(): Promise<void> {
    if (this.searchViewReady) return;
    const connection = await this.ensureConnection();
    const files = this.manifest.outputs
      .filter((output) => output.kind === "search")
      .map((output) => quote(new URL(output.path, this.dataRoot).toString()))
      .join(", ");
    if (!files) throw new Error("The TypeBase manifest has no search projection");
    await connection.query(`CREATE VIEW posets AS SELECT * FROM read_parquet([${files}])`);
    this.searchViewReady = true;
  }

  async search(
    filters: Filters,
    offset: number,
    limit: number,
  ): Promise<{ count: number; records: SearchRow[] }> {
    if (offset === 0 && limit <= 30 && this.isUnfilteredPosetSearch(filters)) {
      return { count: this.manifest.poset_count, records: (await this.landing()).slice(0, limit) };
    }
    const fast = await this.fastSearch(filters, offset, limit);
    if (fast) return fast;
    await this.ensureSearchView();
    const connection = await this.ensureConnection();
    if (filters.scope === "cores") await this.ensureCoreTable(connection);
    const alias = filters.scope === "cores" ? "p." : "";
    const clauses = this.filterClauses(filters, alias);
    if (filters.scope === "cores") clauses.push("p.is_core");
    const where = clauses.length > 0 ? ` WHERE ${clauses.join(" AND ")}` : "";
    const source = filters.scope === "cores" ? "posets p" : "posets";
    const countResult = await connection.query(
      `SELECT count(*)::UINTEGER AS count FROM ${source}${where}`,
    );
    const count = rows<{ count: number }>(countResult)[0]?.count ?? 0;
    const selection = filters.scope === "cores" ? "p.*, f.fiber_count" : "*";
    const recordsSource =
      filters.scope === "cores"
        ? "posets p JOIN core_fibers f ON f.core_label = p.label"
        : "posets";
    const result = await connection.query(
      `SELECT ${selection} FROM ${recordsSource}${where} ORDER BY ${alias}"order", ${alias}dimension, ${alias}bucket_index LIMIT ${limit} OFFSET ${offset}`,
    );
    return { count, records: rows<SearchRow>(result) };
  }

  private async ensureCoreTable(connection: AsyncDuckDBConnection): Promise<void> {
    if (this.coreTableReady) return;
    await connection.query(
      "CREATE TEMP TABLE core_fibers AS SELECT core_label, count(*)::UINTEGER AS fiber_count FROM posets GROUP BY core_label",
    );
    this.coreTableReady = true;
  }

  async detail(label: string): Promise<DetailRow> {
    const parsed = /^(\d+)\.(-?\d+)\.(\d+)$/.exec(label);
    if (!parsed) throw new Error("That is not a TypeBase Poset label");
    const order = Number(parsed[1]);
    const dimension = Number(parsed[2]);
    const index = Number(parsed[3]);
    const bucket = this.manifest.label_buckets.find(
      (candidate) => candidate.order === order && candidate.dimension === dimension,
    );
    if (!bucket || index < 1 || index > bucket.row_count) {
      throw new Error(`No Poset has the label ${label}`);
    }
    const part = Math.floor((index - 1) / this.manifest.rows_per_detail_part);
    const path = `details/order=${order}/dimension=${dimension}/part-${String(part).padStart(5, "0")}.parquet`;
    const url = new URL(path, this.dataRoot).toString();
    const artifact = this.manifest.outputs.find((output) => output.path === path);
    if (!artifact) throw new Error(`The detail projection is missing ${label}`);
    const [{ parquetReadObjects }, { decompress }, response] = await Promise.all([
      import("hyparquet"),
      import("fzstd"),
      fetch(url),
    ]);
    if (!response.ok) throw new Error(`Poset data returned HTTP ${response.status}`);
    const file = await response.arrayBuffer();
    const row = (index - 1) % this.manifest.rows_per_detail_part;
    const records = await parquetReadObjects({
      file,
      compressors: { ZSTD: (input) => decompress(input) },
      columns: [
        "label",
        "core_label",
        "opposite_label",
        "automorphism_lmfdb_label",
        "poset_json",
        "order_complex_json",
      ],
      rowStart: row,
      rowEnd: row + 1,
    });
    const record = records[0] as DetailRow | undefined;
    if (!record || record.label !== label) {
      throw new Error(`The detail projection is missing ${label}`);
    }
    return record;
  }

  private isUnfilteredPosetSearch(filters: Filters): boolean {
    return (
      filters.scope === "posets" &&
      filters.label === "" &&
      filters.order === null &&
      filters.dimension === null &&
      !filters.connected &&
      !filters.selfDual &&
      !filters.lattice &&
      filters.homologyDegree === null &&
      filters.torsion === "any" &&
      !filters.nonzeroReducedHomology
    );
  }

  private async fastSearch(
    filters: Filters,
    offset: number,
    limit: number,
  ): Promise<{ count: number; records: SearchRow[] } | null> {
    if (filters.scope !== "posets") return null;
    if (this.isUnfilteredPosetSearch(filters)) return this.fastBrowse(offset, limit);
    const labelOrder = /^(\d+)/.exec(filters.label)?.[1];
    const order = filters.order ?? (labelOrder === undefined ? null : Number(labelOrder));
    if (order === null || !this.hasSmallSearchProjection(order)) return null;
    const matches = (await this.readOrder(order)).filter((row) => this.matches(row, filters));
    return { count: matches.length, records: matches.slice(offset, offset + limit) };
  }

  private async fastBrowse(
    offset: number,
    limit: number,
  ): Promise<{ count: number; records: SearchRow[] } | null> {
    const orderCounts = new Map<number, number>();
    for (const bucket of this.manifest.label_buckets) {
      orderCounts.set(bucket.order, (orderCounts.get(bucket.order) ?? 0) + bucket.row_count);
    }
    let skipped = offset;
    let needed = limit;
    const slices: Array<{ order: number; start: number; end: number }> = [];
    for (const [order, count] of [...orderCounts].sort(([left], [right]) => left - right)) {
      if (skipped >= count) {
        skipped -= count;
        continue;
      }
      const take = Math.min(needed, count - skipped);
      if (!this.hasSmallSearchProjection(order)) return null;
      slices.push({ order, start: skipped, end: skipped + take });
      needed -= take;
      skipped = 0;
      if (needed === 0) break;
    }
    const pages = await Promise.all(
      slices.map(async ({ order, start, end }) => (await this.readOrder(order)).slice(start, end)),
    );
    return { count: this.manifest.poset_count, records: pages.flat() };
  }

  private hasSmallSearchProjection(order: number): boolean {
    const artifact = this.searchArtifact(order);
    return artifact !== undefined && artifact.byte_count <= FAST_SEARCH_BYTES;
  }

  private searchArtifact(order: number) {
    const path = `search/order=${order}/posets.parquet`;
    return this.manifest.outputs.find((output) => output.kind === "search" && output.path === path);
  }

  private readOrder(order: number): Promise<SearchRow[]> {
    const existing = this.orderRows.get(order);
    if (existing) return existing;
    const artifact = this.searchArtifact(order);
    if (!artifact) return Promise.reject(new Error(`Search order ${order} is unavailable`));
    const request = this.readParquet<SearchRow>(artifact.path, [
      "label",
      "order",
      "dimension",
      "bucket_index",
      "cover_count",
      "comparable_pair_count",
      "minimal_count",
      "maximal_count",
      "component_count",
      "height",
      "width",
      "core_size",
      "is_connected",
      "is_lattice",
      "is_core",
      "is_self_dual",
      "automorphism_order",
      "automorphism_lmfdb_label",
      "core_label",
      "opposite_label",
      "homology_signature",
      "has_torsion",
    ]).then((records) =>
      records.sort(
        (left, right) => left.dimension - right.dimension || left.bucket_index - right.bucket_index,
      ),
    );
    this.orderRows.set(order, request);
    return request;
  }

  private async readParquet<T>(path: string, columns: string[]): Promise<T[]> {
    const url = new URL(path, this.dataRoot).toString();
    const [{ parquetReadObjects }, { decompress }, response] = await Promise.all([
      import("hyparquet"),
      import("fzstd"),
      fetch(url),
    ]);
    if (!response.ok) throw new Error(`Poset data returned HTTP ${response.status}`);
    const records = await parquetReadObjects({
      file: await response.arrayBuffer(),
      compressors: { ZSTD: (input) => decompress(input) },
      columns,
    });
    return records as T[];
  }

  private matches(row: SearchRow, filters: Filters): boolean {
    if (filters.label && !(row.label === filters.label || row.label.startsWith(`${filters.label}.`))) return false;
    if (filters.order !== null && row.order !== filters.order) return false;
    if (filters.dimension !== null && row.dimension !== filters.dimension) return false;
    if (filters.connected && !row.is_connected) return false;
    if (filters.selfDual && !row.is_self_dual) return false;
    if (filters.lattice && !row.is_lattice) return false;
    const groups = row.homology_signature
      .split(";")
      .filter(Boolean)
      .map((group) => {
        const [degree, freeRank, torsion = ""] = group.split(":");
        return { degree: Number(degree), freeRank: Number(freeRank), torsion };
      });
    if (filters.homologyDegree !== null) {
      const group = groups.find((candidate) => candidate.degree === filters.homologyDegree);
      if ((group?.freeRank ?? 0) < filters.minimumBetti) return false;
    }
    const hasTorsion =
      filters.homologyDegree === null
        ? row.has_torsion
        : (groups.find((group) => group.degree === filters.homologyDegree)?.torsion.length ?? 0) > 0;
    if (filters.torsion === "present" && !hasTorsion) return false;
    if (filters.torsion === "absent" && hasTorsion) return false;
    if (
      filters.nonzeroReducedHomology &&
      !groups.some(
        (group) =>
          (group.degree === 0 && group.freeRank > 1) ||
          (group.degree > 0 && (group.freeRank > 0 || group.torsion.length > 0)),
      )
    ) {
      return false;
    }
    return true;
  }

  private landing(): Promise<SearchRow[]> {
    this.landingPromise ??= fetch(
      new URL(this.manifest.landing_path ?? "landing.json", this.dataRoot),
    ).then(async (response) => {
      if (!response.ok) throw new Error(`Landing data returned HTTP ${response.status}`);
      return (await response.json()) as SearchRow[];
    });
    return this.landingPromise;
  }

  private filterClauses(filters: Filters, prefix: string): string[] {
    const clauses: string[] = [];
    const column = (name: string) => `${prefix}${name}`;
    if (filters.label) {
      if (!/^\d+(?:\.-?\d+)?(?:\.\d+)?$/.test(filters.label)) {
        throw new Error("Labels contain two or three numbers separated by periods");
      }
      clauses.push(`(${column("label")} = ${quote(filters.label)} OR ${column("label")} LIKE ${quote(`${filters.label}.%`)})`);
    }
    if (filters.order !== null) clauses.push(`${column('"order"')} = ${filters.order}`);
    if (filters.dimension !== null) clauses.push(`${column("dimension")} = ${filters.dimension}`);
    if (filters.connected) clauses.push(column("is_connected"));
    if (filters.selfDual) clauses.push(column("is_self_dual"));
    if (filters.lattice) clauses.push(column("is_lattice"));
    const signature = column("homology_signature");
    if (filters.homologyDegree !== null) {
      const pattern = `(^|;)${filters.homologyDegree}:([0-9]+):`;
      clauses.push(
        `COALESCE(TRY_CAST(regexp_extract(${signature}, ${quote(pattern)}, 2) AS INTEGER), 0) >= ${filters.minimumBetti}`,
      );
    }
    if (filters.torsion !== "any") {
      const hasTorsion =
        filters.homologyDegree === null
          ? column("has_torsion")
          : `regexp_matches(${signature}, ${quote(`(^|;)${filters.homologyDegree}:[0-9]+:[^;]+`)})`;
      clauses.push(filters.torsion === "present" ? hasTorsion : `NOT (${hasTorsion})`);
    }
    if (filters.nonzeroReducedHomology) {
      const betaZero = `COALESCE(TRY_CAST(regexp_extract(${signature}, '(^|;)0:([0-9]+):', 2) AS INTEGER), 0)`;
      clauses.push(
        `(${betaZero} > 1 OR regexp_matches(${signature}, '(^|;)[1-9][0-9]*:[1-9][0-9]*:') OR regexp_matches(${signature}, '(^|;)[1-9][0-9]*:[0-9]+:[^;]+'))`,
      );
    }
    return clauses;
  }

  private ensureConnection(): Promise<AsyncDuckDBConnection> {
    this.connectionPromise ??= this.openConnection();
    return this.connectionPromise;
  }

  private async openConnection(): Promise<AsyncDuckDBConnection> {
    const [duckdb, mainModule, mainWorker] = await Promise.all([
      import("@duckdb/duckdb-wasm"),
      import("@duckdb/duckdb-wasm/dist/duckdb-eh.wasm?url"),
      import("@duckdb/duckdb-wasm/dist/duckdb-browser-eh.worker.js?url"),
    ]);
    const worker = new Worker(mainWorker.default);
    this.database = new duckdb.AsyncDuckDB(new duckdb.VoidLogger(), worker);
    await this.database.instantiate(mainModule.default, null);
    this.connection = await this.database.connect();
    return this.connection;
  }
}
