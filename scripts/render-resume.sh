#!/bin/bash
# Renders the resume PDF to one SVG per page for inline display on
# /resume/. Run after scripts/fetch-resume.sh and before `cargo run -p site`.
#
# Vector rather than raster: the SVG is ~69 KB gzipped and stays crisp at
# any zoom, where a 150 dpi PNG is ~199 KB and goes soft.
set -euo pipefail

PDF=assets/paul_maxwell_resume.pdf
OUT=static/resume

if [ ! -f "$PDF" ]; then
    echo "Error: $PDF not found — run scripts/fetch-resume.sh first" >&2
    exit 1
fi

PAGES=$(pdfinfo "$PDF" | awk '/^Pages:/ { print $2 }')
if [ -z "$PAGES" ] || [ "$PAGES" -lt 1 ]; then
    echo "Error: could not determine page count of $PDF" >&2
    exit 1
fi

rm -rf "$OUT"
mkdir -p "$OUT"

for p in $(seq 1 "$PAGES"); do
    printf -v name "page-%02d.svg" "$p"
    pdftocairo -svg -f "$p" -l "$p" "$PDF" "$OUT/$name"
done

echo "Rendered $PAGES page(s) to $OUT:"
ls -la "$OUT"
