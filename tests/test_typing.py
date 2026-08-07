from pathlib import Path

import stac_hash


def test_stub_is_installed():
    directory = Path(stac_hash.__file__).parent
    assert (directory / "__init__.pyi").exists()
    assert (directory / "py.typed").exists()
