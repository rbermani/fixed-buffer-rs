#!/usr/bin/env sh
set -e
set -x
./check.sh
git push --follow-tags
