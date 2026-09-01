//! Configurable, sortable spatio-temporal hashes, good for
//! [STAC](https://stacspec.org/)
//! [items](https://github.com/radiantearth/stac-spec/blob/master/item-spec/item-spec.md).
//!
//! # Examples
//!
//! ```
//! use stac_hash::Hasher;
//! use chrono::{Utc, TimeZone};
//!
//! let start_datetime = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
//! let end_datetime = Utc.with_ymd_and_hms(2027, 1, 1, 0, 0, 0).unwrap();
//! let hasher = Hasher::global(start_datetime, end_datetime).unwrap();
//! let hash = hasher.hash(start_datetime, (-105., 40.)).unwrap();
//!
//! // Later datetimes sort after earlier ones
//! let hash_later = hasher.hash(end_datetime, (-105., 40.)).unwrap();
//! assert!(hash < hash_later);
//!
//! // Latitudes and longitudes sort as well
//! let hash_right = hasher.hash(start_datetime, (-104., 40.)).unwrap();
//! assert!(hash < hash_right);
//! let hash_above = hasher.hash(start_datetime, (-105., 41.)).unwrap();
//! assert!(hash < hash_above);
//! ```

use chrono::{DateTime, Utc};
use thiserror::Error;

const BITS_PER_DIMENSION: u8 = 21; // 63 / 3
const MAX_VALUE: f64 = ((1u64 << BITS_PER_DIMENSION) - 1) as f64;

mod config;

pub use config::{
    Algorithm, DType, Encoding, HashConfig, STAC_EXTENSION_SCHEMA_URL, TEMPORAL_PRECISION,
    encode_hash,
};

/// Crate-specific result type.
pub type Result<T> = std::result::Result<T, Error>;

/// A structure for creating sortable spatio-temporal hashes with millisecond temporal precision.
#[derive(Debug)]
pub struct Hasher {
    start_datetime: DateTime<Utc>,
    end_datetime: DateTime<Utc>,
    datetime_range_millis: f64,
    min_longitude: f64,
    max_longitude: f64,
    longitude_range: f64,
    min_latitude: f64,
    max_latitude: f64,
    latitude_range: f64,
}

/// A simple WGS84 point structure.
#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub longitude: f64,
    pub latitude: f64,
}

/// Errors returned when a value falls outside of a [Hasher]'s extent.
#[derive(Debug, Error)]
pub enum Error {
    /// The datetime is outside of the hasher's temporal extent.
    #[error("datetime outside of the hasher's temporal extent: {0}")]
    InvalidDatetime(DateTime<Utc>),

    /// The latitude is outside of the hasher's spatial extent.
    #[error("latitude outside of the hasher's spatial extent: {0}")]
    InvalidLatitude(f64),

    /// The longitude is outside of the hasher's spatial extent.
    #[error("longitude outside of the hasher's spatial extent: {0}")]
    InvalidLongitude(f64),

    /// The temporal extent is empty or reversed.
    #[error("end datetime must be after start datetime")]
    InvalidTemporalExtent,

    /// The longitude extent is outside of WGS84 bounds, empty, or reversed.
    #[error("longitude extent must be within -180..180 and west must be less than east")]
    InvalidLongitudeExtent,

    /// The latitude extent is outside of WGS84 bounds, empty, or reversed.
    #[error("latitude extent must be within -90..90 and south must be less than north")]
    InvalidLatitudeExtent,
}

impl Hasher {
    /// Creates a new hasher for the given datetime and the global extents.
    ///
    /// # Examples
    ///
    /// ```
    /// use stac_hash::Hasher;
    /// use chrono::{Utc, TimeZone};
    ///
    /// let start_datetime = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    /// let end_datetime = Utc.with_ymd_and_hms(2027, 1, 1, 0, 0, 0).unwrap();
    /// let hasher = Hasher::global(start_datetime, end_datetime).unwrap();
    /// ```
    pub fn global(start_datetime: DateTime<Utc>, end_datetime: DateTime<Utc>) -> Result<Self> {
        Self::new(start_datetime, end_datetime, (-180., -90.), (180., 90.))
    }

