#[cfg(test)]
use chrono::{DateTime, Utc};

/// The schema URL for the STAC Hash Extension described by this crate.
#[cfg(test)]
pub(crate) const STAC_EXTENSION_SCHEMA_URL: &str =
    "https://stac-extensions.github.io/hash/v0.1.0/schema.json";

/// The temporal precision used by the current hash algorithm.
#[cfg(test)]
pub(crate) const TEMPORAL_PRECISION: &str = "PT0.001S";

/// The STAC Hash Extension algorithm identifier supported by this crate.
#[cfg(test)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum Algorithm {
    /// Morton, or Z-order, bit interleaving.
    Morton,
}

#[cfg(test)]
impl Algorithm {
    /// Returns the STAC extension string value for this algorithm.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Morton => "morton",
        }
    }
}

/// The unsigned integer width used for hash output.
#[cfg(test)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum DType {
    /// Unsigned 64-bit integer output.
    Uint64,
}

#[cfg(test)]
impl DType {
    /// Returns the STAC extension string value for this dtype.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Uint64 => "uint64",
        }
    }
}

/// The representation used for hash values outside of native `u64` storage.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Encoding {
    /// Store hash values as JSON integers or native unsigned integers.
    Integer,

    /// Store hash values as lowercase hexadecimal strings with no prefix.
    Base16,
}

impl Encoding {
    /// Returns the STAC extension string value for this encoding.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Integer => "integer",
            Self::Base16 => "base16",
        }
    }
}

/// Encodes a hash value using one of the STAC Hash Extension encodings.
pub fn encode_hash(hash: u64, encoding: Encoding) -> String {
    match encoding {
        Encoding::Integer => hash.to_string(),
        Encoding::Base16 => format!("{hash:016x}"),
    }
}

/// STAC Hash Extension metadata for a hasher.
#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HashConfig {
    /// The space-filling curve used to compute hashes.
    pub(crate) algorithm: Algorithm,

    /// The unsigned integer width into which hashes are packed.
    pub(crate) dtype: DType,

    /// The representation used for hash values in JSON or tabular metadata.
    pub(crate) encoding: Encoding,

    /// The coarsest approximate spatial quantization step for this extent, in degrees.
    pub(crate) spatial_precision: f64,

    /// The datetime quantization step as an ISO 8601 duration.
    pub(crate) temporal_precision: &'static str,

    /// The spatial normalization bounds as `[west, south, east, north]`.
    pub(crate) spatial_extent: [f64; 4],

    /// The temporal normalization bounds as `(start, end)`.
    pub(crate) temporal_extent: (DateTime<Utc>, DateTime<Utc>),
}
