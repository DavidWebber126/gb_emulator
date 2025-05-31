use bitflags::bitflags;

use crate::apu::Apu;
use crate::cartridge::Mapper;
use crate::joypad::Joypad;
use crate::ppu::{DisplayStatus, Ppu};
use crate::render::{self, Frame};
use crate::timer::Timer;

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
    pub cartridge: Box<dyn Mapper>,
    pub joypad: Joypad,
    pub timer: Timer,
    pub interrupt_enable: Interrupt, // Address 0xFFFF enables interrupts
    pub interrupt_flag: Interrupt,
    pub ppu: Ppu,
    pub frame: Frame,
    pub apu: Apu,
    pub audio_buffer: Vec<f32>,
}

impl Bus {
    pub fn new(cartridge: Box<dyn Mapper>) -> Self {
        Bus {
            cpu_ram: [0; 0x2000],
            hram: [0; 0x7F],
            cartridge,
            joypad: Joypad::new(),
            timer: Timer::new(),
            interrupt_enable: Interrupt::empty(),
            interrupt_flag: Interrupt::empty(),
            ppu: Ppu::new(),
            frame: Frame::new(),
            apu: Apu::new(),
            audio_buffer: Vec::with_capacity(1024),
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

    pub fn tick(&mut self, cycles: u8) -> bool {
        // Timer
        let timer_interrupt = self.timer.tick(cycles);
        if timer_interrupt {
            self.interrupt_flag.insert(Interrupt::timer);
        }

        // PPU
        let (display_result, lcd_interrupt, vblank_interrupt) = self.ppu.tick(cycles);
        if lcd_interrupt {
            self.interrupt_flag.insert(Interrupt::lcd);
        }
        if vblank_interrupt {
            self.interrupt_flag.insert(Interrupt::vblank);
        }

        // Joypad (check for interrupt)
        if self.joypad.interrupt {
            self.joypad.interrupt = false;
            self.interrupt_flag.insert(Interrupt::joypad);
        }

        // APU
        for _ in 0..cycles {
            if let Some(amp) = self.apu.tick() {
                self.audio_buffer.push(amp);
            }
        }

        match display_result {
            DisplayStatus::DoNothing => false,
            DisplayStatus::OAMScan => {
                // Mode 2 started
                false
            }
            DisplayStatus::NewScanline => {
                self.ppu.oam_scan();
                render::render_scanline(&mut self.ppu, &mut self.frame); // Mode 3 started
                false
            }
            DisplayStatus::NewFrame => {
                // Mode 1 started (vblank)
                true
            }
        }
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
            0xA000..=0xBFFF => self.cartridge.ram_read(addr),
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
            0xFF00 => self.joypad.read(),
            // Serial transfer
            0xFF01 | 0xFF02 => todo!("Implement serial transfer"),
            0xFF04 => self.timer.divider_counter,
            0xFF05 => self.timer.timer_counter,
            0xFF06 => self.timer.timer_modulo,
            0xFF07 => self.timer.tac_read(),
            // APU
            // Channel 1 Sweep
            0xFF10 => self.apu.square1.sweep_read(),
            // Channel 1 length timer & duty cycle
            0xFF11 => self.apu.square1.length_timer_read(),
            // Channel 1 volume & envelope
            0xFF12 => self.apu.square1.envelope_read(),
            // Channel 1 period low
            0xFF13 => self.apu.square1.period_low_read(),
            // Channel 1 period high & control
            0xFF14 => self.apu.square1.control_read(),
            // Sound channel 2 length timer & duty cycle
            // Not used
            0xFF15 => 0xff,
            0xFF16 => self.apu.square2.length_timer_read(),
            // Sound channel 2 volume & envelope
            0xFF17 => self.apu.square2.envelope_read(),
            // Sound channel 2 period low
            0xFF18 => self.apu.square2.period_low_read(),
            // Sound channel 2 period high & control
            0xFF19 => self.apu.square2.control_read(),
            // Sound channel 3 DAC enable
            0xFF1A => self.apu.wave.dac_enable_read(),
            // Sound channel 3 length timer (Read only)
            0xFF1B => 0xff,
            // Sound channel 3 output level
            0xFF1C => self.apu.wave.output_level_read(),
            // Sound channel 3 period low
            0xFF1D => self.apu.wave.period_low_read(),
            // Sound channel 3 period high & control
            0xFF1E => self.apu.wave.control_read(),
            // Not used
            0xFF1F => 0xff,
            // Sound channel 4 length timer (Write only)
            0xFF20 => 0xff,
            // Sound channel 4 volume & envelope
            0xFF21 => self.apu.noise.envelope_read(),
            // Sound channel 4 frequency & randomness
            0xFF22 => self.apu.noise.randomness_read(),
            // Sound channel 4 control
            0xFF23 => self.apu.noise.control_read(),
            // Master Volume & VIN panning
            0xFF24 => self.apu.volume_read(),
            // Sound Panning
            0xFF25 => self.apu.sound_panning_read(),
            // Audio Master Control
            0xFF26 => self.apu.master_control_read(),
            // Empty always read 0xff
            0xFF27..=0xFF2F => 0xff,
            // Wave RAM
            0xFF30..=0xFF3F => self.apu.wave.wave_ram_read(addr),
            // Interrupts
            0xFF0F => self.interrupt_flag.bits(),
            0xFF40 => self.ppu.read_ctrl(),
            0xFF41 => self.ppu.read_status(),
            // LY
            0xFF44 => self.ppu.scanline,
            // LYC
            0xFF45 => self.ppu.lyc,
            // KEY1 (CGB only)
            0xFF4D => 0,

            // High RAM
            0xFF80..=0xFFFE => {
                let mirrored_addr = addr - 0xff80;
                self.hram[mirrored_addr as usize]
            }
            // Interrupt Enable
            0xFFFF => self.interrupt_enable.bits(),
            _ => panic!("Address {:04X} not used in memory map", addr),
        }
    }

    pub fn mem_write(&mut self, addr: u16, data: u8) {
        match addr {
            // Cartridge ROM bank 0
            0x0000..=0x3FFF => {
                self.cartridge.write_bank0(addr, data);
            }
            // Cartridge ROM bank 01-NN. May be mapped
            0x4000..=0x7FFF => {
                self.cartridge.write_bankn(addr, data);
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
            0xFF00 => {
                self.joypad.write(data);
            }
            // Serial transfer
            0xFF01 | 0xFF02 => {}
            0xFF04 => {
                self.timer.divider_counter = 0;
            }
            0xFF05 => {
                self.timer.timer_counter = data;
            } // do nothing
            0xFF06 => {
                self.timer.timer_modulo = data;
            }
            0xFF07 => {
                self.timer.tac_write(data);
            }
            // Interrupts
            0xFF0F => {
                self.interrupt_flag = Interrupt::from_bits_retain(data & 0b0001_1111);
            }
            // APU
            // Channel 1 Sweep
            0xFF10 => self.apu.square1.sweep_write(data),
            // Channel 1 length timer & duty cycle
            0xFF11 => self.apu.square1.length_timer_write(data),
            // Channel 1 volume & envelope
            0xFF12 => self.apu.square1.envelope_write(data),
            // Channel 1 period low
            0xFF13 => self.apu.square1.period_low_write(data),
            // Channel 1 period high & control
            0xFF14 => self.apu.square1.control_write(data),
            // Not used
            0xFF15 => {}
            // Sound channel 2 length timer & duty cycle
            0xFF16 => self.apu.square2.length_timer_write(data),
            // Sound channel 2 volume & envelope
            0xFF17 => self.apu.square2.envelope_write(data),
            // Sound channel 2 period low
            0xFF18 => self.apu.square2.period_low_write(data),
            // Sound channel 2 period high & control
            0xFF19 => self.apu.square2.control_write(data),
            // Sound channel 3 DAC enable
            0xFF1A => self.apu.wave.dac_enable_write(data),
            // Sound channel 3 length timer
            0xFF1B => self.apu.wave.length_timer(data),
            // Sound channel 3 output level
            0xFF1C => self.apu.wave.output_level_write(data),
            // Sound channel 3 period low
            0xFF1D => self.apu.wave.period_low_write(data),
            // Sound channel 3 period high & control
            0xFF1E => self.apu.wave.control_write(data),
            // Not used
            0xFF1F => {}
            // Sound channel 4 length timer
            0xFF20 => self.apu.noise.length_timer(data),
            // Sound channel 4 volume & envelope
            0xFF21 => self.apu.noise.envelope_write(data),
            // Sound channel 4 frequency & randomness
            0xFF22 => self.apu.noise.randomness_write(data),
            // Sound channel 4 control
            0xFF23 => self.apu.noise.control_write(data),
            // Master volume & VIN panning
            0xFF24 => self.apu.volume_write(data),
            // Sound Panning
            0xFF25 => self.apu.sound_panning_write(data),
            // Audio Master Control
            0xFF26 => self.apu.master_control_write(data),
            // Not used
            0xFF27..=0xFF2F => {}
            // Wave RAM
            0xFF30..=0xFF3F => self.apu.wave.wave_ram_write(addr, data),
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
                for (i, byte) in page.iter_mut().enumerate() {
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
            // KEY1 (CGB only)
            0xFF4D => {}
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
                self.interrupt_enable = Interrupt::from_bits_retain(data & 0b0001_1111);
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
