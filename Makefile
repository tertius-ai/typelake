SHELL := /bin/bash

LOCAL_NOETHER_FLINT := $(abspath $(firstword $(wildcard ../noether-poset/.deps/flint ../noether/.deps/flint)))
FLINT_PREFIX ?= $(LOCAL_NOETHER_FLINT)
CANDIDATE_CAP ?= 1000000
BENCH_MAX_ORDER ?= 6
BENCH_CANDIDATE_CAP ?= 100000000
ORDERLY_BENCH_MAX_ORDER ?= 7
ORDERLY_BENCH_ORDER ?= 8
ORDERLY_CANDIDATE_CAP ?= 10000000
SHARD_COUNT ?= 1
SHARD_ID ?= 0
CHECKPOINT_EVERY ?= 1000

ifneq ($(strip $(FLINT_PREFIX)),)
export PKG_CONFIG_PATH := $(FLINT_PREFIX)/lib/pkgconfig$(if $(PKG_CONFIG_PATH),:$(PKG_CONFIG_PATH))
export LD_LIBRARY_PATH := $(FLINT_PREFIX)/lib$(if $(LD_LIBRARY_PATH),:$(LD_LIBRARY_PATH))
export DYLD_LIBRARY_PATH := $(FLINT_PREFIX)/lib$(if $(DYLD_LIBRARY_PATH),:$(DYLD_LIBRARY_PATH))
endif
export CARGO_NET_GIT_FETCH_WITH_CLI ?= true

.PHONY: check-flint fmt check lint test test-oracles finite-group-integral finite-group-modular identify-poset-groups identify-steiner-groups typebase-finite-groups typebase-posets typebase-steiner typebase-simplicial-complexes typebase-simplicial-sets typebase-eilenberg-maclane typebase-web bench-posets bench-pipeline bench-orderly bench-orderly-generation bench-steiner-mappings reference reference-order-3 orderly orderly-shard merge-orderly-shards certify-orderly-shard publish-certified-merge steiner-compare steiner-counts steiner-identity steiner-light steiner-mappings steiner-rich steiner-orderly steiner-gray

check-flint:
	@pkg-config --atleast-version=3.6 flint || { \
		echo "poset-census requires FLINT 3.6+; set FLINT_PREFIX to a compatible installation" >&2; \
		exit 1; \
	}

fmt:
	cargo fmt --all

check: check-flint
	cargo check --workspace

lint: check-flint
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings

test: check-flint
	cargo test --workspace

test-oracles: check-flint
	cargo test --release -p poset-census --test reference -- --ignored
	cargo run --release -p steiner-census -- compare --maximum-atomic-size 8 --candidate-cap 20000000

finite-group-integral: check-flint
	@test -n "$(FINITE_GROUP_COHOMOLOGY_DATA_ROOT)" || { echo "set FINITE_GROUP_COHOMOLOGY_DATA_ROOT explicitly" >&2; exit 1; }
	cargo run --release -p finite-group-cohomology-census -- integral \
		--maximum-order "$(or $(FINITE_GROUP_MAXIMUM_ORDER),32)" \
		--additive-through-degree "$(or $(FINITE_GROUP_INTEGRAL_DEGREE),16)" \
		--bar-entry-limit "$(or $(FINITE_GROUP_INTEGRAL_BAR_ENTRY_LIMIT),100000)" \
		--data-root "$(FINITE_GROUP_COHOMOLOGY_DATA_ROOT)"

finite-group-modular: check-flint
	@test -n "$(FINITE_GROUP_COHOMOLOGY_DATA_ROOT)" || { echo "set FINITE_GROUP_COHOMOLOGY_DATA_ROOT explicitly" >&2; exit 1; }
	cargo run --release -p finite-group-cohomology-census -- modular \
		--maximum-order "$(or $(FINITE_GROUP_MAXIMUM_ORDER),32)" \
		--initial-proof-degree "$(or $(FINITE_GROUP_INITIAL_PROOF_DEGREE),8)" \
		--maximum-proof-degree "$(or $(FINITE_GROUP_MAXIMUM_PROOF_DEGREE),12)" \
		--data-root "$(FINITE_GROUP_COHOMOLOGY_DATA_ROOT)"

