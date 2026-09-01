import logging
from datetime import UTC, datetime, timedelta, timezone

import pytest

from stac_hash import Hasher, encode_hash


@pytest.fixture
def start_datetime() -> datetime:
    return datetime(2026, 1, 1, tzinfo=UTC)


@pytest.fixture
def end_datetime() -> datetime:
    return datetime(2027, 1, 1, tzinfo=UTC)


@pytest.fixture
def hasher(start_datetime: datetime, end_datetime: datetime) -> Hasher:
    return Hasher(start_datetime, end_datetime)


@pytest.fixture
def colorado(start_datetime: datetime, end_datetime: datetime) -> Hasher:
    return Hasher(start_datetime, end_datetime, bbox=(-109.0, 37.0, -102.0, 41.0))


def test_matches_rust(hasher: Hasher):
    hash = hasher.hash(datetime(2026, 6, 14, 12, tzinfo=UTC), -105.0, 40.0)
    assert hash == 3024785829217804842


def test_encode_hash_supports_extension_encodings():
    assert encode_hash(3024785829217804842, "integer") == "3024785829217804842"
    assert encode_hash(3024785829217804842, "base16") == "29fa32af8829a22a"
    assert encode_hash(0, "base16") == "0000000000000000"


def test_encode_hash_rejects_unknown_encoding():
    with pytest.raises(ValueError, match="unsupported encoding"):
        encode_hash(1, "base64")


def test_hash_encoded_matches_hash_then_encode(hasher: Hasher):
    dt = datetime(2026, 6, 14, 12, tzinfo=UTC)
    hash = hasher.hash(dt, -105.0, 40.0)
    assert hasher.hash_encoded(dt, -105.0, 40.0, "integer") == encode_hash(
        hash, "integer"
    )
    assert hasher.hash_encoded(dt, -105.0, 40.0, "base16") == encode_hash(
        hash, "base16"
    )


def test_hash_encoded_rejects_unknown_encoding(
    hasher: Hasher, start_datetime: datetime
):
    with pytest.raises(ValueError, match="unsupported encoding"):
        hasher.hash_encoded(start_datetime, -105.0, 40.0, "base64")


def test_config_returns_stac_extension_fields(
    hasher: Hasher, start_datetime: datetime, end_datetime: datetime
):
    config = hasher.config("base16")
    assert config == {
        "hash:algorithm": "morton",
        "hash:dtype": "uint64",
        "hash:encoding": "base16",
        "hash:spatial_precision": 360.0 / ((2**21) - 1),
        "hash:temporal_precision": "PT0.001S",
        "hash:spatial_extent": [-180.0, -90.0, 180.0, 90.0],
        "hash:temporal_extent": [
            start_datetime.isoformat(timespec="milliseconds").replace("+00:00", "Z"),
            end_datetime.isoformat(timespec="milliseconds").replace("+00:00", "Z"),
        ],
    }


def test_config_uses_custom_bbox(colorado: Hasher):
    config = colorado.config("integer")
    assert config["hash:encoding"] == "integer"
    assert config["hash:spatial_extent"] == [-109.0, 37.0, -102.0, 41.0]
    assert config["hash:spatial_precision"] == 7.0 / ((2**21) - 1)


def test_config_rejects_unknown_encoding(hasher: Hasher):
    with pytest.raises(ValueError, match="unsupported encoding"):
        hasher.config("base64")


def test_constructor_rejects_reversed_temporal_extent(
    start_datetime: datetime, end_datetime: datetime
):
    with pytest.raises(ValueError, match="end datetime must be after start datetime"):
        Hasher(end_datetime, start_datetime)
    with pytest.raises(ValueError, match="end datetime must be after start datetime"):
        Hasher(start_datetime, start_datetime)


def test_constructor_rejects_invalid_longitude_extent(
    start_datetime: datetime, end_datetime: datetime
):
    with pytest.raises(ValueError, match="longitude extent"):
        Hasher(start_datetime, end_datetime, bbox=(-102.0, 37.0, -109.0, 41.0))
    with pytest.raises(ValueError, match="longitude extent"):
        Hasher(start_datetime, end_datetime, bbox=(-181.0, 37.0, -109.0, 41.0))
    with pytest.raises(ValueError, match="longitude extent"):
        Hasher(start_datetime, end_datetime, bbox=(-109.0, 37.0, 181.0, 41.0))


