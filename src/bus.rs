use core::panic;

pub struct Bus {
    pub cpu_ram: [u8; 0x2000], // not sure size of cpu ram
    pub rom: Vec<u8>,
}

impl Bus {
    pub fn new(prg_rom: Vec<u8>) -> Self {
        Bus {
            cpu_ram: [0; 0x2000],
            rom: prg_rom,
        }
    }

    fn bank_read(&mut self, addr: u16) -> u8 {
        todo!()
    }

    pub fn mem_read(&mut self, addr: u16) -> u8 {
        match addr {
            // Cartridge ROM bank 0
            0x0000..=0x3FFF => self.rom[addr as usize],
            // Cartridge ROM bank 01-NN. May be mapped
            0x4000..=0x7FFF => self.bank_read(addr),
            // VRAM
            0x8000..=0x9FFF => {
                todo!()
            }
            // Cartridge RAM (not always present)
            0xA000..=0xBFFF => {
                todo!()
            }
            // CPU RAM
            0xC000..=0xDFFF => {
                let mirrored_addr = addr % 0x2000;
                assert!(mirrored_addr <= 0x2000);
                self.cpu_ram[mirrored_addr as usize]
            }
            // Echo RAM (Mirrors CPU Ram) - Shouldn't be used
            0xE000..=0xFDFF => {
                panic!(
                    "Echo RAM address used (Should not be used). Address: {:04X}",
                    addr
                )
            }
            // OAM RAM
            0xFE00..=0xFE9F => {
                todo!()
            }
            // Not usable
            0xFEA0..=0xFEFF => {
                //panic!("Address {:04X} is in unusable space 0xFEA0 - 0xFEFF", addr)
                // returns 0 on reads
                0
            }
            // IO Registers 0xFF00 - 0xFF7F
            0xFF00..=0xFF7F => {
                todo!()
            }
            // High RAM
            0xFF80..=0xFFFE => {
                todo!()
            }
            // Interrupt Enable
            0xFFFF => {
                todo!()
            }
        }
    }

    pub fn mem_write(&mut self, addr: u16, data: u8) {
        match addr {
            // Cartridge ROM bank 0
            0x0000..=0x3FFF => {
                panic!("Cannot write to Cartridge ROM bank 0 (0x0000 - 0x3FFF) with address {:04X} and value {:04X}", addr, data)
            }
            // Cartridge ROM bank 01-NN. May be mapped
            0x4000..=0x7FFF => {
                panic!("Cannot write to Cartridge ROM bank 01-NN (0x4000 - 0x7FFF) with address {:04X} and value {:04X}", addr, data)
            }
            // VRAM
            0x8000..=0x9FFF => {
                todo!()
            }
            // Cartridge RAM (not always present)
            0xA000..=0xBFFF => {
                todo!()
            }
            // CPU RAM
            0xC000..=0xDFFF => {
                let mirrored_addr = addr % 0x2000;
                assert!(mirrored_addr <= 0x2000);
                self.cpu_ram[mirrored_addr as usize] = data;
            }
            // Echo RAM (Mirrors CPU Ram) - Shouldn't be used
            0xE000..=0xFDFF => {
                panic!(
                    "Echo RAM address used (Should not be used). Address: {:04X}",
                    addr
                )
            }
            // OAM RAM
            0xFE00..=0xFE9F => {
                todo!()
            }
            // Not usable
            0xFEA0..=0xFEFF => {
                // Does nothing on writes
            }
            // IO Registers 0xFF00 - 0xFF7F
            0xFF00..=0xFF7F => {
                todo!()
            }
            // High RAM
            0xFF80..=0xFFFE => {
                todo!()
            }
            // Interrupt Enable
            0xFFFF => {
                todo!()
            }
        }
    }

    pub fn mem_read_u16(&mut self, addr: u16) -> u16 {
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
