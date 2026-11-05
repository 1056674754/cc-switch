#!/usr/bin/env bash
# 构建并安装 ccswitch 命令行：默认软链到 ~/.local/bin/ccswitch。
#
#   scripts/install-cli.sh                 # 安装（release 构建）
#   scripts/install-cli.sh /usr/local/bin  # 指定安装目录
#   CC_SWITCH_CLI_PROFILE=debug scripts/install-cli.sh   # 用 debug 构建（快）
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

PROFILE="${CC_SWITCH_CLI_PROFILE:-release}"
DEST="${1:-$HOME/.local/bin}"

echo "[install-cli] 构建 ccswitch（${PROFILE}）..."
if [[ "$PROFILE" == "debug" ]]; then
  cargo build --manifest-path src-tauri/Cargo.toml --bin ccswitch
  BIN="$REPO_ROOT/src-tauri/target/debug/ccswitch"
else
  cargo build --release --manifest-path src-tauri/Cargo.toml --bin ccswitch
  BIN="$REPO_ROOT/src-tauri/target/release/ccswitch"
fi

mkdir -p "$DEST"
ln -sf "$BIN" "$DEST/ccswitch"
echo "[install-cli] 已安装: $DEST/ccswitch -> $BIN"
echo "[install-cli] 试一下: $DEST/ccswitch --help"
