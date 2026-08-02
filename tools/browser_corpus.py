#!/usr/bin/env python3
"""Run optional browser screenshot and copied-text checks for corpus output."""

import argparse
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path, help="Directory containing one output subdirectory per fixture")
    parser.add_argument("--screenshots", type=Path, default=ROOT / "tests" / "fixtures" / "screenshots")
    args = parser.parse_args()
    try:
        from playwright.sync_api import sync_playwright
    except ImportError as error:
        raise SystemExit("Install Playwright and a browser to run screenshot checks") from error

    manifest = json.loads((ROOT / "tests" / "fixtures" / "manifest.json").read_text())
    args.screenshots.mkdir(parents=True, exist_ok=True)
    results = []
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch()
        context = browser.new_context(permissions=["clipboard-read", "clipboard-write"])
        for fixture in manifest["fixtures"]:
            page_file = args.output / fixture["id"] / "pages" / "1.html"
            page = context.new_page()
            page.goto(page_file.resolve().as_uri())
            screenshot = args.screenshots / f"{fixture['id']}-page-1.png"
            page.screenshot(path=str(screenshot), full_page=True)
            page.keyboard.press("ControlOrMeta+A")
            page.keyboard.press("ControlOrMeta+C")
            copied = page.evaluate("navigator.clipboard.readText()")
            expected = fixture["clipboard"]["expected_text"]
            results.append({
                "id": fixture["id"],
                "screenshot": str(screenshot.relative_to(ROOT)),
                "clipboard_matches": expected in copied,
                "copied_text": copied,
            })
            page.close()
        browser.close()
    print(json.dumps(results, indent=2))
    if not all(result["clipboard_matches"] for result in results):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
