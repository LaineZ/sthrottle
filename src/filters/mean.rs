use super::{ProcessingStep, ProcessingSteps};

pub struct Mean {
    values: [u16; 32],
    current_measure: usize,
}

impl ProcessingStep for Mean {
    fn process(&mut self, value: u16) -> u16 {
        if self.current_measure < self.values.len() - 1 {
            self.current_measure += 1;
        } else {
            self.current_measure = 0;
        }

        self.values[self.current_measure] = value;
        self.output()
    }
}

impl Mean {
    pub fn new() -> ProcessingSteps {
        ProcessingSteps::Mean(Self {
            values: [0; 32],
            current_measure: 0
        })
    }

    pub fn output(&self) -> u16 {
        let mut total: usize = 0;
        for value in self.values {
            total += value as usize;
        }
        (total / self.values.len()) as u16
    }
}
