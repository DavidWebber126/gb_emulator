mod bus;
mod cartridge;
mod cpu;
mod opcodes;

use bus::Bus;
use cpu::Cpu;

fn main() {
    let bus = Bus::new(Vec::new());
    let mut cpu = Cpu::new(bus);
    cpu.run();
}
