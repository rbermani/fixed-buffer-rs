#!/usr/bin/env sh
check() {
  time cargo check --verbose
  time cargo build --verbose
  time cargo test --verbose
  time rustup component add rustfmt
  time cargo fmt --all -- --check
  time rustup component add clippy
  time cargo clippy -- -D warnings
  time cargo publish --dry-run "$@"
  echo "$0 finished"
}
set -e
set -x
time check "$@"
