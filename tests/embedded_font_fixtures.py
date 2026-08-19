#!/usr/bin/env python3
"""Integrity and reproducibility checks for embedded-font PDF fixtures."""

import re
import subprocess
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
GENERATED = ROOT / "tests" / "fixtures" / "generated"
GENERATOR = ROOT / "tools" / "generate_corpus.py"


def objects(pdf: bytes) -> dict[int, bytes]:
    return {
        int(match.group(1)): match.group(2)
        for match in re.finditer(rb"(?m)^(\d+) 0 obj\n(.*?)\nendobj\n", pdf, re.DOTALL)
    }


def embedded_fonts(pdf: bytes) -> list[tuple[str, bytes]]:
    parsed = objects(pdf)
    descriptors = {}
    for body in parsed.values():
        match = re.search(rb"/Type /FontDescriptor.*?/FontName /([^\s]+).*?/FontFile2 (\d+) 0 R", body, re.DOTALL)
        if match:
            descriptors[int(match.group(2))] = match.group(1).decode("ascii")
    result = []
    for body in parsed.values():
        if b"/Subtype /TrueType" not in body:
            continue
        name = re.search(rb"/BaseFont /([^\s]+)", body)
        reference = re.search(rb"/FontDescriptor (\d+) 0 R", body)
        if name is None or reference is None:
            continue
        descriptor = parsed[int(reference.group(1))]
        stream_reference = re.search(rb"/FontFile2 (\d+) 0 R", descriptor)
        if stream_reference is None:
            continue
        stream = parsed[int(stream_reference.group(1))].split(b"stream\n", 1)[1].rsplit(b"\nendstream", 1)[0]
        result.append((name.group(1).decode("ascii"), stream))
    return result


class EmbeddedFontFixtureTests(unittest.TestCase):
    fixtures = {
        "embedded-basic.pdf": (1, {"LiberationSans"}),
        "embedded-subset-name.pdf": (1, {"ABCDEF+LiberationSans"}),
        "embedded-multiple-fonts.pdf": (2, {"LiberationSans", "LiberationSerif"}),
        "embedded-size-transform.pdf": (1, {"LiberationSans"}),
        "embedded-vertical-slice.pdf": (1, {"LiberationSans"}),
    }

    def test_each_fixture_contains_declared_truetype_programs(self):
        for filename, (count, expected_names) in self.fixtures.items():
            with self.subTest(filename=filename):
                fonts = embedded_fonts((GENERATED / filename).read_bytes())
                self.assertEqual(len(fonts), count)
                self.assertEqual({name for name, _ in fonts}, expected_names)
                self.assertTrue(all(data[:4] in (b"\x00\x01\x00\x00", b"OTTO") for _, data in fonts))

    def test_generation_is_byte_for_byte_deterministic(self):
        before = {filename: (GENERATED / filename).read_bytes() for filename in self.fixtures}
        subprocess.run([sys.executable, str(GENERATOR)], check=True, cwd=ROOT)
        after = {filename: (GENERATED / filename).read_bytes() for filename in self.fixtures}
        self.assertEqual(before, after)


if __name__ == "__main__":
    unittest.main()
