import json
from pathlib import Path

ROOT = Path(__file__).parents[1]
EXTENSION_DIR = ROOT / "docs" / "stac-extension"
SCHEMA_URI = "https://stac-extensions.github.io/hash/v1.0.0/schema.json"
UINT64_MAX = 2**64 - 1


def load_json(path: Path):
    return json.loads(path.read_text())


def definition_for(schema: dict, field_name: str):
    field = schema["definitions"]["collectionFields"]["properties"][field_name]
    ref = field["$ref"].removeprefix("#/definitions/")
    return schema["definitions"][ref]


def test_collection_example_matches_extension_schema():
    schema = load_json(EXTENSION_DIR / "schema.json")
    collection = load_json(EXTENSION_DIR / "examples" / "collection.json")

    assert SCHEMA_URI in collection["stac_extensions"]
    assert collection["type"] == "Collection"
    assert (
        collection["hash:algorithm"] in definition_for(schema, "hash:algorithm")["enum"]
    )
    assert collection["hash:dtype"] in definition_for(schema, "hash:dtype")["enum"]
    assert (
        collection["hash:encoding"] in definition_for(schema, "hash:encoding")["enum"]
    )
    assert (
        collection["hash:temporal_precision"]
        == definition_for(schema, "hash:temporal_precision")["const"]
    )

    west, south, east, north = collection["hash:spatial_extent"]
    assert -180 <= west < east <= 180
    assert -90 <= south < north <= 90
    assert len(collection["hash:temporal_extent"]) == 2


def test_item_example_matches_extension_schema():
    item = load_json(EXTENSION_DIR / "examples" / "item.json")
    hash_value = item["properties"]["hash:hash"]

    assert SCHEMA_URI in item["stac_extensions"]
    assert item["type"] == "Feature"
    if isinstance(hash_value, int):
        assert 0 <= hash_value <= UINT64_MAX
    else:
        assert isinstance(hash_value, str)
        assert 1 <= len(hash_value) <= 16
        assert hash_value == hash_value.lower()
        int(hash_value, 16)
