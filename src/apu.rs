pub struct Apu {
    pub square1: SquareChannel,
    pub square2: SquareChannel,
    pub wave: WaveChannel,
    pub noise: NoiseChannel,
    frame_seq_cycles: usize,
    pub frame: u8,
    output_cycles: usize,
    audio_on: bool,
    sound_panning: u8,
    volume: u8,
}

impl Apu {
    pub fn new() -> Self {
        Self {
            square1: SquareChannel::new(true),
            square2: SquareChannel::new(false),
            wave: WaveChannel::new(),
            noise: NoiseChannel::new(),
            frame_seq_cycles: 0,
            frame: 0,
            output_cycles: 0,
            audio_on: false,
            sound_panning: 0,
            volume: 0,
        }
    }

    pub fn tick(&mut self) -> Option<(f32,f32)> {
        self.square1.tick();
        self.square2.tick();
        self.wave.tick();
        self.wave.tick();
        self.noise.tick();
        self.frame_cycle();
        self.output_cycles += 1;
        if self.output_cycles == 23 {
            self.output_cycles = 0;
            Some(self.output())
        } else {
            None
        }
    }

    pub fn output(&mut self) -> (f32, f32) {
        // left
        let mut s1 = 0.0;
        let mut s2 = 0.0;
        let mut wave = 0.0;
        let mut noise = 0.0;
        if self.square1.dac_on && self.audio_on && (self.sound_panning & 0b0001_0000 > 0) {
            s1 = self.square1.output();
        }
        if self.square2.dac_on && self.audio_on && (self.sound_panning & 0b0010_0000 > 0) {
            s2 = self.square2.output();
        }
        if self.wave.dac_on && self.audio_on && (self.sound_panning & 0b0100_0000 > 0) {
            wave = self.wave.output();
        }
        if self.noise.dac_on && self.audio_on && (self.sound_panning & 0b1000_0000 > 0) {
            noise = self.noise.output();
        }

        let left = (s1 + s2 + noise + wave) / 4.0;

        // right
        let mut s1 = 0.0;
        let mut s2 = 0.0;
        let mut wave = 0.0;
        let mut noise = 0.0;
        if self.square1.dac_on && self.audio_on && (self.sound_panning & 0b0000_0001 > 0) {
            s1 = self.square1.output();
        }
        if self.square2.dac_on && self.audio_on && (self.sound_panning & 0b0000_0010 > 0) {
            s2 = self.square2.output();
        }
        if self.wave.dac_on && self.audio_on && (self.sound_panning & 0b0000_0100 > 0) {
            wave = self.wave.output();
        }
        if self.noise.dac_on && self.audio_on && (self.sound_panning & 0b0000_1000 > 0) {
            noise = self.noise.output();
        }

        let right = (s1 + s2 + noise + wave) / 4.0;

        (left, right)
    }

    // 0xFF24 NR50
    pub fn volume_write(&mut self, val: u8) {
        if self.audio_on {
            self.volume = val;
        }
    }

    pub fn volume_read(&self) -> u8 {
        self.volume
    }

    // 0xFF25 NR51
    pub fn sound_panning_write(&mut self, val: u8) {
        if self.audio_on {
            self.sound_panning = val;
        }
    }

    pub fn sound_panning_read(&self) -> u8 {
        self.sound_panning
    }

    // 0xFF26 NR52
    pub fn master_control_write(&mut self, val: u8) {
        let prev_on = self.audio_on;
        self.audio_on = val & 0b1000_0000 > 0;
        // Powering Off
        if !self.audio_on {
            self.square1.power_down();
            self.square2.power_down();
            self.wave.power_down();
            self.noise.power_down();
            self.sound_panning = 0;
            self.volume = 0;
        }

        // Powering On
        if !prev_on && self.audio_on {
            self.frame = 7;
            self.square1.duty_step = 0;
            self.square2.duty_step = 0;
            self.wave.position = 0;
            self.square1.power_on = true;
            self.square2.power_on = true;
            self.wave.power_on = true;
            self.noise.power_on = true;
        }
    }

