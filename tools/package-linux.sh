#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: tools/package-linux.sh (--development|--release) --arch <x86_64|aarch64> --appimagetool <path> [--skip-build] [--force]
EOF
}

mode=
arch=
appimagetool=
skip_build=0
force=0
while (($#)); do
  case "$1" in
    --development) mode=development ;;
    --release) mode=release ;;
    --arch) shift; arch="${1:-}" ;;
    --appimagetool) shift; appimagetool="${1:-}" ;;
    --skip-build) skip_build=1 ;;
    --force) force=1 ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; echo "Unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done

[[ "$(uname -s)" == Linux ]] || { echo 'Linux packaging must run on Linux.' >&2; exit 1; }
[[ "$mode" == development || "$mode" == release ]] || { usage >&2; exit 2; }
[[ "$arch" == x86_64 || "$arch" == aarch64 ]] || { usage >&2; exit 2; }
if [[ "$mode" == release && "${RECENTRY_NATIVE_ACCEPTANCE:-}" != green ]]; then
  echo 'Release mode requires RECENTRY_NATIVE_ACCEPTANCE=green from the protected acceptance job.' >&2
  exit 1
fi
[[ -n "$appimagetool" && -x "$appimagetool" ]] || { echo 'A runnable appimagetool path is required.' >&2; exit 1; }
command -v cargo >/dev/null || { echo 'cargo is required.' >&2; exit 1; }
command -v dpkg-deb >/dev/null || { echo 'dpkg-deb is required.' >&2; exit 1; }
command -v desktop-file-validate >/dev/null || { echo 'desktop-file-validate is required.' >&2; exit 1; }
command -v file >/dev/null || { echo 'file is required.' >&2; exit 1; }
command -v python3 >/dev/null || { echo 'python3 is required for Cargo metadata parsing.' >&2; exit 1; }

workspace="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
desktop-file-validate "$workspace/packaging/linux/recentry.desktop"
version="$(cargo metadata --locked --no-deps --format-version 1 --manifest-path "$workspace/Cargo.toml" | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "recentry-host"))')"
case "$arch" in
  x86_64) target=x86_64-unknown-linux-gnu; deb_arch=amd64; appimage_arch=x86_64 ;;
  aarch64) target=aarch64-unknown-linux-gnu; deb_arch=arm64; appimage_arch=aarch64 ;;
esac

dist="$workspace/dist"
package_root="$workspace/target/package"
stage="$package_root/Recentry-$version-linux-$arch"
deb="$dist/Recentry-$version-linux-$arch.deb"
appimage="$dist/Recentry-$version-linux-$arch.AppImage"
for artifact in "$deb" "$appimage"; do
  if [[ -e "$artifact" && $force -ne 1 ]]; then
    echo "Artifact exists; pass --force to replace it: $artifact" >&2
    exit 1
  fi
done
case "$stage" in
  "$package_root"/*) ;;
  *) echo "Unsafe staging path: $stage" >&2; exit 1 ;;
esac

if [[ $skip_build -ne 1 ]]; then
  cargo build --workspace --release --locked --target "$target" --manifest-path "$workspace/Cargo.toml"
fi
release_root="$workspace/target/$target/release"
host="$release_root/recentry"
ui="$release_root/recentry-ui"
[[ -x "$host" && -x "$ui" ]] || { echo "Required release binaries are missing under $release_root" >&2; exit 1; }

rm -rf -- "$stage"
mkdir -p "$stage/deb/DEBIAN" "$stage/deb/usr/bin" "$stage/deb/usr/share/applications" "$stage/deb/usr/share/icons/hicolor/scalable/apps"
install -m 0755 "$host" "$stage/deb/usr/bin/recentry"
install -m 0755 "$ui" "$stage/deb/usr/bin/recentry-ui"
install -m 0644 "$workspace/packaging/linux/recentry.desktop" "$stage/deb/usr/share/applications/recentry.desktop"
install -m 0644 "$workspace/packaging/linux/recentry.svg" "$stage/deb/usr/share/icons/hicolor/scalable/apps/recentry.svg"
cat >"$stage/deb/DEBIAN/control" <<EOF
Package: recentry
Version: $version
Section: devel
Priority: optional
Architecture: $deb_arch
Maintainer: Recentry contributors <noreply@github.com>
Description: Low-resource launcher for recent development projects
 Recentry opens or focuses stable VS Code recent projects from a compact launcher.
EOF

mkdir -p "$dist"
rm -f -- "$deb" "$appimage"
dpkg-deb --root-owner-group --build "$stage/deb" "$deb"

appdir="$stage/AppDir"
mkdir -p "$appdir/usr/bin" "$appdir/usr/share/applications" "$appdir/usr/share/icons/hicolor/scalable/apps"
install -m 0755 "$host" "$appdir/usr/bin/recentry"
install -m 0755 "$ui" "$appdir/usr/bin/recentry-ui"
install -m 0644 "$workspace/packaging/linux/recentry.desktop" "$appdir/recentry.desktop"
install -m 0644 "$workspace/packaging/linux/recentry.desktop" "$appdir/usr/share/applications/recentry.desktop"
install -m 0644 "$workspace/packaging/linux/recentry.svg" "$appdir/recentry.svg"
install -m 0644 "$workspace/packaging/linux/recentry.svg" "$appdir/usr/share/icons/hicolor/scalable/apps/recentry.svg"
ln -s recentry.svg "$appdir/.DirIcon"
cat >"$appdir/AppRun" <<'EOF'
#!/usr/bin/env sh
set -eu
APPDIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)"
exec "$APPDIR/usr/bin/recentry" "$@"
EOF
chmod 0755 "$appdir/AppRun"
APPIMAGE_EXTRACT_AND_RUN=1 ARCH="$appimage_arch" "$appimagetool" "$appdir" "$appimage"
chmod 0755 "$appimage"

dpkg-deb --info "$deb" >/dev/null
[[ "$(file -b "$host")" == *ELF* ]] || { echo 'Host is not an ELF binary.' >&2; exit 1; }

printf 'Created %s Linux %s %s artifacts:\n%s\n%s\n' "$mode" "$arch" "$version" "$deb" "$appimage"