typebase-finite-groups: check-flint
	@test -n "$(FINITE_GROUP_COHOMOLOGY_DATA_ROOT)" || { echo "set FINITE_GROUP_COHOMOLOGY_DATA_ROOT explicitly" >&2; exit 1; }
	@test -n "$(TYPEBASE_FINITE_GROUP_OUTPUT_ROOT)" || { echo "set TYPEBASE_FINITE_GROUP_OUTPUT_ROOT explicitly" >&2; exit 1; }
	cargo run --release -p typebase-build -- finite-group-cohomology \
		--data-root "$(FINITE_GROUP_COHOMOLOGY_DATA_ROOT)" \
		--output-root "$(TYPEBASE_FINITE_GROUP_OUTPUT_ROOT)" \
		--maximum-order "$(or $(FINITE_GROUP_MAXIMUM_ORDER),32)" \
		--initial-proof-degree "$(or $(FINITE_GROUP_INITIAL_PROOF_DEGREE),8)" \
		--maximum-proof-degree "$(or $(FINITE_GROUP_MAXIMUM_PROOF_DEGREE),12)" \
		--integral-additive-through-degree "$(or $(FINITE_GROUP_INTEGRAL_DEGREE),16)" \
		--integral-bar-entry-limit "$(or $(FINITE_GROUP_INTEGRAL_BAR_ENTRY_LIMIT),100000)"

typebase-posets: check-flint
	@test -n "$(POSET_CENSUS_DATA_ROOT)" || { echo "set POSET_CENSUS_DATA_ROOT explicitly" >&2; exit 1; }
	@test -n "$(TYPEBASE_OUTPUT_ROOT)" || { echo "set TYPEBASE_OUTPUT_ROOT explicitly" >&2; exit 1; }
	cargo run --release -p typebase-build -- posets \
		--data-root "$(POSET_CENSUS_DATA_ROOT)" --output-root "$(TYPEBASE_OUTPUT_ROOT)" \
		--maximum-order "$(or $(TYPEBASE_MAXIMUM_ORDER),10)" \
		--rows-per-detail-part "$(or $(TYPEBASE_DETAIL_ROWS),10000)" \
		$(if $(TYPEBASE_GROUP_IDENTIFICATIONS),--group-identifications "$(TYPEBASE_GROUP_IDENTIFICATIONS)")

typebase-steiner: check-flint
	@test -n "$(STEINER_CENSUS_DATA_ROOT)" || { echo "set STEINER_CENSUS_DATA_ROOT explicitly" >&2; exit 1; }
	@test -n "$(POSET_CENSUS_DATA_ROOT)" || { echo "set POSET_CENSUS_DATA_ROOT explicitly" >&2; exit 1; }
	@test -n "$(TYPEBASE_STEINER_OUTPUT_ROOT)" || { echo "set TYPEBASE_STEINER_OUTPUT_ROOT explicitly" >&2; exit 1; }
	cargo run --release -p typebase-build -- steiner-categories \
		--data-root "$(STEINER_CENSUS_DATA_ROOT)" --output-root "$(TYPEBASE_STEINER_OUTPUT_ROOT)" \
		--poset-data-root "$(POSET_CENSUS_DATA_ROOT)" \
		--maximum-atomic-size "$(or $(TYPEBASE_STEINER_MAXIMUM_ATOMIC_SIZE),11)" \
		$(if $(STEINER_GROUP_IDENTIFICATIONS),--group-identifications "$(STEINER_GROUP_IDENTIFICATIONS)")

typebase-simplicial-complexes: check-flint
	@test -n "$(SIMPLICIAL_COMPLEX_DATA_ROOT)" || { echo "set SIMPLICIAL_COMPLEX_DATA_ROOT explicitly" >&2; exit 1; }
	@test -n "$(TYPEBASE_SIMPLICIAL_COMPLEX_OUTPUT_ROOT)" || { echo "set TYPEBASE_SIMPLICIAL_COMPLEX_OUTPUT_ROOT explicitly" >&2; exit 1; }
	cargo run --release -p typebase-build -- simplicial-complexes \
		--data-root "$(SIMPLICIAL_COMPLEX_DATA_ROOT)" \
		--output-root "$(TYPEBASE_SIMPLICIAL_COMPLEX_OUTPUT_ROOT)" \
		--maximum-vertices "$(or $(TYPEBASE_SIMPLICIAL_COMPLEX_MAXIMUM_VERTICES),6)"