    pub fn master_control_read(&self) -> u8 {
        let audio_on = (self.audio_on as u8) << 7;
        let chnl4 = (self.noise.enabled as u8) << 3;
        let chnl3 = (self.wave.enabled as u8) << 2;
        let chnl2 = (self.square2.enabled as u8) << 1;
        let chnl1 = self.square1.enabled as u8;
        (audio_on | chnl4 | chnl3 | chnl2 | chnl1) | 0x70
    }

    fn frame_cycle(&mut self) {
        self.frame_seq_cycles += 1;
        if self.frame_seq_cycles == 2047 {
            self.frame_seq_cycles = 0;
            self.frame += 1;
            self.frame %= 8;

            match self.frame {
                2 | 6 => {
                    self.square1.sweep_tick();

                    self.square1.len_ctr_tick();
                    self.square2.len_ctr_tick();
                    self.wave.len_ctr_tick();
                    self.noise.len_ctr_tick();
                }
                0 | 4 => {
                    self.square1.len_ctr_tick();
                    self.square2.len_ctr_tick();
                    self.wave.len_ctr_tick();
                    self.noise.len_ctr_tick();
                }
                7 => {
                    self.square1.envelope.tick();
                    self.square2.envelope.tick();
                    self.noise.envelope.tick();
                }
                _ => {}
            }

            if self.frame % 2 == 0 {
                self.square1.length_counter.next_frame_no_clock = true;
                self.square2.length_counter.next_frame_no_clock = true;
                self.wave.length_counter.next_frame_no_clock = true;
                self.noise.length_counter.next_frame_no_clock = true;
            } else {
                self.square1.length_counter.next_frame_no_clock = false;
                self.square2.length_counter.next_frame_no_clock = false;
                self.wave.length_counter.next_frame_no_clock = false;
                self.noise.length_counter.next_frame_no_clock = false;
            }
        }
    }
}

struct Envelope {
    init_vol: u8,
    volume: u8,
    // true is add, false is sub
    mode: bool,
    period: u8,
    counter: u8,
}

impl Envelope {
    fn new() -> Self {
        Self {
            init_vol: 0,
            volume: 0,
            mode: true,
            period: 0,
            counter: 0,
        }
    }

    fn set_vol(&mut self, vol: u8) {
        self.init_vol = vol;
        self.volume = vol;
    }

    fn read(&self) -> u8 {
        let vol = self.init_vol << 4;
        let dir = (self.mode as u8) << 3;
        vol + dir + self.period
    }

    fn tick(&mut self) {
        if self.period == 0 {
            return;
        }

        if self.counter != 0 {
            self.counter -= 1;
        }

        if self.counter == 0 {
            self.counter = self.period;

            if self.volume < 0x0f && self.mode {
                self.volume += 1;
            } else if self.volume > 0 && !self.mode {
                self.volume -= 1;
            }
        }
    }
}

struct LengthCounter {
    enabled: bool,
    counter: u16,
    reset_val: u16,
    next_frame_no_clock: bool,
}

impl LengthCounter {
    fn new() -> Self {
        Self {
            enabled: false,
            counter: 0,
            reset_val: 0,
            next_frame_no_clock: false,
        }
    }

    fn set(&mut self, val: u16) {
        self.counter = val;
        self.reset_val = val;
    }

    fn enable(&mut self, enabled: bool) -> bool {
        let previous_status = self.enabled;
        self.enabled = enabled;
        !previous_status && enabled && self.next_frame_no_clock //&& self.counter != 0
    }

    fn tick(&mut self) {
        if self.enabled && self.counter > 0 {
            self.counter -= 1;
        }
    }
}

struct Sweep {
    enabled: bool,
    period: u8,
    shadow_freq: u16,
    direction: bool,
    shift: u8,
    counter: u8,
    neg_calc_made: bool,
}

