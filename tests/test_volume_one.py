"""The volume selection must detect changed data and reject escaping paths."""

import importlib.util
import json
from pathlib import Path

import pytest

spec = importlib.util.spec_from_file_location(
    "volume_one", Path(__file__).parents[1] / "scripts" / "volume-one.py"
)
volume_one = importlib.util.module_from_spec(spec)
spec.loader.exec_module(volume_one)


def test_changed_artifact_is_rejected(tmp_path):
    artifact = tmp_path / "records.jsonl"
    artifact.write_text('{"id":1}\n')
    expected = volume_one.inspect_file(tmp_path, artifact.name)
    artifact.write_text('{"id":2}\n')
    with pytest.raises(ValueError, match="sha256"):
        volume_one.check_artifact(tmp_path, artifact.name, expected)


def test_wrong_declared_row_count_is_rejected(tmp_path):
    artifact = tmp_path / "records.jsonl"
    artifact.write_text('{"id":1}\n')
    expected = volume_one.inspect_file(tmp_path, artifact.name)
    expected["row_count"] = 2
    with pytest.raises(ValueError, match="row_count"):
        volume_one.check_artifact(tmp_path, artifact.name, expected)


@pytest.mark.parametrize("name", ["../records.jsonl", "/records.jsonl", "a/../../records.jsonl"])
def test_escaping_artifact_path_is_rejected(name):
    with pytest.raises(ValueError, match="unsafe"):
        volume_one.relative_path(name)


def test_selection_round_trip(tmp_path):
    (tmp_path / "records.jsonl").write_text('{"id":1}\n')
    selection = dict(format=volume_one.FORMAT, volume=1, collections=[
        dict(inputs=[volume_one.inspect_file(tmp_path, "records.jsonl")])
    ])
    report = volume_one.verify(tmp_path, json.loads(json.dumps(selection)))
    assert report["files"] == 1
    assert report["status"] == "passed"
    assert report["mathematical_replay"] == "separate"