typebase-simplicial-sets: check-flint
	@test -n "$(SIMPLICIAL_SET_DATA_ROOT)" || { echo "set SIMPLICIAL_SET_DATA_ROOT explicitly" >&2; exit 1; }
	@test -n "$(TYPEBASE_SIMPLICIAL_SET_OUTPUT_ROOT)" || { echo "set TYPEBASE_SIMPLICIAL_SET_OUTPUT_ROOT explicitly" >&2; exit 1; }
	cargo run --release -p typebase-build -- simplicial-sets \
		--data-root "$(SIMPLICIAL_SET_DATA_ROOT)" \
		--output-root "$(TYPEBASE_SIMPLICIAL_SET_OUTPUT_ROOT)" \
		--maximum-total-nondegenerate "$(or $(TYPEBASE_SIMPLICIAL_SET_MAXIMUM_TOTAL_NONDEGENERATE),7)" \
		--maximum-dimension "$(or $(TYPEBASE_SIMPLICIAL_SET_MAXIMUM_DIMENSION),3)" \
		--maximum-dimension-at-top-total "$(or $(TYPEBASE_SIMPLICIAL_SET_MAXIMUM_DIMENSION_AT_TOP_TOTAL),2)"

typebase-eilenberg-maclane: check-flint
	@test -n "$(EILENBERG_MACLANE_DATA_ROOT)" || { echo "set EILENBERG_MACLANE_DATA_ROOT explicitly" >&2; exit 1; }
	@test -n "$(TYPEBASE_EILENBERG_MACLANE_OUTPUT_ROOT)" || { echo "set TYPEBASE_EILENBERG_MACLANE_OUTPUT_ROOT explicitly" >&2; exit 1; }
	cargo run --release -p typebase-build -- eilenberg-mac-lane \
		--data-root "$(EILENBERG_MACLANE_DATA_ROOT)" \
		--output-root "$(TYPEBASE_EILENBERG_MACLANE_OUTPUT_ROOT)" \
		--maximum-finite-order "$(or $(TYPEBASE_EILENBERG_MACLANE_MAXIMUM_FINITE_ORDER),32)" \
		--maximum-delooping-degree "$(or $(TYPEBASE_EILENBERG_MACLANE_MAXIMUM_DELOOPING_DEGREE),4)" \
		--exact-through-degree "$(or $(TYPEBASE_EILENBERG_MACLANE_EXACT_THROUGH_DEGREE),16)"

steiner-compare: check-flint
	cargo run --release -p steiner-census -- compare \
		--maximum-atomic-size "$(or $(STEINER_MAXIMUM_ATOMIC_SIZE),7)" \
		--candidate-cap "$(or $(STEINER_CANDIDATE_CAP),20000000)"

steiner-counts: check-flint
	cargo run --release -p steiner-census -- counts \
		--maximum-atomic-size "$(or $(STEINER_MAXIMUM_ATOMIC_SIZE),7)" \
		--candidate-cap "$(or $(STEINER_CANDIDATE_CAP),20000000)"

steiner-identity: check-flint
	@test -n "$(STEINER_CENSUS_DATA_ROOT)" || { echo "set STEINER_CENSUS_DATA_ROOT explicitly" >&2; exit 1; }
	@test -n "$(ATOMIC_SIZE)" || { echo "set ATOMIC_SIZE explicitly" >&2; exit 1; }
	cargo run --release -p steiner-census -- identity --atomic-size "$(ATOMIC_SIZE)" \
		--data-root "$(STEINER_CENSUS_DATA_ROOT)" \
		--candidate-cap "$(or $(STEINER_CANDIDATE_CAP),20000000)"

steiner-light: check-flint
	@test -n "$(STEINER_CENSUS_DATA_ROOT)" || { echo "set STEINER_CENSUS_DATA_ROOT explicitly" >&2; exit 1; }
	@test -n "$(ATOMIC_SIZE)" || { echo "set ATOMIC_SIZE explicitly" >&2; exit 1; }
	cargo run --release -p steiner-census -- light --atomic-size "$(ATOMIC_SIZE)" \
		--data-root "$(STEINER_CENSUS_DATA_ROOT)"

steiner-mappings: check-flint
	@test -n "$(STEINER_CENSUS_DATA_ROOT)" || { echo "set STEINER_CENSUS_DATA_ROOT explicitly" >&2; exit 1; }
	@test -n "$(ATOMIC_SIZE)" || { echo "set ATOMIC_SIZE explicitly" >&2; exit 1; }
	cargo run --release -p steiner-census -- mappings --atomic-size "$(ATOMIC_SIZE)" \
		--data-root "$(STEINER_CENSUS_DATA_ROOT)"

steiner-rich: check-flint
	@test -n "$(STEINER_CENSUS_DATA_ROOT)" || { echo "set STEINER_CENSUS_DATA_ROOT explicitly" >&2; exit 1; }
	@test -n "$(ATOMIC_SIZE)" || { echo "set ATOMIC_SIZE explicitly" >&2; exit 1; }
	cargo run --release -p steiner-census -- rich --atomic-size "$(ATOMIC_SIZE)" \
		--data-root "$(STEINER_CENSUS_DATA_ROOT)" \
		--candidate-cap "$(or $(STEINER_CANDIDATE_CAP),20000000)" \
		--street-degree "$(or $(STEINER_STREET_DEGREE),2)"

