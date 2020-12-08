#!/usr/bin/env sh
top_level_dir=$(
  cd "$(dirname $0)"
  pwd
)
set -e
set -x
(
  cd $top_level_dir/fixed-buffer/
  "$top_level_dir/check.sh" "$@"
)
(
  cd $top_level_dir/fixed-buffer-tokio/
  "$top_level_dir/check.sh" "$@"
)
