//! Minimal UPS HID class with read-only feature reports and retryable input reports.
use crate::report::{self, PendingReports, Telemetry};
use usb_device::class_prelude::*;
use usb_device::{Result, UsbError};

pub struct UpsHid<'a, B: UsbBus> {
    interface: InterfaceNumber,
    endpoint: EndpointIn<'a, B>,
    reports: PendingReports,
    configured: bool,
    idle: [u8; 4],
    elapsed_ms: [u16; 4],
}
impl<'a, B: UsbBus> UpsHid<'a, B> {
    pub fn new(bus: &'a UsbBusAllocator<B>) -> Self {
        Self {
            interface: bus.interface(),
            endpoint: bus.interrupt(8, 10),
            reports: PendingReports::new(),
            configured: false,
            idle: [0; 4],
            elapsed_ms: [0; 4],
        }
    }
    pub fn tick(&mut self, elapsed_ms: u16) {
        for i in 0..4 {
            if self.idle[i] != 0 {
                self.elapsed_ms[i] = self.elapsed_ms[i].saturating_add(elapsed_ms);
                if self.elapsed_ms[i] >= self.idle[i] as u16 * 4 {
                    self.reports.refresh_one(i);
                    self.elapsed_ms[i] = 0;
                }
            }
        }
    }
    pub fn update(&mut self, telemetry: Telemetry, refresh: bool) {
        self.reports.update(telemetry);
        if refresh {
            self.reports.refresh();
        }
    }
    pub fn set_configured(&mut self, configured: bool) {
        if configured && !self.configured {
            self.reports.refresh();
        }
        self.configured = configured;
    }
    pub fn flush_pending(&mut self) -> Result<()> {
        if !self.configured {
            return Ok(());
        }
        if let Some((index, report)) = self.reports.next() {
            match self.endpoint.write(report.as_ref()) {
                Ok(n) if n == report.as_ref().len() => {
                    self.reports.sent(index);
                    self.elapsed_ms[index] = 0;
                }
                Ok(_) => return Err(UsbError::BufferOverflow),
                Err(error) => return Err(error), // keep pending, including WouldBlock
            }
        }
        Ok(())
    }
}
impl<B: UsbBus> UsbClass<B> for UpsHid<'_, B> {
    fn get_configuration_descriptors(&self, writer: &mut DescriptorWriter) -> Result<()> {
        writer.interface(self.interface, 0x03, 0, 0)?;
        writer.write(
            0x21,
            &[
                0x11,
                0x01,
                0,
                1,
                0x22,
                report::DESCRIPTOR.len() as u8,
                (report::DESCRIPTOR.len() >> 8) as u8,
            ],
        )?;
        writer.endpoint(&self.endpoint)
    }
    fn reset(&mut self) {
        self.configured = false;
        self.reports.refresh();
        self.idle = [0; 4];
        self.elapsed_ms = [0; 4];
    }
    fn control_in(&mut self, xfer: ControlIn<B>) {
        let req = *xfer.request();
        if req.recipient != control::Recipient::Interface
            || req.index != u8::from(self.interface) as u16
        {
            return;
        }
        match (req.request_type, req.request) {
            (control::RequestType::Standard, control::Request::GET_DESCRIPTOR)
                if req.value == 0x2200 =>
            {
                xfer.accept_with_static(report::DESCRIPTOR).ok();
            }
            (control::RequestType::Standard, control::Request::GET_DESCRIPTOR)
                if req.value == 0x2100 =>
            {
                xfer.accept_with(&[
                    9,
                    0x21,
                    0x11,
                    0x01,
                    0,
                    1,
                    0x22,
                    report::DESCRIPTOR.len() as u8,
                    (report::DESCRIPTOR.len() >> 8) as u8,
                ])
                .ok();
            }
            (control::RequestType::Class, 0x01) if req.length > 0 => {
                // GET_REPORT
                let report_type = (req.value >> 8) as u8;
                let id = req.value as u8;
                let is_input = matches!(
                    id,
                    report::STATUS | report::CAPACITY | report::RUNTIME | report::VOLTAGE
                );
                if (report_type == 3 || (report_type == 1 && is_input))
                    && self.reports.telemetry.available(id)
                {
                    if let Some(report) = self.reports.telemetry.report(id) {
                        xfer.accept_with(report.as_ref()).ok();
                        return;
                    }
                }
                xfer.reject().ok();
            }
            (control::RequestType::Class, 0x02) if req.value >> 8 == 0 && req.length == 1 => {
                if let Some(index) = report_index(req.value as u8) {
                    xfer.accept_with(&[self.idle[index]]).ok();
                } else {
                    xfer.reject().ok();
                }
            }
            (control::RequestType::Class, _) => {
                xfer.reject().ok();
            }
            _ => {} // allow usb-device to handle standard interface requests
        }
    }
    fn control_out(&mut self, xfer: ControlOut<B>) {
        let req = *xfer.request();
        if req.recipient != control::Recipient::Interface
            || req.index != u8::from(self.interface) as u16
        {
            return;
        }
        // Read-only reports: reject SET_REPORT and unsupported protocol requests.
        if req.request_type != control::RequestType::Class {
            return;
        }
        if req.request == 0x0A && req.length == 0 {
            let id = req.value as u8;
            let rate = (req.value >> 8) as u8;
            if id == 0 {
                self.idle = [rate; 4];
                self.elapsed_ms = [0; 4];
            } else if let Some(index) = report_index(id) {
                self.idle[index] = rate;
                self.elapsed_ms[index] = 0;
            } else {
                xfer.reject().ok();
                return;
            }
            xfer.accept().ok();
        } else {
            xfer.reject().ok();
        }
    }

    fn endpoint_in_complete(&mut self, address: EndpointAddress) {
        if address == self.endpoint.address() {
            self.flush_pending().ok();
        }
    }
}

fn report_index(id: u8) -> Option<usize> {
    report::LIVE_IDS
        .iter()
        .position(|candidate| *candidate == id)
}
