# stac-hash

Sortable spatio-temporal hashes for [STAC](https://stacspec.org/)
[items](https://github.com/radiantearth/stac-spec/blob/master/item-spec/item-spec.md).

Our simple hashing algorithm turns a (datetime, point) pair into a single `u64`
hash. The hash is a [Morton code](https://en.wikipedia.org/wiki/Z-order_curve)
in space and time, which makes the hash useful as a sort key or an index prefix
for range scans.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](https://github.com/gadomski/stac-hash/blob/main/LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](https://github.com/gadomski/stac-hash/blob/main/LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
