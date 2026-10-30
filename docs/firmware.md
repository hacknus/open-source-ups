# Firmware measurement and USB reporting

The firmware uses PA0/PA1/PA2 for pack voltage, boost-branch current,
and input voltage. DMA2 stream 0/channel 0 owns one buffer while software
owns a separate spare. The completion handler exchanges them, averages 16
scans, and publishes a copied snapshot. A task starts a scan every 20 ms.
A missing ADC scan becomes stale after roughly one second. Each channel is
validated independently: a current-sensor fault does not discard pack voltage,
capacity, or low-battery detection. Missing pack voltage cannot request shutdown
or produce a zero-capacity input report. PresentStatus reports CommunicationLost
when battery voltage or current is unavailable, while preserving the independently
known power-source flags. Feature reads stall only for unavailable battery
measurements. Runtime is transmitted in seconds when pack voltage is valid. The 65535-second
unknown sentinel is never sent: both macOS and NUT can display it as 18:12.
A first average is collected before USB initialization. The old five-second
boot LED test was removed so monitoring starts promptly.

Both voltage dividers are 72 kOhm / 27 kOhm. Conversion uses their actual gain,
99/27, and the HAL's calibrated ADC reference voltage. Separate pack/input
conversion constants are in `src/battery.rs`. A near-full-scale ADC count marks
that channel unavailable, independently of reference-voltage calibration.

The input divider has little headroom: 12 V produces about 3.273 V at PA2 and
12.1 V produces 3.3 V. Clipped input voltage is shown as `None` in serial output;
it never changes source detection or invalidates battery measurements. Accurate
input-voltage readings above this range still require a hardware divider change.
For a future board, R24 = 100 kOhm with R25 = 27 kOhm would give a nominal
15.52 V full scale at 3.3 V; update only `INPUT_DIVIDER_GAIN` to 127/27 if making
that physical change. The current hardware files and resistor values are unchanged.

## Source detection and status LED

PB12 reads LTC4418 VALID1 (external 12V_IN); PB13 reads VALID2 (12V_BAT boost).
Both are active-low open-drain outputs with existing 3.3 V external pull-ups.
The firmware follows their qualified validity and V1-over-V2 priority rather
than comparing input ADC voltage to independent software thresholds:

| PB12 / VALID1 | PB13 / VALID2 | Expected power path | STAT0 / PB1 |
| --- | --- | --- | --- |
| Low | Either | External | Smooth breath, 2-second period |
| High | Low | Battery boost | Smooth breath, 0.5-second period |
| High | High | Neither valid | Double flash once per second |

