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

/// Crate-specific result type.
pub type Result<T> = std::result::Result<T, Error>;

/// A structure for creating sortable spatio-temporal hashes with millisecond temporal precision.
// TODO Configurable datetime precision
// TODO Configurable output type (currently hardcoded to u64)
// TODO Configurable primary sort order (currently hardcoded to datetime)
// TODO Configurable clamping (instead of erroring)
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
    /// # Examples
    ///
    /// ```
    /// use stac_hash::Hasher;
    /// use chrono::{Utc, TimeZone};
    ///
    /// let start = Utc.ymd(2023, 1, 1).and_hms(0, 0, 0);
    /// let end = Utc.ymd(2024, 1, 1).and_hms(0, 0, 0);
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
        Ok(hash)
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

#[cfg(test)]
mod tests {
    use crate::Point;

    use super::Hasher;
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
