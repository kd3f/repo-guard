#!/bin/sh
# Stages a 55 MiB regular Git blob for pre-commit ordering tests.
set -eu
path="precommit-large.bin"
dd if=/dev/zero of="$path" bs=1048576 count=55 2>/dev/null || {
  # Fallback when dd lacks /dev/zero semantics (Git Bash on Windows).
  python -c "open('precommit-large.bin','wb').write(b'x' * (55 * 1024 * 1024))"
}
git add -- "precommit-large.bin"
