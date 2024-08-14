#[derive(Default, Copy, Clone)]
pub struct DataContainer {
    pub input_voltage: f32,
    pub output_voltage: f32,
    pub current : f32,
    pub battery_voltage: f32,
}