def test_constructor_rejects_invalid_latitude_extent(
    start_datetime: datetime, end_datetime: datetime
):
    with pytest.raises(ValueError, match="latitude extent"):
        Hasher(start_datetime, end_datetime, bbox=(-109.0, 41.0, -102.0, 37.0))
    with pytest.raises(ValueError, match="latitude extent"):
        Hasher(start_datetime, end_datetime, bbox=(-109.0, -91.0, -102.0, 41.0))
    with pytest.raises(ValueError, match="latitude extent"):
        Hasher(start_datetime, end_datetime, bbox=(-109.0, 37.0, -102.0, 91.0))


def test_hash_clamped_matches_hash_inside_the_extent(
    colorado: Hasher, start_datetime: datetime
):
    assert colorado.hash_clamped(start_datetime, -105.0, 40.0) == colorado.hash(
        start_datetime, -105.0, 40.0
    )


def test_hash_clamped_clamps_longitude(colorado: Hasher, start_datetime: datetime):
    assert colorado.hash_clamped(start_datetime, -120.0, 40.0) == colorado.hash(
        start_datetime, -109.0, 40.0
    )


def test_hash_clamped_clamps_latitude(colorado: Hasher, start_datetime: datetime):
    assert colorado.hash_clamped(start_datetime, -105.0, 90.0) == colorado.hash(
        start_datetime, -105.0, 41.0
    )


def test_hash_clamped_clamps_datetime(
    colorado: Hasher, end_datetime: datetime, start_datetime: datetime
):
    beyond = end_datetime + timedelta(days=365)
    assert colorado.hash_clamped(beyond, -105.0, 40.0) == colorado.hash(
        end_datetime, -105.0, 40.0
    )
    assert colorado.hash_clamped(beyond, -105.0, 40.0) != colorado.hash(
        start_datetime, -105.0, 40.0
    )


def test_hash_clamped_requires_aware_datetime(colorado: Hasher):
    with pytest.raises(TypeError):
        colorado.hash_clamped(datetime(2026, 6, 14, 12), -105.0, 40.0)  # noqa: DTZ001


def test_hash_all_clamped_matches_hash_clamped(
    colorado: Hasher, start_datetime: datetime
):
    datetimes = [start_datetime] * 3
    longitudes = [-105.0, -120.0, -100.0]
    latitudes = [40.0, 90.0, 38.0]
    assert colorado.hash_all_clamped(datetimes, longitudes, latitudes) == [
        colorado.hash_clamped(dt, lon, lat)
        for dt, lon, lat in zip(datetimes, longitudes, latitudes)
    ]


def test_hash_all_clamped_never_raises_on_extent(
    colorado: Hasher, start_datetime: datetime
):
    hashes = colorado.hash_all_clamped([start_datetime], [181.0], [91.0])
    assert len(hashes) == 1
    assert hashes[0] is not None


def test_hash_all_clamped_empty(colorado: Hasher):
    assert colorado.hash_all_clamped([], [], []) == []


def test_hash_all_clamped_length_mismatch(colorado: Hasher, start_datetime: datetime):
    with pytest.raises(ValueError):
        colorado.hash_all_clamped([start_datetime, start_datetime], [-105.0], [40.0])


def test_clamping_warns_through_python_logging(
    colorado: Hasher, start_datetime: datetime, caplog: pytest.LogCaptureFixture
):
    with caplog.at_level(logging.WARNING, logger="stac_hash"):
        colorado.hash_clamped(start_datetime, -120.0, 40.0)
    warnings = [r for r in caplog.records if r.levelno == logging.WARNING]
    assert warnings
    assert all(r.name == "stac_hash" for r in warnings)


def test_in_extent_hash_does_not_warn(
    colorado: Hasher, start_datetime: datetime, caplog: pytest.LogCaptureFixture
):
    with caplog.at_level(logging.WARNING, logger="stac_hash"):
        colorado.hash_clamped(start_datetime, -105.0, 40.0)
    assert not caplog.records


def test_bbox_changes_the_hash(
    start_datetime: datetime, end_datetime: datetime, hasher: Hasher
):
    colorado = Hasher(start_datetime, end_datetime, bbox=(-109.0, 37.0, -102.0, 41.0))
    dt = datetime(2026, 6, 14, 12, tzinfo=UTC)
    assert colorado.hash(dt, -105.0, 40.0) != hasher.hash(dt, -105.0, 40.0)


