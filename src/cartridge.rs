use chrono::{offset::Local, Datelike, Timelike};

const ROM_PAGE_SIZE: usize = 32768;
const KIB: usize = 1024;
const MIB: usize = 1048576;

pub trait Mapper {
    fn read_bank0(&mut self, addr: u16) -> u8;
    fn read_bankn(&mut self, addr: u16) -> u8;
    fn write_bank0(&mut self, addr: u16, val: u8);
    fn write_bankn(&mut self, addr: u16, val: u8);
    fn ram_read(&mut self, addr: u16) -> u8;
    fn ram_write(&mut self, addr: u16, val: u8);
}

// Function to get the mapper as indicated by the code (i.e byte 0x0147)
pub fn get_mapper(raw: &[u8]) -> Box<dyn Mapper> {
    // let header = &raw[0x0100..=0x014F];
    // let cgb = raw[0x0143];
    // let sgb = raw[0x0146];

    let rom_size = ROM_PAGE_SIZE * (1 << raw[0x0148]);
    let ram_size = match raw[0x0149] {
        0 => 0,
        2 => 8 * KIB,
        3 => 32 * KIB,
        4 => 128 * KIB,
        5 => 64 * KIB,
        _ => panic!(
            "Cartridge RAM should not be value other than 0,2,3,4,5. Received: {}",
            raw[0x0149]
        ),
    };

    let mapper = raw[0x0147];
    eprintln!("Mapper is: {mapper}");
    eprintln!("Rom Size: 0x{rom_size:X}, Ram Size: 0x{ram_size:X}");
    match mapper {
        0 => Box::new(Mbc0::new(raw, ram_size)),
        1..=3 => Box::new(Mbc1::new(raw, rom_size, ram_size)),
        16..=19 => Box::new(Mbc3::new(raw, ram_size)),
        _ => panic!("Mapper value {mapper} not implemented yet"),
    }
}

pub struct Mbc3 {
    cartridge_rom: Vec<u8>,
    cartridge_ram: Vec<u8>,
    ram_size: usize,
    ram_enabled: bool,
    rom_bank: u8,
    bank_or_register: u8,
    rtc_prior_val: bool,
    rtc_s: u8,
    rtc_m: u8,
    rtc_h: u8,
    rtc_dl: u8,
    rtc_day_upper: bool,
    rtc_halt: bool,
    rtc_carry: bool,
}

impl Mbc3 {
    fn new(rom: &[u8], ram_size: usize) -> Self {
        let cartridge_rom = rom.to_vec();
        let cartridge_ram = vec![0; ram_size];
        Self {
            cartridge_rom,
            cartridge_ram,
            ram_size,
            ram_enabled: false,
            rom_bank: 1,
            bank_or_register: 0,
            rtc_prior_val: false,
            rtc_s: 0,
            rtc_m: 0,
            rtc_h: 0,
            rtc_dl: 0,
            rtc_day_upper: false,
            rtc_halt: false,
            rtc_carry: false,
        }
    }
}

impl Mapper for Mbc3 {
    fn read_bank0(&mut self, addr: u16) -> u8 {
        let addr = addr as usize;
        self.cartridge_rom[addr]
    }

    fn read_bankn(&mut self, addr: u16) -> u8 {
        let addr = addr as usize - 0x4000; // get addr relative to base
        let bank_base = (self.rom_bank as usize) << 14;
        self.cartridge_rom[addr + bank_base]
    }

    fn write_bank0(&mut self, addr: u16, val: u8) {
        // RAM Enable register
        if addr <= 0x1FFF {
            self.ram_enabled = self.ram_size > 0 && (val & 0x0f == 0xa);
        }
        // ROM Bank Number
        if (0x2000..=0x3FFF).contains(&addr) {
            self.rom_bank = if val == 0 { 1 } else { val & 0x7f }
        }
    }

    fn write_bankn(&mut self, addr: u16, val: u8) {
        // RAM Bank Number or RTC select
        if (0x4000..=0x5fff).contains(&addr) && val <= 0x0c {
            self.bank_or_register = val;
        }

        // Latch Clock Data
        if (0x6000..=0x7fff).contains(&addr) {
            if val == 0 {
                self.rtc_prior_val = true;
            } else if self.rtc_prior_val && val == 1 {
                self.rtc_prior_val = false;
                let now = Local::now();

                self.rtc_s = now.second() as u8;
                self.rtc_m = now.minute() as u8;
                self.rtc_h = now.hour() as u8;
                let day = now.ordinal0();
                self.rtc_dl = day as u8;
                self.rtc_day_upper = day & 0xf0 > 0;
            } else {
                self.rtc_prior_val = false;
            }
        }
    }

    fn ram_read(&mut self, addr: u16) -> u8 {
        match self.bank_or_register {
            0..=0x07 => {
                let addr = addr - 0xA000;
                self.cartridge_ram[addr as usize]
            }
            0x08 => self.rtc_s,
            0x09 => self.rtc_m,
            0x0a => self.rtc_h,
            0x0b => self.rtc_dl,
            0x0c => {
                let carry = (self.rtc_carry as u8) << 7;
                let halt = (self.rtc_halt as u8) << 6;
                carry + halt + self.rtc_day_upper as u8
            }
            _ => panic!("Impossible"),
        }
    }

