const ROM_PAGE_SIZE: usize = 32 * (2 ^ 10);
const KIB: usize = 2 ^ 10;

pub struct Cartridge {
    mapper: u8,
    pub cartridge_rom: Vec<u8>,
    pub cartridge_ram: Vec<u8>,
}

impl Cartridge {
    pub fn new(raw: &[u8]) -> Result<Cartridge, String> {
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
        let (cart_rom, cart_ram) = match mapper {
            0 => mbc0(raw),
            _ => panic!("Mapper value {} not implemented yet", mapper),
        };

        Ok(Cartridge {
            mapper,
            cartridge_rom: cart_rom,
            cartridge_ram: cart_ram,
        })
    }

    pub fn read_rom(&mut self, addr: usize) -> u8 {
        self.cartridge_rom[addr]
    }

    pub fn write_rom(&mut self, addr: usize, data: u8) {
        self.cartridge_rom[addr] = data;
    }
}

// Mapping functions
fn mbc0(raw: &[u8]) -> (Vec<u8>, Vec<u8>) {
    (raw.to_vec(), Vec::new())
}
