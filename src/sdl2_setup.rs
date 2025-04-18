use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::video::{Window, WindowContext};
use sdl2::EventPump;

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
pub fn dummy_texture<'a>(
    creator: &'a TextureCreator<WindowContext>,
) -> Result<Texture<'a>, String> {
    let texture = creator
        .create_texture_target(PixelFormatEnum::RGB24, WIDTH as u32, HEIGHT as u32)
        .map_err(|e| e.to_string())?;

    Ok(texture)
}
