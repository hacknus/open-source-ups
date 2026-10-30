use crate::power::PowerSource;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LEDState {
    FastBreathing,
    SlowBreathing,
    Fault,
}

impl LEDState {
    pub fn for_power(source: PowerSource, measurement_fault: bool) -> Self {
        if measurement_fault {
            Self::Fault
        } else {
            match source {
                PowerSource::External => Self::SlowBreathing,
                PowerSource::Battery => Self::FastBreathing,
                PowerSource::Unavailable => Self::Fault,
            }
        }
    }

    pub const fn period_ms(self) -> u16 {
        match self {
            Self::SlowBreathing => 2000,
            Self::FastBreathing => 500,
            Self::Fault => 1000,
        }
    }

    /// Smoothstep fade up/down over the complete cycle, using bounded integer
    /// arithmetic. PB1 drives the LED anode, so increasing duty increases light.
    pub fn duty_at(self, phase_ms: u16, max_duty: u16) -> u16 {
        let phase = phase_ms % self.period_ms();
        if self == Self::Fault {
            return if phase < 100 || (200..300).contains(&phase) {
                max_duty
            } else {
                0
            };
        }
        let half = self.period_ms() as u32 / 2;
        let phase = phase as u32;
        let distance = if phase <= half {
            phase
        } else {
            2 * half - phase
        };
        let x = distance * 1024 / half;
        let eased = x * x * (3 * 1024 - 2 * x) / (1024 * 1024);
        (max_duty as u32 * eased / 1024) as u16
    }
}

pub struct LedAnimation {
    state: LEDState,
    phase_ms: u16,
}
impl LedAnimation {
    pub const fn new() -> Self {
        Self {
            state: LEDState::Fault,
            phase_ms: 0,
        }
    }

    pub fn step(&mut self, state: LEDState, max_duty: u16, elapsed_ms: u16) -> u16 {
        if state != self.state {
            self.state = state;
            self.phase_ms = 0;
        }
        let duty = state.duty_at(self.phase_ms, max_duty);
        self.phase_ms =
            ((self.phase_ms as u32 + elapsed_ms as u32) % state.period_ms() as u32) as u16;
        duty
    }
}
