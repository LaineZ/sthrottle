#[derive(PartialEq, Clone, Copy)]
/// Allowed stages to operate
/// TODO: Implement all stages
pub enum Stage {
    /// Default mode, 3 axes, one button for enable/disable reverse
    Normal,
    /// Multiplexes the thorttle axis. This stage made for workaround X-Plane input system which
    /// allows bind ONLY ONE action on axis, and for somre reason reverse thrust can be set for
    /// separate engines only
    NormalXplane,
    /// Calibration stage for minimum range for all axes
    CalibrationStageLow,
    /// Calibration stage for max range for all axes
    CalibrationStageHigh,
}

pub trait StageImpl {
    /// Calls once when user enters the stage
    fn on_enter(&mut self, throttle_readings: u16, prop_readings: u16, mixture_readings: u16) {}
    /// Calls once when user leaves the stage
    fn on_leave(&mut self, throttle_readings: u16, prop_readings: u16, mixture_readings: u16) {}
    /// Loop update the stage
    fn update(&mut self, throttle_readings: u16, prop_readings: u16, mixture_readings: u16);
    /// LED indication pattern bitmask with 100 ms delay
    fn indication(&self) -> u8;
}
