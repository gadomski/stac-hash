import importlib.metadata

import stac_hash


def test_version():
    assert stac_hash.__version__ == importlib.metadata.version("stac-hash")