A missing/invalid battery voltage or current reading also selects the double
flash, while retaining any known source in the USB status. Input-voltage clipping
alone does not select the fault pattern. Source/status changes are checked every
500 ms. The validity outputs report supply qualification, not actual MOSFET
conduction; source inference assumes the controller is enabled and functioning.
See the [LTC4418 datasheet](https://www.analog.com/media/en/technical-documentation/data-sheets/ltc4418.pdf).

The LED animation uses a bounded phase and an integer smoothstep rise/fall,
updated every 5 ms. It has no signed-to-unsigned sine clipping or unbounded
phase counter. Fault mode is two 100 ms flashes separated by 100 ms, followed
by 700 ms off. PC13's existing HID-mode heartbeat is unchanged.


U9 is CC6905S8-3FB005: nominal sensitivity 0.264 V/A and zero-current output
1.650 V at 3.3 V supply. Constants are in `src/battery.rs`. Positive current
flows from VBAT1 through IP+ to IP- at VBAT2, toward the boost converter.
The firmware averages samples and applies a 0.15 A zero-current deadband.
Manufacturer datasheet rev 1.3:
https://datasheet.lcsc.com/datasheet/pdf/81a1e0bc6663e21b6354e013ad7b1200.pdf

The charger connects directly to VBAT1. This sensor cannot observe net pack
charging current, and charger STAT is not connected to an MCU input.
Charging is consequently omitted from the HID status descriptor. The selected
battery-boost path is reported as discharge even at light load or if current
sensing fails. A measurable positive boost current refines the runtime load estimate.
Battery presence is estimated from pack voltage, independently of AC presence.

Capacity is a bounded linear estimate using average cell voltage for two
series cells: 3.3 V = 0%, 4.15 V = 100%. Low voltage asserts below 3.5 V/cell
and recovers at 3.6 V; critical asserts below 3.2 V/cell and recovers at 3.3 V.
ShutdownRequested and ShutdownImminent apply during external-power loss and
clear on recovery. AC presence follows external-input validity from the LTC4418.
Runtime is an estimate from remaining energy in a nominal 2.1 Ah, 7.4 V pack
and estimated battery-side load. Initially the load is 12 W, configured by
`DEFAULT_RUNTIME_POWER_W` in `src/battery.rs`; change this for the NAS plus boost
converter losses. During backup operation, measurable current updates the load
with a 1/20 exponential filter every 500 ms (about a ten-second time constant).
Measured power has a 1 W floor. On AC the last filtered load is retained so
runtime represents estimated backup endurance, not time until discharge on AC.
During backup with unmeasurable/faulty current, the load is the larger of the
last filtered load and 12 W. The communication-fault flag still signals a missing
current reading. A missing pack reading suppresses runtime rather than sending
65535. Hosts may retain a previous runtime while measurements are unavailable;
source/fault flags and voltage thresholds remain the authoritative status.
These are estimates, not calibrated endurance guarantees. Pack voltage
cannot reveal individual cell imbalance. Review these thresholds for the
fitted cells and required host shutdown time.

There is one USB device with either a HID class or a serial class, selected
by the existing PC15 boot switch. The HID implementation lives in this repo;
the previous HID and bitfield dependencies are no longer used.

HID exposes PresentStatus, RemainingCapacity, RunTimeToEmpty, and Voltage as
input and read-only feature reports. Rechargeable, CapacityMode (percentage),
DesignCapacity, FullChargeCapacity, WarningCapacityLimit, and
RemainingCapacityLimit are read-only features. Unsupported power-off/reboot
commands and optional string-index reports are absent. USB manufacturer,
product, and serial strings remain available through standard USB descriptors.

The bounded queue coalesces each report to its latest value, prioritizes
status, retries endpoint-busy writes, and refreshes all reports on configuration
and every five seconds. GET_REPORT reads the same state as input reports.
SET_REPORT is rejected because every advertised feature is read-only.
Per-report HID idle rates are honored by the task's 20 ms service tick;
the interrupt endpoint polling interval is 10 ms.

## USB attachment recovery

The board is self-powered; the USB configuration declares this and requests
no bus power. PA9 is unconnected, so firmware cannot directly detect cable VBUS.
The USB driver disables VBUS sensing. Instead, the driver keeps a host-activity
sequence which advances on observed SOF events, host reset/enumeration events,
received SETUP/OUT packets, and completed IN transfers. Local endpoint writes
and locally synthesized resets do not advance it. The sequence survives
peripheral restarts and wraps safely. SOF's latched status is sampled and cleared
by polling, without enabling a 1 kHz SOF interrupt.

The service task checks that sequence every 20 ms. Two seconds without host
activity triggers recovery even if software remains Configured. Unlike the old
frame-counter watchdog, activity also protects a connection while the host is
still enumerating it. A separate ten-second deadline retries enumeration which
never completes even though SOFs continue. Communication proves an active host;
it does not distinguish an unplug from a sleeping or suspended host.

A second sequence counts completed IN transfers and received SETUP/OUT packets
without counting SOFs or bus resets. Once Configured, fifteen seconds without
one of these transactions also triggers recovery, even if SOFs keep arriving.
Normal five-second report refreshes or host feature reads satisfy this deadline.
No special exemption is made for host sleep: sleeping and unresponsive hosts
are treated alike. Re-enumeration attempts continue without a retry limit,
including across repeated successful configurations and subsequent disconnects.


Recovery disconnects the pull-up and masks core interrupts for one second, then
reinitializes the USB controller using the same RCC/peripheral/PHY initialization
as boot. Endpoint allocations remain fixed; the next poll delivers a Reset to
usb-device, clearing address, control transfers, HID/CDC class state, and buffered
OUT packets before processing new host traffic. While disconnected, polling and
endpoint writes are suppressed. A subsequent host configuration resends current
telemetry. The firmware never resets the MCU for USB recovery.

This uses a local patch to synopsys-usb-otg 0.4.0; see
`vendor/synopsys-usb-otg/PATCH.md`. USB device release is 1.02, visible in the
host USB device information, to identify this firmware when checking a flash.
The previous configured/frame-only watchdog could interrupt active enumeration;
its two-second timer was not extended by reset or control traffic.
Since VBUS is unconnected, host sleep cannot reliably be distinguished from an
unplug. A host asleep longer than two seconds may see USB re-enumerate on wake.
Physical unplug/replug and sleep/wake still need validation on the target.

After flashing, boot with USB disconnected, then attach it without resetting.
Check `system_profiler SPUSBDataType` for UPS and `pmset -g ps` for its power source.
If it appears only in the former, USB enumerated but macOS did not register the
UPS power source; capture both outputs to distinguish these failure modes.
Also test ordinary unplug/replug and Mac sleep/wake. Runtime-based thresholds are available again; the MCU's voltage-based status
thresholds remain independent of the runtime estimate.

## Verification

Host tests run without the FreeRTOS build or ARM hardware:

```sh
cargo test --manifest-path tests/host/Cargo.toml --target <host-triple>
```

Obtain the host triple with `rustc -vV`. Tests cover conversion, capacity
bounds, active-low supply validity and priority, clipping and partial sensor
failures, outage/recovery, breathing symmetry/period/wrap and fault indication,
runtime units, descriptor sizes and flags, USB feature requests and rejection, endpoint busy retries,
reconfiguration refresh, and idle handling using a simulated USB bus.

Build the firmware with the pinned toolchain and an ARM C compiler:

```sh
cargo build --locked --release
```

Before deployment, check pack/input voltage against a meter, measure the
sensor's zero-current output, and verify the sign and sensitivity with a
known boost load. The nominal offset is not a board-specific calibration.
Test both USB modes, host feature reads, unplug/replug, external-power loss,
and low-voltage transitions on the target. With both supplies valid, STAT0
should breathe slowly. Disconnect the adapter and confirm the fast breath;
reconnect it and confirm slow breathing after the controller requalifies V1.
Neither source valid or unavailable battery measurements should double-flash.
Check PB12/PB13 levels against the source shown in serial diagnostics. No board was available for these
checks during implementation.

## Synology / NUT

The USB compatibility ID is now `03f0:1fe7`, recognized by the legacy NUT
`mge-hid` subdriver (including upstream 2.7.4), instead of `03f0:1f06`, which
is absent from that table. This continues the project's existing borrowed HP
identity for compatibility; it is not a USB ID allocation for this project.
The standard `UPS.PowerSummary.RunTimeToEmpty` HID path maps directly to NUT
`battery.runtime` in seconds; AC present, discharge and low-battery paths are
also supported by that subdriver. No vendor shutdown commands are implemented.

Sources: [NUT legacy device table and report mappings](https://github.com/networkupstools/nut/blob/v2.7.4/drivers/mge-hid.c),
[NUT runtime sentinel caveat](https://networkupstools.org/docs/man/usbhid-ups.html).

In DSM, enable UPS support under Control Panel > Hardware & Power > UPS and
choose USB UPS. DSM recognition and its bundled NUT version still require an
on-device check; this custom device is not Synology-certified. If SSH is enabled,
try `upsc ups@localhost` (the binary may be `/usr/syno/bin/upsc`). Verify
`battery.runtime` is numeric seconds, `battery.charge` is percentage, and
`ups.status` changes from `OL` to `OB` after disconnecting external power, then
returns to `OL`. `DISCHRG` and `LB` can accompany battery operation. Test under
the actual NAS load and compare the estimate with measured endurance before
relying on a runtime shutdown threshold. Keep an adequate shutdown margin.

## Build artifact

Run `sh scripts/build-firmware.sh` from the development container to build and
export the current release to the repository's **firmware.bin**. No alternate
firmware filenames are produced. A plain `cargo build` creates the ELF only;
it does not refresh firmware.bin. Copy firmware.bin to the Mac and flash it
there using the existing DFU procedure. Older alternate binaries have been
removed to avoid accidentally flashing an outdated build.
