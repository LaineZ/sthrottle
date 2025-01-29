use super::{ProcessingStep, ProcessingSteps};

pub struct DeltaPass {
    sensivity: u16,
    old_value: u16,
}

impl ProcessingStep for DeltaPass {
    fn process(&mut self, value: u16) -> u16 {
        if (value as i16 - self.old_value as i16).abs() >= self.sensivity as i16 {
            self.old_value = value;
            value
        } else {
            self.old_value
        }
    }
}

impl DeltaPass {
    pub fn new(sensivity: u16) -> ProcessingSteps {
        ProcessingSteps::DeltaPass(Self {
            sensivity, old_value: 0
        })
    }
}
