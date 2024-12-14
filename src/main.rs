mod cpu;
mod bus;
mod opcodes;

use bus::Bus;
use cpu::CPU;

fn main() {
    let bus = Bus::new(Vec::new());
    let mut cpu = CPU::new(bus);
    cpu.run();
}
