pub struct Apu {
    pub square1: SquareChannel,
    pub square2: SquareChannel,
    pub wave: WaveChannel,
    pub noise: NoiseChannel,
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
            audio_on: false,
            sound_panning: 0,
            volume: 0,
        }
    }

    // 0xFF26 NR52
    pub fn master_control_write(&mut self, val: u8) {
        self.audio_on = val & 0b1000_0000 > 0;
    }

    pub fn master_control_read(&self) -> u8 {
        let audio_on = (self.audio_on as u8) << 7;
        let chnl4 = (self.noise.enabled as u8) << 3;
        let chnl3 = (self.wave.enabled as u8) << 2;
        let chnl2 = (self.square2.enabled as u8) << 1;
        let chnl1 = self.square1.enabled as u8;
        audio_on | chnl4 | chnl3 | chnl2 | chnl1
    }

    // 0xFF25 NR51
    pub fn sound_panning_write(&mut self, val: u8) {
        self.sound_panning = val;
    }

    pub fn sound_panning_read(&self) -> u8 {
        self.sound_panning
    }

    // 0xFF24 NR50
    pub fn volume_write(&mut self, val: u8) {
        self.volume = val;
    }

    pub fn volume_read(&self) -> u8 {
        self.volume
    }
}

pub struct SquareChannel {
    enabled: bool,
    sweep_enabled: bool,
    sweep: u8,
    wave_pattern: usize,
    wave_index: usize,
    length_timer: u8,
    envelope: u8,
    period: u16,
    period_divider: u16,
    control: u8,
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
            enabled: false,
            sweep_enabled,
            sweep: 0,
            wave_pattern: 0,
            wave_index: 0,
            length_timer: 0,
            envelope: 0,
            period: 0,
            period_divider: 0,
            control: 0,
        }
    }

    // 0xFF10 NR10
    pub fn sweep_write(&mut self, val: u8) {
        self.sweep = val & 0b0111_1111;
    }

    pub fn sweep_read(&self) -> u8 {
        self.sweep
    }

    // 0xFF11 NR11
    pub fn length_timer_write(&mut self, val: u8) {
        self.length_timer = val;
    }

    pub fn length_timer_read(&self) -> u8 {
        self.length_timer
    }

    // 0xFF12 NR12
    pub fn envelope_write(&mut self, val: u8) {
        self.envelope = val;
    }

    pub fn envelope_read(&self) -> u8 {
        self.envelope
    }

    // 0xFF13 NR13
    pub fn period_low_write(&mut self, val: u8) {
        // Set period's lower 8 bits to val
        self.period = (self.period & 0x0700) + val as u16;
    }

    pub fn period_low_read(&self) -> u8 {
        self.period as u8
    }

    // 0xFF14 NR14
    pub fn control_write(&mut self, val: u8) {
        self.period = (self.period & 0xff) + (val as u16 & 0x07) << 8;
        self.enabled = val & 0b1000_0000 > 0;
        self.control = val;
    }

    pub fn control_read(&self) -> u8 {
        self.control
    }

    fn tick(&mut self) {
        self.period_divider = self.period_divider.wrapping_add(1);
        if self.period_divider == 0 {
            self.period_divider = (!self.period).wrapping_add(1) & 0x7ff;
            self.wave_index += 1;
            self.wave_index %= 8;
        }
    }

    fn output(&self) -> u8 {
        SquareChannel::WAVEFORM[self.wave_pattern][self.wave_index]
    }
}

pub struct WaveChannel {
    enabled: bool,
}

impl WaveChannel {
    pub fn new() -> Self {
        Self { enabled: false }
    }
}

pub struct NoiseChannel {
    enabled: bool,
}

impl NoiseChannel {
    pub fn new() -> Self {
        Self { enabled: false }
    }
}
