#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
WORKERS="${WORKERS:-2}"
PROBE_PORT="${PROBE_PORT:-5004}"
FORWARD_PORT="${FORWARD_PORT:-5006}"
KMS_URL="${KMS_URL:-http://127.0.0.1:8081}"
CONTROL_URL="${CONTROL_URL:-http://127.0.0.1:8080}"

TMPDIR="$(mktemp -d /tmp/edge-probe-e2e.XXXXXX)"
PIDS=()

cleanup() {
    for pid in "${PIDS[@]:-}"; do
        kill "$pid" 2>/dev/null || true
    done
    wait 2>/dev/null || true
    rm -rf "$TMPDIR"
}
trap cleanup EXIT INT TERM

cd "$REPO_DIR"
cargo build --bins -p edge-probe

cd "$TMPDIR"

KMS_KEY_TTL_SECS="${KMS_KEY_TTL_SECS:-3600}" "$REPO_DIR/target/debug/kms" > "$TMPDIR/kms.log" 2>&1 &
PIDS+=($!)

"$REPO_DIR/target/debug/control-plane" > "$TMPDIR/cp.log" 2>&1 &
PIDS+=($!)

for _ in $(seq 1 50); do
    if curl -fsS "${CONTROL_URL}/v1/stats" >/dev/null 2>&1; then
        break
    fi
    sleep 0.2
done
curl -fsS "${CONTROL_URL}/v1/stats" >/dev/null

curl -fsS -X POST "${KMS_URL}/v1/keys/0x1000" >/dev/null

DIAG="${DIAG:-none}"
TRACE_FILE="${TRACE_FILE:-$REPO_DIR/edge-probe-trace.json}"

"$REPO_DIR/target/debug/edge-probe" \
    --listen "127.0.0.1:${PROBE_PORT}" \
    --forward "127.0.0.1:${FORWARD_PORT}" \
    --workers "$WORKERS" \
    --diag "$DIAG" \
    --trace-file "$TRACE_FILE" &
PIDS+=($!)

sleep 0.5

KMS_URL="$KMS_URL" "$REPO_DIR/scripts/receiver.sh" 0x1000 "${FORWARD_PORT}" &
RECV_PID=$!
PIDS+=($RECV_PID)

sleep 0.3

KMS_URL="$KMS_URL" "$REPO_DIR/scripts/sender.sh" 0x1000 "127.0.0.1:${PROBE_PORT}" &
S1_PID=$!
PIDS+=($S1_PID)

echo "Zamknij okno ffplay (lub nacisnij Ctrl+C), aby zakonczyc test E2E."
wait "$RECV_PID" || true

echo "=== /v1/streams ==="
curl -fsS "${CONTROL_URL}/v1/streams" || true
echo ""
echo "=== /v1/stats ==="
curl -fsS "${CONTROL_URL}/v1/stats" || true
echo ""