impl Sweep {
    fn new() -> Self {
        Self {
            enabled: false,
            period: 0,
            shadow_freq: 0,
            direction: true,
            shift: 0,
            counter: 0,
            neg_calc_made: false,
        }
    }

    fn reload_counter(&mut self) {
        if self.period == 0 {
            self.counter = 8;
        } else {
            self.counter = self.period;
        }
    }
}

pub struct SquareChannel {
    power_on: bool,
    enabled: bool,
    pub dac_on: bool,
    sweep: Sweep,
    sweep_enabled: bool, // true for Square 1 and false for Square 2
    wave_pattern: usize,
    duty_step: usize,
    period: u16,
    period_divider: u16,
    envelope: Envelope,
    length_counter: LengthCounter,
}

impl SquareChannel {
    const WAVEFORM: [[u8; 8]; 4] = [
        [0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 1, 1, 1],
        [0, 1, 1, 1, 1, 1, 1, 0],
    ];

    pub fn new(sweep_enabled: bool) -> Self {
        Self {
            power_on: false,
            enabled: false,
            dac_on: false,
            sweep: Sweep::new(),
            sweep_enabled,
            wave_pattern: 0,
            duty_step: 0,
            period: 0,
            period_divider: 0,
            envelope: Envelope::new(),
            length_counter: LengthCounter::new(),
        }
    }

    fn power_down(&mut self) {
        self.sweep_write(0);
        // Length timer is not affected by power down
        // self.length_timer_write(0);
        self.wave_pattern = 0;
        self.envelope_write(0);
        self.period_low_write(0);
        self.control_write(0);
        self.power_on = false;
    }

    fn trigger(&mut self) {
        self.enabled = self.dac_on;
        if self.length_counter.counter == 0
            && self.length_counter.next_frame_no_clock
            && self.length_counter.enabled
        {
            self.length_counter.counter = 63;
        } else if self.length_counter.counter == 0 {
            self.length_counter.counter = 64;
        }
        self.period_divider = self.period;
        self.envelope.counter = self.envelope.period;
        self.envelope.volume = self.envelope.init_vol;
        if self.sweep_enabled {
            self.sweep.neg_calc_made = false;
            self.sweep.shadow_freq = self.period;
            self.sweep.reload_counter();
            self.sweep.enabled = self.sweep.period != 0 || self.sweep.shift != 0;
            if self.sweep.shift != 0 {
                self.sweep_cal();
            }
        }
    }

    fn sweep_cal(&mut self) -> u16 {
        let offset = self.sweep.shadow_freq >> (self.sweep.shift);
        let new_period = if self.sweep.direction {
            self.sweep.shadow_freq.wrapping_add(offset)
        } else {
            self.sweep.neg_calc_made = true;
            self.sweep.shadow_freq.wrapping_sub(offset)
        };

        if new_period > 0x7ff {
            self.enabled = false;
        }

        new_period
    }

    fn sweep_tick(&mut self) {
        if self.sweep.counter > 0 {
            self.sweep.counter -= 1;
        }

        if self.sweep.counter == 0 {
            self.sweep.reload_counter();

            if self.sweep.enabled && self.sweep.period > 0 {
                let new_period = self.sweep_cal();

                if new_period <= 0x7ff && self.sweep.shift > 0 {
                    self.period = new_period;
                    self.sweep.shadow_freq = new_period;

                    self.sweep_cal();
                }
            }
        }
    }

    fn len_ctr_tick(&mut self) {
        self.length_counter.tick();
        if self.length_counter.counter == 0 {
            self.enabled = false;
        }
    }

    // 0xFF10 NR10
    pub fn sweep_write(&mut self, val: u8) {
        if !self.power_on {
            return;
        }

        self.sweep.period = (val & 0b0111_0000) >> 4;
        self.sweep.direction = val & 0b0000_1000 == 0;
        if self.sweep.neg_calc_made && self.sweep.direction {
            self.sweep.neg_calc_made = false;
            self.enabled = false;
        }
        self.sweep.shift = val & 0b0000_0111;
    }

