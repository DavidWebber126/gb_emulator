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
    let (mut canvas, event_pump) = sdl2_setup::setup();
    let texture_creator = canvas.texture_creator();
    let mut texture = sdl2_setup::dummy_texture(&texture_creator).unwrap();
    let bytes: Vec<u8> = std::fs::read("roms/tetris.gb").unwrap();
    let cartridge = cartridge::get_mapper(&bytes);
    let bus = Bus::new(cartridge);
    let mut cpu = Cpu::new(bus);
    let trace_on = args.len() > 1 && &args[1] == "trace";
    if trace_on {
        eprintln!("Trace is on");
    }
    // Enter game loop
    loop {
        let frame = if trace_on {
            cpu.step_with_trace()
        } else {
            cpu.step(|_| {})
        };

        if let Some(frame) = frame {
            texture.update(None, &frame.data, 256 * 3).unwrap();
            canvas.copy(&texture, None, None).unwrap();
            canvas.present();
        }
    }
}
