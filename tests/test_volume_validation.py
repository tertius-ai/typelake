import hashlib
import importlib.util
import json
from pathlib import Path

import pytest


def test_freeze_requires_actual_validation_evidence(tmp_path, monkeypatch):
    path = Path(__file__).resolve().parents[1] / "scripts/volume-one.py"
    spec = importlib.util.spec_from_file_location("volume_one_review", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    data = b"{}\n"
    (tmp_path / "records.jsonl").write_bytes(data)
    (tmp_path / "run.json").write_text(json.dumps({"outputs": [{"path": "records.jsonl",
        "byte_count": len(data), "sha256": hashlib.sha256(data).hexdigest(), "row_count": 1}]}))
    monkeypatch.setattr(module, "selected_collections", lambda: [dict(collection="fixture", manifests=["run.json"])])
    with pytest.raises(ValueError, match="missing validation"):
        module.freeze(tmp_path)
    (tmp_path / "validation.json").write_text('{"status":"failed"}')
    with pytest.raises(ValueError, match="unvalidated"):
        module.freeze(tmp_path)
    (tmp_path / "validation.json").write_text('{"status":"passed"}')
    selection = module.freeze(tmp_path)
    assert any(item["path"] == "validation.json" for item in selection["collections"][0]["inputs"])
