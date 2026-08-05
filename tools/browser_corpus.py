#!/usr/bin/env python3
"""Check generated corpus HTML through a real browser."""

import argparse
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "tests" / "fixtures" / "manifest.json"


def check_fixture(page, fixture, output, screenshots):
    fixture_id = fixture["id"]
    expected = fixture["expected"]
    page_file = output / fixture_id / "index.html"
    result = {"id": fixture_id, "passed": False, "failures": []}
    if not page_file.is_file():
        result["failures"].append(f"missing {page_file}")
        return result

    page.goto(page_file.resolve().as_uri())
    pages = page.locator(".page")
    page_count = pages.count()
    result["page_count"] = page_count
    if page_count != expected["pages"]:
        result["failures"].append(f"expected {expected['pages']} pages, found {page_count}")
    page_ids = [pages.nth(index).get_attribute("id") for index in range(page_count)]
    expected_ids = [f"page-{index}" for index in range(1, expected["pages"] + 1)]
    if page_ids != expected_ids:
        result["failures"].append(f"page order is {page_ids}, expected {expected_ids}")
    if page_count == 0:
        result["failures"].append("output contains no pages")
        return result

    native_text = "".join(page.locator(".text-glyph").all_inner_texts())
    result["native_text"] = native_text
    for text in expected["native_text"]:
        if text not in native_text:
            result["failures"].append(f"native text is missing {text!r}")

    page.keyboard.press("ControlOrMeta+A")
    page.keyboard.press("ControlOrMeta+C")
    try:
        copied = page.evaluate("navigator.clipboard.readText()")
    except Exception:
        copied = page.evaluate("window.getSelection().toString()")
    result["copied_text"] = copied
    if fixture["clipboard"]["expected_text"] not in copied:
        result["failures"].append("copied text does not contain the expected text")

    hrefs = sorted(page.locator("a[href]").evaluate_all("links => links.map(link => link.href)"))
    expected_hrefs = sorted(fixture["navigation"]["hrefs"])
    result["hrefs"] = hrefs
    if hrefs != expected_hrefs:
        result["failures"].append(f"navigation hrefs are {hrefs}, expected {expected_hrefs}")
    for fragment in fixture["navigation"]["fragments"]:
        if page.locator(f'a[href="{fragment}"]').count() == 0 and page.locator(f"{fragment}").count() == 0:
            result["failures"].append(f"missing navigation fragment {fragment}")

    missing_assets = [
        asset for asset in fixture["assets"]["required"] if not (output / fixture_id / "assets" / asset).is_file()
    ]
    result["missing_assets"] = missing_assets
    if missing_assets:
        result["failures"].append(f"missing assets: {', '.join(missing_assets)}")

    screenshot = fixture["screenshot"]
    screenshot_file = screenshots / f"{fixture_id}-page-1.png"
    pages.first.screenshot(path=str(screenshot_file))
    result["screenshot"] = str(screenshot_file.relative_to(ROOT))
    if screenshot["status"] == "compare":
        result["screenshot_comparison"] = compare_screenshot(screenshot_file, ROOT / screenshot["baseline"], screenshot)
        if not result["screenshot_comparison"]["passed"]:
            result["failures"].append("screenshot is outside the configured tolerance")

    result["passed"] = not result["failures"]
    return result


def compare_screenshot(actual, baseline, tolerance):
    try:
        from PIL import Image, ImageChops
    except ImportError:
        return {"passed": False, "error": "Pillow is required for screenshot comparison"}
    if not baseline.is_file():
        return {"passed": False, "error": f"missing baseline {baseline}"}
    with Image.open(actual).convert("RGBA") as actual_image, Image.open(baseline).convert("RGBA") as baseline_image:
        if actual_image.size != baseline_image.size:
            return {"passed": False, "error": "screenshot dimensions differ"}
        difference = ImageChops.difference(actual_image, baseline_image)
        pixels = sum(1 for pixel in difference.getdata() if max(pixel) > 8)
        total = actual_image.width * actual_image.height
    ratio = pixels / total if total else 1.0
    return {"passed": pixels <= tolerance["max_diff_pixels"] and ratio <= tolerance["max_diff_ratio"], "pixels": pixels, "ratio": ratio}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path, help="Directory containing one output subdirectory per fixture")
    parser.add_argument("--screenshots", type=Path, default=ROOT / "target" / "corpus-screenshots")
    args = parser.parse_args()
    try:
        from playwright.sync_api import sync_playwright
    except ImportError as error:
        raise SystemExit("Install Playwright and a browser to run browser corpus checks") from error

    manifest = json.loads(MANIFEST.read_text())
    args.screenshots.mkdir(parents=True, exist_ok=True)
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch()
        context = browser.new_context(permissions=["clipboard-read", "clipboard-write"])
        results = []
        for fixture in manifest["fixtures"]:
            page = context.new_page()
            try:
                results.append(check_fixture(page, fixture, args.output, args.screenshots))
            finally:
                page.close()
        context.close()
        browser.close()
    passed = all(result["passed"] for result in results)
    print(json.dumps({"fixtures": results, "passed": passed}, indent=2))
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
