#!/usr/bin/env bash
set -euo pipefail

# LIN3: replace the AppRun inside a built AppImage with the composed wrapper
# (WebKitGTK 4.1 dependency check + the original Tauri AppRun body), then
# repack the AppImage deterministically using the original runtime ELF prefix
# and squashfs-tools. No network access and no silent installation.
#
# Usage: patch-appimage-apprun.sh <AppImage>
# Optional: RHO_APPRUN_CHECK overrides the check fragment path;
# RHO_SQUASHFS_COMPRESSION overrides the repack compressor (default zstd).
#
# Requires: squashfs-tools (mksquashfs), grep, head, tail, cat. The AppImage
# runtime reads the compressor from the squashfs superblock, so repacking with
# zstd (smaller artifacts than gzip for the same payload; supported by
# squashfs-tools on the ubuntu-22.04 hosted lane and by the modern AppImage
# runtime) keeps the original runtime compatible; verification runs the
# repacked image's own `--appimage-extract` (no FUSE needed), so a runtime
# that cannot read the chosen compressor fails the build instead of shipping
# a broken artifact. It does not depend on unsquashfs offset auto-detection.

if [[ "$#" -ne 1 ]]; then
  echo "Usage: patch-appimage-apprun.sh <AppImage>" >&2
  exit 1
fi

RHO_SCRIPT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RHO_APPRUN_CHECK="${RHO_APPRUN_CHECK:-$RHO_SCRIPT_ROOT/rho-apprun-check.sh}"
RHO_COMPOSE="$RHO_SCRIPT_ROOT/compose-apprun.sh"
RHO_APPIMAGE="$(realpath "$1")"

if [[ ! -f "$RHO_APPIMAGE" ]]; then
  echo "AppImage not found: $RHO_APPIMAGE" >&2
  exit 1
fi
if [[ ! -x "$RHO_APPIMAGE" ]]; then
  echo "AppImage is not executable: $RHO_APPIMAGE (chmod +x first)" >&2
  exit 1
fi
if [[ ! -f "$RHO_APPRUN_CHECK" ]]; then
  echo "AppRun check fragment not found: $RHO_APPRUN_CHECK" >&2
  exit 1
fi
for RHO_TOOL in mksquashfs grep head tail cat tr realpath; do
  if ! command -v "$RHO_TOOL" >/dev/null 2>&1; then
    echo "Required tool not found: $RHO_TOOL (install squashfs-tools and coreutils)" >&2
    exit 1
  fi
done

RHO_WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/rho-appimage-patch.XXXXXX")"
trap 'rm -rf -- "$RHO_WORK_DIR"' EXIT

# 1. Extract the current AppImage contents with the embedded runtime
#    (works without FUSE; the runtime falls back to extraction mode).
(
  cd "$RHO_WORK_DIR"
  "$RHO_APPIMAGE" --appimage-extract >/dev/null
)
RHO_EXTRACT_ROOT="$RHO_WORK_DIR/squashfs-root"
if [[ ! -f "$RHO_EXTRACT_ROOT/AppRun" ]]; then
  echo "Extracted AppImage has no AppRun; aborting." >&2
  exit 1
fi

# 2. Compose the new AppRun: original shebang (or sh) + check fragment +
#    original body, exactly one shebang.
"$RHO_COMPOSE" "$RHO_APPRUN_CHECK" "$RHO_EXTRACT_ROOT/AppRun" > "$RHO_EXTRACT_ROOT/AppRun.new"
sh -n "$RHO_EXTRACT_ROOT/AppRun.new"
if ! grep -q 'libwebkit2gtk-4\.1\.so\.0' "$RHO_EXTRACT_ROOT/AppRun.new"; then
  echo "Composed AppRun does not contain the WebKitGTK 4.1 dependency check." >&2
  exit 1
fi
chmod 755 "$RHO_EXTRACT_ROOT/AppRun.new"
mv "$RHO_EXTRACT_ROOT/AppRun.new" "$RHO_EXTRACT_ROOT/AppRun"

# 3. The AppImage runtime reads the compressor from the squashfs superblock,
#    so any supported compressor works with the original runtime. Default to
#    zstd (better ratio than gzip for the same payload, fast enough for CI,
#    universally available in squashfs-tools >= 4.4) with an explicit override.
RHO_COMPRESSION="${RHO_SQUASHFS_COMPRESSION:-zstd}"
case "$RHO_COMPRESSION" in
  gzip|lzma|lzo|xz|zstd) ;;
  *)
    echo "Unsupported squashfs compression: '$RHO_COMPRESSION'" >&2
    exit 1
    ;;
esac

# 4. Ask the AppImage's own runtime for the squashfs offset, then reuse the
#    original runtime ELF prefix (everything before the embedded squashfs).
#    The runtime locates the squashfs by ELF size, so reusing the exact
#    prefix keeps the rebuilt image self-locating; no trailer is needed.
RHO_SQUASHFS_OFFSET="$("$RHO_APPIMAGE" --appimage-offset | tr -d '[:space:]')"
if [[ -z "$RHO_SQUASHFS_OFFSET" || ! "$RHO_SQUASHFS_OFFSET" =~ ^[0-9]+$ || "$RHO_SQUASHFS_OFFSET" == "0" ]]; then
  echo "Could not determine the embedded squashfs offset of the AppImage." >&2
  exit 1
fi
head -c "$RHO_SQUASHFS_OFFSET" "$RHO_APPIMAGE" > "$RHO_WORK_DIR/runtime.prefix"

# 5. Repack and append.
mksquashfs "$RHO_EXTRACT_ROOT" "$RHO_WORK_DIR/rho.squashfs" \
  -noappend -no-xattrs -no-progress -comp "$RHO_COMPRESSION"
cat "$RHO_WORK_DIR/runtime.prefix" "$RHO_WORK_DIR/rho.squashfs" > "$RHO_WORK_DIR/Rho.AppImage.new"
chmod 755 "$RHO_WORK_DIR/Rho.AppImage.new"

# 6. Verify the repacked AppImage by running its own extraction (no FUSE
#    needed) and checking the extracted AppRun still carries the check.
RHO_VERIFY_ROOT="$RHO_WORK_DIR/verify"
mkdir -p "$RHO_VERIFY_ROOT"
(
  cd "$RHO_VERIFY_ROOT"
  "$RHO_WORK_DIR/Rho.AppImage.new" --appimage-extract >/dev/null
)
if ! grep -q 'libwebkit2gtk-4\.1\.so\.0' "$RHO_VERIFY_ROOT/squashfs-root/AppRun"; then
  echo "Repacked AppImage verification failed: AppRun check missing." >&2
  exit 1
fi

mv "$RHO_WORK_DIR/Rho.AppImage.new" "$RHO_APPIMAGE"
echo "Patched AppRun with the WebKitGTK 4.1 dependency check: $RHO_APPIMAGE"
