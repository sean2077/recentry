#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo 'Usage: tools/test-macos-package.sh --zip <app.zip> --dmg <path>'
}

zip=
dmg=
while (($#)); do
  case "$1" in
    --zip) shift; zip="${1:-}" ;;
    --dmg) shift; dmg="${1:-}" ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; echo "Unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done
[[ "$(uname -s)" == Darwin ]] || { echo 'macOS package smoke tests must run on macOS.' >&2; exit 1; }
[[ -f "$zip" && -f "$dmg" ]] || { usage >&2; exit 2; }

scratch="$(mktemp -d "${TMPDIR:-/tmp}/recentry-package-smoke.XXXXXX")"
mount="$scratch/mount"
cleanup() {
  if [[ -n "${host_pid:-}" ]]; then kill "$host_pid" 2>/dev/null || true; fi
  if mount | grep -Fq " on $mount "; then hdiutil detach -quiet "$mount" || true; fi
  rm -rf -- "$scratch"
}
trap cleanup EXIT

mkdir "$scratch/zip" "$mount" "$scratch/runtime" "$scratch/home"
chmod 0700 "$scratch/runtime" "$scratch/home"
ditto -x -k "$zip" "$scratch/zip"
app="$scratch/zip/Recentry.app"
host="$app/Contents/MacOS/recentry"
ui="$app/Contents/MacOS/recentry-ui"
[[ -x "$host" && -x "$ui" ]] || { echo 'App ZIP payload is incomplete.' >&2; exit 1; }
plutil -lint "$app/Contents/Info.plist" >/dev/null
lipo -verify_arch x86_64 arm64 "$host"
lipo -verify_arch x86_64 arm64 "$ui"
codesign --verify --deep --strict --verbose=2 "$app"

XDG_RUNTIME_DIR="$scratch/runtime" HOME="$scratch/home" "$host" --background &
host_pid=$!
endpoint="$scratch/runtime/recentry/recentry-host-v1.sock"
ready=0
for _ in $(seq 1 100); do
  if [[ -S "$endpoint" ]]; then
    ready=1
    break
  fi
  kill -0 "$host_pid" 2>/dev/null || { wait "$host_pid"; echo 'macOS host exited before its IPC endpoint was ready.' >&2; exit 1; }
  sleep 0.02
done
[[ $ready -eq 1 ]] || { echo 'macOS host IPC endpoint did not become ready.' >&2; exit 1; }
XDG_RUNTIME_DIR="$scratch/runtime" HOME="$scratch/home" "$host" quit
for _ in $(seq 1 100); do
  if ! kill -0 "$host_pid" 2>/dev/null; then
    wait "$host_pid"
    host_pid=
    break
  fi
  sleep 0.02
done
[[ -z "${host_pid:-}" ]] || { echo 'macOS host did not stop after an acknowledged quit request.' >&2; exit 1; }

hdiutil attach -quiet -readonly -nobrowse -mountpoint "$mount" "$dmg"
[[ -d "$mount/Recentry.app" && -L "$mount/Applications" ]] || { echo 'DMG payload is incomplete.' >&2; exit 1; }
hdiutil detach -quiet "$mount"
echo 'macOS Universal 2 app ZIP and DMG structure/lifecycle smoke passed.'
