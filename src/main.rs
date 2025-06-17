mod apu;
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
use std::time::Instant;

fn main() {
    let args: String = env::args().collect();
    let (mut canvas, mut event_pump, audio_device) = sdl2_setup::setup();
    let texture_creator = canvas.texture_creator();
    let mut texture = sdl2_setup::dummy_texture(&texture_creator).unwrap();
    let bytes: Vec<u8> =
        std::fs::read("roms/interrupt_time.gb").expect("No ROM File with that name");
    let cartridge = cartridge::get_mapper(&bytes);
    let bus = Bus::new(cartridge);
    let mut cpu = Cpu::new(bus);

    let trace_on = args.contains("trace");
    if trace_on {
        eprintln!("Trace is on");
    }
    let show_fps = args.contains("show-fps");
    let mut frame_count = 0;
    let mut baseline = Instant::now();
    if show_fps {
        eprintln!("Show FPS is on");
    }
    // Enter game loop
    loop {
        if show_fps && frame_count == 0 {
            baseline = Instant::now();
        } else if frame_count == 30 {
            let thirty_frame_time = baseline.elapsed().as_secs_f32();
            frame_count = 1;
            baseline = Instant::now();
            if show_fps {
                let fps = 30.0 / thirty_frame_time;
                println!("FPS is {}", fps);
            }
        }

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

            // play audio
            audio_device.queue_audio(&cpu.bus.audio_buffer).unwrap();
            //eprintln!("Size: {}", cpu.bus.audio_buffer.len());
            cpu.bus.audio_buffer.clear();

            // check user input
            sdl2_setup::get_user_input(&mut event_pump, &mut cpu.bus.joypad);

            // If FPS enabled, increment counter
            if show_fps {
                frame_count += 1;
            }
        }
    }
}
