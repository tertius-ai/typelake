export interface Artifact {
  kind: "search" | "details" | "landing";
  path: string;
  row_count: number;
  byte_count: number;
  sha256: string;
}

export interface LabelBucket {
  order: number;
  dimension: number;
  row_count: number;
  detail_part_count: number;
}

export interface Manifest {
  component: string;
  status: "passed";
  volume: number;
  collection: string;
  maximum_order: number;
  label_scheme: string;
  rows_per_detail_part: number;
  landing_path?: string;
  poset_count: number;
  stores_labeled_posets: false;
  label_buckets: LabelBucket[];
  outputs: Artifact[];
}

export interface SearchRow {
  label: string;
  order: number;
  dimension: number;
  bucket_index: number;
  cover_count: number;
  comparable_pair_count: number;
  minimal_count: number;
  maximal_count: number;
  component_count: number;
  height: number;
  width: number;
  core_size: number;
  is_connected: boolean;
  is_lattice: boolean;
  is_core: boolean;
  is_self_dual: boolean;
  automorphism_order: string;
  automorphism_lmfdb_label: string | null;
  core_label: string;
  opposite_label: string;
  homology_signature: string;
  has_torsion: boolean;
  fiber_count?: number;
}

export interface DetailRow {
  label: string;
  core_label: string;
  opposite_label: string;
  automorphism_lmfdb_label: string | null;
  poset_json: string;
  order_complex_json: string;
}

export interface PosetRecord {
  key: string[];
  order: number;
  covers: [number, number][];
  invariants: {
    cover_count: number;
    strict_comparable_pair_count: number;
    strict_lower_counts: number[];
    strict_upper_counts: number[];
    minimal_count: number;
    maximal_count: number;
    component_count: number;
    component_sizes: number[];
    is_connected: boolean;
    height: number;
    width: number;
    is_lattice: boolean;
  };
  automorphism_group: {
    degree: number;
    generators: number[][];
    order: string;
    orbits: number[][];
    is_transitive: boolean;
    generators_verified: boolean;
  };
  relations: {
    has_core: {
      core_size: number;
      is_core: boolean;
      deletions: unknown[];
      retained_vertices: number[];
    };
    opposite: { is_self_dual: boolean };
  };
}

export interface ComplexRecord {
  key: string;
  vertex_count: number;
  facets: number[][];
  is_empty: boolean;
  dimension: number;
  f_vector: number[];
  euler_characteristic: number;
  integral_homology: Array<{
    degree: number;
    free_rank: number;
    invariant_factors: string[];
  }>;
}

export interface Filters {
  scope: "posets" | "cores";
  label: string;
  order: number | null;
  dimension: number | null;
  connected: boolean;
  selfDual: boolean;
  lattice: boolean;
  homologyDegree: number | null;
  minimumBetti: number;
  torsion: "any" | "present" | "absent";
  nonzeroReducedHomology: boolean;
}
