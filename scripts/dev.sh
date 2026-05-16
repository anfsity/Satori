#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

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

if curl -fsS "http://127.0.0.1:3000/api/health" >/dev/null 2>&1; then
  echo "Using existing Satori API at http://127.0.0.1:3000"
else
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
