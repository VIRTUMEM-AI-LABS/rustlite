#!/usr/bin/env bash
set -euo pipefail

# Generate PNG exports and favicon from SVG assets using rsvg-convert or imagemagick
# Usage: ./scripts/generate-assets.sh

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
ASSETS_DIR="$ROOT_DIR/assets"
OUT_DIR="$ASSETS_DIR/exports"

mkdir -p "$OUT_DIR"

SVG_WORDMARK="$ASSETS_DIR/logo-wordmark.svg"
SVG_ICON="$ASSETS_DIR/logo-icon.svg"

echo "Generating PNG exports into $OUT_DIR"

if command -v rsvg-convert >/dev/null 2>&1; then
  echo "Using rsvg-convert"
  rsvg-convert -w 360 -h 68 "$SVG_WORDMARK" -o "$OUT_DIR/logo-wordmark-360x68.png"
  rsvg-convert -w 180 -h 34 "$SVG_WORDMARK" -o "$OUT_DIR/logo-wordmark-180x34.png"
  rsvg-convert -w 256 -h 256 "$SVG_ICON" -o "$OUT_DIR/logo-icon-256.png"
  rsvg-convert -w 128 -h 128 "$SVG_ICON" -o "$OUT_DIR/logo-icon-128.png"
  rsvg-convert -w 64 -h 64 "$SVG_ICON" -o "$OUT_DIR/logo-icon-64.png"
  rsvg-convert -w 32 -h 32 "$SVG_ICON" -o "$OUT_DIR/logo-icon-32.png"
else
  echo "rsvg-convert not found; trying ImageMagick (convert)"
  if command -v convert >/dev/null 2>&1; then
    convert -background none "$SVG_WORDMARK" -resize 360x68 "$OUT_DIR/logo-wordmark-360x68.png"
    convert -background none "$SVG_WORDMARK" -resize 180x34 "$OUT_DIR/logo-wordmark-180x34.png"
    convert -background none "$SVG_ICON" -resize 256x256 "$OUT_DIR/logo-icon-256.png"
    convert -background none "$SVG_ICON" -resize 128x128 "$OUT_DIR/logo-icon-128.png"
    convert -background none "$SVG_ICON" -resize 64x64 "$OUT_DIR/logo-icon-64.png"
    convert -background none "$SVG_ICON" -resize 32x32 "$OUT_DIR/logo-icon-32.png"
  else
    echo "No SVG renderer found (rsvg-convert or ImageMagick). Install one to generate raster assets." >&2
    exit 2
  fi
fi

# favicon.ico (contains 32x32 and 16x16)
if command -v convert >/dev/null 2>&1; then
  convert "$OUT_DIR/logo-icon-32.png" -define icon:auto-resize=16,32 "$ASSETS_DIR/favicon.ico"
  echo "Created $ASSETS_DIR/favicon.ico"
else
  echo "ImageMagick 'convert' not available; favicon not created. You can generate favicon.ico manually." >&2
fi

echo "Exports written to $OUT_DIR"
