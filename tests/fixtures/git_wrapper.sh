#!/bin/sh
# Git wrapper that captures stdin for `git lfs pre-push` invocations.
set -eu
REAL_GIT="${REAL_GIT:?REAL_GIT must point to the real git binary}"
if [ "$1" = "lfs" ] && [ "$2" = "pre-push" ]; then
  cat > "${GIT_LFS_STDIN_CAPTURE:?GIT_LFS_STDIN_CAPTURE must be set}"
  exit 0
fi
exec "$REAL_GIT" "$@"
