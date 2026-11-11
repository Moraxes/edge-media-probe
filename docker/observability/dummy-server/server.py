from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json


def parse_ssrc(path: str) -> int:
    tail = path.rstrip("/").rsplit("/", 1)[-1]
    try:
        return int(tail, 0) & 0xFFFF_FFFF
    except ValueError:
        return 0x1000


class DummyKmsHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        payload = {
            "ssrc": parse_ssrc(self.path),
            "master_key_hex": "00112233445566778899aabbccddeeff",
            "master_salt_hex": "00112233445566778899aabbccdd",
            "issued_at": datetime.now(timezone.utc)
            .isoformat(timespec="seconds")
            .replace("+00:00", "Z"),
        }
        body = json.dumps(payload).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


if __name__ == "__main__":
    server = ThreadingHTTPServer(("0.0.0.0", 8080), DummyKmsHandler)
    server.serve_forever()
