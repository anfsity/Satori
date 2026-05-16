#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MCSRAINBOW_URL="https://raw.githubusercontent.com/mcsrainbow/chinese-internet-jargon/master/readme.md"
MCSRAINBOW_RAW_PATH="${ROOT_DIR}/data/raw/mcsrainbow/readme.md"
MCSRAINBOW_CARDS_PATH="${ROOT_DIR}/data/processed/imported/mcsrainbow_cards.json"

require_command() {
  local name="$1"

  if ! command -v "${name}" >/dev/null 2>&1; then
    echo "Missing required command: ${name}" >&2
    exit 1
  fi
}

require_command cargo
require_command curl
require_command npm

cd "${ROOT_DIR}"

echo "Installing frontend dependencies"
npm --prefix "${ROOT_DIR}/frontend" ci

echo "Downloading mcsrainbow corpus"
mkdir -p "$(dirname "${MCSRAINBOW_RAW_PATH}")"
curl -fL "${MCSRAINBOW_URL}" -o "${MCSRAINBOW_RAW_PATH}"

echo "Importing mcsrainbow corpus"
cargo run -p satori-indexer -- import-mcsrainbow \
  "${MCSRAINBOW_RAW_PATH}" \
  "${MCSRAINBOW_CARDS_PATH}"

echo "Validating imported corpus"
cargo run -p satori-indexer -- validate "${MCSRAINBOW_CARDS_PATH}"

echo "Satori install complete"
echo "Run scripts/dev.sh to start the app"
