mod bus;
mod cartridge;
mod cpu;
mod joypad;
mod opcodes;
mod ppu;
mod render;
mod sdl2_setup;
mod timer;
mod trace;

use bus::Bus;
use cpu::Cpu;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let (mut canvas, mut event_pump) = sdl2_setup::setup();
    let texture_creator = canvas.texture_creator();
    let mut texture = sdl2_setup::dummy_texture(&texture_creator).unwrap();
    let bytes: Vec<u8> = std::fs::read("roms/09-op r,r.gb").expect("No ROM File with that name");
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
            // present frame
            texture.update(None, &frame.data, 160 * 3).unwrap();
            canvas.copy(&texture, None, None).unwrap();
            canvas.present();

            // check user input
            sdl2_setup::get_user_input(&mut event_pump, &mut cpu.bus.joypad);
        }
    }
}
