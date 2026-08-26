#!/usr/bin/env bash
set -euo pipefail
# Optional project-local toolchain; standard installed Rust/LLVM works as well.
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
if [[ -d .tools/cargo ]]; then
  export CARGO_HOME="$ROOT/.tools/cargo" RUSTUP_HOME="$ROOT/.tools/rustup"
  export PATH="$CARGO_HOME/bin:$ROOT/.tools/sysroot/usr/lib/llvm-19/bin:$ROOT/.tools/sysroot/usr/bin:$PATH"
  export LD_LIBRARY_PATH="$ROOT/.tools/sysroot/usr/lib/x86_64-linux-gnu:${LD_LIBRARY_PATH:-}"
  export NSISDIR="$ROOT/.tools/sysroot/usr/share/nsis"
fi
mkdir -p .cache/rust-tmp
export XDG_CACHE_HOME="$ROOT/.cache"
export TMPDIR="$ROOT/.cache/rust-tmp" XWIN_CACHE_DIR="$ROOT/.cache/xwin" CARGO_BUILD_JOBS=1
export XWIN_ARCH=x86_64
export XWIN_CROSS_COMPILER="${XWIN_CROSS_COMPILER:-clang}"
npm run tauri build -- --runner cargo-xwin --target x86_64-pc-windows-msvc "$@"
