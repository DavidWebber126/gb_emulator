pub struct Bus {
    pub cpu_vram: [u8; 0xffff],
}

impl Bus {
    pub fn new(mut prg_rom: Vec<u8>) -> Self {
        prg_rom.resize(0xffff, 0);
        let prg: [u8; 0xffff] = prg_rom.try_into().unwrap();
        Bus { cpu_vram: prg }
    }

    pub fn mem_read(&self, addr: u16) -> u8 {
        self.cpu_vram[addr as usize]
    }

    pub fn mem_write(&mut self, addr: u16, data: u8) {
        self.cpu_vram[addr as usize] = data;
    }

    pub fn mem_read_u16(&self, addr: u16) -> u16 {
        let lo = self.mem_read(addr);
        let hi = self.mem_read(addr + 1);
        u16::from_le_bytes([lo, hi])
    }

    pub fn mem_write_u16(&mut self, addr: u16, data: u16) {
        let bytes = data.to_le_bytes();
        self.mem_write(addr, bytes[0]);
        self.mem_write(addr + 1, bytes[1]);
    }
}
