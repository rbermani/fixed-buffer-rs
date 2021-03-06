#!/usr/bin/env sh
set -e
#set -x
top_level_dir=$(
  cd "$(dirname $0)"
  pwd
)
du_before=$(du -sh "$top_level_dir" |cut -f 1)
find "$top_level_dir" -name Cargo.toml -maxdepth 2 -print0 |xargs -n 1 -0 -t cargo clean --manifest-path
rm -f */Cargo.lock
du_after=$(du -sh "$top_level_dir" |cut -f 1)
echo "$du_before -> $du_after"
