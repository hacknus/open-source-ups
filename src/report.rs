//! UPS HID wire reports and a bounded, coalescing transmit queue.

pub const STATUS: u8 = 7;
pub const CAPACITY: u8 = 12;
pub const RUNTIME: u8 = 13;
pub const VOLTAGE: u8 = 11;
pub const RECHARGEABLE: u8 = 6;
pub const FULL_CAPACITY: u8 = 14;
pub const WARNING_CAPACITY: u8 = 15;
pub const LOW_CAPACITY: u8 = 17;
pub const CAPACITY_MODE: u8 = 22;
pub const DESIGN_CAPACITY: u8 = 23;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Report {
    bytes: [u8; 3],
    len: usize,
}
impl Report {
    pub const fn u8(id: u8, value: u8) -> Self {
        Self {
            bytes: [id, value, 0],
            len: 2,
        }
    }
    pub const fn u16(id: u8, value: u16) -> Self {
        Self {
            bytes: [id, value as u8, (value >> 8) as u8],
            len: 3,
        }
    }
}
impl AsRef<[u8]> for Report {
    fn as_ref(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Telemetry {
    pub status: u16,
    pub capacity: u8,
    pub runtime: u16,
    pub voltage_cv: u16,
}
impl Telemetry {
    pub const fn unknown() -> Self {
        Self {
            status: 1 << 12,
            capacity: 0,
            runtime: u16::MAX,
            voltage_cv: 0,
        }
    }
    pub fn available(self, id: u8) -> bool {
        self.status & (1 << 12) == 0 || !matches!(id, CAPACITY | RUNTIME | VOLTAGE)
    }
    pub fn report(self, id: u8) -> Option<Report> {
        Some(match id {
            STATUS => Report::u16(id, self.status),
            CAPACITY => Report::u8(id, self.capacity),
            RUNTIME => Report::u16(id, self.runtime),
            VOLTAGE => Report::u16(id, self.voltage_cv),
            RECHARGEABLE => Report::u8(id, 1),
            CAPACITY_MODE => Report::u8(id, 2), // percentage
            FULL_CAPACITY | DESIGN_CAPACITY => Report::u8(id, 100),
            WARNING_CAPACITY => Report::u8(id, 20),
            LOW_CAPACITY => Report::u8(id, 10),
            _ => return None,
        })
    }
}

const LIVE_IDS: [u8; 4] = [STATUS, CAPACITY, RUNTIME, VOLTAGE];
pub struct PendingReports {
    pub telemetry: Telemetry,
    dirty: u8,
}
impl PendingReports {
    pub const fn new() -> Self {
        Self {
            telemetry: Telemetry::unknown(),
            dirty: 15,
        }
    }
    pub fn update(&mut self, telemetry: Telemetry) {
        for (i, id) in LIVE_IDS.iter().enumerate() {
            if self.telemetry.report(*id) != telemetry.report(*id) {
                self.dirty |= 1 << i;
            }
        }
        self.telemetry = telemetry;
    }
    pub fn refresh_one(&mut self, index: usize) {
        self.dirty |= 1 << index;
    }
    pub fn refresh(&mut self) {
        self.dirty = 15;
    }
    pub fn next(&self) -> Option<(usize, Report)> {
        LIVE_IDS.iter().enumerate().find_map(|(i, id)| {
            if self.dirty & (1 << i) != 0 && self.telemetry.available(*id) {
                Some((i, self.telemetry.report(*id).unwrap()))
            } else {
                None
            }
        })
    }
    pub fn sent(&mut self, index: usize) {
        self.dirty &= !(1 << index);
    }
}

// Read-only feature reports. Unsupported shutdown controls and string references
// are deliberately absent. Each item explicitly establishes its size and range.
pub const DESCRIPTOR: &[u8] = &[
    0x05,
    0x84,
    0x09,
    0x04,
    0xA1,
    0x01, // Power Device / UPS
    0x09,
    0x24,
    0xA1,
    0x02, // PowerSummary (logical)
    0x05,
    0x85,
    0x15,
    0x00,
    0x95,
    0x01,
    0x75,
    0x08,
    0x25,
    0x64, // 8-bit percentages
    0x85,
    DESIGN_CAPACITY,
    0x09,
    0x83,
    0xB1,
    0x02,
    0x85,
    FULL_CAPACITY,
    0x09,
    0x67,
    0xB1,
    0x02,
    0x85,
    WARNING_CAPACITY,
    0x09,
    0x8C,
    0xB1,
    0x02,
    0x85,
    LOW_CAPACITY,
    0x09,
    0x29,
    0xB1,
    0x02,
    0x85,
    CAPACITY,
    0x09,
    0x66,
    0x81,
    0x02,
    0x09,
    0x66,
    0xB1,
    0x02,
    0x25,
    0x02,
    0x85,
    CAPACITY_MODE,
    0x09,
    0x2C,
    0xB1,
    0x02,
    0x25,
    0x01,
    0x85,
    RECHARGEABLE,
    0x09,
    0x8B,
    0xB1,
    0x02,
    0x75,
    0x10,
    0x27,
    0xFF,
    0xFF,
    0x00,
    0x00,
    0x66,
    0x01,
    0x10,
    0x55,
    0x00, // seconds
    0x85,
    RUNTIME,
    0x09,
    0x68,
    0x81,
    0x02,
    0x09,
    0x68,
    0xB1,
    0x02,
    0x05,
    0x84,
    0x67,
    0x21,
    0xD1,
    0xF0,
    0x00,
    0x55,
    0x05, // centivolts
    0x85,
    VOLTAGE,
    0x09,
    0x30,
    0x81,
    0x02,
    0x09,
    0x30,
    0xB1,
    0x02,
    0x65,
    0x00,
    0x55,
    0x00,
    0x09,
    0x02,
    0xA1,
    0x02, // PresentStatus
    0x85,
    STATUS,
    0x75,
    0x01,
    0x25,
    0x01,
    // Bit 0 is padding: the MCU cannot observe charger current or charger STAT.
    0x81,
    0x01,
    0xB1,
    0x01,
    0x05,
    0x85,
    0x09,
    0x45,
    0x81,
    0x02,
    0x09,
    0x45,
    0xB1,
    0x02, // discharging
    0x09,
    0xD0,
    0x81,
    0x02,
    0x09,
    0xD0,
    0xB1,
    0x02, // AC present
    0x09,
    0xD1,
    0x81,
    0x02,
    0x09,
    0xD1,
    0xB1,
    0x02, // battery present
    0x09,
    0x42,
    0x81,
    0x02,
    0x09,
    0x42,
    0xB1,
    0x02, // low capacity
    0x09,
    0x43,
    0x81,
    0x02,
    0x09,
    0x43,
    0xB1,
    0x02, // runtime limit
    0x95,
    0x04,
    0x81,
    0x01,
    0xB1,
    0x01, // unsupported bits 6..9
    0x95,
    0x01,
    0x05,
    0x84,
    0x09,
    0x68,
    0x81,
    0x02,
    0x09,
    0x68,
    0xB1,
    0x02, // shutdown requested
    0x09,
    0x69,
    0x81,
    0x02,
    0x09,
    0x69,
    0xB1,
    0x02, // shutdown imminent
    0x09,
    0x73,
    0x81,
    0x02,
    0x09,
    0x73,
    0xB1,
    0x02, // communication lost
    0x95,
    0x03,
    0x81,
    0x01,
    0xB1,
    0x01, // bits 13..15 padding
    0xC0,
    0xC0,
    0xC0,
];
