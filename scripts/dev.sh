#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEFAULT_CARDS_PATH="data/processed/cards.json"
IMPORTED_CARDS_PATH="data/processed/imported/mcsrainbow_cards.json"

api_pid=""
frontend_pid=""

cleanup() {
  if [[ -n "${api_pid}" ]] && kill -0 "${api_pid}" 2>/dev/null; then
    kill "${api_pid}" 2>/dev/null || true
  fi

  if [[ -n "${frontend_pid}" ]] && kill -0 "${frontend_pid}" 2>/dev/null; then
    kill "${frontend_pid}" 2>/dev/null || true
  fi
}

trap cleanup EXIT INT TERM

cd "${ROOT_DIR}"

if [[ -z "${SATORI_CARDS_PATH:-}" ]]; then
  if [[ -f "${IMPORTED_CARDS_PATH}" ]]; then
    export SATORI_CARDS_PATH="${IMPORTED_CARDS_PATH}"
  else
    export SATORI_CARDS_PATH="${DEFAULT_CARDS_PATH}"
  fi
fi

if curl -fsS "http://127.0.0.1:3000/api/health" >/dev/null 2>&1; then
  if [[ "${SATORI_REUSE_API:-}" == "1" ]]; then
    echo "Using existing Satori API at http://127.0.0.1:3000"
  else
    echo "Satori API is already running at http://127.0.0.1:3000"
    echo "Stop that process first, or set SATORI_REUSE_API=1 to reuse it."
    exit 1
  fi
else
  echo "Starting Satori API with SATORI_CARDS_PATH=${SATORI_CARDS_PATH}"
  cargo run -p satori-api &
  api_pid="$!"
fi

npm --prefix "${ROOT_DIR}/frontend" run dev -- --host 127.0.0.1 &
frontend_pid="$!"

if [[ -n "${api_pid}" ]]; then
  wait -n "${api_pid}" "${frontend_pid}"
else
  wait "${frontend_pid}"
fi
