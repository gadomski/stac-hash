import json
from pathlib import Path
from typing import TypeAlias, cast

ROOT = Path(__file__).parents[1]
EXTENSION_DIR = ROOT / "docs" / "stac-extension"
SCHEMA_URI = "https://stac-extensions.github.io/hash/v0.1.0/schema.json"
UINT64_MAX = 2**64 - 1

JsonValue: TypeAlias = (
    None | bool | int | float | str | list["JsonValue"] | dict[str, "JsonValue"]
)
JsonObject: TypeAlias = dict[str, JsonValue]


def load_json(path: Path) -> JsonObject:
    return cast(JsonObject, json.loads(path.read_text()))


def definition_for(schema: JsonObject, field_name: str) -> JsonObject:
    definitions = cast(JsonObject, schema["definitions"])
    collection_fields = cast(
        JsonObject, cast(JsonObject, definitions["collectionFields"])["properties"]
    )
    field = cast(JsonObject, collection_fields[field_name])
    ref = cast(str, field["$ref"]).removeprefix("#/definitions/")
    return cast(JsonObject, definitions[ref])


def test_collection_example_matches_extension_schema():
    schema = load_json(EXTENSION_DIR / "schema.json")
    collection = load_json(EXTENSION_DIR / "examples" / "collection.json")
    collection_schema = cast(JsonObject, cast(list[JsonValue], schema["oneOf"])[1])
    required_fields = cast(list[str], collection_schema["required"])

    assert SCHEMA_URI in cast(list[str], collection["stac_extensions"])
    assert collection["type"] == "Collection"
    assert required_fields == [
        "type",
        "hash:spatial_extent",
        "hash:temporal_extent",
    ]
    assert collection["hash:algorithm"] in cast(
        list[str], definition_for(schema, "hash:algorithm")["enum"]
    )
    assert definition_for(schema, "hash:algorithm")["default"] == "morton"
    assert collection["hash:dtype"] in cast(
        list[str], definition_for(schema, "hash:dtype")["enum"]
    )
    assert definition_for(schema, "hash:dtype")["default"] == "uint64"
    assert collection["hash:encoding"] in cast(
        list[str], definition_for(schema, "hash:encoding")["enum"]
    )
    assert definition_for(schema, "hash:encoding")["default"] == "integer"
    assert (
        collection["hash:temporal_precision"]
        == definition_for(schema, "hash:temporal_precision")["const"]
    )
    assert definition_for(schema, "hash:temporal_precision")["default"] == "PT0.001S"

    west, south, east, north = cast(list[float], collection["hash:spatial_extent"])
    assert -180 <= west < east <= 180
    assert -90 <= south < north <= 90
    assert len(cast(list[str], collection["hash:temporal_extent"])) == 2


def test_item_example_matches_extension_schema():
    item = load_json(EXTENSION_DIR / "examples" / "item.json")
    properties = cast(JsonObject, item["properties"])
    hash_value = cast(object, properties["hash:hash"])

    assert SCHEMA_URI in cast(list[str], item["stac_extensions"])
    assert item["type"] == "Feature"
    if isinstance(hash_value, int):
        assert 0 <= hash_value <= UINT64_MAX
    else:
        assert isinstance(hash_value, str)
        assert 1 <= len(hash_value) <= 16
        assert hash_value == hash_value.lower()
        int(hash_value, 16)
