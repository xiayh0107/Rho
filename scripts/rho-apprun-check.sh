# Rho Linux AppImage dependency check (LIN3).
#
# This fragment is prepended to the AppImage's original AppRun by
# scripts/compose-apprun.sh so it runs before the bundled Rho binary is exec'd.
# AppImage bundles the application's own libraries but NOT WebKitGTK; a clean
# machine would otherwise fail inside the dynamic linker with an opaque
# "libwebkit2gtk-4.1.so.0: cannot open shared object file" traceback.
#
# Contract (must stay true):
#   - POSIX sh only; runs on any system shell;
#   - when libwebkit2gtk-4.1.so.0 is absent, prints the exact install command
#     for the detected distribution family and exits 1;
#   - unknown families get a generic pointer;
#   - never installs, downloads, or requires network access;
#   - otherwise falls through so the original AppRun body execs Rho unchanged.
#
# Test hooks: RHO_OS_RELEASE overrides /etc/os-release; a fake `ldconfig`
# earlier on PATH can simulate the library being present or absent.

RHO_OS_RELEASE="${RHO_OS_RELEASE:-/etc/os-release}"

rho_distro_family() {
  if [ ! -r "$RHO_OS_RELEASE" ]; then
    echo unknown
    return
  fi
  case "$(sed -n 's/^ID=//p' "$RHO_OS_RELEASE" 2>/dev/null | tr -d '"' | head -n 1)" in
    debian|ubuntu|linuxmint|pop|raspbian) echo debian ;;
    rhel|fedora|rocky|alma|centos|ol) echo rhel ;;
    suse|opensuse|opensuse-leap|opensuse-tumbleweed|sles) echo suse ;;
    arch|manjaro|endeavouros|arcolinux) echo arch ;;
    *) echo unknown ;;
  esac
}

rho_ldconfig_cache() {
  # ldconfig is not always on PATH in minimal containers/servers.
  if command -v ldconfig >/dev/null 2>&1; then
    ldconfig -p 2>/dev/null
  elif [ -x /sbin/ldconfig ]; then
    /sbin/ldconfig -p 2>/dev/null
  elif [ -x /usr/sbin/ldconfig ]; then
    /usr/sbin/ldconfig -p 2>/dev/null
  fi
}

if ! rho_ldconfig_cache | grep -q 'libwebkit2gtk-4\.1\.so\.0'; then
  case "$(rho_distro_family)" in
    debian) RHO_HINT="sudo apt install libwebkit2gtk-4.1-0" ;;
    rhel) RHO_HINT="sudo dnf install webkit2gtk4.1" ;;
    suse) RHO_HINT="sudo zypper install libwebkit2gtk-4_1-0" ;;
    arch) RHO_HINT="sudo pacman -S webkit2gtk-4.1" ;;
    *) RHO_HINT="install the WebKitGTK 4.1 runtime library (libwebkit2gtk-4.1.so.0) for your distribution" ;;
  esac
  echo "Rho requires the WebKitGTK 4.1 runtime library, which is not installed on this system." >&2
  echo "Install it with: $RHO_HINT" >&2
  echo "Then run this AppImage again." >&2
  exit 1
fi
