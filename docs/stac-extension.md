# STAC Extension

- **Title:** Hash
- **Identifier:** <https://stac-extensions.github.io/hash/v1.0.0/schema.json>
- **Field Name Prefix:** hash
- **Scope:** Collection, Item
- **Extension [Maturity Classification](https://github.com/radiantearth/stac-spec/tree/master/extensions/README.md#extension-maturity):** Proposal

This document explains the Hash Extension to the
[SpatioTemporal Asset Catalog](https://github.com/radiantearth/stac-spec) (STAC) specification.

The Hash Extension defines a sortable spatio-temporal hash for STAC Items, plus the Collection-level parameters needed to interpret
that hash. The initial algorithm is a Morton, or Z-order, code over quantized datetime, latitude, and longitude. Sorting by this value
groups Items that are close in space and time, which can help tabular query engines skip data outside a search's spatial and temporal
bounds.

The originating STAC discussion is
[radiantearth/stac-spec#1378](https://github.com/radiantearth/stac-spec/discussions/1378). The reference implementation is
[stac-hash](https://github.com/gadomski/stac-hash).

- Examples:
  - [Item example](stac-extension/examples/item.json): Shows the hash value on a STAC Item
  - [Collection example](stac-extension/examples/collection.json): Shows the Collection-level hash configuration
- [JSON Schema](stac-extension/schema.json)

## Fields

The fields in the tables below can be used in these parts of STAC documents:

- [ ] Catalogs
- [x] Collections
- [x] Item Properties
- [ ] Assets
- [ ] Links

### Item Properties

| Field Name | Type | Description |
| ---------- | ---- | ----------- |
| hash:hash | integer \| string | **REQUIRED**. The computed sortable hash value for the Item. |

### Collection Fields

| Field Name | Type | Description |
| ---------- | ---- | ----------- |
| hash:algorithm | string | **REQUIRED**. The space-filling curve used to compute the hash. Currently only `morton` is allowed. |
| hash:dtype | string | **REQUIRED**. The unsigned integer width into which the hash is packed: `uint32` or `uint64`. |
| hash:encoding | string | **REQUIRED**. The representation used for `hash:hash` in JSON: `integer`, `base64`, or `base16`. |
| hash:spatial_precision | number | **REQUIRED**. The quantization step for latitude and longitude, in degrees. |
| hash:temporal_precision | string | **REQUIRED**. The quantization step for datetime as an ISO 8601 duration, such as `PT1S`. |
| hash:spatial_extent | [number] | **REQUIRED**. The spatial normalization bounds as `[west, south, east, north]`. |
| hash:temporal_extent | [string] | **REQUIRED**. The temporal normalization bounds as `[start, end]`; both values are concrete datetimes. |

## Design Choices

### Collection-level Configuration

The hash value varies per Item, but the parameters used to compute and decode it are constant for a Collection. Keeping those
parameters on the Collection avoids repeating the same values on every Item and makes precision or extent changes explicit at the
Collection boundary.

### JSON Encoding

The `hash:encoding` field describes how `hash:hash` is represented in JSON. A `uint64` value can exceed JavaScript's safe integer
range, so JSON serializations may need a lossless string representation such as `base64` or `base16`. Native GeoParquet `uint64`
columns do not have this limitation and can use `integer`.

### Algorithm Versioning

The extension does not define a separate field for the implementation version that produced a hash. Breaking changes to the field
shape are handled through the versioned schema URL. Structurally different indexing schemes can be introduced as new
`hash:algorithm` values under the schema when appropriate.

### Temporal Extent

The `hash:temporal_extent` field is the concrete normalization interval baked into every `hash:hash` value in the Collection. It is
not the same as the descriptive STAC `extent.temporal.interval` field. Both bounds are required. Open or continuously updated
Collections should use a deliberate far-future end datetime and expect hashes may need to be rewritten when the true extent is known.
