#!/usr/bin/env bash
set -euo pipefail

# LIN2: copy the verified Ark runtime resources from .rho/runtime into the
# Tauri resource tree before a Linux build, mirroring
# prepare-runtime-resources.ps1. The ark sidecar itself is consumed through
# Tauri externalBin (binaries/ark-x86_64-unknown-linux-gnu), so only the
# license resources are copied here; the sidecar is verified to exist.

RHO_SCRIPT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RHO_REPOSITORY_ROOT="$(cd "$RHO_SCRIPT_ROOT/.." && pwd)"
RHO_RUNTIME_ROOT="${RHO_RUNTIME_ROOT:-$RHO_REPOSITORY_ROOT/.rho/runtime}"
RHO_DESTINATION="${RHO_RUNTIME_RESOURCES_DESTINATION:-$RHO_REPOSITORY_ROOT/desktop/resources/runtime}"
RHO_MANIFEST="$RHO_REPOSITORY_ROOT/runtime/ark.json"

read_manifest() {
  node -e '
    const fs = require("node:fs");
    const manifest = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
    const value = process.argv[2].split(".").reduce((current, key) => current?.[key], manifest);
    if (typeof value !== "string" || value.length === 0) process.exit(2);
    process.stdout.write(value);
  ' "$RHO_MANIFEST" "$1"
}

RHO_ARK_VERSION="$(read_manifest version)"
RHO_RUNTIME_SOURCE="$RHO_RUNTIME_ROOT/ark-$RHO_ARK_VERSION-linux-x64"
RHO_REQUIRED_FILES=(LICENSE NOTICE)
RHO_SIDECAR="$RHO_REPOSITORY_ROOT/desktop/src-tauri/binaries/ark-x86_64-unknown-linux-gnu"

if [[ ! -f "$RHO_SIDECAR" ]]; then
  echo "Required Ark sidecar is missing: $RHO_SIDECAR. Run scripts/bootstrap-ark-linux.sh first." >&2
  exit 1
fi
for RHO_NAME in "${RHO_REQUIRED_FILES[@]}"; do
  if [[ ! -f "$RHO_RUNTIME_SOURCE/$RHO_NAME" ]]; then
    echo "Required Ark runtime file is missing: $RHO_RUNTIME_SOURCE/$RHO_NAME. Run scripts/bootstrap-ark-linux.sh first." >&2
    exit 1
  fi
done

mkdir -p "$RHO_DESTINATION"
for RHO_NAME in "${RHO_REQUIRED_FILES[@]}"; do
  RHO_SOURCE_FILE="$RHO_RUNTIME_SOURCE/$RHO_NAME"
  RHO_DESTINATION_FILE="$RHO_DESTINATION/$RHO_NAME"
  RHO_SOURCE_HASH="$(sha256sum "$RHO_SOURCE_FILE" | awk '{print $1}')"
  if [[ -f "$RHO_DESTINATION_FILE" ]]; then
    RHO_DESTINATION_HASH="$(sha256sum "$RHO_DESTINATION_FILE" | awk '{print $1}')"
    if [[ "$RHO_SOURCE_HASH" == "$RHO_DESTINATION_HASH" ]]; then
      echo "Runtime resource is current: $RHO_DESTINATION_FILE"
      continue
    fi
  fi
  cp "$RHO_SOURCE_FILE" "$RHO_DESTINATION_FILE.partial"
  mv "$RHO_DESTINATION_FILE.partial" "$RHO_DESTINATION_FILE"
  RHO_COPIED_HASH="$(sha256sum "$RHO_DESTINATION_FILE" | awk '{print $1}')"
  if [[ "$RHO_COPIED_HASH" != "$RHO_SOURCE_HASH" ]]; then
    echo "Runtime resource checksum mismatch after copying $RHO_DESTINATION_FILE." >&2
    exit 1
  fi
  echo "Prepared runtime resource: $RHO_DESTINATION_FILE"
done
