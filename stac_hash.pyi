from collections.abc import Sequence
from datetime import datetime
from typing import Literal, overload

__version__: str

class Hasher:
    """Creates sortable spatio-temporal hashes.

    The hasher is built from a datetime range and, optionally, a bounding box.
    Without a `bbox` the hasher covers the whole world.
    """

    def __init__(
        self,
        start_datetime: datetime,
        end_datetime: datetime,
        bbox: tuple[float, float, float, float] | None = None,
    ) -> None: ...
    def hash(self, datetime: datetime, longitude: float, latitude: float) -> int:
        """Hashes a datetime and a point into an `int`."""

    @overload
    def hash_all(
        self,
        datetimes: Sequence[datetime],
        longitudes: Sequence[float],
        latitudes: Sequence[float],
        skip_invalid: Literal[False] = False,
    ) -> list[int]:
        """Hashes parallel sequences of datetimes, longitudes, and latitudes."""

    @overload
    def hash_all(
        self,
        datetimes: Sequence[datetime],
        longitudes: Sequence[float],
        latitudes: Sequence[float],
        skip_invalid: Literal[True],
    ) -> list[int | None]:
        """Hashes parallel sequences of datetimes, longitudes, and latitudes.

        Every item is checked independently; out-of-extent items become
        `None` in the result instead of raising.
        """

    @overload
    def hash_all(
        self,
        datetimes: Sequence[datetime],
        longitudes: Sequence[float],
        latitudes: Sequence[float],
        skip_invalid: bool,
    ) -> list[int | None]:
        """Hashes parallel sequences of datetimes, longitudes, and latitudes.

        Raises a `ValueError` if the sequences are not the same length, or if
        any item falls outside the hasher's extent. Pass `skip_invalid=True`
        to get `None` for out-of-extent items instead of raising.
        """
