#!/usr/bin/env bash
# Build and supervise the local server and GUI as one development session.
set -euo pipefail

sonora_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
sonora_profile=debug
sonora_build=1
sonora_build_flags=()
sonora_server_pid=
sonora_gui_pid=
sonora_ready_pid=
sonora_build_pid=
sonora_artifacts=

usage() {
    printf 'Usage: %s [--release] [--no-build] [-- GUI_ARGUMENTS...]\n' "$0"
}

# Signal only the process groups created by this invocation, including their helpers.
cleanup() {
    local status=$?
    trap - EXIT INT TERM
    local pid
    local pids=("$sonora_gui_pid" "$sonora_ready_pid" "$sonora_server_pid" "$sonora_build_pid")
    for pid in "${pids[@]}"; do
        [[ -n "$pid" ]] && kill -TERM -- "-$pid" 2>/dev/null || true
    done
    local deadline=$((SECONDS + 10))
    local alive
    while (( SECONDS < deadline )); do
        alive=0
        for pid in "${pids[@]}"; do
            if [[ -n "$pid" ]] && kill -0 -- "-$pid" 2>/dev/null; then alive=1; fi
        done
        (( alive == 0 )) && break
        sleep 0.05
    done
    for pid in "${pids[@]}"; do
        if [[ -n "$pid" ]]; then
            kill -KILL -- "-$pid" 2>/dev/null || true
            wait "$pid" 2>/dev/null || true
        fi
    done
    [[ -z "$sonora_artifacts" ]] || rm -f -- "$sonora_artifacts"
    exit "$status"
}

while (( $# )); do
    case "$1" in
        --release) sonora_profile=release; sonora_build_flags+=(--release); shift ;;
        --no-build) sonora_build=0; shift ;;
        --help|-h) usage; exit 0 ;;
        --) shift; break ;;
        *) usage >&2; exit 2 ;;
    esac
done

for sonora_tool in cargo python3 setsid; do
    command -v "$sonora_tool" >/dev/null || { printf 'Missing command: %s\n' "$sonora_tool" >&2; exit 1; }
done
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
cd -- "$sonora_root"

if (( sonora_build )); then
    sonora_artifacts="$(mktemp -t sonora-build.XXXXXXXX)"
    setsid cargo build --locked --package service --bin sonora-server --package sonora --bin sonora \
        --message-format=json-render-diagnostics "${sonora_build_flags[@]}" > "$sonora_artifacts" &
    sonora_build_pid=$!
    if wait "$sonora_build_pid"; then
        sonora_build_pid=
    else
        exit $?
    fi
    mapfile -t sonora_binaries < <(python3 - "$sonora_artifacts" <<'PY'
import json
import sys

executables = {}
with open(sys.argv[1]) as artifacts:
    for line in artifacts:
        artifact = json.loads(line)
        if artifact.get("reason") == "compiler-artifact" and artifact.get("executable"):
            executables[artifact["target"]["name"]] = artifact["executable"]
for name in ("sonora-server", "sonora"):
    print(executables.get(name, ""))
PY
)
    sonora_server_bin="${sonora_binaries[0]:-}"
    sonora_gui_bin="${sonora_binaries[1]:-}"
else
    sonora_target="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
    sonora_server_bin="$sonora_target/$sonora_profile/sonora-server"
    sonora_gui_bin="$sonora_target/$sonora_profile/sonora"
fi

[[ -x "$sonora_server_bin" && -x "$sonora_gui_bin" ]] || {
    printf 'Server or GUI executable is missing; run without --no-build first.\n' >&2
    exit 1
}

setsid "$sonora_server_bin" &
sonora_server_pid=$!
setsid "$sonora_server_bin" --wait-for "$sonora_server_pid" >/dev/null &
sonora_ready_pid=$!
sonora_finished=
if wait -n -p sonora_finished "$sonora_server_pid" "$sonora_ready_pid"; then
    sonora_status=0
else
    sonora_status=$?
fi
if [[ "$sonora_finished" != "$sonora_ready_pid" ]] || (( sonora_status != 0 )); then
    printf 'Server failed to become ready.\n' >&2
    exit 1
fi
sonora_ready_pid=
kill -0 "$sonora_server_pid" 2>/dev/null || { printf 'Server exited during startup.\n' >&2; exit 1; }

setsid "$sonora_gui_bin" "$@" &
sonora_gui_pid=$!
printf 'Server PID: %s; GUI PID: %s. Press Ctrl+C to stop both.\n' "$sonora_server_pid" "$sonora_gui_pid"
if wait -n "$sonora_server_pid" "$sonora_gui_pid"; then
    exit 0
else
    exit $?
fi
