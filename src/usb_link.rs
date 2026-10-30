//! Host activity watchdog for boards without a VBUS-sense connection.
use usb_device::device::UsbDeviceState;

const SILENCE_MS: u16 = 2000;
const ENUMERATION_MS: u16 = 10000;
// Full refresh is every five seconds; allow three refresh periods for host
// transactions. Frames alone must not keep a non-responsive host alive.
const RESPONSE_MS: u16 = 15000;
const DISCONNECT_MS: u16 = 1000;

#[derive(Debug, PartialEq, Eq)]
pub enum LinkAction {
    Disconnect,
    Connect,
}

pub struct UsbLink {
    silent_ms: u16,
    unconfigured_ms: u16,
    disconnected_ms: Option<u16>,
    last_activity: u32,
    last_transactions: u32,
    no_response_ms: u16,
}

impl UsbLink {
    pub const fn new() -> Self {
        Self {
            silent_ms: 0,
            unconfigured_ms: 0,
            disconnected_ms: None,
            last_activity: 0,
            last_transactions: 0,
            no_response_ms: 0,
        }
    }

    pub fn is_disconnected(&self) -> bool {
        self.disconnected_ms.is_some()
    }

    pub fn tick(
        &mut self,
        state: UsbDeviceState,
        activity: u32,
        transactions: u32,
        elapsed_ms: u16,
    ) -> Option<LinkAction> {
        let received = activity != self.last_activity;
        self.last_activity = activity;
        let responded = transactions != self.last_transactions;
        self.last_transactions = transactions;
        // Complete a disconnect even if cached stack state says Configured.
        if let Some(elapsed) = self.disconnected_ms.as_mut() {
            *elapsed = elapsed.saturating_add(elapsed_ms);
            if *elapsed >= DISCONNECT_MS {
                self.disconnected_ms = None;
                self.silent_ms = 0;
                self.unconfigured_ms = 0;
                self.no_response_ms = 0;
                return Some(LinkAction::Connect);
            }
            return None;
        }
        self.silent_ms = if received {
            0
        } else {
            self.silent_ms.saturating_add(elapsed_ms)
        };
        self.unconfigured_ms = if state == UsbDeviceState::Configured {
            0
        } else {
            self.unconfigured_ms.saturating_add(elapsed_ms)
        };
        self.no_response_ms = if responded || state != UsbDeviceState::Configured {
            0
        } else {
            self.no_response_ms.saturating_add(elapsed_ms)
        };
        // Host resets, SETUP and SOFs protect slow enumeration from the silence
        // timeout. SOFs alone cannot keep a broken enumeration alive forever.
        // Suspend/unplug are indistinguishable here; both permit recovery.
        if self.silent_ms >= SILENCE_MS
            || self.unconfigured_ms >= ENUMERATION_MS
            || self.no_response_ms >= RESPONSE_MS
        {
            self.disconnected_ms = Some(0);
            Some(LinkAction::Disconnect)
        } else {
            None
        }
    }
}
