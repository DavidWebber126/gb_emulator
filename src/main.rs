mod bus;
mod cartridge;
mod cpu;
mod opcodes;
mod trace;

use bus::Bus;
use cartridge::Cartridge;
use cpu::Cpu;

fn main() {
    let bytes: Vec<u8> = std::fs::read("roms/tetris.gb").unwrap();
    let cartridge = Cartridge::new(&bytes).unwrap();
    let bus = Bus::new(cartridge);
    let mut cpu = Cpu::new(bus);
    cpu.run_with_trace();
}
