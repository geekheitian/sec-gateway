#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"
GW_LOG="$(mktemp /tmp/sec-gateway-gw.XXXXXX.log)"
UPSTREAM_LOG="$(mktemp /tmp/sec-gateway-upstream.XXXXXX.log)"
REQUEST_ID="110101199001011234"
GATEWAY_PORT="${SERVER_PORT:-8080}"

cleanup() {
    if [[ -n "${GW_PID:-}" ]]; then
        kill "$GW_PID" 2>/dev/null || true
    fi
    if [[ -n "${UPSTREAM_PID:-}" ]]; then
        kill "$UPSTREAM_PID" 2>/dev/null || true
    fi
    rm -f "$GW_LOG" "$UPSTREAM_LOG"
}

trap cleanup EXIT INT TERM

LISTENER_PID=""
LISTENER_CMD=""
LISTENER_PID="$(lsof -i :"${GATEWAY_PORT}" -sTCP:LISTEN -t 2>/dev/null | head -n 1 || true)"
if [[ -n "$LISTENER_PID" ]]; then
    LISTENER_CMD="$(ps -p "$LISTENER_PID" -o command= 2>/dev/null || true)"
fi

if [[ -n "$LISTENER_PID" && "$LISTENER_CMD" != *"sec-gateway"* ]]; then
    echo "端口 ${GATEWAY_PORT} 已被其他进程占用（PID: ${LISTENER_PID}）。请先停止该进程，再重新运行脚本。"
    exit 1
fi

USE_EXISTING_GATEWAY=false
if [[ -n "$LISTENER_PID" ]]; then
    USE_EXISTING_GATEWAY=true
    echo "检测到已有 sec-gateway 在 ${GATEWAY_PORT} 监听（PID: ${LISTENER_PID}），将直接发送请求。"
    echo "如果你想同时验证网关日志，请切回运行该进程的终端查看输出。"
fi

UPSTREAM_PORT="$(
    python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
)"

if [[ "$USE_EXISTING_GATEWAY" == false ]]; then
python3 -u - "$UPSTREAM_PORT" >"$UPSTREAM_LOG" 2>&1 <<'PY' &
import json
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

port = int(sys.argv[1])


class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        length = int(self.headers.get("Content-Length", "0"))
        body = self.rfile.read(length).decode("utf-8", errors="replace")
        payload = {
            "path": self.path,
            "content_type": self.headers.get("Content-Type"),
            "received_body": body,
        }
        data = json.dumps(payload, ensure_ascii=False).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def log_message(self, format, *args):
        return


server = ThreadingHTTPServer(("127.0.0.1", port), Handler)
server.serve_forever()
PY
UPSTREAM_PID=$!

TARGET_URL="http://127.0.0.1:${UPSTREAM_PORT}/v1/chat/completions" \
SERVER_PORT="${GATEWAY_PORT}" \
RUST_LOG=debug \
cargo run --release >"$GW_LOG" 2>&1 &
GW_PID=$!

echo "等待网关启动..."
for _ in {1..30}; do
    if curl -fsS "http://127.0.0.1:${GATEWAY_PORT}/health" >/dev/null 2>&1; then
        break
    fi
    sleep 1
done

if ! curl -fsS "http://127.0.0.1:${GATEWAY_PORT}/health" >/dev/null 2>&1; then
    echo "网关启动失败，请查看日志：$GW_LOG"
    exit 1
fi
fi

REQUEST="$(cat <<JSON
{
  "model": "gpt-4o-mini",
  "messages": [
      {
        "role": "user",
        "content": "请处理这条测试消息：我的身份证号是 ${REQUEST_ID}，请确认已脱敏后再转发。"
      }
  ],
  "temperature": 0
}
JSON
)"

echo
echo "==== 待发送原始内容 ===="
echo "原始身份证号: ${REQUEST_ID}"
echo "原始请求体:"
if command -v jq >/dev/null 2>&1; then
    echo "$REQUEST" | jq .
else
    echo "$REQUEST"
fi

echo "发送 OpenAI 兼容请求到 http://localhost:${GATEWAY_PORT}/v1/chat/completions ..."
RESPONSE="$(curl -sS -X POST "http://127.0.0.1:${GATEWAY_PORT}/v1/chat/completions" \
    -H "Content-Type: application/json" \
    -d "$REQUEST")"

echo
echo "==== 上游回显响应（应包含脱敏后的内容） ===="
if command -v jq >/dev/null 2>&1; then
    echo "$RESPONSE" | jq .
else
    echo "$RESPONSE"
fi

MASKED_BODY="$RESPONSE"
if command -v jq >/dev/null 2>&1; then
    MASKED_BODY="$(printf '%s' "$RESPONSE" | jq -r '.received_body // empty' 2>/dev/null || true)"
fi

if [[ -z "$MASKED_BODY" ]]; then
    MASKED_BODY="$RESPONSE"
fi

echo
echo "==== 脱敏后内容比对 ===="
echo "脱敏后回显内容:"
echo "$MASKED_BODY"

echo
echo "==== 断言 ===="
if [[ "$USE_EXISTING_GATEWAY" == false ]]; then
    if echo "$MASKED_BODY" | grep -q "$REQUEST_ID"; then
        echo "❌ 响应里仍然包含原始身份证号"
        exit 1
    fi

    if echo "$MASKED_BODY" | grep -q "REDACTED_ID_001"; then
        echo "✅ 已检测到身份证号并脱敏为 REDACTED_ID_001"
    else
        echo "⚠️  回显内容中没有看到 REDACTED_ID_001，但已确认没有原始身份证号。"
    fi
else
    if echo "$MASKED_BODY" | grep -q "$REQUEST_ID"; then
        echo "❌ 响应里仍然包含原始身份证号"
    else
        echo "✅ 回显未包含原始身份证号；请结合网关终端日志判断具体脱敏形式。"
    fi
    echo "ℹ️  由于当前复用了已运行的 sec-gateway，脱敏日志请在运行该进程的终端中查看。"
fi

echo
echo "==== 网关日志片段 ===="
if [[ -f "$GW_LOG" ]]; then
    grep -E "Detected|Masked body preview|Proxying request to target" "$GW_LOG" || true
else
    echo "未启动本地网关进程，因此没有新的本地日志文件。"
fi

echo
echo "==== 上游日志文件 ===="
if [[ -f "$UPSTREAM_LOG" ]]; then
    echo "$UPSTREAM_LOG"
else
    echo "未启动本地回显上游。"
fi
