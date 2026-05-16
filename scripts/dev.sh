#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEFAULT_CARDS_PATH="data/processed/cards.json"
IMPORTED_CARDS_PATH="data/processed/imported/mcsrainbow_cards.json"
API_ADDR="${SATORI_API_ADDR:-127.0.0.1:3000}"
API_URL="http://${API_ADDR}"

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

if [[ -z "${SATORI_CARDS_PATH:-}" && -z "${SATORI_CARDS_PATHS:-}" ]]; then
  if [[ -f "${IMPORTED_CARDS_PATH}" ]]; then
    export SATORI_CARDS_PATHS="${DEFAULT_CARDS_PATH}:${IMPORTED_CARDS_PATH}"
  else
    export SATORI_CARDS_PATH="${DEFAULT_CARDS_PATH}"
  fi
fi

if curl -fsS "${API_URL}/api/health" >/dev/null 2>&1; then
  if [[ "${SATORI_REUSE_API:-}" == "1" ]]; then
    echo "Using existing Satori API at ${API_URL}"
  else
    echo "Satori API is already running at ${API_URL}"
    echo "Stop that process first, or set SATORI_REUSE_API=1 to reuse it."
    exit 1
  fi
else
  if [[ -n "${SATORI_CARDS_PATHS:-}" ]]; then
    echo "Starting Satori API with SATORI_CARDS_PATHS=${SATORI_CARDS_PATHS}"
  else
    echo "Starting Satori API with SATORI_CARDS_PATH=${SATORI_CARDS_PATH}"
  fi
  cargo run -p satori-api &
  api_pid="$!"
fi

export VITE_SATORI_API_BASE_URL="${VITE_SATORI_API_BASE_URL:-${API_URL}}"
npm --prefix "${ROOT_DIR}/frontend" run dev -- --host 127.0.0.1 &
frontend_pid="$!"

if [[ -n "${api_pid}" ]]; then
  wait -n "${api_pid}" "${frontend_pid}"
else
  wait "${frontend_pid}"
fi
