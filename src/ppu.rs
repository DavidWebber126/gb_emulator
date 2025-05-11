use bitflags::bitflags;

// 0xFF40
bitflags! {
    #[derive(PartialEq, Debug, Clone)]
    pub struct Control: u8 {
        // LCD & PPU Enable
        const lcd_enable = 0b1000_0000;
        // Window Tile Map Area 0 = 0x9800 - 0x9BFF; 1 = 0x9C00 - 9FFF
        const window_map_area = 0b0100_0000;
        // Window Enable
        const window_enable = 0b0010_0000;
        // BG & Window tile data area
        const bg_win_mode = 0b0001_0000;
        // BG Tile map area
        const bg_tile_area = 0b0000_1000;
        // OBJ Size
        const obj_size = 0b0000_0100;
        // OBJ Enable
        const obj_enable = 0b0000_0010;
        // BG & Window Enable / Priority
        const bg_win_enable = 0b0000_0001;
    }
}

// 0xFF41
bitflags! {
    #[derive(PartialEq, Debug, Clone)]
    pub struct Status: u8 {
        // LYC Int Select
        const lyc_select = 0b0100_0000;
        // Mode 2 Int Select
        const mode_two_select = 0b0010_0000;
        // Mode 1 Int Select
        const mode_one_select = 0b0001_0000;
        // Mode 0 Int Select
        const mode_zero_select = 0b0000_1000;
        // LYC == LY
        const compare = 0b0000_0100;
        // PPU Mode last two bits. Not modeled here
    }
}

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    MODE2, // oam scan
    MODE3, // render pixels
    MODE0, // hblank
    MODE1, // vblank
}

// Tell Bus what should be rendered or done
#[derive(Debug)]
pub enum DisplayStatus {
    DoNothing,
    OAMScan,
    NewScanline, // Changed from
    NewFrame,
}

pub struct Ppu {
    pub vram: [u8; 0x2000],
    pub oam: [u8; 0xA0],

    pub control: Control,
    pub status: Status,
    pub lyc: u8,
    pub scy: u8,
    pub scx: u8,
    pub wy: u8,
    pub wx: u8,
    pub bg_palette: u8,
    pub obp0: u8,
    pub obp1: u8,
    pub bcps: u8,
    pub bcpd: u8,
    pub cycle: usize,
    pub scanline: u8,
    mode: Mode,
    pub scanline_oams: Vec<usize>, // hold the up to 10 OAMs on current scanline. Referenced by first byte in four byte sequence
}

impl Ppu {
    const MODE2_END: usize = 80;
    const MODE3_START: usize = 81;
    const MODE3_END: usize = 172 + Ppu::MODE2_END;
    const MODE0_START: usize = Ppu::MODE3_END + 1;
    const MODE0_END: usize = 456;
    const SCANLINE_LENGTH: usize = 456;
    const MAX_SCANLINE: u8 = 153;
    const MODE1_START: u8 = 144;

    pub fn new() -> Self {
        Self {
            vram: [0; 0x2000],
            oam: [0; 0xA0],
            control: Control::from_bits_retain(0),
            status: Status::from_bits_retain(0),
            lyc: 0,
            scy: 0,
            scx: 0,
            wy: 0,
            wx: 0,
            bg_palette: 0,
            obp0: 0,
            obp1: 0,
            bcps: 0,
            bcpd: 0,
            mode: Mode::MODE2,
            scanline_oams: Vec::with_capacity(10),

            cycle: 0,
            scanline: 0,
        }
    }

    pub fn write_to_ctrl(&mut self, val: u8) {
        self.control = Control::from_bits_retain(val);
    }

    pub fn read_ctrl(&self) -> u8 {
        self.control.bits()
    }

    pub fn write_status(&mut self, val: u8) {
        self.status = Status::from_bits_retain(val);
    }

    pub fn read_status(&self) -> u8 {
        let mut mode = match self.mode {
            Mode::MODE0 => 0,
            Mode::MODE1 => 1,
            Mode::MODE2 => 2,
            Mode::MODE3 => 3,
        };
        if !self.control.contains(Control::lcd_enable) {
            mode = 0
        }
        self.status.bits() + mode
    }

    pub fn read_vram(&self, addr: u16) -> u8 {
        let mirrored_addr = addr - 0x8000;
        assert!(mirrored_addr < 0x2000);
        self.vram[mirrored_addr as usize]
    }

    pub fn write_vram(&mut self, addr: u16, val: u8) {
        let mirrored_addr = addr - 0x8000;
        assert!(mirrored_addr < 0x2000);
        self.vram[mirrored_addr as usize] = val;
    }

