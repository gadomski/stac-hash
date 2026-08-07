# stac-hash

Configurable, sortable spatio-temporal hashes, good for [STAC](https://stacspec.org/) [items](https://github.com/radiantearth/stac-spec/blob/master/item-spec/item-spec.md).

A `Hasher` is built from a datetime range and a bounding box, and turns a (datetime, point) pair into a single `u64`. Because the hash is a [Morton code](https://en.wikipedia.org/wiki/Z-order_curve), items that are near each other in space and time get numerically nearby hashes, which makes the hash useful as a sort key or an index prefix for range scans.

## Usage

```sh
cargo add stac-hash
```

```rust
use chrono::{TimeZone, Utc};
use stac_hash::Hasher;

let hasher = Hasher::global(
    Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
    Utc.with_ymd_and_hms(2027, 1, 1, 0, 0, 0).unwrap(),
)?;
let hash = hasher.hash(
    Utc.with_ymd_and_hms(2026, 6, 14, 12, 0, 0).unwrap(),
    (-105., 40.),
)?;
assert_eq!(hash, 3024785829217804842);
```

Use `Hasher::new` instead of `Hasher::global` to hash against a smaller bounding box, which spends the available bits on a smaller area and so gives you finer spatial resolution.

Datetimes and points outside of the hasher's extent are an error, not a clamp.

## How it works

Each of datetime, latitude, and longitude is normalized to `0..1` within the hasher's extent, then quantized to a 21-bit integer. Those three values are bit-interleaved into a single `u64`, using 63 of its 64 bits. The datetime bit occupies the most significant slot of each interleaved triple, so sorting by hash orders coarsely by time first, then latitude, then longitude.

The usual Z-order caveat applies: nearby hashes always mean nearby items, but nearby items can occasionally land far apart in hash space when they straddle a high-order boundary.

## Limitations

The following are hardcoded for now:

- Millisecond temporal precision
- `u64` output
- Datetime as the primary sort dimension

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](./LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](./LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
