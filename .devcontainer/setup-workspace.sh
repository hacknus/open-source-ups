#!/bin/sh
set -eu

# A normal clone may not have initialized the kernel submodule yet. Preserve
# existing local kernel changes by initializing only when its headers are absent.
if [ ! -f FreeRTOS-Kernel/include/FreeRTOS.h ]; then
    git submodule update --init --recursive FreeRTOS-Kernel
fi

rustc --version
cargo --version
rustup target list --installed
cargo fetch --locked
printf '\nRust and the ARM build dependencies are ready. See .devcontainer/README.md for flashing.\n'
