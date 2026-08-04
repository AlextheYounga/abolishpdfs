#!/usr/bin/env bash
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

in="$here/tests/fixtures/pdf2htmlEx/AlexYounger.pdf"
out="$here/out/alex-out"
pdfium_path="$here/vendor/pdfium-7881/lib/libpdfium.so"

ABOLISHPDFS_PDFIUM_PATH="$pdfium_path" "$here/bin/abolishpdfs" "$in" --output "$out"
