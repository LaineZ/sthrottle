use cortex_m_semihosting::hprintln;

use super::{floor, ProcessingSteps};

#[derive(Default)]
pub struct Lerp {
    smoothed_value: Option<f32>, 
    lerp_factor: f32, 
}

impl super::ProcessingStep for Lerp {
    fn process(&mut self, value: u16) -> u16 {
        let value_f32 = value as f32;
        let sv = self.smoothed_value.unwrap_or(value_f32);
        let new_value = sv + (value_f32 - sv) * self.lerp_factor;

        self.smoothed_value = Some(new_value);
        floor(new_value) as u16 
    }
}

impl Lerp {
    pub fn new(lerp_factor: f32) -> ProcessingSteps {
        ProcessingSteps::Lerp(Self {
            smoothed_value: None,
            lerp_factor: lerp_factor.clamp(0.0, 1.0),
        })
    }
}