steiner-orderly: check-flint
	@test -n "$(STEINER_CENSUS_DATA_ROOT)" || { echo "set STEINER_CENSUS_DATA_ROOT explicitly" >&2; exit 1; }
	@test -n "$(ATOMIC_SIZE)" || { echo "set ATOMIC_SIZE explicitly" >&2; exit 1; }
	cargo run --release -p steiner-census -- orderly --atomic-size "$(ATOMIC_SIZE)" \
		--data-root "$(STEINER_CENSUS_DATA_ROOT)" \
		--candidate-cap "$(or $(STEINER_CANDIDATE_CAP),20000000)" \
		--street-degree "$(or $(STEINER_STREET_DEGREE),2)"

steiner-gray: check-flint
	@test -n "$(STEINER_GRAY_OUTPUT)" || { echo "set STEINER_GRAY_OUTPUT explicitly" >&2; exit 1; }
	cargo run --release -p steiner-census -- gray-table --output "$(STEINER_GRAY_OUTPUT)"

identify-poset-groups: check-flint
	@test -n "$(POSET_CENSUS_DATA_ROOT)" || { echo "set POSET_CENSUS_DATA_ROOT explicitly" >&2; exit 1; }
	@test -n "$(GROUP_IDENTIFICATION_OUTPUT_ROOT)" || { echo "set GROUP_IDENTIFICATION_OUTPUT_ROOT explicitly" >&2; exit 1; }
	cargo run --release -p typebase-build -- identify-poset-groups \
		--data-root "$(POSET_CENSUS_DATA_ROOT)" --output-root "$(GROUP_IDENTIFICATION_OUTPUT_ROOT)" \
		--maximum-order "$(or $(TYPEBASE_MAXIMUM_ORDER),10)" --gap-executable "$(or $(GAP),gap)"

identify-steiner-groups: check-flint
	@test -n "$(STEINER_CENSUS_DATA_ROOT)" || { echo "set STEINER_CENSUS_DATA_ROOT explicitly" >&2; exit 1; }
	@test -n "$(STEINER_GROUP_IDENTIFICATION_OUTPUT_ROOT)" || { echo "set STEINER_GROUP_IDENTIFICATION_OUTPUT_ROOT explicitly" >&2; exit 1; }
	cargo run --release -p typebase-build -- identify-steiner-groups \
		--data-root "$(STEINER_CENSUS_DATA_ROOT)" \
		--output-root "$(STEINER_GROUP_IDENTIFICATION_OUTPUT_ROOT)" \
		--maximum-atomic-size "$(or $(TYPEBASE_STEINER_MAXIMUM_ATOMIC_SIZE),11)" \
		--gap-executable "$(or $(GAP),gap)"

typebase-web:
	npm --prefix web/typebase install
	npm --prefix web/typebase run build

bench-posets: check-flint
	cargo run --release -p poset-census --example profile_noether -- operations

bench-steiner-mappings: check-flint
	@test -n "$(STEINER_CENSUS_DATA_ROOT)" || { echo "set STEINER_CENSUS_DATA_ROOT explicitly" >&2; exit 1; }
	@test -n "$(ATOMIC_SIZE)" || { echo "set ATOMIC_SIZE explicitly" >&2; exit 1; }
	cargo run --release -p steiner-census --example benchmark_mappings -- \
		"$(STEINER_CENSUS_DATA_ROOT)" "$(ATOMIC_SIZE)" $(MAPPING_SAMPLE)

bench-pipeline: check-flint
	cargo run --release -p poset-census --example profile_noether -- pipeline \
		--maximum-order "$(BENCH_MAX_ORDER)" \
		--candidate-cap "$(BENCH_CANDIDATE_CAP)"

bench-orderly: check-flint
	cargo run --release -p poset-census --example profile_noether -- orderly-pipeline \
		--maximum-order "$(ORDERLY_BENCH_MAX_ORDER)" \
		--candidate-cap "$(ORDERLY_CANDIDATE_CAP)"

bench-orderly-generation: check-flint
	cargo run --release -p poset-census --example profile_noether -- orderly-generation \
		--order "$(ORDERLY_BENCH_ORDER)" \
		--candidate-cap "$(ORDERLY_CANDIDATE_CAP)"

