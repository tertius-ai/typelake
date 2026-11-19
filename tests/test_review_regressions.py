import pytest

from scitype.canonical import CanonicalError, jcs
from scitype.validate import identify, validate_instance


@pytest.mark.parametrize("mnemonic,instance", [
    ("DAG", {"n": 2, "arcs": [[0, 5]]}),
    ("Graph", {"n": 2, "edges": [[0, 5]]}),
    ("NumberFieldElement", {"num": ["2", "4"], "den": "2"}),
    ("ChainComplex", {"base": {"kind": "Z", "params": {}}, "bottom_degree": 0,
                      "ranks": [1, 1], "differentials": [
                          {"base": {"kind": "Z", "params": {}}, "nrows": -1,
                           "ncols": 1, "entries": ["x"]}]}),
])
def test_invalid_defining_data_cannot_receive_an_identifier(schemas, mnemonic, instance):
    schema = next(value for value in schemas.values() if value.mnemonic == mnemonic)
    assert validate_instance(instance, schema, schemas=schemas)
    with pytest.raises(CanonicalError):
        identify(instance, schema, schemas=schemas)


@pytest.mark.parametrize("value", [{"a": "\ud800"}, {"\udfff": 1}])
def test_invalid_unicode_has_a_typed_encoding_error(value):
    with pytest.raises(CanonicalError):
        jcs(value)
