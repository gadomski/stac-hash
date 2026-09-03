use ::stac_hash::{Encoding, Error, Hasher as RustHasher, encode_hash as rust_encode_hash};
use chrono::{DateTime, FixedOffset, Utc};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Creates sortable spatio-temporal hashes.
///
/// The hasher is built from a datetime range and, optionally, a bounding box.
/// Without a `bbox` the hasher covers the whole world.
#[pyclass(module = "stac_hash")]
struct Hasher(RustHasher);

#[pymethods]
impl Hasher {
    #[new]
    #[pyo3(signature = (start_datetime, end_datetime, bbox = None))]
    fn new(
        start_datetime: DateTime<FixedOffset>,
        end_datetime: DateTime<FixedOffset>,
        bbox: Option<(f64, f64, f64, f64)>,
    ) -> PyResult<Self> {
        let start_datetime = start_datetime.with_timezone(&Utc);
        let end_datetime = end_datetime.with_timezone(&Utc);
        let hasher = if let Some((min_longitude, min_latitude, max_longitude, max_latitude)) = bbox
        {
            RustHasher::new(
                start_datetime,
                end_datetime,
                (min_longitude, min_latitude),
                (max_longitude, max_latitude),
            )
        } else {
            RustHasher::global(start_datetime, end_datetime)
        };
        hasher.map(Hasher).map_err(to_py_err)
    }

    /// Hashes a datetime and a point into an `int`.
    fn hash(
        &self,
        datetime: DateTime<FixedOffset>,
        longitude: f64,
        latitude: f64,
    ) -> PyResult<u64> {
        self.0
            .hash(datetime.with_timezone(&Utc), (longitude, latitude))
            .map_err(to_py_err)
    }

    /// Hashes a datetime and a point, then encodes the hash value.
    fn hash_encoded(
        &self,
        datetime: DateTime<FixedOffset>,
        longitude: f64,
        latitude: f64,
        encoding: &str,
    ) -> PyResult<String> {
        self.0
            .hash_encoded(
                datetime.with_timezone(&Utc),
                (longitude, latitude),
                parse_encoding(encoding)?,
            )
            .map_err(to_py_err)
    }

    /// Hashes a datetime and a point into an `int`, clamping anything outside
    /// of this hasher's extent onto its boundary.
    ///
    /// Unlike `hash`, this never raises for out-of-extent input. A datetime
    /// before the hasher's start hashes as that start, a longitude west of the
    /// minimum hashes as that minimum, and so on.
    fn hash_clamped(&self, datetime: DateTime<FixedOffset>, longitude: f64, latitude: f64) -> u64 {
        self.0
            .hash_clamped(datetime.with_timezone(&Utc), (longitude, latitude))
    }

    /// Hashes parallel sequences of datetimes, longitudes, and latitudes.
    ///
    /// Raises a `ValueError` if the sequences are not the same length, or if
    /// any item falls outside the hasher's extent. Pass `skip_invalid=True` to
    /// get `None` for out-of-extent items instead of raising.
    #[pyo3(signature = (datetimes, longitudes, latitudes, skip_invalid = false))]
    fn hash_all(
        &self,
        datetimes: Vec<DateTime<FixedOffset>>,
        longitudes: Vec<f64>,
        latitudes: Vec<f64>,
        skip_invalid: bool,
    ) -> PyResult<Vec<Option<u64>>> {
        if datetimes.len() != longitudes.len() || datetimes.len() != latitudes.len() {
            return Err(PyValueError::new_err(format!(
                "sequences must be the same length: {} datetimes, {} longitudes, {} latitudes",
                datetimes.len(),
                longitudes.len(),
                latitudes.len()
            )));
        }
        let mut hashes = Vec::with_capacity(datetimes.len());
        for (i, ((datetime, longitude), latitude)) in datetimes
            .into_iter()
            .zip(longitudes)
            .zip(latitudes)
            .enumerate()
        {
            match self
                .0
                .hash(datetime.with_timezone(&Utc), (longitude, latitude))
            {
                Ok(hash) => hashes.push(Some(hash)),
                Err(_) if skip_invalid => hashes.push(None),
                Err(error) => return Err(PyValueError::new_err(format!("item {i}: {error}"))),
            }
        }
        Ok(hashes)
    }

    /// Hashes parallel sequences, clamping anything outside of this hasher's
    /// extent onto its boundary.
    ///
    /// Raises a `ValueError` only if the sequences are not the same length.
    /// No item can fall outside the extent, so there is no `skip_invalid`.
    fn hash_all_clamped(
        &self,
        datetimes: Vec<DateTime<FixedOffset>>,
        longitudes: Vec<f64>,
        latitudes: Vec<f64>,
    ) -> PyResult<Vec<u64>> {
        if datetimes.len() != longitudes.len() || datetimes.len() != latitudes.len() {
            return Err(PyValueError::new_err(format!(
                "sequences must be the same length: {} datetimes, {} longitudes, {} latitudes",
                datetimes.len(),
                longitudes.len(),
                latitudes.len()
            )));
        }
        Ok(datetimes
            .into_iter()
            .zip(longitudes)
            .zip(latitudes)
            .map(|((datetime, longitude), latitude)| {
                self.0
                    .hash_clamped(datetime.with_timezone(&Utc), (longitude, latitude))
            })
            .collect())
    }
}

fn parse_encoding(encoding: &str) -> PyResult<Encoding> {
    match encoding {
        "integer" => Ok(Encoding::Integer),
        "base16" => Ok(Encoding::Base16),
        _ => Err(PyValueError::new_err(format!(
            "unsupported encoding: {encoding}"
        ))),
    }
}

fn to_py_err(error: Error) -> PyErr {
    PyValueError::new_err(error.to_string())
}

/// Encodes a hash value using one of the STAC Hash Extension encodings.
#[pyfunction]
fn encode_hash(hash: u64, encoding: &str) -> PyResult<String> {
    Ok(rust_encode_hash(hash, parse_encoding(encoding)?))
}

#[pymodule]
fn stac_hash(m: &Bound<'_, PyModule>) -> PyResult<()> {
    pyo3_log::init();
    m.add_class::<Hasher>()?;
    m.add_function(wrap_pyfunction!(encode_hash, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
