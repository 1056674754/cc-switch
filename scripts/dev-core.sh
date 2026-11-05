#!/usr/bin/env bash
# 启动 headless core：同一个 Rust 进程，只开 HTTP IPC / 事件服务，不创建窗口。
# 数据目录默认隔离到 /tmp/cc-switch-dev-home，不会碰你真实的 ~/.cc-switch。
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

export CC_SWITCH_TEST_HOME="${CC_SWITCH_DEV_HOME:-/tmp/cc-switch-dev-home}"
mkdir -p "$CC_SWITCH_TEST_HOME"

export CC_SWITCH_DEV_IPC_PORT="${CC_SWITCH_DEV_IPC_PORT:-3999}"

# 可选：用真实数据库的副本起 dev core（CC_SWITCH_DEV_SEED_DB=1），方便在浏览器里验证真实内容。
if [[ "${CC_SWITCH_DEV_SEED_DB:-0}" == "1" ]]; then
  REAL_DB="${HOME:-}/.cc-switch/cc-switch.db"
  if [[ -z "${HOME:-}" || ! -f "$REAL_DB" ]]; then
    echo "[dev-core] 警告: 没有找到 ~/.cc-switch/cc-switch.db，跳过真实数据拷贝" >&2
  elif ! command -v sqlite3 >/dev/null 2>&1; then
    echo "[dev-core] 警告: 没有 sqlite3，跳过真实数据拷贝" >&2
  else
    mkdir -p "$CC_SWITCH_TEST_HOME/.cc-switch"
    sqlite3 "$REAL_DB" ".backup '$CC_SWITCH_TEST_HOME/.cc-switch/cc-switch.db'"
    echo "[dev-core] 已把 ~/.cc-switch/cc-switch.db 拷贝到 $CC_SWITCH_TEST_HOME/.cc-switch/"
  fi
fi

echo "[dev-core] CC_SWITCH_TEST_HOME=$CC_SWITCH_TEST_HOME"
echo "[dev-core] CC_SWITCH_DEV_IPC_PORT=$CC_SWITCH_DEV_IPC_PORT"
echo "[dev-core] 启动 headless core（首次编译会比较久）..."

exec cargo run --manifest-path src-tauri/Cargo.toml --features dev-ipc -- --headless
