use bitflags::bitflags;

use crate::cartridge::Cartridge;
use crate::ppu::Ppu;

bitflags! {
    #[derive(PartialEq, Debug, Clone)]
    pub struct Interrupt: u8 {
        // VBlank Enable
        const vblank = 0b0000_0001;
        // LCD Enable
        const lcd = 0b0000_0010;
        // Timer Enable
        const timer = 0b0000_0100;
        // Serial Enable
        const serial = 0b0000_1000;
        // Joypad Enable
        const joypad = 0b0001_0000;
    }
}

pub struct Bus {
    pub cpu_ram: [u8; 0x2000], // not sure size of cpu ram
    pub hram: [u8; 0x7F],      // CPU high ram 0xFF80 - 0xFFFE
    pub cartridge: Cartridge,
    pub interrupt_enable: Interrupt, // Address 0xFFFF enables interrupts
    pub interrupt_flag: Interrupt,
    pub ppu: Ppu,
}

impl Bus {
    pub fn new(cartridge: Cartridge) -> Self {
        Bus {
            cpu_ram: [0; 0x2000],
            hram: [0; 0x7F],
            cartridge,
            interrupt_enable: Interrupt::empty(),
            interrupt_flag: Interrupt::empty(),
            ppu: Ppu::new(),
        }
    }

    pub fn vblank_enabled(&self) -> bool {
        self.interrupt_enable.contains(Interrupt::vblank)
    }

    pub fn vblank_flag(&self) -> bool {
        self.interrupt_flag.contains(Interrupt::vblank)
    }

    pub fn lcd_enabled(&self) -> bool {
        self.interrupt_enable.contains(Interrupt::lcd)
    }

    pub fn lcd_flag(&self) -> bool {
        self.interrupt_flag.contains(Interrupt::lcd)
    }

    pub fn timer_enabled(&self) -> bool {
        self.interrupt_enable.contains(Interrupt::timer)
    }

    pub fn timer_flag(&self) -> bool {
        self.interrupt_flag.contains(Interrupt::timer)
    }

    pub fn serial_enabled(&self) -> bool {
        self.interrupt_enable.contains(Interrupt::serial)
    }

    pub fn serial_flag(&self) -> bool {
        self.interrupt_flag.contains(Interrupt::serial)
    }

    pub fn joypad_enabled(&self) -> bool {
        self.interrupt_enable.contains(Interrupt::joypad)
    }

    pub fn joypad_flag(&self) -> bool {
        self.interrupt_flag.contains(Interrupt::joypad)
    }

    pub fn mem_read(&mut self, addr: u16) -> u8 {
        match addr {
            // Cartridge ROM bank 0
            0x0000..=0x3FFF => self.cartridge.read_bank0(addr),
            // Cartridge ROM bank 01-NN. May be mapped
            0x4000..=0x7FFF => self.cartridge.read_bankn(addr),
            // VRAM
            0x8000..=0x9FFF => self.ppu.read_vram(addr),
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
            0xFE00..=0xFE9F => self.ppu.oam_read(addr),
            // Not usable
            0xFEA0..=0xFEFF => {
                //panic!("Address {:04X} is in unusable space 0xFEA0 - 0xFEFF", addr)
                // returns 0 on reads
                0
            }
            // IO Registers 0xFF00 - 0xFF7F
            // Joypad Input
            0xFF00 => todo!("Implement Joypad input"),
            // Serial transfer
            0xFF01 | 0xFF02 => todo!("Implement serial transfer"),
            // Timer and divider
            0xFF04..=0xFF07 => todo!("Implement timer and divider"),
            // Interrupts
            0xFF0F => self.interrupt_flag.bits(),
            0xFF40 => self.ppu.read_ctrl(),
            // Rest tbd
            0xFF10..=0xFF7F => {
                todo!()
            }
            // High RAM
            0xFF80..=0xFFFE => {
                let mirrored_addr = addr - 0xff80;
                self.hram[mirrored_addr as usize]
            }
            // Interrupt Enable
            0xFFFF => self.interrupt_enable.bits(),
            _ => panic!("Address {} not used in memory map", addr),
        }
    }

