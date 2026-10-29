#!/usr/bin/env bash

set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "Usage: test-install-unix.sh <target>" >&2
  exit 2
fi

TARGET="$1"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
TEST_ROOT="$(mktemp -d)"
trap 'rm -rf "$TEST_ROOT"' EXIT

ORIGINAL_HOME="$HOME"
export RUSTUP_HOME="${RUSTUP_HOME:-${ORIGINAL_HOME}/.rustup}"
export CARGO_HOME="${CARGO_HOME:-${ORIGINAL_HOME}/.cargo}"
export HOME="${TEST_ROOT}/home"
export ALEX_CLI_BIN_DIR="${TEST_ROOT}/bin"
export CARGO_BUILD_TARGET="$TARGET"
mkdir -p "$HOME"

hash_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

bash "${REPO_ROOT}/src/apps/cli/install.sh"
bash "${REPO_ROOT}/src/apps/cli/install.sh"

"${ALEX_CLI_BIN_DIR}/alex" --version >/dev/null
[ -f "${ALEX_CLI_BIN_DIR}/resources/ext-host/extension-host.js" ]

for rc_file in "$HOME/.bashrc" "$HOME/.zshrc"; do
  [ "$(grep -Fc '# >>> Alex CLI PATH (managed by src/apps/cli/install.sh) >>>' "$rc_file")" -eq 1 ]
done

PRIMARY_HASH="$(hash_file "${ALEX_CLI_BIN_DIR}/alex")"
REAL_MV="$(command -v mv)"
SHIM_DIR="${TEST_ROOT}/shim"
mkdir -p "$SHIM_DIR"
cat >"${SHIM_DIR}/mv" <<'EOF'
#!/bin/sh
if [ "${1:-}" = "${ALEX_CLI_TEST_FAIL_SOURCE:-}" ]; then
  exit 91
fi
exec "${ALEX_CLI_TEST_REAL_MV}" "$@"
EOF
chmod +x "${SHIM_DIR}/mv"

if PATH="${SHIM_DIR}:$PATH" \
  ALEX_CLI_SKIP_SHELLRC=1 \
  ALEX_CLI_TEST_REAL_MV="$REAL_MV" \
  ALEX_CLI_TEST_FAIL_SOURCE="${ALEX_CLI_BIN_DIR}/alex" \
  bash "${REPO_ROOT}/src/apps/cli/install.sh"; then
  echo "Error: installer unexpectedly succeeded during injected replacement failure" >&2
  exit 1
fi

[ "$(hash_file "${ALEX_CLI_BIN_DIR}/alex")" = "$PRIMARY_HASH" ]