    /// Creates a new hasher for the given datetime and spatial intervals.
    ///
    /// # Examples
    ///
    /// ```
    /// use stac_hash::Hasher;
    /// use chrono::{Utc, TimeZone};
    ///
    /// // CONUS (roughly)
    /// let start_datetime = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    /// let end_datetime = Utc.with_ymd_and_hms(2027, 1, 1, 0, 0, 0).unwrap();
    /// let hasher = Hasher::new(start_datetime, end_datetime, (-125., 25.), (-66., 50.)).unwrap();
    /// ```
    pub fn new(
        start_datetime: DateTime<Utc>,
        end_datetime: DateTime<Utc>,
        min: impl Into<Point>,
        max: impl Into<Point>,
    ) -> Result<Self> {
        let min = min.into();
        let max = max.into();
        if end_datetime <= start_datetime {
            return Err(Error::InvalidTemporalExtent);
        }
        if !valid_longitude(min.longitude)
            || !valid_longitude(max.longitude)
            || min.longitude >= max.longitude
        {
            return Err(Error::InvalidLongitudeExtent);
        }
        if !valid_latitude(min.latitude)
            || !valid_latitude(max.latitude)
            || min.latitude >= max.latitude
        {
            return Err(Error::InvalidLatitudeExtent);
        }
        let datetime_range_millis = (end_datetime - start_datetime).num_milliseconds();
        let longitude_range = max.longitude - min.longitude;
        let latitude_range = max.latitude - min.latitude;
        Ok(Self {
            start_datetime,
            end_datetime,
            datetime_range_millis: datetime_range_millis as f64,
            min_longitude: min.longitude,
            max_longitude: max.longitude,
            longitude_range,
            min_latitude: min.latitude,
            max_latitude: max.latitude,
            latitude_range,
        })
    }

    /// Converts a datetime and a Point into a hash.
    ///
    /// Returns an error if the datetime or the point falls outside of this
    /// hasher's extent. Use [Hasher::hash_clamped] to clamp onto the boundary
    /// instead of erroring.
    ///
    /// # Examples
    ///
    /// ```
    /// use stac_hash::Hasher;
    /// use chrono::{Utc, TimeZone};
    ///
    /// let start = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
    /// let end = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    /// let hasher = Hasher::global(start, end).unwrap();
    /// let hash = hasher.hash(start, (0., 0.)).unwrap();
    /// ```
    pub fn hash(&self, datetime: DateTime<Utc>, point: impl Into<Point>) -> Result<u64> {
        let point = point.into();
        if datetime < self.start_datetime || datetime > self.end_datetime {
            return Err(Error::InvalidDatetime(datetime));
        }
        if point.latitude < self.min_latitude || point.latitude > self.max_latitude {
            return Err(Error::InvalidLatitude(point.latitude));
        }
        if point.longitude < self.min_longitude || point.longitude > self.max_longitude {
            return Err(Error::InvalidLongitude(point.longitude));
        }
        Ok(self.interleave(datetime, point))
    }

    /// Hashes a datetime and point, then encodes the hash value.
    pub fn hash_encoded(
        &self,
        datetime: DateTime<Utc>,
        point: impl Into<Point>,
        encoding: Encoding,
    ) -> Result<String> {
        self.hash(datetime, point)
            .map(|hash| encode_hash(hash, encoding))
    }

    /// Returns the STAC Hash Extension metadata for this hasher.
    pub fn config(&self, encoding: Encoding) -> HashConfig {
        HashConfig {
            algorithm: Algorithm::Morton,
            dtype: DType::Uint64,
            encoding,
            spatial_precision: self.longitude_range.max(self.latitude_range) / MAX_VALUE,
            temporal_precision: TEMPORAL_PRECISION,
            spatial_extent: [
                self.min_longitude,
                self.min_latitude,
                self.max_longitude,
                self.max_latitude,
            ],
            temporal_extent: (self.start_datetime, self.end_datetime),
        }
    }

