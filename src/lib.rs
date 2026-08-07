//! Configurable, sortable spatio-temporal hashes, good for [STAC](https://stacspec.org/) [items](https://github.com/radiantearth/stac-spec/blob/master/item-spec/item-spec.md).

use chrono::{DateTime, Utc};

const BITS_PER_DIMENSION: u8 = 21; // 63 / 3
const MAX_VALUE: f64 = ((1u64 << BITS_PER_DIMENSION) - 1) as f64;

pub type Result<T> = std::result::Result<T, Error>;

/// Create sortable spatio-temporal hashes with millisecond temporal precision.
///
/// TODO Configurable datetime precision
/// TODO Configurable output type (currently hardcoded to u64)
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
#[derive(Debug)]
pub struct Point {
    pub longitude: f64,
    pub latitude: f64,
}

#[derive(Debug)]
pub enum Error {
    InvalidDatetime(DateTime<Utc>),
    InvalidLatitude(f64),
    InvalidLongitude(f64),
}

impl Hasher {
    /// Creates a new hasher for the given datetime and the global extents.
    pub fn global(start_datetime: DateTime<Utc>, end_datetime: DateTime<Utc>) -> Result<Self> {
        Self::new(start_datetime, end_datetime, (-180., -90.), (180., 90.))
    }

    /// Creates a new hasher for the given datetime and spatial intervals.
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
    fn hasher(start_datetime: DateTime<Utc>, end_datetime: DateTime<Utc>) -> Hasher {
        Hasher::global(start_datetime, end_datetime).unwrap()
    }

    #[rstest]
    fn one_year_global(hasher: Hasher) {
        let hash = hasher
            .hash(
                Utc.with_ymd_and_hms(2026, 6, 14, 12, 0, 0).unwrap(),
                (-105., 40.),
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
}
