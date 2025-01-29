#[derive(Default)]
pub struct ADCSmoother {
    value: u16,
    current_measure_index: u16,
    max_measurements: u16,
}

impl ADCSmoother {
    pub fn new(averaging_measurements: u16) -> Self {
        Self {
            max_measurements: averaging_measurements,
            ..Default::default()
        }
    }

    pub fn process(&mut self, value: u16) {
        if self.current_measure_index < self.max_measurements {
            self.current_measure_index += 1;
        } else {
            self.current_measure_index = 0;
            self.value = 0;
        }

        self.value += value;
    }

    pub fn output(&self) -> u16 {
        (self.value / self.current_measure_index) as u16
    }
}
