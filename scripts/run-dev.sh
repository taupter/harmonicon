#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# Rebuild and (re)launch the game with the Bevy Remote Protocol server on,
# then wait until that server actually answers — so the next command can
# drive it (see scripts/brpctl.py) without guessing at a sleep.
#
#   ./scripts/run-dev.sh            # rebuild, relaunch, wait for BRP
#   ./scripts/run-dev.sh --no-build # relaunch only
#   ./scripts/run-dev.sh --stop     # just stop a running instance
#
# One command rather than a chain of three, because a chain can only be
# permitted as a whole: `cargo build && nohup env … && python3 …` matches no
# narrow allowlist pattern, so it prompts every time.

set -euo pipefail
cd "$(dirname "$0")/.."

BIN=./target/release/harmonicon
PORT=15702
LOG=/tmp/harmonicon-dev.log

stop() {
    # Match on the built path, not the bare name: `pkill -f harmonicon` also
    # matches the shell running this script, and kills it mid-run.
    pkill -f "$BIN" 2>/dev/null || true
    for _ in $(seq 20); do
        pgrep -f "$BIN" >/dev/null || return 0
        sleep 0.25
    done
    pkill -9 -f "$BIN" 2>/dev/null || true
}

if [[ "${1:-}" == "--stop" ]]; then
    stop
    echo "stopped"
    exit 0
fi

if [[ "${1:-}" != "--no-build" ]]; then
    cargo build --release --features dev
fi

stop

# `BEVY_ASSET_ROOT` because Bevy resolves `assets/` relative to the
# executable, not the working directory, unless it (or `cargo run`) says
# otherwise — without it every asset fails to load and the window is blank.
BEVY_ASSET_ROOT="$PWD" nohup "$BIN" >"$LOG" 2>&1 &

for _ in $(seq 120); do
    if curl -s -m 2 -X POST "http://127.0.0.1:$PORT" \
        -H 'Content-Type: application/json' \
        -d '{"jsonrpc":"2.0","id":1,"method":"world.list_resources"}' >/dev/null 2>&1; then
        echo "running, BRP up on 127.0.0.1:$PORT (log: $LOG)"
        exit 0
    fi
    sleep 0.5
done

echo "game did not answer on 127.0.0.1:$PORT within 60s; last log lines:" >&2
tail -20 "$LOG" >&2
exit 1
