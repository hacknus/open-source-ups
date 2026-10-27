#!/bin/sh
# Run on the Mac, not inside the container. Keep this terminal open.
set -eu
if [ "$(uname -s)" != Darwin ]; then
    echo 'Run this script in a macOS host terminal.' >&2
    exit 1
fi
if ! command -v openocd >/dev/null 2>&1; then
    echo 'Install OpenOCD on the Mac first: brew install open-ocd' >&2
    exit 1
fi
project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
# Docker Desktop connects through host.docker.internal. Expose only the GDB
# service; the host firewall should permit the Docker Desktop connection.
exec openocd -c 'bindto 0.0.0.0' \
    -c 'tcl_port disabled' -c 'telnet_port disabled' \
    -f "$project_dir/openocd.cfg"
