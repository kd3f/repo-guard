#!/bin/sh
# Test stand-in for `git lfs pre-push` — records stdin for acceptance tests.
set -eu
cat > "${GIT_LFS_STDIN_CAPTURE:?GIT_LFS_STDIN_CAPTURE must be set}"
