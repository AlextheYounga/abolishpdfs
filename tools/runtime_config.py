#!/usr/bin/env python3
"""Read the tracked development runtime manifest for shell entry points."""

from __future__ import annotations

import shlex
import sys
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "tools" / "runtime.toml"


def load_manifest() -> dict:
    with MANIFEST.open("rb") as manifest:
        return tomllib.load(manifest)


def shell_values() -> dict[str, str]:
    manifest = load_manifest()
    versions = manifest["versions"]
    pdfium = manifest["pdfium"]
    runtime = manifest["runtime"]
    runtime_directory = ROOT / runtime["directory"]
    return {
        "PDFIUM_VERSION": versions["pdfium"],
        "FONTFORGE_VERSION": versions["fontforge"],
        "PLAYWRIGHT_VERSION": versions["playwright"],
        "PDFIUM_URL": pdfium["url"],
        "PDFIUM_SHA256": pdfium["sha256"],
        "PDFIUM_LIBRARY_RELATIVE": pdfium["library"],
        "RUNTIME_DIRECTORY": str(runtime_directory),
        "PDFIUM_DIRECTORY": str(runtime_directory / runtime["pdfium_directory"]),
        "PYTHON_DIRECTORY": str(runtime_directory / runtime["python_directory"]),
        "BROWSER_DIRECTORY": str(runtime_directory / runtime["browser_directory"]),
        "INTEGRATION_DIRECTORY": str(runtime_directory / runtime["integration_directory"]),
        "SCREENSHOT_DIRECTORY": str(runtime_directory / runtime["screenshot_directory"]),
    }


def main() -> int:
    if len(sys.argv) != 2 or sys.argv[1] != "shell":
        print(f"usage: {Path(sys.argv[0]).name} shell", file=sys.stderr)
        return 2

    try:
        values = shell_values()
    except (KeyError, OSError, tomllib.TOMLDecodeError) as error:
        print(f"runtime manifest is invalid: {error}", file=sys.stderr)
        return 1

    for name, value in values.items():
        print(f"{name}={shlex.quote(value)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
