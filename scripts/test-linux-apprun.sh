#!/usr/bin/env bash
set -euo pipefail

# LIN3 fixture tests for the AppRun dependency check and the AppRun composer.
# Deterministic: no network, no AppImage tooling, no FUSE. A fake `ldconfig`
# earlier on PATH simulates the WebKitGTK library being present or absent, and
# RHO_OS_RELEASE drives the distribution family.

RHO_TEST_SCRIPT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RHO_CHECK_FRAGMENT="$RHO_TEST_SCRIPT_ROOT/rho-apprun-check.sh"
RHO_COMPOSE="$RHO_TEST_SCRIPT_ROOT/compose-apprun.sh"
RHO_TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/rho-apprun-test.XXXXXX")"
trap 'rm -rf -- "$RHO_TEST_ROOT"' EXIT

# --- fixtures ----------------------------------------------------------------

RHO_FAKE_BIN="$RHO_TEST_ROOT/bin"
mkdir -p "$RHO_FAKE_BIN"

cat > "$RHO_FAKE_BIN/ldconfig" <<'EOF'
#!/bin/sh
if [ -f "${RHO_LDCONFIG_MARKER:-}" ]; then
  printf 'libwebkit2gtk-4.1.so.0 (libc6,x86-64) => /lib/x86_64-linux-gnu/libwebkit2gtk-4.1.so.0\n'
fi
EOF
chmod 755 "$RHO_FAKE_BIN/ldconfig"

RHO_FAKE_TARGET="$RHO_TEST_ROOT/fake-rho-desktop"
cat > "$RHO_FAKE_TARGET" <<'EOF'
#!/bin/sh
printf 'exec-ok\n' > "${RHO_EXEC_MARKER:?}"
printf '%s\n' "$@" >> "${RHO_EXEC_MARKER:?}"
exit 0
EOF
chmod 755 "$RHO_FAKE_TARGET"

# The fixture original AppRun body: sets nothing, execs the recorded target
# with the original arguments (mimicking Tauri's final `exec` line).
RHO_ORIGINAL_APPRUN="$RHO_TEST_ROOT/original-AppRun"
cat > "$RHO_ORIGINAL_APPRUN" <<'EOF'
#!/bin/sh
exec "$RHO_FAKE_TARGET" "$@"
EOF

write_os_release() {
  local name="$1"
  local id="$2"
  printf 'NAME="%s"\nID=%s\n' "$name" "$id" > "$RHO_TEST_ROOT/os-release-$name"
}

write_os_release ubuntu ubuntu
write_os_release debian debian
write_os_release rocky rocky
write_os_release fedora fedora
write_os_release opensuse opensuse-tumbleweed
write_os_release arch arch
write_os_release unknown weirdos

# --- composer contract -------------------------------------------------------

RHO_COMPOSED="$RHO_TEST_ROOT/AppRun"
"$RHO_COMPOSE" "$RHO_CHECK_FRAGMENT" "$RHO_ORIGINAL_APPRUN" > "$RHO_COMPOSED"
chmod 755 "$RHO_COMPOSED"

sh -n "$RHO_COMPOSED"
if [[ "$(head -n 1 "$RHO_COMPOSED")" != "#!/bin/sh" ]]; then
  echo "composed AppRun does not start with a single sh shebang" >&2
  exit 1
fi
if [[ "$(grep -c '^#!' "$RHO_COMPOSED")" -ne 1 ]]; then
  echo "composed AppRun has more than one shebang" >&2
  exit 1
fi
if ! grep -q 'libwebkit2gtk-4\.1\.so\.0' "$RHO_COMPOSED"; then
  echo "composed AppRun is missing the WebKitGTK 4.1 dependency check" >&2
  exit 1
fi
if ! grep -q 'exec "\$RHO_FAKE_TARGET" "\$@"' "$RHO_COMPOSED"; then
  echo "composed AppRun is missing the original body exec line" >&2
  exit 1
fi

# A bash-based original AppRun keeps its own interpreter (Tauri's AppImage
# AppRun is bash), so its body keeps working under bash on dash systems.
RHO_ORIGINAL_APPRUN_BASH="$RHO_TEST_ROOT/original-AppRun-bash"
cat > "$RHO_ORIGINAL_APPRUN_BASH" <<'EOF'
#!/bin/bash
exec "$RHO_FAKE_TARGET" "$@"
EOF
RHO_COMPOSED_BASH="$RHO_TEST_ROOT/AppRun-bash"
"$RHO_COMPOSE" "$RHO_CHECK_FRAGMENT" "$RHO_ORIGINAL_APPRUN_BASH" > "$RHO_COMPOSED_BASH"
if [[ "$(head -n 1 "$RHO_COMPOSED_BASH")" != "#!/bin/bash" ]]; then
  echo "bash-based original AppRun lost its interpreter" >&2
  exit 1
