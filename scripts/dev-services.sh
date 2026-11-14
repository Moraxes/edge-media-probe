#!/usr/bin/env bash
set -euo pipefail

KMS_PID=""
CP_PID=""

cleanup() {
    if [[ -n "$KMS_PID" ]]; then
        kill "$KMS_PID" 2>/dev/null || true
    fi
    if [[ -n "$CP_PID" ]]; then
        kill "$CP_PID" 2>/dev/null || true
    fi
    wait 2>/dev/null || true
}

trap cleanup INT TERM EXIT

cargo run --bin kms &
KMS_PID=$!

cargo run --bin control-plane &
CP_PID=$!

echo "Uruchomiono kms (PID $KMS_PID, :8081) oraz control-plane (PID $CP_PID, :8080). Ctrl+C kończy oba procesy."
wait
