#!/usr/bin/env bash
# 一条命令起「浏览器 UI + headless core」：
#   - 3999 端口已有 core 就复用，不重复起；
#   - 否则后台起 scripts/dev-core.sh，日志写 /tmp/cc-switch-dev-core.log；
#   - 前台跑 vite，退出时 trap 收掉自己起的 core。
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

PORT="${CC_DEV_IPC_PORT:-3999}"
HEALTH_URL="http://127.0.0.1:${PORT}/ipc/health"
CORE_LOG="${CC_SWITCH_DEV_CORE_LOG:-/tmp/cc-switch-dev-core.log}"
CORE_TIMEOUT="${CC_SWITCH_DEV_CORE_TIMEOUT:-600}"

# dev-core.sh 认的是 CC_SWITCH_DEV_IPC_PORT，这里把端口统一传下去。
export CC_SWITCH_DEV_IPC_PORT="$PORT"

CORE_PID=""

# 递归杀掉 core 的整棵进程树：真正监听的二进制是 cargo run 的子进程。
kill_tree() {
  local pid="$1"
  local child
  for child in $(pgrep -P "$pid" 2>/dev/null || true); do
    kill_tree "$child"
  done
  kill "$pid" 2>/dev/null || true
}

cleanup() {
  if [[ -n "$CORE_PID" ]] && kill -0 "$CORE_PID" 2>/dev/null; then
    echo "[dev-web] 停止 headless core (pid=$CORE_PID)"
    kill_tree "$CORE_PID"
    wait "$CORE_PID" 2>/dev/null || true
  fi
  CORE_PID=""
}

trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

if curl -sf --max-time 2 "$HEALTH_URL" >/dev/null 2>&1; then
  echo "[dev-web] 复用已在运行的 headless core：$HEALTH_URL"
else
  echo "[dev-web] 后台启动 headless core，日志：$CORE_LOG"
  bash scripts/dev-core.sh >"$CORE_LOG" 2>&1 &
  CORE_PID=$!

  echo -n "[dev-web] 等待 headless core 就绪（首次 cargo 编译可能要几分钟）"
  WAITED=0
  READY=0
  while true; do
    if curl -sf --max-time 1 "$HEALTH_URL" >/dev/null 2>&1; then
      READY=1
      break
    fi
    if ! kill -0 "$CORE_PID" 2>/dev/null; then
      echo
      echo "[dev-web] headless core 启动失败，末尾日志：" >&2
      tail -n 60 "$CORE_LOG" >&2 || true
      cleanup
      exit 1
    fi
    if ((WAITED >= CORE_TIMEOUT)); then
      echo
      echo "[dev-web] 等待 ${CORE_TIMEOUT}s 仍未就绪，继续启动 vite；页面里的 IPC 会自动重试。" >&2
      tail -n 20 "$CORE_LOG" >&2 || true
      break
    fi
    sleep 2
    WAITED=$((WAITED + 2))
    if ((WAITED % 10 == 0)); then echo -n "."; fi
  done
  if ((READY == 1)); then
    echo
    echo "[dev-web] headless core 已就绪 (pid=$CORE_PID)"
  fi
fi

echo "[dev-web] 启动 vite（http://localhost:3000）..."
pnpm dev:ui