    pub fn mem_write(&mut self, addr: u16, data: u8) {
        match addr {
            // Cartridge ROM bank 0
            0x0000..=0x3FFF => {
                //self.cartridge.write_rom(addr as usize, data);
                //panic!("Cannot write to Cartridge ROM bank 0 (0x0000 - 0x3FFF) with address {:04X} and value {:04X}", addr, data)
            }
            // Cartridge ROM bank 01-NN. May be mapped
            0x4000..=0x7FFF => {
                panic!("Cannot write to Cartridge ROM bank 01-NN (0x4000 - 0x7FFF) with address {:04X} and value {:04X}", addr, data)
            }
            // VRAM
            0x8000..=0x9FFF => {
                self.ppu.write_vram(addr, data);
            }
            // Cartridge RAM (not always present)
            0xA000..=0xBFFF => {
                self.cartridge.ram_write(addr, data);
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
                self.ppu.oam_write(addr, data);
            }
            // Not usable
            0xFEA0..=0xFEFF => {
                // Does nothing on writes
            }
            // IO Registers 0xFF00 - 0xFF7F
            // Joypad Input
            0xFF00 => todo!("Implement Joypad input"),
            // Serial transfer
            0xFF01 | 0xFF02 => {}
            // Timer and divider
            0xFF04..=0xFF07 => todo!("Implement timer and divider"),
            // Sound channel 1 sweep
            0xFF10 => {}
            // Sound channel 1 volume & envelope
            0xFF12 => {}
            // Sound channel 1 period high & control
            0xFF14 => {}
            // Sound channel 2 volume & envelope
            0xFF17 => {}
            // Sound channel 2 period high & control
            0xFF19 => {}
            // Sound channel 3 DAC enable
            0xFF1A => {}
            // Sound channel 4 volume & envelope
            0xFF21 => {}
            // Sound channel 4 control
            0xFF23 => {}
            // Master volume & VIN panning
            0xFF24 => {}
            // Sound panning
            0xFF25 => {}
            // Sound on/off
            0xFF26 => {}
            // Interrupts
            0xFF0F => {
                self.interrupt_flag = Interrupt::from_bits_retain(data);
            }
            // PPU Registers
            // LCD Control
            0xFF40 => self.ppu.write_to_ctrl(data),
            // LCD Status (STAT Register)
            0xFF41 => self.ppu.write_status(data),
            // SCY: Scroll Y value
            0xFF42 => self.ppu.scy = data,
            // SCX: Scroll X value
            0xFF43 => self.ppu.scx = data,
            // LCD Y coordinate is read only
            0xFF44 => panic!(
                "LCD Y coordinate is read-only. Addr: {} Data: {}",
                addr, data
            ),
            // LYC
            0xFF45 => self.ppu.lyc = data,
            // OAM DMA source address and start
            0xFF46 => {
                assert!(data <= 0xDF);
                let start_addr = (data as u16) << 8;
                let mut page: [u8; 0xA0] = [0; 0xA0];
                for (i, byte) in page.iter_mut().enumerate()  {
                    *byte = self.mem_read(start_addr + i as u16);
                }
                self.ppu.oam_dma(page);
            }
            // BGP: BG Palette data
            0xFF47 => self.ppu.bg_palette = data,
            // OBP0: OBJ Palette 0
            0xFF48 => self.ppu.obp0 = data,
            // OBP1: OBJ Palette 1
            0xFF49 => self.ppu.obp1 = data,
            // Window Y position
            0xFF4A => self.ppu.wy = data,
            // Window X position
            0xFF4B => self.ppu.wx = data,
            // BCPS/BGPI: Background color palette specification
            0xFF68 => self.ppu.bcps = data,
            // BCPD/BGPD: Background color palette data
            0xFF69 => self.ppu.bcpd = data,
            0xFF6A | 0xFF6B => todo!(),
            // Unused but doesn't crash run
            0xFF78..=0xFF7F => {}
            // High RAM
            0xFF80..=0xFFFE => {
                let mirrored_addr = addr - 0xff80;
                self.hram[mirrored_addr as usize] = data;
            }
            // Interrupt Enable
            0xFFFF => {
                self.interrupt_enable = Interrupt::from_bits_retain(data);
            }
            _ => panic!("Address {:04X} not used in memory map", addr),
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
