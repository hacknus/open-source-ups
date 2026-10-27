# Open Source UPS development container

The image includes Rustup and the toolchain selected by `rust-toolchain.toml`,
the `thumbv7em-none-eabihf` target, rustfmt, rustc-dev, and rust-src. It also
includes ARM GCC/Newlib, GDB, OpenOCD, Python, Node.js, and Codex.
Rust is installed as `ubuntu` in `/home/ubuntu/.cargo/bin`, on the image PATH
for both terminals and IDE processes. No toolchain installation is needed after
opening the container. The first workspace setup initializes a missing
FreeRTOS submodule and downloads the locked Cargo dependencies.

## RustRover

Rebuild the devcontainer after these configuration changes; restarting the old
container does not install the new image packages. In RustRover, open
`.devcontainer/devcontainer.json` and select **Create Dev Container and Mount
Sources**, using RustRover as the backend. Connect to that backend.

In its terminal, check:

```sh
rustc --version
cargo --version
rustup target list --installed
arm-none-eabi-gcc --version
arm-none-eabi-gdb --version
cargo build --locked --release
```

If the IDE retains an old missing-toolchain setting, set its Rust toolchain
location to `/home/ubuntu/.cargo/bin` inside the container. The standard-library
sources are installed through Rustup. The host Mac's Rust installation is
separate from the container's installation.

For host-only regression tests inside the container, substitute the host
triple from `rustc -vV`:

```sh
cargo test --manifest-path tests/host/Cargo.toml --target aarch64-unknown-linux-gnu
```

On an Intel Mac the container host triple is `x86_64-unknown-linux-gnu`.

## Flashing from macOS: OpenOCD on the Mac

For ST-Link flashing, OpenOCD on the Mac owns the USB probe and the
container's GDB connects to its TCP server.

On the **Mac**, install OpenOCD once:

```sh
brew install open-ocd
```

Connect the ST-Link to the Mac and to the board's SWD connector. Power the
board separately, as described in the repository README. From the repository
root in a **Mac terminal**, start:

```sh
sh .devcontainer/openocd-macos.sh
```

Keep that terminal open. In the **container terminal**, flash and run:

```sh
cargo run --locked --release
```

Compose sets Cargo's runner to `flash.sh`. It connects to
`host.docker.internal:3333`, loads the ELF, resets/runs the MCU, and detaches.
`OPENOCD_HOST` and `OPENOCD_PORT` can override the destination. No GDB port
needs to be published from the container: this connection is outbound to the
Mac. The host script disables OpenOCD's Tcl and telnet listeners and binds
GDB to the host interfaces so Docker can reach it; permit Docker Desktop in
the Mac firewall and keep port 3333 limited to your development network.

For an interactive debug session in RustRover, use the remote GDB server at
`host.docker.internal:3333`, GDB `/usr/bin/gdb-multiarch`, and the ELF
`target/thumbv7em-none-eabihf/release/open-source-ups`.

## Codex app server

The existing server stays available at `ws://localhost:4503` on the Mac
(container port 4500). Authentication persists in the `codex-data` volume.
Log in once from the Mac:

```sh
docker compose -f .devcontainer/compose.yaml run --rm app codex login --device-auth
```

The container waits for login before starting the app server; the IDE and Rust
tools can still be used while it waits. The capability token stays in the
ignored `.devcontainer/codex-ws-token` file.
