#!/usr/bin/env bash
set -euo pipefail

SSRC="${1:-0x1000}"
PROBE_ADDR="${2:-127.0.0.1:5004}"
KMS_URL="${KMS_URL:-http://127.0.0.1:8081}"

KEY_JSON=$(curl -fsS "${KMS_URL}/v1/keys/${SSRC}")
KEY_HEX=$(printf '%s' "$KEY_JSON" | jq -r '.master_key_hex + .master_salt_hex')
SRTP_PARAMS=$(printf '%s' "$KEY_HEX" | xxd -r -p | base64)

SSRC_DEC=$((SSRC))

exec ffmpeg -re \
    -f lavfi -i testsrc=size=1280x720:rate=30 \
    -c:v libx264 -tune zerolatency -preset ultrafast \
    -ssrc "$SSRC_DEC" \
    -f rtp \
    -srtp_out_suite AES_CM_128_HMAC_SHA1_80 \
    -srtp_out_params "$SRTP_PARAMS" \
    "srtp://${PROBE_ADDR}"
