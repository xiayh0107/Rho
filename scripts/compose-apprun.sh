#!/usr/bin/env bash
set -euo pipefail

# LIN3: compose the final AppRun for the AppImage.
#
# Usage: compose-apprun.sh <check-fragment> <original-apprun>
# Prints the composed AppRun to stdout: the original AppRun's shebang (or a
# sh shebang when the original has none), then the dependency-check fragment,
# then the original AppRun body (with its own shebang line stripped so the
# composed file has exactly one). Preserving the original interpreter keeps
# Tauri's bash-based AppRun body working unchanged; the check fragment itself
# is POSIX-sh portable, so it runs correctly under either interpreter. The
# original body keeps Tauri's environment setup and final `exec` semantics,
# so the check runs first and argument passthrough is preserved.

if [[ "$#" -ne 2 ]]; then
  echo "Usage: compose-apprun.sh <check-fragment> <original-apprun>" >&2
  exit 1
fi
RHO_CHECK_FRAGMENT="$1"
RHO_ORIGINAL_APPRUN="$2"

if [[ ! -f "$RHO_CHECK_FRAGMENT" ]]; then
  echo "Check fragment not found: $RHO_CHECK_FRAGMENT" >&2
  exit 1
fi
if [[ ! -f "$RHO_ORIGINAL_APPRUN" ]]; then
  echo "Original AppRun not found: $RHO_ORIGINAL_APPRUN" >&2
  exit 1
fi

RHO_COMPOSED_TMP="$(mktemp "${TMPDIR:-/tmp}/rho-apprun.XXXXXX")"
trap 'rm -f -- "$RHO_COMPOSED_TMP"' EXIT

RHO_FIRST_LINE="$(head -n 1 "$RHO_ORIGINAL_APPRUN")"
{
  if printf '%s' "$RHO_FIRST_LINE" | grep -q '^#!'; then
    printf '%s\n' "$RHO_FIRST_LINE"
  else
    printf '#!/bin/sh\n'
  fi
  cat "$RHO_CHECK_FRAGMENT"
  if printf '%s' "$RHO_FIRST_LINE" | grep -q '^#!'; then
    tail -n +2 "$RHO_ORIGINAL_APPRUN"
  else
    cat "$RHO_ORIGINAL_APPRUN"
  fi
} > "$RHO_COMPOSED_TMP"
cat "$RHO_COMPOSED_TMP"