def test_sorts_by_datetime(hasher: Hasher, start_datetime: datetime):
    a = hasher.hash(start_datetime, -105.0, 40.0)
    b = hasher.hash(start_datetime + timedelta(days=1), -105.0, 40.0)
    assert a < b


def test_datetime_out_of_extent(hasher: Hasher):
    with pytest.raises(ValueError):
        hasher.hash(datetime(2025, 1, 1, tzinfo=UTC), -105.0, 40.0)


def test_latitude_out_of_extent(hasher: Hasher, start_datetime: datetime):
    with pytest.raises(ValueError):
        hasher.hash(start_datetime, -105.0, 91.0)


def test_longitude_out_of_extent(hasher: Hasher, start_datetime: datetime):
    with pytest.raises(ValueError):
        hasher.hash(start_datetime, 181.0, 40.0)


def test_bbox_out_of_extent(start_datetime: datetime, end_datetime: datetime):
    colorado = Hasher(start_datetime, end_datetime, bbox=(-109.0, 37.0, -102.0, 41.0))
    with pytest.raises(ValueError):
        colorado.hash(start_datetime, -75.0, 40.0)


def test_non_utc_timezone_is_accepted(hasher: Hasher):
    mountain = timezone(timedelta(hours=-7))
    aware = datetime(2026, 6, 14, 5, tzinfo=mountain)
    assert hasher.hash(aware, -105.0, 40.0) == 3024785829217804842


def test_naive_datetime_raises(hasher: Hasher):
    with pytest.raises(TypeError):
        hasher.hash(datetime(2026, 6, 14, 12), -105.0, 40.0)  # noqa: DTZ001


def test_invalid_extent(end_datetime: datetime, start_datetime: datetime):
    with pytest.raises(ValueError):
        Hasher(start_datetime, end_datetime).hash(
            datetime(2028, 1, 1, tzinfo=UTC), 0.0, 0.0
        )


def test_hash_all_matches_hash(hasher: Hasher, start_datetime: datetime):
    datetimes = [start_datetime + timedelta(days=i) for i in range(3)]
    longitudes = [-105.0, -106.0, -107.0]
    latitudes = [40.0, 41.0, 42.0]
    assert hasher.hash_all(datetimes, longitudes, latitudes) == [
        hasher.hash(dt, lon, lat)
        for dt, lon, lat in zip(datetimes, longitudes, latitudes)
    ]


def test_hash_all_empty(hasher: Hasher):
    assert hasher.hash_all([], [], []) == []


def test_hash_all_accepts_tuples(hasher: Hasher, start_datetime: datetime):
    assert hasher.hash_all((start_datetime,), (-105.0,), (40.0,)) == [
        hasher.hash(start_datetime, -105.0, 40.0)
    ]


def test_hash_all_rejects_generators(hasher: Hasher, start_datetime: datetime):
    with pytest.raises(TypeError):
        hasher.hash_all(
            iter([start_datetime]),  # pyright: ignore[reportArgumentType]
            iter([-105.0]),  # pyright: ignore[reportArgumentType]
            iter([40.0]),  # pyright: ignore[reportArgumentType]
        )


def test_hash_all_raises_on_invalid(hasher: Hasher, start_datetime: datetime):
    with pytest.raises(ValueError):
        hasher.hash_all([start_datetime] * 2, [-105.0, 181.0], [40.0, 40.0])


def test_hash_all_skip_invalid(hasher: Hasher, start_datetime: datetime):
    hashes = hasher.hash_all(
        [start_datetime] * 3,
        [-105.0, 181.0, -107.0],
        [40.0, 40.0, 42.0],
        skip_invalid=True,
    )
    assert hashes[0] is not None
    assert hashes[1] is None
    assert hashes[2] is not None


def test_hash_all_length_mismatch(hasher: Hasher, start_datetime: datetime):
    with pytest.raises(ValueError):
        hasher.hash_all([start_datetime, start_datetime], [-105.0], [40.0])


def test_hash_all_skip_invalid_still_checks_length(
    hasher: Hasher, start_datetime: datetime
):
    with pytest.raises(ValueError):
        hasher.hash_all([start_datetime], [-105.0], [], skip_invalid=True)
