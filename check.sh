#!/usr/bin/env bash
# This script uses bash because it has a built-in 'time' command.
# The rust docker images have no 'time' binary and use dash for 'sh' which has
# no built-in 'time' command.
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
