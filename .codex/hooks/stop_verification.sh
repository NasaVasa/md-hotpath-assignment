#!/usr/bin/env bash
set -euo pipefail

rust_paths=(
  Cargo.toml
  Cargo.lock
  rust-toolchain.toml
  .cargo/config.toml
  src
  tests
)

root="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "$root" ]]; then
  echo "[codex-hook] skip: not inside a git repository"
  exit 0
fi

cd "$root"

if [[ "${CODEX_HOOK_FORCE:-0}" != "1" ]] &&
  ! git status --porcelain -- "${rust_paths[@]}" | grep -q .; then
  echo "[codex-hook] skip: no Rust/Cargo changes"
  exit 0
fi

echo "[codex-hook] running full Rust verification"
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
