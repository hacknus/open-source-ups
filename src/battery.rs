//! Board-specific measurement conversion and recoverable battery status.
use crate::report::Telemetry;

pub const SERIES_CELLS: f32 = 2.0;
pub const DIVIDER_GAIN: f32 = (72.0 + 27.0) / 27.0;
// U9 CC6905S8-3FB005, datasheet rev 1.3 (3.3 V): 264 mV/A.
// https://datasheet.lcsc.com/datasheet/pdf/81a1e0bc6663e21b6354e013ad7b1200.pdf
pub const CURRENT_ZERO_V: f32 = 1.650;
pub const CURRENT_SENSITIVITY_V_PER_A: f32 = 0.264;
// IP+ is VBAT1 (battery), IP- is VBAT2 (boost input): positive = boost draw.
pub const CURRENT_DIRECTION: f32 = 1.0;
pub const CURRENT_DEADBAND_A: f32 = 0.15;
const PACK_ENERGY_WH: f32 = SERIES_CELLS * 3.7 * 2.1;

#[derive(Clone, Copy, Debug)]
pub struct Measurements {
    pub pack_v: f32,
    pub input_v: f32,
    pub boost_current_a: f32,
}
impl Measurements {
    pub fn from_adc_volts(pack: f32, input: f32, current: f32) -> Option<Self> {
        if !pack.is_finite()
            || !input.is_finite()
            || !current.is_finite()
            || !(0.0..3.29).contains(&pack)
            || !(0.0..3.29).contains(&input)
            || !(0.1..3.2).contains(&current)
        {
            return None;
        }
        let amps = (current - CURRENT_ZERO_V) / CURRENT_SENSITIVITY_V_PER_A * CURRENT_DIRECTION;
        Some(Self {
            pack_v: pack * DIVIDER_GAIN,
            input_v: input * DIVIDER_GAIN,
            boost_current_a: if amps > -CURRENT_DEADBAND_A && amps < CURRENT_DEADBAND_A {
                0.0
            } else {
                amps
            },
        })
    }
}

pub fn capacity_percent(pack_v: f32) -> u8 {
    ((pack_v / SERIES_CELLS - 3.3) * 100.0 / (4.15 - 3.3)).clamp(0.0, 100.0) as u8
}

pub struct BatteryState {
    ac: bool,
    present: bool,
    discharging: bool,
    low: bool,
    critical: bool,
}
impl BatteryState {
    pub const fn new() -> Self {
        Self {
            ac: false,
            present: false,
            discharging: false,
            low: false,
            critical: false,
        }
    }
    pub fn update(&mut self, sample: Option<Measurements>) -> Telemetry {
        let Some(m) = sample else {
            return Telemetry::unknown();
        };
        if !m.pack_v.is_finite() || !m.input_v.is_finite() || !m.boost_current_a.is_finite() {
            return Telemetry::unknown();
        }
        self.ac = if self.ac {
            m.input_v > 9.5
        } else {
            m.input_v > 10.0
        };
        // Voltage-based presence is an estimate; there is no dedicated pack-detect pin.
        self.present = if self.present {
            m.pack_v > 2.0
        } else {
            m.pack_v > 3.0
        };
        self.discharging = self.present
            && !self.ac
            && if self.discharging {
                m.boost_current_a > 0.10
            } else {
                m.boost_current_a > 0.25
            };
        let cell_v = m.pack_v / SERIES_CELLS;
        self.low = self.present && if self.low { cell_v < 3.6 } else { cell_v < 3.5 };
        self.critical = self.present
            && if self.critical {
                cell_v < 3.3
            } else {
                cell_v < 3.2
            };
        let capacity = if self.present {
            capacity_percent(m.pack_v)
        } else {
            0
        };
        let runtime = if self.discharging && m.pack_v > 0.0 {
            (PACK_ENERGY_WH * capacity as f32 / 100.0 * 3600.0 / (m.pack_v * m.boost_current_a))
                .clamp(0.0, 65534.0) as u16
        } else {
            u16::MAX
        }; // unknown / not discharging
        let shutdown = self.low && !self.ac;
        let imminent = self.critical && !self.ac;
        let runtime_low = self.discharging && !self.ac && runtime <= 120;
        let mut status = 0;
        for (bit, set) in [
            (1, self.discharging),
            (2, self.ac),
            (3, self.present),
            (4, self.low),
            (5, runtime_low),
            (10, shutdown),
            (11, imminent),
        ] {
            if set {
                status |= 1 << bit;
            }
        }
        Telemetry {
            status,
            capacity,
            runtime,
            voltage_cv: (m.pack_v * 100.0) as u16,
        }
    }
}