fi
if [[ "$(grep -c '^#!' "$RHO_COMPOSED_BASH")" -ne 1 ]]; then
  echo "bash-composed AppRun has more than one shebang" >&2
  exit 1
fi
sh -n "$RHO_COMPOSED_BASH"

# --- library present: exec passthrough ----------------------------------------

RHO_PRESENT_MARKER="$RHO_TEST_ROOT/webkit-present"
touch "$RHO_PRESENT_MARKER"
RHO_EXEC_MARKER="$RHO_TEST_ROOT/exec.marker"
rm -f "$RHO_EXEC_MARKER"
RHO_OS_RELEASE="$RHO_TEST_ROOT/os-release-ubuntu" \
  RHO_LDCONFIG_MARKER="$RHO_PRESENT_MARKER" \
  RHO_FAKE_TARGET="$RHO_FAKE_TARGET" \
  RHO_EXEC_MARKER="$RHO_EXEC_MARKER" \
  PATH="$RHO_FAKE_BIN:$PATH" \
  sh "$RHO_COMPOSED" first "second argument"
if [[ ! -f "$RHO_EXEC_MARKER" ]]; then
  echo "present-library fixture did not reach the exec line" >&2
  exit 1
fi
if ! grep -q '^exec-ok$' "$RHO_EXEC_MARKER"; then
  echo "present-library fixture did not exec the target" >&2
  exit 1
fi
if ! grep -q '^first$' "$RHO_EXEC_MARKER" || ! grep -q '^second argument$' "$RHO_EXEC_MARKER"; then
  echo "present-library fixture did not pass arguments through" >&2
  sed -n '1,20p' "$RHO_EXEC_MARKER" >&2
  exit 1
fi

# The bash-composed AppRun must also reach the exec line under bash, proving
# the POSIX check fragment runs correctly under the preserved interpreter.
RHO_EXEC_MARKER_BASH="$RHO_TEST_ROOT/exec-bash.marker"
rm -f "$RHO_EXEC_MARKER_BASH"
RHO_OS_RELEASE="$RHO_TEST_ROOT/os-release-ubuntu" \
  RHO_LDCONFIG_MARKER="$RHO_PRESENT_MARKER" \
  RHO_FAKE_TARGET="$RHO_FAKE_TARGET" \
  RHO_EXEC_MARKER="$RHO_EXEC_MARKER_BASH" \
  PATH="$RHO_FAKE_BIN:$PATH" \
  bash "$RHO_COMPOSED_BASH" bash-arg
if ! grep -q '^bash-arg$' "$RHO_EXEC_MARKER_BASH"; then
  echo "bash-composed AppRun did not exec the target with the argument" >&2
  exit 1
fi

# --- library absent: per-family hints -----------------------------------------

expect_hint() {
  local label="$1"
  local os_release="$2"
  local expected="$3"
  local output="$RHO_TEST_ROOT/$label.out"
  set +e
  RHO_OS_RELEASE="$os_release" PATH="$RHO_FAKE_BIN:$PATH" sh "$RHO_COMPOSED" >"$output" 2>&1
  local exit_code=$?
  set -e
  if [[ "$exit_code" -ne 1 ]]; then
    echo "$label: expected exit 1, got $exit_code" >&2
    sed -n '1,40p' "$output" >&2
    exit 1
  fi
  if ! grep -q "$expected" "$output"; then
    echo "$label: missing expected hint '$expected'" >&2
    sed -n '1,40p' "$output" >&2
    exit 1
  fi
}

expect_hint hint-ubuntu "$RHO_TEST_ROOT/os-release-ubuntu" "sudo apt install libwebkit2gtk-4.1-0"
expect_hint hint-debian "$RHO_TEST_ROOT/os-release-debian" "sudo apt install libwebkit2gtk-4.1-0"
expect_hint hint-rocky "$RHO_TEST_ROOT/os-release-rocky" "sudo dnf install webkit2gtk4.1"
expect_hint hint-fedora "$RHO_TEST_ROOT/os-release-fedora" "sudo dnf install webkit2gtk4.1"
expect_hint hint-opensuse "$RHO_TEST_ROOT/os-release-opensuse" "sudo zypper install libwebkit2gtk-4_1-0"
expect_hint hint-arch "$RHO_TEST_ROOT/os-release-arch" "sudo pacman -S webkit2gtk-4.1"
expect_hint hint-unknown "$RHO_TEST_ROOT/os-release-unknown" "install the WebKitGTK 4.1 runtime library"

# --- no network surface in the wrapper ----------------------------------------

if grep -Eq 'curl|wget|ftp|nc\b|apt-get' "$RHO_CHECK_FRAGMENT"; then
  echo "check fragment contains a network/download surface; it must only print hints" >&2
  exit 1
fi

echo "Linux AppRun dependency-check fixtures passed"