    /// Converts a datetime and a Point into a hash, clamping anything outside
    /// of this hasher's extent onto its boundary.
    ///
    /// This cannot fail. A datetime before `start_datetime` hashes as
    /// `start_datetime`, a longitude west of the minimum hashes as that
    /// minimum, and so on. Each clamped value is logged at warn level. Use
    /// [Hasher::hash] to get an error instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use stac_hash::Hasher;
    /// use chrono::{Utc, TimeZone};
    ///
    /// let start = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    /// let end = Utc.with_ymd_and_hms(2027, 1, 1, 0, 0, 0).unwrap();
    /// let hasher = Hasher::new(start, end, (-109., 37.), (-102., 41.)).unwrap();
    ///
    /// // Well west of the bounding box, so it hashes as if it were on the edge.
    /// let hash = hasher.hash_clamped(start, (-120., 40.));
    /// assert_eq!(hash, hasher.hash(start, (-109., 40.)).unwrap());
    /// ```
    pub fn hash_clamped(&self, datetime: DateTime<Utc>, point: impl Into<Point>) -> u64 {
        let point = point.into();
        if datetime < self.start_datetime || datetime > self.end_datetime {
            log::warn!("datetime outside of the hasher's temporal extent: {datetime}");
        }
        if point.latitude < self.min_latitude || point.latitude > self.max_latitude {
            log::warn!(
                "latitude outside of the hasher's spatial extent: {}",
                point.latitude
            );
        }
        if point.longitude < self.min_longitude || point.longitude > self.max_longitude {
            log::warn!(
                "longitude outside of the hasher's spatial extent: {}",
                point.longitude
            );
        }
        self.interleave(datetime, point)
    }

    fn interleave(&self, datetime: DateTime<Utc>, point: Point) -> u64 {
        let datetime_normalized = (((datetime.timestamp_millis()
            - self.start_datetime.timestamp_millis()) as f64)
            / self.datetime_range_millis)
            .clamp(0., 1.);
        let latitude_normalized =
            ((point.latitude - self.min_latitude) / self.latitude_range).clamp(0., 1.);
        let longitude_normalized =
            ((point.longitude - self.min_longitude) / self.longitude_range).clamp(0., 1.);

        let datetime_quantized = (datetime_normalized * MAX_VALUE) as u64;
        let latitude_quantized = (latitude_normalized * MAX_VALUE) as u64;
        let longitude_quantized = (longitude_normalized * MAX_VALUE) as u64;

        let mut hash = 0u64;
        for i in 0..BITS_PER_DIMENSION {
            let src = i as u64;
            let dst = (i as u64) * 3;
            hash |= ((longitude_quantized >> src) & 1) << dst;
            hash |= ((latitude_quantized >> src) & 1) << (dst + 1);
            hash |= ((datetime_quantized >> src) & 1) << (dst + 2);
        }
        hash
    }
}

impl From<(f64, f64)> for Point {
    fn from((longitude, latitude): (f64, f64)) -> Self {
        Self {
            longitude,
            latitude,
        }
    }
}

fn valid_longitude(longitude: f64) -> bool {
    (-180.0..=180.0).contains(&longitude)
}

fn valid_latitude(latitude: f64) -> bool {
    (-90.0..=90.0).contains(&latitude)
}

#[cfg(test)]
mod tests {
    use crate::Point;

    use super::{
        Algorithm, DType, Encoding, Hasher, MAX_VALUE, STAC_EXTENSION_SCHEMA_URL,
        TEMPORAL_PRECISION, encode_hash,
    };
    use chrono::{DateTime, TimeZone, Utc};
    use rstest::{fixture, rstest};

