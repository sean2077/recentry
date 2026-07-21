#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo 'Usage: tools/test-linux-package.sh --deb <path> --appimage <path>'
}

deb=
appimage=
while (($#)); do
  case "$1" in
    --deb) shift; deb="${1:-}" ;;
    --appimage) shift; appimage="${1:-}" ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; echo "Unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done
[[ "$(uname -s)" == Linux ]] || { echo 'Linux package smoke tests must run on Linux.' >&2; exit 1; }
[[ -f "$deb" && -f "$appimage" ]] || { usage >&2; exit 2; }
command -v dpkg-deb >/dev/null || { echo 'dpkg-deb is required.' >&2; exit 1; }

scratch="$(mktemp -d "${TMPDIR:-/tmp}/recentry-package-smoke.XXXXXX")"
cleanup() {
  if [[ -n "${host_pid:-}" ]]; then kill "$host_pid" 2>/dev/null || true; fi
  rm -rf -- "$scratch"
}
trap cleanup EXIT

dpkg-deb --info "$deb" >/dev/null
dpkg-deb --extract "$deb" "$scratch/deb"
host="$scratch/deb/usr/bin/recentry"
ui="$scratch/deb/usr/bin/recentry-ui"
[[ -x "$host" && -x "$ui" ]] || { echo 'DEB payload is incomplete.' >&2; exit 1; }

mkdir -m 0700 "$scratch/runtime" "$scratch/config"
XDG_RUNTIME_DIR="$scratch/runtime" XDG_CONFIG_HOME="$scratch/config" "$host" --background &
host_pid=$!
endpoint="$scratch/runtime/recentry/recentry-host-v1.sock"
ready=0
for _ in $(seq 1 100); do
  if [[ -S "$endpoint" ]]; then
    ready=1
    break
  fi
  kill -0 "$host_pid" 2>/dev/null || { wait "$host_pid"; echo 'DEB host exited before its IPC endpoint was ready.' >&2; exit 1; }
  sleep 0.02
done
[[ $ready -eq 1 ]] || { echo 'DEB host IPC endpoint did not become ready.' >&2; exit 1; }
XDG_RUNTIME_DIR="$scratch/runtime" XDG_CONFIG_HOME="$scratch/config" "$host" quit
for _ in $(seq 1 100); do
  if ! kill -0 "$host_pid" 2>/dev/null; then
    wait "$host_pid"
    host_pid=
    break
  fi
  sleep 0.02
done
[[ -z "${host_pid:-}" ]] || { echo 'DEB host did not stop after an acknowledged quit request.' >&2; exit 1; }

mkdir "$scratch/appimage"
(cd "$scratch/appimage" && "$appimage" --appimage-extract >/dev/null)
[[ -x "$scratch/appimage/squashfs-root/AppRun" ]] || { echo 'AppImage extraction failed.' >&2; exit 1; }
[[ -x "$scratch/appimage/squashfs-root/usr/bin/recentry-ui" ]] || { echo 'AppImage UI payload is missing.' >&2; exit 1; }
echo 'Linux DEB lifecycle and AppImage structure smoke passed.'