reference: check-flint
	@test -n "$(POSET_CENSUS_DATA_ROOT)" || { \
		echo "set POSET_CENSUS_DATA_ROOT explicitly" >&2; \
		exit 1; \
	}
	@test -n "$(ORDER)" || { \
		echo "set ORDER explicitly" >&2; \
		exit 1; \
	}
	cargo run -p poset-census -- reference --order "$(ORDER)" --candidate-cap "$(CANDIDATE_CAP)" --data-root "$(POSET_CENSUS_DATA_ROOT)"

reference-order-3: ORDER := 3
reference-order-3: reference

orderly: check-flint
	@test -n "$(POSET_CENSUS_DATA_ROOT)" || { \
		echo "set POSET_CENSUS_DATA_ROOT explicitly" >&2; \
		exit 1; \
	}
	@test -n "$(ORDER)" || { \
		echo "set ORDER explicitly" >&2; \
		exit 1; \
	}
	cargo run --release -p poset-census -- orderly --order "$(ORDER)" --candidate-cap "$(ORDERLY_CANDIDATE_CAP)" --data-root "$(POSET_CENSUS_DATA_ROOT)"

orderly-shard: check-flint
	@test -n "$(POSET_CENSUS_DATA_ROOT)" || { echo "set POSET_CENSUS_DATA_ROOT explicitly" >&2; exit 1; }
	@test -n "$(ORDER)" || { echo "set ORDER explicitly" >&2; exit 1; }
	@test -n "$(RUN_ID)" || { echo "set RUN_ID explicitly" >&2; exit 1; }
	cargo run --release -p poset-census -- orderly-shard \
		--order "$(ORDER)" --data-root "$(POSET_CENSUS_DATA_ROOT)" --run-id "$(RUN_ID)" \
		--shard-id "$(SHARD_ID)" --shard-count "$(SHARD_COUNT)" \
		--candidate-cap "$(ORDERLY_CANDIDATE_CAP)" --checkpoint-every "$(CHECKPOINT_EVERY)" $(if $(PARENT_BUDGET),--parent-budget "$(PARENT_BUDGET)")

merge-orderly-shards: check-flint
	@test -n "$(POSET_CENSUS_DATA_ROOT)" || { echo "set POSET_CENSUS_DATA_ROOT explicitly" >&2; exit 1; }
	@test -n "$(ORDER)" || { echo "set ORDER explicitly" >&2; exit 1; }
	@test -n "$(RUN_ID)" || { echo "set RUN_ID explicitly" >&2; exit 1; }
	cargo run --release -p poset-census -- merge-orderly-shards \
		--order "$(ORDER)" --data-root "$(POSET_CENSUS_DATA_ROOT)" --run-id "$(RUN_ID)" \
		--shard-count "$(SHARD_COUNT)"

certify-orderly-shard: check-flint
	@test -n "$(POSET_CENSUS_DATA_ROOT)" || { echo "set POSET_CENSUS_DATA_ROOT explicitly" >&2; exit 1; }
	@test -n "$(ORDER)" || { echo "set ORDER explicitly" >&2; exit 1; }
	@test -n "$(RUN_ID)" || { echo "set RUN_ID explicitly" >&2; exit 1; }
	cargo run --release -p poset-census -- certify-orderly-shard \
		--order "$(ORDER)" --data-root "$(POSET_CENSUS_DATA_ROOT)" --run-id "$(RUN_ID)" \
		--shard-id "$(SHARD_ID)" --shard-count "$(SHARD_COUNT)"

publish-certified-merge: check-flint
	@test -n "$(POSET_CENSUS_DATA_ROOT)" || { echo "set POSET_CENSUS_DATA_ROOT explicitly" >&2; exit 1; }
	@test -n "$(ORDER)" || { echo "set ORDER explicitly" >&2; exit 1; }
	@test -n "$(RUN_ID)" || { echo "set RUN_ID explicitly" >&2; exit 1; }
	cargo run --release -p poset-census -- publish-certified-merge \
		--order "$(ORDER)" --data-root "$(POSET_CENSUS_DATA_ROOT)" --run-id "$(RUN_ID)" \
		--shard-count "$(SHARD_COUNT)"

.PHONY: test-hap-oracle
test-hap-oracle: check-flint
	@test -n "$(FINITE_GROUP_COHOMOLOGY_DATA_ROOT)" || { echo "set FINITE_GROUP_COHOMOLOGY_DATA_ROOT explicitly" >&2; exit 1; }
	TYPELAKE_VALIDATE_HAP=1 cargo test --release -p finite-group-cohomology-census --test hap_parity -- --ignored
