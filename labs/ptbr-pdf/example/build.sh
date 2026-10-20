#!/usr/bin/env bash
# Smoke build: the sample through the same command a consumer runs (to-pdf).
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out="${OUT_DIR:-$here/build}"
mkdir -p "$out"
to-pdf "$here/smoke.md" "$out/smoke.pdf"
