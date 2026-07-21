#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo 'Usage: tools/package-manifest.sh (--development|--release) [--dist <directory>] [--gpg-key <id>] [--force]'
}

mode=
dist=
gpg_key=
force=0
while (($#)); do
  case "$1" in
    --development) mode=development ;;
    --release) mode=release ;;
    --dist) shift; dist="${1:-}" ;;
    --gpg-key) shift; gpg_key="${1:-}" ;;
    --force) force=1 ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; echo "Unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done
[[ "$mode" == development || "$mode" == release ]] || { usage >&2; exit 2; }

workspace="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
[[ -n "$dist" ]] || dist="$workspace/dist"
[[ -d "$dist" ]] || { echo "Distribution directory does not exist: $dist" >&2; exit 1; }
command -v cargo >/dev/null || { echo 'cargo is required.' >&2; exit 1; }
command -v python3 >/dev/null || { echo 'python3 is required for Cargo metadata parsing.' >&2; exit 1; }
version="$(cargo metadata --locked --no-deps --format-version 1 --manifest-path "$workspace/Cargo.toml" | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "recentry-host"))')"
manifest="$dist/Recentry-$version-SHA256SUMS.txt"
signature="$manifest.asc"
if [[ -e "$signature" && $force -ne 1 ]]; then
  echo "Signature exists; pass --force to replace it: $signature" >&2
  exit 1
fi
if [[ -e "$manifest" && $force -ne 1 ]]; then
  echo "Manifest exists; pass --force to replace it: $manifest" >&2
  exit 1
fi
if [[ "$mode" == release ]]; then
  [[ "${RECENTRY_NATIVE_ACCEPTANCE:-}" == green ]] || { echo 'Release mode requires RECENTRY_NATIVE_ACCEPTANCE=green.' >&2; exit 1; }
  [[ -n "$gpg_key" ]] || { echo '--gpg-key is required in release mode.' >&2; exit 1; }
  command -v gpg >/dev/null || { echo 'gpg is required in release mode.' >&2; exit 1; }
fi
if [[ "$mode" == development ]]; then
  rm -f -- "$signature"
fi

assets=(
  "Recentry-$version-windows-x64-setup.exe"
  "Recentry-$version-windows-x64.zip"
  "Recentry-$version-linux-x86_64.deb"
  "Recentry-$version-linux-x86_64.AppImage"
  "Recentry-$version-linux-aarch64.deb"
  "Recentry-$version-linux-aarch64.AppImage"
  "Recentry-$version-macos-universal.app.zip"
  "Recentry-$version-macos-universal.dmg"
)
for asset in "${assets[@]}"; do
  [[ -f "$dist/$asset" ]] || { echo "Mandatory distribution asset is missing: $asset" >&2; exit 1; }
done

temporary="$(mktemp "$dist/.Recentry-SHA256SUMS.XXXXXX")"
cleanup() { rm -f -- "$temporary"; }
trap cleanup EXIT
(
  cd "$dist"
  if command -v sha256sum >/dev/null; then
    sha256sum "${assets[@]}"
  else
    shasum -a 256 "${assets[@]}"
  fi
) >"$temporary"
[[ "$(wc -l <"$temporary" | tr -d ' ')" == "${#assets[@]}" ]] || { echo 'Manifest coverage count is invalid.' >&2; exit 1; }
mv -f -- "$temporary" "$manifest"
trap - EXIT

if [[ "$mode" == release ]]; then
  rm -f -- "$signature"
  gpg --batch --armor --detach-sign --local-user "$gpg_key" --output "$signature" "$manifest"
  gpg --batch --verify "$signature" "$manifest"
fi
printf 'Created %s complete distribution manifest:\n%s\n' "$mode" "$manifest"
if [[ -f "$signature" ]]; then printf '%s\n' "$signature"; fi
