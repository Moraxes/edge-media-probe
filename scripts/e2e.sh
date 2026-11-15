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
    local rc=$?
    if [[ $rc -ne 0 ]]; then
        echo "=== E2E FAILED (rc=$rc) ===" >&2
        for f in "$TMPDIR"/*.log; do
            if [[ -f "$f" ]]; then
                echo "--- $f ---" >&2
                cat "$f" >&2 || true
            fi
        done
    fi
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

"$REPO_DIR/target/debug/kms" > "$TMPDIR/kms.log" 2>&1 &
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

curl -fsS -X POST "${KMS_URL}/v1/keys/0x1000" > "$TMPDIR/key_1000.json"
curl -fsS -X POST "${KMS_URL}/v1/keys/0x2000" > "$TMPDIR/key_2000.json"

"$REPO_DIR/target/debug/edge-probe" \
    --listen "127.0.0.1:${PROBE_PORT}" \
    --forward "127.0.0.1:${FORWARD_PORT}" \
    --workers "$WORKERS" > "$TMPDIR/probe.log" 2>&1 &
PIDS+=($!)

sleep 0.5

KEY_HEX=$(jq -r '.master_key_hex + .master_salt_hex' "$TMPDIR/key_1000.json")
SRTP_PARAMS=$(printf '%s' "$KEY_HEX" | xxd -r -p | base64)
cat > "$TMPDIR/recv.sdp" <<EOF
v=0
o=- 0 0 IN IP4 127.0.0.1
s=edge-media-probe
c=IN IP4 127.0.0.1
t=0 0
m=video ${FORWARD_PORT} RTP/SAVP 96
a=rtpmap:96 H264/90000
a=crypto:1 AES_CM_128_HMAC_SHA1_80 inline:${SRTP_PARAMS}
EOF

ffmpeg -y -v error \
    -protocol_whitelist file,udp,rtp,crypto,srtp \
    -i "$TMPDIR/recv.sdp" \
    -t 2 -f null - > "$TMPDIR/recv.log" 2>&1 &
RECV_PID=$!
PIDS+=($RECV_PID)

sleep 0.3

KMS_URL="$KMS_URL" "$REPO_DIR/scripts/sender.sh" 0x1000 "127.0.0.1:${PROBE_PORT}" > "$TMPDIR/sender1.log" 2>&1 &
S1_PID=$!
PIDS+=($S1_PID)

wait "$RECV_PID"
kill "$S1_PID" 2>/dev/null || true

KMS_URL="$KMS_URL" "$REPO_DIR/scripts/sender.sh" 0x2000 "127.0.0.1:${PROBE_PORT}" > "$TMPDIR/sender2.log" 2>&1 &
S2_PID=$!
PIDS+=($S2_PID)
sleep 2
kill "$S2_PID" 2>/dev/null || true

STREAMS_JSON=$(curl -fsS "${CONTROL_URL}/v1/streams")
STATS_JSON=$(curl -fsS "${CONTROL_URL}/v1/stats")

echo "=== /v1/streams ==="
echo "$STREAMS_JSON"
echo "=== /v1/stats ==="
echo "$STATS_JSON"
echo "=== edge-probe ==="
cat "$TMPDIR/probe.log"

printf '%s' "$STREAMS_JSON" | jq -e 'any(.[]; .ssrc == 4096) and any(.[]; .ssrc == 8192)' >/dev/null
echo "E2E OK (workers=${WORKERS})"