    #[fixture]
    fn start_datetime() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
    }

    #[fixture]
    fn end_datetime() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2027, 1, 1, 0, 0, 0).unwrap()
    }

    #[fixture]
    fn longmont() -> Point {
        Point {
            longitude: -105.,
            latitude: 40.,
        }
    }

    #[fixture]
    fn hasher(start_datetime: DateTime<Utc>, end_datetime: DateTime<Utc>) -> Hasher {
        Hasher::global(start_datetime, end_datetime).unwrap()
    }

    #[rstest]
    fn one_year_global(hasher: Hasher, longmont: Point) {
        let hash = hasher
            .hash(
                Utc.with_ymd_and_hms(2026, 6, 14, 12, 0, 0).unwrap(),
                longmont,
            )
            .unwrap();
        assert_eq!(hash, 3024785829217804842);
    }

    #[fixture]
    fn colorado(start_datetime: DateTime<Utc>, end_datetime: DateTime<Utc>) -> Hasher {
        Hasher::new(start_datetime, end_datetime, (-109., 37.), (-102., 41.)).unwrap()
    }

    #[rstest]
    fn new_rejects_reversed_temporal_extent(
        start_datetime: DateTime<Utc>,
        end_datetime: DateTime<Utc>,
    ) {
        assert!(matches!(
            Hasher::global(end_datetime, start_datetime),
            Err(super::Error::InvalidTemporalExtent)
        ));
        assert!(matches!(
            Hasher::global(start_datetime, start_datetime),
            Err(super::Error::InvalidTemporalExtent)
        ));
    }

    #[rstest]
    fn new_rejects_invalid_longitude_extent(
        start_datetime: DateTime<Utc>,
        end_datetime: DateTime<Utc>,
    ) {
        assert!(matches!(
            Hasher::new(start_datetime, end_datetime, (-102., 37.), (-109., 41.)),
            Err(super::Error::InvalidLongitudeExtent)
        ));
        assert!(matches!(
            Hasher::new(start_datetime, end_datetime, (-181., 37.), (-109., 41.)),
            Err(super::Error::InvalidLongitudeExtent)
        ));
        assert!(matches!(
            Hasher::new(start_datetime, end_datetime, (-109., 37.), (181., 41.)),
            Err(super::Error::InvalidLongitudeExtent)
        ));
    }

    #[rstest]
    fn new_rejects_invalid_latitude_extent(
        start_datetime: DateTime<Utc>,
        end_datetime: DateTime<Utc>,
    ) {
        assert!(matches!(
            Hasher::new(start_datetime, end_datetime, (-109., 41.), (-102., 37.)),
            Err(super::Error::InvalidLatitudeExtent)
        ));
        assert!(matches!(
            Hasher::new(start_datetime, end_datetime, (-109., -91.), (-102., 41.)),
            Err(super::Error::InvalidLatitudeExtent)
        ));
        assert!(matches!(
            Hasher::new(start_datetime, end_datetime, (-109., 37.), (-102., 91.)),
            Err(super::Error::InvalidLatitudeExtent)
        ));
    }

    #[rstest]
    fn encode_hash_supports_stac_extension_encodings() {
        assert_eq!(
            encode_hash(3024785829217804842, Encoding::Integer),
            "3024785829217804842"
        );
        assert_eq!(
            encode_hash(3024785829217804842, Encoding::Base16),
            "29fa32af8829a22a"
        );
        assert_eq!(encode_hash(0, Encoding::Base16), "0000000000000000");
    }

    #[rstest]
    fn hash_encoded_matches_hash_then_encode(hasher: Hasher, longmont: Point) {
        let datetime = Utc.with_ymd_and_hms(2026, 6, 14, 12, 0, 0).unwrap();
        let hash = hasher.hash(datetime, longmont).unwrap();
        assert_eq!(
            hasher
                .hash_encoded(datetime, longmont, Encoding::Base16)
                .unwrap(),
            encode_hash(hash, Encoding::Base16)
        );
        assert_eq!(
            hasher
                .hash_encoded(datetime, longmont, Encoding::Integer)
                .unwrap(),
            encode_hash(hash, Encoding::Integer)
        );
    }

    #[rstest]
    fn global_config_matches_stac_extension_fields(
        hasher: Hasher,
        start_datetime: DateTime<Utc>,
        end_datetime: DateTime<Utc>,
    ) {
        let config = hasher.config(Encoding::Base16);
        assert_eq!(config.algorithm, Algorithm::Morton);
        assert_eq!(config.algorithm.as_str(), "morton");
        assert_eq!(config.dtype, DType::Uint64);
        assert_eq!(config.dtype.as_str(), "uint64");
        assert_eq!(config.encoding, Encoding::Base16);
        assert_eq!(config.encoding.as_str(), "base16");
        assert_eq!(config.temporal_precision, TEMPORAL_PRECISION);
        assert_eq!(config.temporal_precision, "PT0.001S");
        assert_eq!(config.spatial_extent, [-180., -90., 180., 90.]);
        assert_eq!(config.temporal_extent, (start_datetime, end_datetime));
        assert_eq!(
            STAC_EXTENSION_SCHEMA_URL,
            "https://stac-extensions.github.io/hash/v1.0.0/schema.json"
        );
    }

    #[rstest]
    fn config_spatial_precision_uses_coarsest_dimension(hasher: Hasher) {
        assert_eq!(
            hasher.config(Encoding::Integer).spatial_precision,
            360. / MAX_VALUE
        );
    }

    #[rstest]
    fn custom_config_uses_hasher_extents(
        colorado: Hasher,
        start_datetime: DateTime<Utc>,
        end_datetime: DateTime<Utc>,
    ) {
        let config = colorado.config(Encoding::Integer);
        assert_eq!(config.encoding, Encoding::Integer);
        assert_eq!(config.encoding.as_str(), "integer");
        assert_eq!(config.spatial_extent, [-109., 37., -102., 41.]);
        assert_eq!(config.temporal_extent, (start_datetime, end_datetime));
        assert_eq!(config.spatial_precision, 7. / MAX_VALUE);
    }

    #[rstest]
    fn hash_clamped_matches_hash_inside_the_extent(
        colorado: Hasher,
        start_datetime: DateTime<Utc>,
        longmont: Point,
    ) {
        assert_eq!(
            colorado.hash_clamped(start_datetime, longmont),
            colorado.hash(start_datetime, longmont).unwrap()
        );
    }

    #[rstest]
    fn hash_clamped_clamps_longitude(colorado: Hasher, start_datetime: DateTime<Utc>) {
        assert_eq!(
            colorado.hash_clamped(start_datetime, (-120., 40.)),
            colorado.hash(start_datetime, (-109., 40.)).unwrap()
        );
    }

    #[rstest]
    fn hash_clamped_clamps_latitude(colorado: Hasher, start_datetime: DateTime<Utc>) {
        assert_eq!(
            colorado.hash_clamped(start_datetime, (-105., 90.)),
            colorado.hash(start_datetime, (-105., 41.)).unwrap()
        );
    }

    #[rstest]
    fn hash_clamped_clamps_datetime(
        colorado: Hasher,
        end_datetime: DateTime<Utc>,
        longmont: Point,
    ) {
        let beyond = end_datetime + chrono::Duration::days(365);
        assert_eq!(
            colorado.hash_clamped(beyond, longmont),
            colorado.hash(end_datetime, longmont).unwrap()
        );
    }

    #[rstest]
    fn hash_still_errors_outside_the_extent(colorado: Hasher, start_datetime: DateTime<Utc>) {
        assert!(colorado.hash(start_datetime, (-120., 40.)).is_err());
        assert!(colorado.hash(start_datetime, (-105., 90.)).is_err());
        assert!(
            colorado
                .hash(start_datetime - chrono::Duration::days(1), (-105., 40.))
                .is_err()
        );
    }

    #[rstest]
    fn nearby_points_have_close_hashes(hasher: Hasher, start_datetime: DateTime<Utc>) {
        let hash = hasher.hash(start_datetime, (-105., 40.)).unwrap();
        let hash_near = hasher.hash(start_datetime, (-105.1, 40.1)).unwrap();
        let hash_far = hasher.hash(start_datetime, (-106., 41.)).unwrap();
        assert!(hash.abs_diff(hash_near) < hash.abs_diff(hash_far));
    }

    #[rstest]
    fn sort_datetime(hasher: Hasher, start_datetime: DateTime<Utc>, longmont: Point) {
        let hash_a = hasher.hash(start_datetime, longmont).unwrap();
        let hash_b = hasher
            .hash(start_datetime + chrono::Duration::days(1), longmont)
            .unwrap();
        assert!(hash_a < hash_b);
    }

    #[rstest]
    fn sort_latitude(hasher: Hasher, start_datetime: DateTime<Utc>, longmont: Point) {
        let hash_a = hasher.hash(start_datetime, longmont).unwrap();
        let hash_b = hasher
            .hash(
                start_datetime,
                Point {
                    latitude: 41.,
                    longitude: -105.,
                },
            )
            .unwrap();
        assert!(hash_a < hash_b);
    }

    #[rstest]
    fn sort_longitude(hasher: Hasher, start_datetime: DateTime<Utc>, longmont: Point) {
        let hash_a = hasher.hash(start_datetime, longmont).unwrap();
        let hash_b = hasher
            .hash(
                start_datetime,
                Point {
                    latitude: 40.,
                    longitude: -104.,
                },
            )
            .unwrap();
        assert!(hash_a < hash_b);
    }
}
