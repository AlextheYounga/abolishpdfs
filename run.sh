#!/usr/bin/env bash
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
git_root="$(cd "$(dirname "$(git -C "$here" rev-parse --git-common-dir)")" && pwd)"

in="${1:-$here/tests/fixtures/pdf2htmlEx/AlexYounger.pdf}"
out="${2:-$here/out/alex-out}"
pdfium_path="${ABOLISHPDFS_PDFIUM_PATH:-$git_root/thirdparty/pdfium/out/linux-x64-release/libpdfium.so}"
fontforge_path="${ABOLISHPDFS_FONTFORGE_PATH:-fontforge}"

[[ -f "$in" ]] || { printf 'Input PDF not found: %s\n' "$in" >&2; exit 1; }
[[ -f "$pdfium_path" ]] || {
    printf 'PDFium runtime not found: %s\n' "$pdfium_path" >&2
    printf 'Build the patched runtime or set ABOLISHPDFS_PDFIUM_PATH.\n' >&2
    exit 1
}
command -v "$fontforge_path" >/dev/null 2>&1 || {
    printf 'FontForge executable not found: %s\n' "$fontforge_path" >&2
    printf 'Install compatible FontForge or set ABOLISHPDFS_FONTFORGE_PATH.\n' >&2
    exit 1
}

mkdir -p "$out"
cargo run --quiet --bin abolishpdfs -- \
    --pdfium-path "$pdfium_path" \
    --fontforge-path "$fontforge_path" \
    --output "$out" \
    "$in"

printf 'Wrote HTML output to %s/index.html\n' "$out"