    pub fn sweep_read(&self) -> u8 {
        let period = self.sweep.period << 4;
        let dir = (!self.sweep.direction as u8) << 3;
        (period + dir + self.sweep.shift) | 0x80
    }

    // 0xFF11 NR11
    pub fn length_timer_write(&mut self, val: u8) {
        if self.power_on {
            self.wave_pattern = ((val & 0b1100_0000) >> 6) as usize;
        }
        self.length_counter.set(64 - (val as u16 & 0b0011_1111));
    }

    pub fn length_timer_read(&self) -> u8 {
        let wave = (self.wave_pattern as u8) << 6;
        wave | 0x3f
    }

    // 0xFF12 NR12
    pub fn envelope_write(&mut self, val: u8) {
        if !self.power_on {
            return;
        }

        self.envelope.set_vol((val & 0b1111_0000) >> 4);
        self.envelope.mode = val & 0b0000_1000 > 0;
        self.envelope.period = val & 0b0000_0111;

        self.dac_on = val & 0xf8 > 0;
        if !self.dac_on {
            self.enabled = false;
        }
    }

    pub fn envelope_read(&self) -> u8 {
        self.envelope.read()
    }

    // 0xFF13 NR13
    pub fn period_low_write(&mut self, val: u8) {
        if !self.power_on {
            return;
        }

        // Set period's lower 8 bits to val
        self.period = (self.period & 0x0700) + val as u16;
    }

    pub fn period_low_read(&self) -> u8 {
        0xff
    }

    // 0xFF14 NR14
    pub fn control_write(&mut self, val: u8) {
        if !self.power_on {
            return;
        }

        self.period = (self.period & 0xff) + ((val as u16 & 0x07) << 8);

        if self.length_counter.enable(val & 0b0100_0000 > 0) {
            self.len_ctr_tick();
        }

        if val & 0b1000_0000 > 0 {
            self.trigger();
        }
    }

    pub fn control_read(&self) -> u8 {
        ((self.length_counter.enabled as u8) << 6) | 0xbf
    }

    fn tick(&mut self) {
        if self.period_divider != 0 {
            self.period_divider -= 1;
        }

        if self.period_divider == 0 {
            self.period_divider = 0x800 - self.period;
            self.duty_step += 1;
            self.duty_step %= 8;
        }
    }

    fn output(&self) -> f32 {
        let dac_input = if self.enabled {
            self.envelope.volume * SquareChannel::WAVEFORM[self.wave_pattern][self.duty_step]
        } else {
            0
        };
        1.0 - (dac_input as f32 / 7.5)
    }
}

pub struct WaveChannel {
    power_on: bool,
    enabled: bool,
    dac_on: bool,
    length_counter: LengthCounter,
    volume: u8,
    output_level: u8,
    period: u16,
    period_divider: u16,
    wave_ram: [u8; 16],
    sample: u8,
    position: usize,
}

impl WaveChannel {
    pub fn new() -> Self {
        Self {
            power_on: false,
            enabled: false,
            dac_on: false,
            length_counter: LengthCounter::new(),
            volume: 0,
            output_level: 0,
            period: 0,
            period_divider: 0,
            wave_ram: [
                0x84, 0x40, 0x43, 0xAA, 0x2D, 0x78, 0x92, 0x3C, 0x60, 0x59, 0x59, 0xB0, 0x34, 0xB8,
                0x2E, 0xDA,
            ],
            sample: 0,
            position: 0,
        }
    }

    fn len_ctr_tick(&mut self) {
        self.length_counter.tick();
        if self.length_counter.counter == 0 {
            self.enabled = false;
        }
    }

