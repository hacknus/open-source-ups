//! Expected LTC4418 power path from its active-low, qualified validity outputs.
//! V1 (external input) has priority over V2 (battery boost). These pins report
//! validity, not MOSFET conduction; this assumes the controller is enabled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerSource {
    External,
    Battery,
    Unavailable,
}

impl PowerSource {
    pub fn from_pin_levels(valid1_high: bool, valid2_high: bool) -> Self {
        if !valid1_high {
            Self::External
        } else if !valid2_high {
            Self::Battery
        } else {
            Self::Unavailable
        }
    }
}
