#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: tools/package-macos.sh (--development|--release) [--skip-build] [--force] [--keychain-profile <name>]
EOF
}

mode=
skip_build=0
force=0
keychain_profile=
while (($#)); do
  case "$1" in
    --development) mode=development ;;
    --release) mode=release ;;
    --skip-build) skip_build=1 ;;
    --force) force=1 ;;
    --keychain-profile) shift; keychain_profile="${1:-}" ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; echo "Unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done

[[ "$(uname -s)" == Darwin ]] || { echo 'macOS packaging must run on macOS.' >&2; exit 1; }
[[ "$mode" == development || "$mode" == release ]] || { usage >&2; exit 2; }
if [[ "$mode" == release ]]; then
  [[ "${RECENTRY_NATIVE_ACCEPTANCE:-}" == green ]] || { echo 'Release mode requires RECENTRY_NATIVE_ACCEPTANCE=green.' >&2; exit 1; }
  [[ -n "${RECENTRY_APPLE_SIGN_IDENTITY:-}" ]] || { echo 'RECENTRY_APPLE_SIGN_IDENTITY is required.' >&2; exit 1; }
  [[ -n "$keychain_profile" ]] || { echo '--keychain-profile is required for notarization.' >&2; exit 1; }
fi
for command in cargo codesign ditto hdiutil lipo plutil python3; do
  command -v "$command" >/dev/null || { echo "$command is required." >&2; exit 1; }
done

workspace="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
version="$(cargo metadata --locked --no-deps --format-version 1 --manifest-path "$workspace/Cargo.toml" | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "recentry-host"))')"
bundle_version="$(printf '%s' "$version" | sed -E 's/[^0-9]+/./g; s/^\.|\.$//g')"
package_root="$workspace/target/package"
stage="$package_root/Recentry-$version-macos-universal"
app="$stage/Recentry.app"
dist="$workspace/dist"
zip="$dist/Recentry-$version-macos-universal.app.zip"
dmg="$dist/Recentry-$version-macos-universal.dmg"
for artifact in "$zip" "$dmg"; do
  if [[ -e "$artifact" && $force -ne 1 ]]; then
    echo "Artifact exists; pass --force to replace it: $artifact" >&2
    exit 1
  fi
done
case "$stage" in
  "$package_root"/*) ;;
  *) echo "Unsafe staging path: $stage" >&2; exit 1 ;;
esac

targets=(x86_64-apple-darwin aarch64-apple-darwin)
if [[ $skip_build -ne 1 ]]; then
  for target in "${targets[@]}"; do
    cargo build --workspace --release --locked --target "$target" --manifest-path "$workspace/Cargo.toml"
  done
fi
for target in "${targets[@]}"; do
  [[ -x "$workspace/target/$target/release/recentry" ]] || { echo "Missing $target recentry binary." >&2; exit 1; }
  [[ -x "$workspace/target/$target/release/recentry-ui" ]] || { echo "Missing $target recentry-ui binary." >&2; exit 1; }
done

rm -rf -- "$stage"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$stage/dmg-root"
lipo -create \
  "$workspace/target/x86_64-apple-darwin/release/recentry" \
  "$workspace/target/aarch64-apple-darwin/release/recentry" \
  -output "$app/Contents/MacOS/recentry"
lipo -create \
  "$workspace/target/x86_64-apple-darwin/release/recentry-ui" \
  "$workspace/target/aarch64-apple-darwin/release/recentry-ui" \
  -output "$app/Contents/MacOS/recentry-ui"
chmod 0755 "$app/Contents/MacOS/recentry" "$app/Contents/MacOS/recentry-ui"
sed -e "s/@VERSION@/$version/g" -e "s/@BUNDLE_VERSION@/$bundle_version/g" \
  "$workspace/packaging/macos/Info.plist" >"$app/Contents/Info.plist"
cp "$workspace/README.md" "$workspace/CHANGELOG.md" "$workspace/LICENSE" "$app/Contents/Resources/"
plutil -lint "$app/Contents/Info.plist" >/dev/null
lipo -verify_arch x86_64 arm64 "$app/Contents/MacOS/recentry"
lipo -verify_arch x86_64 arm64 "$app/Contents/MacOS/recentry-ui"

if [[ "$mode" == release ]]; then
  identity="$RECENTRY_APPLE_SIGN_IDENTITY"
  codesign --force --timestamp --options runtime --entitlements "$workspace/packaging/macos/Recentry.entitlements" --sign "$identity" "$app/Contents/MacOS/recentry-ui"
  codesign --force --timestamp --options runtime --entitlements "$workspace/packaging/macos/Recentry.entitlements" --sign "$identity" "$app"
else
  codesign --force --deep --sign - "$app"
fi
codesign --verify --deep --strict --verbose=2 "$app"

mkdir -p "$dist"
rm -f -- "$zip" "$dmg"
ditto -c -k --keepParent "$app" "$zip"
if [[ "$mode" == release ]]; then
  xcrun notarytool submit "$zip" --keychain-profile "$keychain_profile" --wait
  xcrun stapler staple "$app"
  rm -f -- "$zip"
  ditto -c -k --keepParent "$app" "$zip"
fi

cp -R "$app" "$stage/dmg-root/Recentry.app"
ln -s /Applications "$stage/dmg-root/Applications"
hdiutil create -quiet -format UDZO -fs HFS+ -volname Recentry -srcfolder "$stage/dmg-root" "$dmg"
if [[ "$mode" == release ]]; then
  codesign --force --timestamp --sign "$RECENTRY_APPLE_SIGN_IDENTITY" "$dmg"
  xcrun notarytool submit "$dmg" --keychain-profile "$keychain_profile" --wait
  xcrun stapler staple "$dmg"
  xcrun stapler validate "$app"
  xcrun stapler validate "$dmg"
  spctl --assess --type execute --verbose=2 "$app"
  spctl --assess --type open --context context:primary-signature --verbose=2 "$dmg"
fi

printf 'Created %s macOS Universal 2 %s artifacts:\n%s\n%s\n' "$mode" "$version" "$zip" "$dmg"
