#!/bin/sh
# Cargo passes the compiled ELF as the first argument to this runner.
set -eu
if [ "$#" -ne 1 ] || [ ! -f "$1" ]; then
    echo 'Usage: cargo run --locked --release (or: sh .devcontainer/flash.sh <firmware.elf>)' >&2
    exit 2
fi
host="${OPENOCD_HOST:-127.0.0.1}"
port="${OPENOCD_PORT:-3333}"
case "$host" in ''|*[!a-zA-Z0-9_.-]*) echo 'OPENOCD_HOST must be a hostname or IPv4 address.' >&2; exit 2;; esac
case "$port" in ''|*[!0-9]*) echo 'OPENOCD_PORT must be a numeric TCP port.' >&2; exit 2;; esac
printf 'Flashing through OpenOCD at %s:%s\n' "$host" "$port"
exec "${ARM_GDB:-arm-none-eabi-gdb}" --batch \
    -ex 'set confirm off' \
    -ex "target extended-remote ${host}:${port}" \
    -ex load \
    -ex 'monitor reset run' \
    -ex detach \
    "$1"
