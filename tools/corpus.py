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
        expected = fixture.get("expected", {})
        if len(expected.get("page_sizes", [])) != expected.get("pages"):
            failures.append(f"{fixture['id']}: page_sizes must cover every page")
        if not expected.get("native_text"):
            failures.append(f"{fixture['id']}: no native text expectations")
        for name in ("clipboard", "navigation", "assets", "screenshot"):
            if name not in fixture:
                failures.append(f"{fixture['id']}: missing {name} expectations")
        screenshot = fixture.get("screenshot", {})
        if screenshot.get("status") not in {"capture-only", "compare"}:
            failures.append(f"{fixture['id']}: invalid screenshot status")
        if not 0 <= screenshot.get("max_diff_ratio", -1) <= 1:
            failures.append(f"{fixture['id']}: screenshot ratio tolerance must be between 0 and 1")
        if screenshot.get("status") == "compare" and not screenshot.get("baseline"):
            failures.append(f"{fixture['id']}: compare screenshots require a baseline")
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
        result = {"id": fixture["id"], "exit_code": completed.returncode, "passed": False}
        if completed.returncode == 0:
            try:
                report = json.loads(completed.stdout)
                expected = fixture["expected"]
                pages = report["pages"]
                observed_sizes = [
                    {"width": page["crop_box"]["right"] - page["crop_box"]["left"], "height": page["crop_box"]["top"] - page["crop_box"]["bottom"]}
                    for page in pages
                ]
                observed_fallback = any(page["background"] is not None for page in pages)
                result.update({
                    "pages": len(pages),
                    "text_objects": sum(len(page["text_objects"]) for page in pages),
                    "links": sum(len(page["links"]) for page in pages),
                    "page_sizes": observed_sizes,
                    "fallback": "background" if observed_fallback else "none",
                })
                result["passed"] = (
                    len(pages) == expected["pages"]
                    and result["text_objects"] >= expected["text_objects_min"]
                    and result["links"] == expected["links"]
                    and observed_sizes == expected["page_sizes"]
                    and result["fallback"] == expected["fallback"]
                )
            except (KeyError, json.JSONDecodeError, TypeError) as error:
                result["error"] = f"invalid diagnostic output: {error}"
        else:
            result["stderr"] = completed.stderr
            result["error"] = "diagnostic command failed"
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
        print("native text, clipboard, navigation, asset, fallback, and screenshot expectations are valid")
        return
    if not args.pdfium:
        raise SystemExit("--pdfium is required for run")
    results = run_diagnostics(manifest, args.binary, args.pdfium)
    print(json.dumps({"fixtures": results, "passed": all(result["passed"] for result in results)}, indent=2))
    if not all(result["passed"] for result in results):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
