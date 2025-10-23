#!/usr/bin/env bash
set -euo pipefail

ensure_dir() {
  local dir="$1"
  sudo mkdir -p "${dir}"
  sudo chown -R "$(id -u)":"$(id -g)" "${dir}"
}

main() {
  ensure_dir /workspace/target
  ensure_dir /usr/local/cargo/registry
  ensure_dir /usr/local/cargo/git

  rustup component add rustfmt clippy >/tmp/rustup-components.log 2>&1 || true

  exec cargo watch -i target -i .git \
    -s 'cargo fmt --all' \
    -s 'cargo clippy --workspace --all-targets -- -D warnings -D clippy::pedantic' \
    -s 'cargo test --workspace --all-targets'
}

main "$@"
