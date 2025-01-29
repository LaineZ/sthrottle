use delta_pass::DeltaPass;
use lerp::Lerp;
use mean::Mean;

pub mod mean;
pub mod lerp;
pub mod delta_pass;

pub trait ProcessingStep {
    fn process(&mut self, value: u16) -> u16;
}

pub enum ProcessingSteps {
    Mean(Mean),
    DeltaPass(DeltaPass),
    Lerp(Lerp),
}

impl ProcessingSteps {
    pub fn process(&mut self, value: u16) -> u16 {
        match self {
            ProcessingSteps::Mean(m) => m.process(value),
            ProcessingSteps::DeltaPass(d) => d.process(value),
            ProcessingSteps::Lerp(l) => l.process(value),
        }
    }
}

pub fn floor(x: f32) -> f32 {
    let xi = x as i32; 
    if x < 0.0 && (x - xi as f32) != 0.0 {
        (xi - 1) as f32
    } else {
        xi as f32
    }
}
