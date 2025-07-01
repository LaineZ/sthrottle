use cortex_m_semihosting::hprintln;
use embedded_hal::digital::v2::InputPin;
const DEBOUNCE_DELAY: u32 = 20;

pub struct Button<P> {
    pub pin: P,
    old_state: bool,
    /// Ticks elapsed since last press
    last_change: u32,
    current: u32,
}

impl<P: InputPin> Button<P> {
    pub fn new(pin: P) -> Self {
        Self {
            pin,
            old_state: false,
            last_change: 0,
            current: 0,
        }
    }

    pub fn toggle(&mut self) -> bool {
        self.pressed();
        self.old_state
    }

    pub fn pressed(&mut self) -> bool {
        self.current = self.current.wrapping_add(1);
        let state = self.pin.is_low().unwrap_or_default();
        //hprintln!("{} {}", self.current, self.last_change);
        if state != self.old_state {
            if self.current - self.last_change > DEBOUNCE_DELAY {
                self.old_state = state;
                self.last_change = self.current;
                return state;
            }
        }

        false
    }
}
