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
        const compare_result = 0b0000_0100;
        // PPU Mode
        const ppu_mode = 0b0000_0011;
    }
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
}

impl Ppu {
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
        self.status.bits()
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

    pub fn tick(&mut self, cycles: usize) {
        
    }
}
