pub struct Timer {
    pub divider_counter: u8, // DIV
    divider_cycle: u8,
    pub timer_counter: u8, // TIMA
    timer_cycle: usize,
    pub timer_modulo: u8, // TMA
    pub tac_enable: bool, // TAC - enable
    pub tac_clock: usize, // TAC - clock select
}

impl Timer {
    const TIMER_CYCLES: [usize; 4] = [256, 4, 16, 64];

    pub fn new() -> Self {
        Self {
            divider_counter: 0,
            divider_cycle: 0,
            timer_counter: 0,
            timer_cycle: 0,
            timer_modulo: 0,
            tac_enable: false,
            tac_clock: 0,
        }
    }

    // FF04 DIV
    pub fn div_write(&mut self) {
        self.divider_counter = 0;
    }

    // FF05 TIMA
    pub fn tima_write(&mut self, val: u8) {
        self.timer_counter = val;
    }

    // FF06 TMA
    pub fn tma_write(&mut self, val: u8) {
        self.timer_modulo = val;
    }

    // FF07 TAC
    pub fn tac_write(&mut self, val: u8) {
        self.tac_enable = val & 0b0000_0100 > 0;
        self.tac_clock = (val & 0b0000_0011) as usize;
    }

    pub fn tac_read(&self) -> u8 {
        let tac_enable = (self.tac_enable as u8) << 2;
        tac_enable + self.tac_clock as u8
    }

    fn divider_tick(&mut self, cycles: u8) {
        self.divider_cycle += cycles;
        if self.divider_cycle as usize >= Timer::TIMER_CYCLES[3] {
            self.divider_counter = self.divider_counter.wrapping_add(1);
            self.divider_cycle -= Timer::TIMER_CYCLES[3] as u8;
        }
    }

    fn timer_tick(&mut self, cycles: u8) -> bool {
        if self.tac_enable {
            self.timer_cycle += cycles as usize;
        }
        while self.tac_enable && self.timer_cycle >= Timer::TIMER_CYCLES[self.tac_clock] {
            let (val, carry) = self.timer_counter.overflowing_add(1);
            self.timer_cycle -= Timer::TIMER_CYCLES[self.tac_clock];
            if carry {
                self.timer_counter = self.timer_modulo;
                return true;
            } else {
                self.timer_counter = val;
            }
        }
        false
    }

    pub fn tick(&mut self, cycles: u8) -> bool {
        // Divider
        self.divider_tick(cycles);

        // Timer Counter. Returns true if a timer interrupt
        self.timer_tick(cycles)
    }
}