    pub fn oam_read(&self, addr: u16) -> u8 {
        let mirrored_addr = addr - 0xFE00;
        assert!(mirrored_addr < 0xA0);
        self.oam[mirrored_addr as usize]
    }

    pub fn oam_write(&mut self, addr: u16, val: u8) {
        let mirrored_addr = addr - 0xFE00;
        assert!(mirrored_addr < 0xA0);
        self.oam[mirrored_addr as usize] = val;
    }

    pub fn oam_dma(&mut self, page: [u8; 0xA0]) {
        self.oam = page;
    }

    // Called once Ppu has entered Mode 2. Scan objects that are on current scanline and put into scanline_oams
    pub fn oam_scan(&mut self) {
        self.scanline_oams.clear();
        for i in 0..40 {
            let y_byte = self.oam[4 * i];
            let in_scanline = self.scanline + 16 >= y_byte
                && self.scanline + 8 * (!self.control.contains(Control::obj_size) as u8) < y_byte;
            if in_scanline && self.scanline_oams.len() < 10 {
                self.scanline_oams.push(i)
            }
        }
    }

    // 456 cycles per scanline. 154 scanlines, last 10 (144-153 inclusive) are vblank
    // First bool is LCD interrupt, second is vblank interrupt
    pub fn tick(&mut self, cycles: u8) -> (DisplayStatus, bool, bool) {
        self.cycle += cycles as usize;
        let prior_mode = self.mode;
        let mut result: (DisplayStatus, bool, bool) = (DisplayStatus::DoNothing, false, false);
        if self.cycle > Ppu::SCANLINE_LENGTH {
            self.cycle %= Ppu::SCANLINE_LENGTH;
            self.scanline += 1;

            // After vblank, reset to scanline 0
            if self.scanline > Ppu::MAX_SCANLINE {
                self.scanline = 0;
                self.mode = Mode::MODE2;
            }

            // vblank has started
            if self.scanline == Ppu::MODE1_START {
                self.mode = Mode::MODE1;
                result.2 = true;
                if self.status.contains(Status::mode_one_select) {
                    // Trigger LCD Interrupt through return
                    result.1 = true;
                }
            }

            if self.scanline == self.lyc {
                self.status.insert(Status::compare);
                // Trigger LCD Interrupt through return
                if self.status.contains(Status::lyc_select) {
                    result.1 = true;
                }
            }
        }

        if self.mode != Mode::MODE1 {
            match self.cycle {
                0..=Ppu::MODE2_END => {
                    self.mode = Mode::MODE2;
                }
                Ppu::MODE3_START..=Ppu::MODE3_END => {
                    self.mode = Mode::MODE3;
                }
                Ppu::MODE0_START..=Ppu::MODE0_END => {
                    self.mode = Mode::MODE0;
                }
                _ => {
                    self.cycle %= Ppu::MODE0_END;
                }
            }
        }
        // If mode changed then trigger mode interrupt (if Stat for that mode is set)
        if prior_mode != self.mode {
            if self.mode == Mode::MODE0 {
                // Entered HBlank. Do nothing
                result.0 = DisplayStatus::DoNothing;
                if self.status.contains(Status::mode_zero_select) {
                    // Trigger LCD Interrupt through return
                    result.1 = true;
                }
            }
            if self.mode == Mode::MODE1 {
                // Entered VBlank. Display new frame
                result.0 = DisplayStatus::NewFrame;
                if self.status.contains(Status::mode_one_select) {
                    // Trigger LCD Interrupt through return
                    result.1 = true;
                }
            }
            if self.mode == Mode::MODE2 {
                // Entered Mode 2. Do OAM Scan
                result.0 = DisplayStatus::OAMScan;
                if self.status.contains(Status::mode_two_select) {
                    // Trigger LCD Interrupt through return
                    result.1 = true;
                }
            }
            if self.mode == Mode::MODE3 {
                // Entered drawing stage. Draw new scanline
                result.0 = DisplayStatus::NewScanline;
            }

            // Update PPU mode in status. Need to use bits since PPU mode is 2 bits wide
            let mut new_mode = match self.mode {
                Mode::MODE0 => 0,
                Mode::MODE1 => 1,
                Mode::MODE2 => 2,
                Mode::MODE3 => 3,
            };
            // If PPU/LCD is off, set PPU mode to 0
            if !self.control.contains(Control::lcd_enable) {
                new_mode = 0;
            }
            // Set only bottom 2 bits
            self.status = Status::from_bits_retain((self.status.bits() & 0b1111_1100) | new_mode);
        }
        result
    }
}
