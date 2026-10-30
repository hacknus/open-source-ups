# Local USB recovery patch

Source: crates.io `synopsys-usb-otg` 0.4.0, MIT license (included).
Registry checksum: e948d523b316939545d8b21a48c27aef150ce25321b9f95ff7978647a806a6fe.

Changes relative to that release:

- Factor the existing enable sequence into `initialize(&self)` without changing
  the sequence. The HAL's `UsbPeripheral::enable()` resets the RCC peripheral.
- Add `disconnect()` to disable global USB interrupts and assert SDIS.
- Add `restart()` to rerun initialization without reallocating endpoints and
  enqueue a synthetic `PollResult::Reset`. This ensures usb-device and classes
  reset even before a new host reset is received. Calls are serialized with
  polling, reads and writes by the firmware's USB critical section.
- Clear buffered OUT/setup packets when configuring endpoints after a reset.
- Suppress pre-existing unused/generated-register warnings in the RAL and
  transition modules, previously suppressed for the registry dependency.

Firmware waits one second between disconnect and restart and suppresses USB servicing
during that interval. Its watchdog uses a monotonically wrapping
host-activity sequence updated by latched SOFs, real host resets/enumeration,
new received SETUP/OUT packets and completed IN transfers. The sequence is not
cleared by initialization; a locally synthesized reset is not host activity.
SOF status is polled and cleared without enabling SOF interrupts. This avoids
relying on a sampled frame number or Configured state as evidence of a live host.
The two-second silence timeout is separate from the ten-second enumeration
deadline, so real traffic can protect a connection while it is enumerating.

Host regression tests exercise watchdog decisions and USB stack/class reset
behavior. They cannot validate the hardware RCC/PHY/register sequence; repeated
unplug/replug and suspend/resume must be verified on the STM32F405 board.

A separate `host_transactions()` sequence advances only on newly received
SETUP/OUT packets or completed IN transfers. It excludes SOFs, reset events and
local writes. The firmware also recovers a Configured device after fifteen
seconds without transactions, even when SOFs continue, with no sleep exemption
and no retry limit. USB device release 1.02 identifies this build.