    fn trigger(&mut self) {
        self.enabled = self.dac_on;
        if self.length_counter.counter == 0
            && self.length_counter.next_frame_no_clock
            && self.length_counter.enabled
        {
            self.length_counter.counter = 255;
        } else if self.length_counter.counter == 0 {
            self.length_counter.counter = 256;
        }
        self.volume = self.output_level;
        self.period_divider = self.period;
        self.position = 0;
    }

    // 0xFF1A NR30
    pub fn dac_enable_write(&mut self, val: u8) {
        if !self.power_on {
            return;
        }

        self.dac_on = val & 0b1000_0000 > 0;
        if !self.dac_on {
            self.enabled = false;
        }
    }

    pub fn dac_enable_read(&self) -> u8 {
        ((self.dac_on as u8) << 7) | 0x7f
    }

    // 0xFF1B NR31
    pub fn length_timer(&mut self, val: u8) {
        self.length_counter.set(256 - val as u16);
    }

    // 0xFF1C NR32
    pub fn output_level_write(&mut self, val: u8) {
        if !self.power_on {
            return;
        }

        self.output_level = (val & 0b0110_0000) >> 5;
    }

    pub fn output_level_read(&self) -> u8 {
        (self.output_level << 5) | 0x9f
    }

    // 0xFF1D NR33
    pub fn period_low_write(&mut self, val: u8) {
        if !self.power_on {
            return;
        }

        self.period = (self.period & 0x700) + val as u16;
    }

    pub fn period_low_read(&self) -> u8 {
        0xff
    }

    // 0xFF1E NR34
    pub fn control_write(&mut self, val: u8) {
        if !self.power_on {
            return;
        }

        self.period = (self.period & 0xff) + ((val as u16 & 0x07) << 8);

        if self.length_counter.enable(val & 0b0100_0000 > 0) {
            self.len_ctr_tick();
        }

        if val & 0b1000_0000 > 0 {
            self.trigger()
        }
    }

    pub fn control_read(&self) -> u8 {
        ((self.length_counter.enabled as u8) << 6) | 0xbf
    }

    // 0xFF30 - 0xFF3F Wave RAM
    pub fn wave_ram_write(&mut self, addr: u16, val: u8) {
        if self.enabled {
            return;
        }

        //eprintln!("Here");
        let offset = (addr - 0xff30) as usize;
        //self.wave_ram[offset] = val;
        self.wave_ram[offset] = val;
    }

    pub fn wave_ram_read(&mut self, addr: u16) -> u8 {
        if !self.enabled {
            let offset = (addr - 0xff30) as usize;
            self.wave_ram[offset]
        } else {
            self.sample
        }
    }

    fn tick(&mut self) {
        if self.period_divider != 0 {
            self.period_divider -= 1;
        }

        if self.period_divider == 0 {
            self.period_divider = 0x800 - self.period;
            self.position += 1;
            self.position %= 32;
            self.sample = self.wave_ram[self.position / 2];
        }
    }

    fn power_down(&mut self) {
        self.dac_enable_write(0);
        self.output_level_write(0);
        self.period_low_write(0);
        self.control_write(0);
        self.power_on = false;
    }

    fn output(&self) -> f32 {
        let sample = if self.position % 2 == 0 {
            (self.sample & 0xf0) >> 4
        } else {
            self.sample & 0x0f
        };

        let mut dac_input = match self.volume {
            0 => 0,
            1 => sample,
            2 => sample >> 1,
            3 => sample >> 2,
            _ => panic!("APU Wave Channel's Output Level cannot be anything other than 0-3"),
        };

        if !self.enabled {
            dac_input = 0;
        }

        1.0 - (dac_input as f32 / 7.5)
    }
}

pub struct NoiseChannel {
    power_on: bool,
    enabled: bool,
    dac_on: bool,
    length_counter: LengthCounter,
    envelope: Envelope,
    clock_shift: u8,
    lfsr_width: bool,
    lfsr: u16,
    clock_divider: u8,
    timer: usize,
}

