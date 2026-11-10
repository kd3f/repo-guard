#!/bin/sh
# Test hook mirroring `.githooks/pre-push` with a capture script instead of Git LFS.
set -eu
REF_STDIN="$(mktemp -t repo-guard-pre-push.XXXXXX)"
trap 'rm -f "$REF_STDIN"' EXIT INT TERM
cat > "$REF_STDIN"
repo-guard hook pre-push < "$REF_STDIN" || exit $?
sh "${GIT_LFS_CAPTURE_SH:?GIT_LFS_CAPTURE_SH must be set}" "$@" < "$REF_STDIN"