    fn ram_write(&mut self, addr: u16, val: u8) {
        match self.bank_or_register {
            0..=0x07 => {
                let addr = addr - 0xA000;
                self.cartridge_ram[addr as usize] = val;
            }
            0x08 => self.rtc_s = val,
            0x09 => self.rtc_m = val,
            0x0a => self.rtc_h = val,
            0x0b => self.rtc_dl = val,
            0x0c => {
                self.rtc_carry = val & 0b1000_0000 > 0;
                self.rtc_halt = val & 0b0100_0000 > 0;
                self.rtc_day_upper = val & 0b0000_0001 > 0;
            }
            _ => panic!("Impossible"),
        }
    }
}

pub struct Mbc1 {
    ram_enabled: bool,
    rom_bank: u8,
    ram_bank: u8,
    banking_mode: bool,
    max_bank: u8,
    rom_size: usize,
    ram_size: usize,
    cartridge_rom: Vec<u8>,
    cartridge_ram: Vec<u8>,
}

impl Mbc1 {
    fn new(rom: &[u8], rom_size: usize, ram_size: usize) -> Self {
        let cartridge_rom = rom.to_vec();
        let cartridge_ram = vec![0; ram_size];
        let max_bank = (rom_size / (16 * KIB)) as u8;
        Self {
            rom_bank: 1,
            ram_bank: 0,
            max_bank,
            banking_mode: false,
            ram_enabled: false,
            rom_size,
            ram_size,
            cartridge_rom,
            cartridge_ram,
        }
    }
}

impl Mapper for Mbc1 {
    fn read_bank0(&mut self, addr: u16) -> u8 {
        let addr = addr as usize;
        if self.banking_mode && self.rom_size > MIB {
            // mode = 1
            let bank = (self.ram_bank as usize) << 18; // ram_bank is also upper bits for rom bank
            self.cartridge_rom[bank + addr]
        } else {
            // mode = 0
            self.cartridge_rom[addr]
        }
    }

    // Addr should be between 0x4000 and 0x7FFF
    // bits 19-20: Upper bank, 14-18: bank register, 0-13: from addr
    fn read_bankn(&mut self, addr: u16) -> u8 {
        let addr = addr as usize - 0x4000; // get addr relative to base
        let bank_base = (self.rom_bank as usize) << 14;
        //println!("Addr: {:04X}, bank: {:04X}", addr, self.rom_bank);
        if self.rom_size > MIB {
            let upper_bank = (self.ram_bank as usize) << 18;
            self.cartridge_rom[addr + bank_base + upper_bank]
        } else {
            self.cartridge_rom[addr + bank_base]
        }
    }

    fn write_bank0(&mut self, addr: u16, val: u8) {
        // RAM Enable register
        if addr <= 0x1FFF {
            self.ram_enabled = self.ram_size > 0 && val & 0x0f == 0xa;
        }
        // ROM Bank Number
        if (0x2000..=0x3FFF).contains(&addr) {
            let masked_bank = if val & 0x1f == 0 { 1 } else { val & 0x1f };
            if self.max_bank > 2 ^ 32 {
                // Large Cart - use ram_bank as extra two bits
                self.rom_bank = (self.ram_bank << 5) + masked_bank;
            } else {
                self.rom_bank = masked_bank & (self.max_bank - 1); // max_bank - 1 gives the mask since max_
            }
        }
    }

    fn write_bankn(&mut self, addr: u16, val: u8) {
        // RAM Bank Number or Upper bits
        if (0x4000..=0x5fff).contains(&addr) {
            self.ram_bank = val & 0x11;
        }

        // Mode select
        if (0x6000..=0x7fff).contains(&addr) {
            self.banking_mode = val % 2 == 1;
        }
    }

    fn ram_write(&mut self, addr: u16, val: u8) {
        // make addr relative to base address
        let addr = (addr as usize) - 0xA000;
        if addr >= self.ram_size {
            return;
        }
        if self.banking_mode && self.ram_size >= 512 * KIB {
            // Mode 1
            let bank = (self.ram_bank as usize) << 13;
            self.cartridge_ram[addr + bank] = val;
        } else {
            // Mode 0
            self.cartridge_ram[addr] = val;
        }
    }

    fn ram_read(&mut self, addr: u16) -> u8 {
        // make addr relative to base address
        let addr = (addr as usize) - 0xA000;
        if self.banking_mode && self.ram_size > 512 * KIB {
            // Mode 1
            let bank = (self.ram_bank as usize) << 13;
            self.cartridge_ram[addr + bank]
        } else {
            // Mode 0
            self.cartridge_ram[addr]
        }
    }
}

pub struct Mbc0 {
    cartridge_rom: Vec<u8>,
    cartridge_ram: Vec<u8>,
}

impl Mbc0 {
    fn new(rom: &[u8], ram_size: usize) -> Self {
        let cartridge_ram = vec![0; ram_size];
        Self {
            cartridge_rom: rom.to_vec(),
            cartridge_ram,
        }
    }
}

impl Mapper for Mbc0 {
    fn read_bank0(&mut self, addr: u16) -> u8 {
        self.cartridge_rom[addr as usize]
    }

    fn read_bankn(&mut self, addr: u16) -> u8 {
        self.cartridge_rom[addr as usize]
    }

    fn write_bank0(&mut self, _addr: u16, _val: u8) {
        // do nothing
    }

    fn write_bankn(&mut self, _addr: u16, _val: u8) {
        // do nothing
    }

    fn ram_write(&mut self, addr: u16, val: u8) {
        self.cartridge_ram[addr as usize] = val;
    }

    fn ram_read(&mut self, addr: u16) -> u8 {
        self.cartridge_ram[addr as usize]
    }
}
