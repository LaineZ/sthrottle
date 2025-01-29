use crate::filters::{floor, ProcessingStep, ProcessingSteps};

pub struct Axis<const N: usize> {
    min: u16,
    max: u16,
    value: u16,
    reversed: bool,
    processing_chain: heapless::Vec<ProcessingSteps, N>
}

impl<'a, const N: usize> Axis<N>  {
    pub fn new(min: u16, max: u16, reversed: bool) -> Self {
        Self { min, max, reversed, processing_chain: heapless::Vec::new(), value: min }
    }

    pub fn add_filter(&mut self, filter: ProcessingSteps) {
        self.processing_chain.push(filter);
    }

    pub fn process(&mut self, value: u16) {
        self.value = value;        
        for filter in self.processing_chain.iter_mut() {
            self.value = filter.process(self.value);
        }
    }

    pub fn output_raw(&self) -> u16 {
        return self.value;
    }

    pub fn output(&self) -> u16 {
        let mut normalized = self.value; 
        
        if self.reversed {
            normalized = self.max - (self.value - self.min);
        }

        normalized.clamp(self.min, self.max)
    }

    pub fn output_ranged(&self, range_min: u16, range_max: u16) -> u16 {
        let scale = (range_max - range_min) as f32 / (self.max - self.min) as f32;
        let result = range_min as f32 + (self.output() - self.min) as f32 * scale;

        floor(result) as u16 
    }
}
