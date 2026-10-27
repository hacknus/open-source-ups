# Firmware measurement and USB reporting

The firmware uses PA0/PA1/PA2 for pack voltage, boost-branch current,
and input voltage. DMA2 stream 0/channel 0 owns one buffer while software
owns a separate spare. The completion handler exchanges them, averages 16
scans, and publishes a copied snapshot. A task starts a scan every 20 ms.
Missing or invalid measurements become unavailable after roughly one second;
unavailable measurements cannot request shutdown or produce zero-capacity
input reports. PresentStatus reports CommunicationLost until measurements
recover. Measurement feature reads stall while measurements are unavailable.
A first average is collected before USB initialization. The old five-second
boot LED test was removed so monitoring starts promptly.

Both voltage dividers are 72 kOhm / 27 kOhm. Conversion uses their actual gain,
99/27, and the HAL's calibrated ADC reference voltage.

U9 is CC6905S8-3FB005: nominal sensitivity 0.264 V/A and zero-current output
1.650 V at 3.3 V supply. Constants are in `src/battery.rs`. Positive current
flows from VBAT1 through IP+ to IP- at VBAT2, toward the boost converter.
The firmware averages samples and applies a 0.15 A zero-current deadband.
Manufacturer datasheet rev 1.3:
https://datasheet.lcsc.com/datasheet/pdf/81a1e0bc6663e21b6354e013ad7b1200.pdf

The charger connects directly to VBAT1. This sensor cannot observe net pack
charging current, and charger STAT is not connected to an MCU input.
Charging is consequently omitted from the HID status descriptor. Boost
current is interpreted as discharge only when external power is absent.
Battery presence is estimated from pack voltage, independently of AC presence.

Capacity is a bounded linear estimate using average cell voltage for two
series cells: 3.3 V = 0%, 4.15 V = 100%. Low voltage asserts below 3.5 V/cell
and recovers at 3.6 V; critical asserts below 3.2 V/cell and recovers at 3.3 V.
ShutdownRequested and ShutdownImminent apply during external-power loss and
clear on recovery. AC asserts above 10 V and clears at 9.5 V. Runtime is an
estimate from remaining energy in a nominal 2.1 Ah, 7.4 V pack and measured
boost input power; 65535 means runtime unknown/not discharging. Pack voltage
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

## Verification

Host tests run without the FreeRTOS build or ARM hardware:

```sh
cargo test --manifest-path tests/host/Cargo.toml --target <host-triple>
```

Obtain the host triple with `rustc -vV`. Tests cover conversion, capacity
bounds, outage/recovery and missing-data behavior, runtime units, descriptor
sizes and flags, USB feature requests and rejection, endpoint busy retries,
reconfiguration refresh, and idle handling using a simulated USB bus.

Build the firmware with the pinned toolchain and an ARM C compiler:

```sh
cargo build --locked --release
```

Before deployment, check pack/input voltage against a meter, measure the
sensor's zero-current output, and verify the sign and sensitivity with a
known boost load. The nominal offset is not a board-specific calibration.
Test both USB modes, host feature reads, unplug/replug, external-power loss,
and low-voltage transitions on the target. No board was available for these
checks during implementation.
