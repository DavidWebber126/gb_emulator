use std::collections::HashMap;

use lazy_static::lazy_static;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::video::{Window, WindowContext};
use sdl2::EventPump;

use crate::joypad::Joypad;

const WIDTH: f64 = 160.0;
const HEIGHT: f64 = 144.0;

pub fn setup() -> (Canvas<Window>, EventPump) {
    // init sdl2
    let sdl_context = sdl2::init().unwrap();

    // Video System
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("GB Emulator", (WIDTH * 3.0) as u32, (HEIGHT * 3.0) as u32)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let event_pump = sdl_context.event_pump().unwrap();
    canvas.set_scale(3.0, 3.0).unwrap();

    (canvas, event_pump)
}

// Create a "target" texture so that we can use our Renderer with it later
pub fn dummy_texture(creator: &TextureCreator<WindowContext>) -> Result<Texture, String> {
    let texture = creator
        .create_texture_target(PixelFormatEnum::RGB24, WIDTH as u32, HEIGHT as u32)
        .map_err(|e| e.to_string())?;

    Ok(texture)
}

lazy_static! {
    static ref KEY_MAP: HashMap<Keycode, (bool, u8)> = {
        let mut key_map = HashMap::new();

        key_map.insert(Keycode::Down, (true, 0b0000_1000));
        key_map.insert(Keycode::Up, (true, 0b0000_0100));
        key_map.insert(Keycode::Left, (true, 0b0000_0010));
        key_map.insert(Keycode::Right, (true, 0b0000_0001));
        key_map.insert(Keycode::Return, (false, 0b0000_1000));
        key_map.insert(Keycode::Space, (false, 0b0000_0100));
        key_map.insert(Keycode::B, (false, 0b0000_0010));
        key_map.insert(Keycode::A, (false, 0b0000_0001));

        key_map
    };
}

pub fn get_user_input(event_pump: &mut EventPump, joypad: &mut Joypad) {
    for event in event_pump.poll_iter() {
        match event {
            Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => std::process::exit(0),
            Event::KeyDown { keycode, .. } => {
                if let Some(&(mode, button)) = KEY_MAP.get(&keycode.unwrap_or(Keycode::Ampersand)) {
                    joypad.button_pressed_status(mode, button, true);
                }
            }
            Event::KeyUp { keycode, .. } => {
                if let Some(&(mode, button)) = KEY_MAP.get(&keycode.unwrap_or(Keycode::Ampersand)) {
                    joypad.button_pressed_status(mode, button, false);
                }
            }
            _ => { /* do nothing */ }
        }
    }
}
