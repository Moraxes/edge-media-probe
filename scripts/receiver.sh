#!/usr/bin/env bash
set -euo pipefail

SSRC="${1:-0x1000}"
LISTEN_PORT="${2:-5006}"
KMS_URL="${KMS_URL:-http://127.0.0.1:8081}"

KEY_JSON=$(curl -fsS "${KMS_URL}/v1/keys/${SSRC}")
KEY_HEX=$(printf '%s' "$KEY_JSON" | jq -r '.master_key_hex + .master_salt_hex')
SRTP_PARAMS=$(printf '%s' "$KEY_HEX" | xxd -r -p | base64)

SDP_DIR=$(mktemp -d /tmp/edge-probe-receiver.XXXXXX)
SDP_FILE="${SDP_DIR}/recv.sdp"
FFPLAY_PID=""
cleanup() {
    if [[ -n "$FFPLAY_PID" ]]; then
        kill "$FFPLAY_PID" 2>/dev/null || true
    fi
    rm -rf "$SDP_DIR"
}
trap cleanup EXIT INT TERM

cat > "$SDP_FILE" <<EOF
v=0
o=- 0 0 IN IP4 127.0.0.1
s=edge-media-probe
c=IN IP4 127.0.0.1
t=0 0
m=video ${LISTEN_PORT} RTP/SAVP 96
a=rtpmap:96 H264/90000
a=crypto:1 AES_CM_128_HMAC_SHA1_80 inline:${SRTP_PARAMS}
EOF

ffplay -protocol_whitelist file,udp,rtp,crypto,srtp -i "$SDP_FILE" &
FFPLAY_PID=$!
wait "$FFPLAY_PID"
