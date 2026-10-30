//! Board-specific measurement conversion and recoverable battery status.
use crate::power::PowerSource;
use crate::report::Telemetry;

pub const SERIES_CELLS: f32 = 2.0;
pub const PACK_DIVIDER_GAIN: f32 = (72.0 + 27.0) / 27.0;
pub const INPUT_DIVIDER_GAIN: f32 = (72.0 + 27.0) / 27.0;
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
    pub pack_v: Option<f32>,
    // None at the ADC rail: the existing input divider only measures to ~12.1 V.
    // This diagnostic channel must never determine which source is selected.
    pub input_v: Option<f32>,
    pub boost_current_a: Option<f32>,
}
impl Measurements {
    pub fn from_adc_volts(pack: f32, input: f32, current: f32) -> Self {
        let voltage = |v: f32, gain: f32| {
            if v.is_finite() && (0.0..3.29).contains(&v) {
                Some(v * gain)
            } else {
                None
            }
        };
        let amps = if current.is_finite() && (0.1..3.2).contains(&current) {
            let amps = (current - CURRENT_ZERO_V) / CURRENT_SENSITIVITY_V_PER_A * CURRENT_DIRECTION;
            Some(if amps > -CURRENT_DEADBAND_A && amps < CURRENT_DEADBAND_A {
                0.0
            } else {
                amps
            })
        } else {
            None
        };
        Self {
            pack_v: voltage(pack, PACK_DIVIDER_GAIN),
            input_v: voltage(input, INPUT_DIVIDER_GAIN),
            boost_current_a: amps,
        }
    }

    pub fn from_adc_samples(raw: [u16; 4], volts_per_count: f32) -> Self {
        let mut measurement = Self::from_adc_volts(
            raw[1] as f32 * volts_per_count,
            raw[2] as f32 * volts_per_count,
            raw[3] as f32 * volts_per_count,
        );
        // Detect clipping from the 12-bit counts, independently of calibrated VDDA.
        if raw[1] >= 4080 {
            measurement.pack_v = None;
        }
        if raw[2] >= 4080 {
            measurement.input_v = None;
        }
        if raw[3] >= 4080 {
            measurement.boost_current_a = None;
        }
        measurement
    }

    pub fn has_battery_fault(self) -> bool {
        self.pack_v.filter(|v| v.is_finite() && *v >= 0.0).is_none()
            || self.boost_current_a.filter(|i| i.is_finite()).is_none()
    }
}

pub fn capacity_percent(pack_v: f32) -> u8 {
    ((pack_v / SERIES_CELLS - 3.3) * 100.0 / (4.15 - 3.3)).clamp(0.0, 100.0) as u8
}

pub struct BatteryState {
    present: bool,
    low: bool,
    critical: bool,
    runtime_power_w: f32,
}

// Estimated battery-side load before a measurable backup load is available.
// Adjust for the actual NAS plus converter losses; this is not a measured value.
pub const DEFAULT_RUNTIME_POWER_W: f32 = 12.0;
const MIN_RUNTIME_POWER_W: f32 = 1.0;
impl BatteryState {
    pub const fn new() -> Self {
        Self {
            present: false,
            low: false,
            critical: false,
            runtime_power_w: DEFAULT_RUNTIME_POWER_W,
        }
    }
    pub fn update(&mut self, source: PowerSource, sample: Option<Measurements>) -> Telemetry {
        let ac = source == PowerSource::External;
        let mut telemetry = Telemetry::unknown();
        telemetry.status = match source {
            PowerSource::External => 1 << 2,
            PowerSource::Battery => 1 << 1,
            PowerSource::Unavailable => 0,
        };
        let Some(m) = sample else {
            telemetry.status |= 1 << 12;
            return telemetry;
        };
        let current = m.boost_current_a.filter(|i| i.is_finite());
        let Some(pack_v) = m.pack_v.filter(|v| v.is_finite() && *v >= 0.0) else {
            telemetry.status |= 1 << 12;
            return telemetry;
        };
        telemetry.battery_measurements_valid = true;
        // Voltage-based presence is an estimate; there is no dedicated pack-detect pin.
        self.present = if self.present {
            pack_v > 2.0
        } else {
            pack_v > 3.0
        };
        // A valid selected backup path indicates battery operation even at light
        // load or when current sensing fails. Current is only needed for runtime.
        let discharging = source == PowerSource::Battery;
        let cell_v = pack_v / SERIES_CELLS;
        self.low = self.present && if self.low { cell_v < 3.6 } else { cell_v < 3.5 };
        self.critical = self.present
            && if self.critical {
                cell_v < 3.3
            } else {
                cell_v < 3.2
            };
        let capacity = if self.present {
            capacity_percent(pack_v)
        } else {
            0
        };
        // Updates occur every 500 ms. Smooth load over roughly ten seconds;
        // source transitions must not replace a real estimate with 65535.
        if discharging {
            if let Some(amps) = current.filter(|amps| *amps > CURRENT_DEADBAND_A) {
                let power = (pack_v * amps).max(MIN_RUNTIME_POWER_W);
                self.runtime_power_w += (power - self.runtime_power_w) / 20.0;
            }
        }
        // With no measurable load (or a sensor fault), avoid extrapolating a
        // near-zero current to many hours. Keep a conservative nominal floor.
        let power = if discharging && current.map_or(true, |amps| amps <= CURRENT_DEADBAND_A) {
            self.runtime_power_w.max(DEFAULT_RUNTIME_POWER_W)
        } else {
            self.runtime_power_w
        };
        let runtime =
            (PACK_ENERGY_WH * capacity as f32 / 100.0 * 3600.0 / power).clamp(0.0, 65534.0) as u16;
        let shutdown = self.low && !ac;
        let imminent = self.critical && !ac;
        let runtime_low = discharging && runtime != u16::MAX && runtime <= 120;
        for (bit, set) in [
            (1, discharging),
            (3, self.present),
            (4, self.low),
            (5, runtime_low),
            (10, shutdown),
            (11, imminent),
            (12, current.is_none()),
        ] {
            if set {
                telemetry.status |= 1 << bit;
            }
        }
        telemetry.capacity = capacity;
        telemetry.runtime = runtime;
        telemetry.voltage_cv = (pack_v * 100.0) as u16;
        telemetry
    }
}
