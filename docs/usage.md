# Usage

## Rust

```sh
cargo add stac-hash
```

```rust
use chrono::{TimeZone, Utc};
use stac_hash::Hasher;

let start_datetime = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
let end_datetime = Utc.with_ymd_and_hms(2027, 1, 1, 0, 0, 0).unwrap();
let hasher = Hasher::global(start_datetime, end_datetime).unwrap();
let hash = hasher
    .hash(Utc.with_ymd_and_hms(2026, 6, 14, 12, 0, 0).unwrap(), (-105., 40.))
    .unwrap();
assert_eq!(hash, 3024785829217804842);
```

Use `Hasher::new` instead of `Hasher::global` to hash against a smaller bounding box, which spends the available bits on a smaller area and so gives you finer spatial resolution.

Datetimes and points outside of the hasher's extent are an error, not a clamp.

## Python

```sh
pip install stac-hash
```

```python
from datetime import datetime, timezone

from stac_hash import Hasher

hasher = Hasher(
    datetime(2026, 1, 1, tzinfo=timezone.utc),
    datetime(2027, 1, 1, tzinfo=timezone.utc),
)
hash = hasher.hash(datetime(2026, 6, 14, 12, tzinfo=timezone.utc), -105.0, 40.0)
assert hash == 3024785829217804842
```

Pass `bbox=(min_lon, min_lat, max_lon, max_lat)` to hash against a smaller extent, and use `hash_all(datetimes, longitudes, latitudes)` to hash a batch in one call. Datetimes must be timezone-aware, and the sequences must be the same length. Out-of-extent input raises `ValueError`; pass `skip_invalid=True` to `hash_all` to get `None` for those items instead of raising.

## STAC extension fields

Use `hash_encoded` for Item-level hash values. The `base16` encoding is lossless in JSON, including clients that cannot safely represent every `uint64` value as a number.

```python
from datetime import datetime, timezone

from stac_hash import Hasher

hasher = Hasher(
    datetime(2026, 1, 1, tzinfo=timezone.utc),
    datetime(2027, 1, 1, tzinfo=timezone.utc),
)

item_hash = hasher.hash_encoded(
    datetime(2026, 6, 14, 12, tzinfo=timezone.utc),
    -105.0,
    40.0,
    "base16",
)

collection = {
    "stac_extensions": ["https://stac-extensions.github.io/hash/v0.1.0/schema.json"],
    "hash:spatial_extent": [-180.0, -90.0, 180.0, 90.0],
    "hash:temporal_extent": ["2026-01-01T00:00:00Z", "2027-01-01T00:00:00Z"],
}
item_properties = {"hash:hash": item_hash}
```

```rust
use chrono::{TimeZone, Utc};
use stac_hash::{Encoding, Hasher};

let hasher = Hasher::global(
    Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
    Utc.with_ymd_and_hms(2027, 1, 1, 0, 0, 0).unwrap(),
)
.unwrap();

let collection_fields = hasher.config(Encoding::Base16);
let item_hash = hasher
    .hash_encoded(
        Utc.with_ymd_and_hms(2026, 6, 14, 12, 0, 0).unwrap(),
        (-105.0, 40.0),
        Encoding::Base16,
    )
    .unwrap();
```

See the [STAC Hash Extension](stac-extension.md) for the complete field definitions and examples.

To clamp instead of erroring, use `hash_clamped` and `hash_all_clamped`. They mirror the Rust `Hasher::hash_clamped` method: anything outside the extent is pinned to the nearest boundary, in time as well as space, so they never raise for out-of-range input.

Each clamped value is logged as a warning through the standard library's `logging`, on a logger named `stac_hash`. Nothing is printed unless you configure logging:

```python
import logging

logging.basicConfig(level=logging.WARNING)
```

```text
WARNING:stac_hash:longitude outside of the hasher's spatial extent: -120
```

Silence them with `logging.getLogger("stac_hash").setLevel(logging.ERROR)`.

## How it works

Each of datetime, latitude, and longitude is normalized to `0..1` within the hasher's extent, then quantized to a 21-bit integer. Those three values are bit-interleaved into a single `u64`, using 63 of its 64 bits. The datetime bit occupies the most significant slot of each interleaved triple, so sorting by hash orders coarsely by time first, then latitude, then longitude.

The usual Z-order caveat applies: nearby hashes always mean nearby items, but nearby items can occasionally land far apart in hash space when they straddle a high-order boundary.

## Limitations

The following are hardcoded for now:

- Millisecond temporal precision
- `u64` output
- Datetime as the primary sort dimension
