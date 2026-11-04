"""Partitioned validation must retain complete, non-overlapping group coverage."""

import importlib.util
from pathlib import Path

import pytest

spec = importlib.util.spec_from_file_location(
    "group_migration", Path(__file__).parents[1] / "scripts" / "migrate-group-cohomology.py"
)
migration = importlib.util.module_from_spec(spec)
spec.loader.exec_module(migration)


def report(label, rows):
    return dict(group_label=label, noether_version="0.3.2", noether_revision=migration.REVISION,
                validation=dict(contract="finite-group-cohomology/modular-v7", maximum_order=32,
                                initial_proof_degree=8, maximum_proof_degree=12,
                                groups=1, coefficient_rows=rows, complete_presentations=rows,
                                proof_depths={"8": rows}, algebra_recomputed=True))


def test_disjoint_groups_preserve_rows_and_proof_coverage():
    combined = migration.combine_reports([report("1.1", 0), report("6.1", 2)])
    assert combined["groups"] == 2
    assert combined["coefficient_rows"] == 2
    assert combined["proof_depths"] == {"8": 2}


def test_coordinate_and_schema_differences_are_recorded():
    assert migration.changed_paths({"products": [1, 2]}, {"products": [1, 3]}) == ["/products/1"]
    assert migration.changed_paths({}, {"new_field": None}) == ["/new_field"]


def test_duplicate_group_is_rejected():
    with pytest.raises(ValueError, match="duplicate"):
        migration.combine_reports([report("6.1", 2), report("6.1", 2)])


def test_inconsistent_report_fields_are_rejected():
    other = report("8.1", 1)
    other["validation"]["unexpected_gate"] = False
    with pytest.raises(ValueError, match="report fields"):
        migration.combine_reports([report("6.1", 2), other])


def test_failed_gate_or_mixed_engine_is_rejected():
    item = report("6.1", 2)
    item["validation"]["algebra_recomputed"] = False
    with pytest.raises(ValueError, match="failed"):
        migration.combine_reports([item])
    item = report("6.1", 2)
    item["noether_revision"] = "0" * 40
    with pytest.raises(ValueError, match="engine"):
        migration.combine_reports([item])