impl NoiseChannel {
    pub fn new() -> Self {
        Self {
            power_on: false,
            enabled: false,
            dac_on: false,
            length_counter: LengthCounter::new(),
            envelope: Envelope::new(),
            clock_shift: 0,
            lfsr_width: false,
            lfsr: 0,
            clock_divider: 0,
            timer: 0,
        }
    }

    fn len_ctr_tick(&mut self) {
        self.length_counter.tick();
        if self.length_counter.counter == 0 {
            self.enabled = false;
        }
    }

    fn power_down(&mut self) {
        //self.length_timer(0);
        self.envelope_write(0);
        self.randomness_write(0);
        self.control_write(0);
        self.power_on = false;
    }

    fn trigger(&mut self) {
        self.enabled = self.dac_on;
        if self.length_counter.counter == 0
            && self.length_counter.next_frame_no_clock
            && self.length_counter.enabled
        {
            self.length_counter.counter = 63;
        } else if self.length_counter.counter == 0 {
            self.length_counter.counter = 64;
        }
        self.envelope.counter = self.envelope.period;
        self.envelope.volume = self.envelope.init_vol;
        self.lfsr = 0x7ff;
    }

    fn tick(&mut self) {
        if self.timer != 0 {
            self.timer -= 1;
        }

        if self.timer == 0 {
            self.timer = (self.clock_divider as usize) << self.clock_shift;
            let xor_result = (self.lfsr & 0b1) ^ ((self.lfsr & 0b10) >> 1);
            self.lfsr = (self.lfsr >> 1) | (xor_result << 14);

            if self.lfsr_width {
                self.lfsr &= 0xffbf;
                self.lfsr |= xor_result << 6;
            }
        }
    }

    fn output(&self) -> f32 {
        let dac_input = if self.enabled {
            self.envelope.volume * ((!self.lfsr as u8) & 0b1)
        } else {
            0
        };
        1.0 - (dac_input as f32 / 7.5)
    }

    // 0xFF20 NR41
    pub fn length_timer(&mut self, val: u8) {
        self.length_counter.set(64 - (val & 0b0011_1111) as u16);
    }

    // 0xFF21 NR42
    pub fn envelope_write(&mut self, val: u8) {
        if !self.power_on {
            return;
        }

        self.envelope.set_vol((val & 0b1111_0000) >> 4);
        self.envelope.mode = val & 0b0000_1000 > 0;
        self.envelope.period = val & 0b0000_0111;

        self.dac_on = val & 0xf8 > 0;
        if !self.dac_on {
            self.enabled = false;
        }
    }

    pub fn envelope_read(&self) -> u8 {
        self.envelope.read()
    }

    // 0xFF22 NR43
    pub fn randomness_write(&mut self, val: u8) {
        if !self.power_on {
            return;
        }

        self.clock_shift = (val & 0xf0) >> 4;
        self.lfsr_width = val & 0b0000_1000 > 0;
        // self.clock_divider = match val & 0b0000_0111 {
        //     0 => 8,
        //     1 => 16,
        //     2 => 32,
        //     3 => 48,
        //     4 => 64,
        //     5 => 80,
        //     6 => 96,
        //     7 => 112,
        //     _ => panic!(),
        // };
        let div_code = val & 0b0000_0111;
        self.clock_divider = if div_code == 0 { 8 } else { 16 * div_code };
    }

    pub fn randomness_read(&self) -> u8 {
        let clock_shift = self.clock_shift << 4;
        let lfsr_width = (self.lfsr_width as u8) << 3;
        let code = self.clock_divider / 16;
        clock_shift + lfsr_width + code
    }

    // 0xFF23 NR44
    pub fn control_write(&mut self, val: u8) {
        if !self.power_on {
            return;
        }

        if self.length_counter.enable(val & 0b0100_0000 > 0) {
            self.len_ctr_tick();
        }

        if val & 0b1000_0000 > 0 {
            self.trigger();
        }
    }

    pub fn control_read(&self) -> u8 {
        ((self.length_counter.enabled as u8) << 6) | 0xbf
    }
}
