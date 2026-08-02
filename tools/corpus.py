#!/usr/bin/env python3
"""Validate and optionally execute the compatibility corpus manifest."""

import argparse
import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "tests" / "fixtures"
MANIFEST = FIXTURES / "manifest.json"


def load_manifest():
    return json.loads(MANIFEST.read_text())


def validate(manifest):
    failures = []
    for fixture in manifest["fixtures"]:
        path = FIXTURES / fixture["path"]
        if not path.is_file():
            failures.append(f"{fixture['id']}: missing {path.relative_to(ROOT)}")
        if not fixture["classifications"]:
            failures.append(f"{fixture['id']}: no feature classifications")
        if not fixture.get("expected"):
            failures.append(f"{fixture['id']}: no expected diagnostic classification")
    for external in manifest.get("external_corpus", []):
        if external["status"] != "pending":
            failures.append(f"{external['id']}: external corpus status must be explicit")
    return failures


def run_diagnostics(manifest, binary, pdfium):
    results = []
    for fixture in manifest["fixtures"]:
        path = FIXTURES / fixture["path"]
        command = [str(binary), "--pdfium-path", str(pdfium), "--diagnostic", str(path)]
        completed = subprocess.run(command, capture_output=True, text=True, check=False)
        result = {"id": fixture["id"], "exit_code": completed.returncode}
        if completed.returncode == 0:
            report = json.loads(completed.stdout)
            expected = fixture["expected"]
            result["pages"] = report["pages"]
            result["classification_passed"] = (
                len(report["pages"]) == expected["pages"]
                and sum(len(page["text_objects"]) for page in report["pages"]) >= expected["text_objects_min"]
                and sum(len(page["links"]) for page in report["pages"]) == expected["links"]
            )
        else:
            result["stderr"] = completed.stderr
            result["classification_passed"] = False
        results.append(result)
    return results


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=["validate", "run"])
    parser.add_argument("--binary", type=Path, default=ROOT / "target" / "debug" / "abolishpdfs")
    parser.add_argument("--pdfium", type=Path)
    args = parser.parse_args()
    manifest = load_manifest()
    failures = validate(manifest)
    if failures:
        for failure in failures:
            print(f"ERROR: {failure}")
        raise SystemExit(1)
    if args.command == "validate":
        print(f"validated {len(manifest['fixtures'])} generated fixtures")
        print("screenshot and clipboard classifications are recorded as pending")
        return
    if not args.pdfium:
        raise SystemExit("--pdfium is required for run")
    print(json.dumps(run_diagnostics(manifest, args.binary, args.pdfium), indent=2))


if __name__ == "__main__":
    main()
