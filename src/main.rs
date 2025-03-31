mod bus;
mod cartridge;
mod cpu;
mod opcodes;
mod ppu;
mod render;
mod sdl2_setup;
mod trace;

use bus::Bus;
use cpu::Cpu;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let (canvas, event_pump) = sdl2_setup::sdl2_setup();
    let bytes: Vec<u8> = std::fs::read("roms/tetris.gb").unwrap();
    let cartridge = cartridge::get_mapper(&bytes);
    let bus = Bus::new(Box::new(cartridge), canvas);
    let mut cpu = Cpu::new(bus);
    if args.len() > 1 && &args[1] == "trace" {
        cpu.run_with_trace();
    } else {
        cpu.run_with_callback(|_| {});
    }
}
