export interface SteinerManifest {
  component: string;
  status: "passed";
  maximum_atomic_size: number;
  category_count: number;
  empty_category_label: "0.-1.1";
  stores_labeled_categories: false;
  street_nerve_complete_through_degree: number;
  catalogue_path: string;
  gallery_index_path: string;
  interesting_path: string;
  gray_table_path: string;
  label_buckets: Array<{
    atomic_size: number;
    dimension: number;
    row_count: number;
  }>;
}

export interface HomologyGroup {
  degree: number;
  free_rank: number;
  torsion: string[];
}

export interface BoundaryRecord {
  negative: number[];
  positive: number[];
}

export interface CellRecord {
  dimension: number;
  negative_supports: number[][];
  positive_supports: number[][];
}

export interface CategoryRecord {
  key: string[];
  atom_counts: number[];
  boundaries: BoundaryRecord[][];
  invariants: {
    atomic_size: number;
    dimension: number;
    object_count: number;
    generating_arrow_count: number;
    boundary_nonzero_counts: number[];
    precedence_height: number;
    precedence_width: number;
    precedence_component_sizes: number[];
    generating_skeleton_component_sizes: number[];
    is_connected: boolean;
    exact_cell_counts: number[];
    is_empty: boolean;
    is_discrete: boolean;
    polygraphic_euler_characteristic: number;
    integral_polygraphic_homology: HomologyGroup[];
  };
  automorphism_group: {
    degree: number;
    generators: number[][];
    order: string;
    orbits: number[][];
    generators_verified: boolean;
    lmfdb_label?: string;
  };
  recognized_families: Array<{ family: string; parameter: number }>;
}

export interface CategorySearchRow {
  label: string;
  atomic_size: number;
  dimension: number;
  bucket_index: number;
  atom_counts: number[];
  object_count: number;
  generating_arrow_count: number;
  exact_cell_counts: number[];
  total_nonidentity_cells: number;
  is_empty: boolean;
  is_discrete: boolean;
  is_connected: boolean;
  is_self_dual: boolean;
  automorphism_order: string;
  automorphism_lmfdb_label: string | null;
  polygraphic_homology_signature: string;
  has_polygraphic_torsion: boolean;
  basepoint_orbit_count: number;
  maximum_homotopy_poset_order: number;
  has_interesting_homotopy_poset: boolean;
  street_simplex_counts: number[];
  recognized_families: string[];
}

export interface MappingRecord {
  mapping_category?: string[];
  mapping_atomic_size?: number;
  mapping_label: string | null;
  has_strong_steiner_model: boolean;
  source: CellRecord;
  target: CellRecord;
  relative_dimension: number;
  exact_cell_counts: number[];
  cells: CellRecord[][];
}

export interface BasepointRecord {
  basepoint_dimension: number;
  homotopy_degree: number;
  local_index: number;
  pairs: [CellRecord, CellRecord][];
  orbit_size: string;
  stabilizer_order: string;
  homotopy_poset_key: string[];
  homotopy_poset_order: number;
  description: string;
  homotopy_poset_label: string | null;
  homotopy_poset_summary: {
    shape: string;
    component_count: number;
    core_order: number;
    integral_homology: HomologyGroup[];
    has_nontrivial_reduced_homology: boolean;
    is_interesting: boolean;
  };
}

export interface HomotopyPosetRecord {
  key: string[];
  order: number;
  covers: [number, number][];
}

export interface CategoricalHomologyRecord {
  model: string;
  coefficient_monoid: string;
  complete_through_degree: number;
  simplex_counts: number[];
  thin_simplex_counts: number[];
}

export interface CategoricalHomotopyRecord {
  spectrum_model: string;
  infinite_loop_model: string;
  component_particle_number_complete_through: number;
  component_poset_slices: Array<{
    homotopy_degree: number;
    particle_number: number;
    poset_key?: string[];
    poset_order: number;
    covers: [number, number][];
    object_multisets: number[][];
  }>;
  component_poset_links: Array<{
    particle_number: number;
    poset_label: string | null;
  }>;
  higher_particle_sector_model: string;
}

export interface CategoryDetail {
  label: string;
  automorphism_lmfdb_label: string | null;
  category: CategoryRecord;
  dualities: Array<{
    dimensions: number[];
    target_label: string | null;
    is_self_dual: boolean;
    remains_strong_steiner: boolean;
  }>;
  mappings: MappingRecord[];
  basepoints: BasepointRecord[];
  homotopy_posets: HomotopyPosetRecord[];
  categorical_homology: CategoricalHomologyRecord;
  categorical_homotopy: CategoricalHomotopyRecord;
}

export interface GrayRow {
  left_name: string;
  right_name: string;
  left_label: string;
  right_label: string;
  product_label: string | null;
  product_key: string[];
  product_atom_counts: number[];
  within_complete_region: boolean;
}
