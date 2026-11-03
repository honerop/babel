#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
: "${BABEL_BROWSER_DIR:=${HOME}/.local/share/babel-browser}"
export BABEL_BROWSER_DIR
command -v chromium >/dev/null || { echo "Chromium is required (use nix develop .#browser)" >&2; exit 1; }
python3 apps/browser/install_extensions.py
cargo run --quiet -- build apps/browser/main.babel -o target/babel-browser
exec target/babel-browser "$@"
