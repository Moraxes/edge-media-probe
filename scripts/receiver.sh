#!/usr/bin/env bash
set -euo pipefail

SSRC="${1:-0x1000}"
LISTEN_PORT="${2:-5006}"
KMS_URL="${KMS_URL:-http://127.0.0.1:8081}"

KEY_JSON=$(curl -fsS "${KMS_URL}/v1/keys/${SSRC}")
KEY_HEX=$(printf '%s' "$KEY_JSON" | jq -r '.master_key_hex + .master_salt_hex')
SRTP_PARAMS=$(printf '%s' "$KEY_HEX" | xxd -r -p | base64)

SDP_FILE=$(mktemp /tmp/edge-probe-receiver.XXXXXX.sdp)
trap 'rm -f "$SDP_FILE"' EXIT

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

exec ffplay -protocol_whitelist file,udp,rtp,crypto,srtp -i "$SDP_FILE"
