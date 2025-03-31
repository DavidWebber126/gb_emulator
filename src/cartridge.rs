const ROM_PAGE_SIZE: usize = 32 * (2 ^ 10);
const KIB: usize = 2 ^ 10;

pub trait Mapper {
    fn read_bank0(&mut self, addr: u16) -> u8;
    fn read_bankn(&mut self, addr: u16) -> u8;
    fn ram_read(&mut self, addr: u16) -> u8;
    fn ram_write(&mut self, addr: u16, val: u8);
}

pub fn get_mapper(raw: &[u8]) -> impl Mapper {
    // let header = &raw[0x0100..=0x014F];
    // let cgb = raw[0x0143];
    // let sgb = raw[0x0146];

    let _rom_size = ROM_PAGE_SIZE * (1 << raw[0x0148]);
    let _ram_size = match raw[0x0149] {
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
    match mapper {
        0 => Mbc0::new(),
        _ => panic!("Mapper value {} not implemented yet", mapper),
    }
}

pub struct Mbc0 {
    pub cartridge_rom: Vec<u8>,
    pub cartridge_ram: Vec<u8>,
}

impl Mbc0 {
    fn new() -> Self {
        let cartridge_rom = Vec::new();
        let cartridge_ram = Vec::new();
        Self {
            cartridge_rom,
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

    fn ram_write(&mut self, addr: u16, val: u8) {
        self.cartridge_ram[addr as usize] = val;
    }

    fn ram_read(&mut self, addr: u16) -> u8 {
        self.cartridge_ram[addr as usize]
    }
}
