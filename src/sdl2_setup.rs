use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::EventPump;

pub fn sdl2_setup() -> (Canvas<Window>, EventPump) {
    // init sdl2
    let sdl_context = sdl2::init().unwrap();

    // Video System
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("GB Emulator", (144.0 * 3.0) as u32, (160.0 * 3.0) as u32)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let event_pump = sdl_context.event_pump().unwrap();
    canvas.set_scale(3.0, 3.0).unwrap();

    (canvas, event_pump)
}
