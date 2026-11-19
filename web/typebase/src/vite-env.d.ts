/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_TYPEBASE_DATA_URL?: string;
  readonly VITE_TYPEBASE_STEINER_DATA_URL?: string;
  readonly VITE_TYPEBASE_SIMPLICIAL_COMPLEX_DATA_URL?: string;
  readonly VITE_TYPEBASE_SIMPLICIAL_SET_DATA_URL?: string;
  readonly VITE_TYPEBASE_FINITE_GROUP_COHOMOLOGY_DATA_URL?: string;
  /** Undocumented alias of VITE_TYPEBASE_FINITE_GROUP_COHOMOLOGY_DATA_URL; home.ts reads it as a fallback. */
  readonly VITE_TYPEBASE_GROUP_COHOMOLOGY_DATA_URL?: string;
  readonly VITE_TYPEBASE_EILENBERG_MACLANE_DATA_URL?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
