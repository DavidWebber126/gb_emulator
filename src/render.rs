use crate::ppu::{Control, Ppu};

pub static GB_PALETTE: [(u8, u8, u8); 4] =
    [(155, 188, 15), (139, 172, 15), (48, 98, 48), (15, 56, 15)];

pub struct Frame {
    pub data: Vec<u8>,
}

impl Frame {
    const WIDTH: usize = 160;
    const HEIGHT: usize = 144;

    pub fn new() -> Frame {
        Self {
            data: vec![0; 3 * Frame::WIDTH * Frame::HEIGHT],
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, rgb: (u8, u8, u8)) {
        let base = y * 3 * Frame::WIDTH + x * 3;
        if base + 2 < self.data.len() {
            self.data[base] = rgb.0;
            self.data[base + 1] = rgb.1;
            self.data[base + 2] = rgb.2;
        }
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> (u8, u8, u8) {
        let base = y * 3 * Frame::WIDTH + x * 3;
        (self.data[base], self.data[base + 1], self.data[base + 2])
    }
}

fn get_win_tile_id(ppu: &Ppu, x: usize, y: usize) -> u8 {
    // Translate screen x, y coords onto window tile map by subtracting WX/WY
    let x = x + 7 - ppu.wx as usize; // Plus 7 since WX is corner upper left + 7 pixels for some reason
    let y = y - ppu.wy as usize;
    let tilemap_base = 0x9800 + 0x0400 * (ppu.control.contains(Control::window_map_area) as u16);
    let tile_x = x / 8;
    let tile_y = y / 8;
    ppu.read_vram(tilemap_base + tile_x as u16 + 32 * tile_y as u16)
}

// x,y are screen coordinates i.e 0 <= x < 160 and 0 <= y < 144
fn get_bg_tile_id(ppu: &Ppu, x: usize, y: usize) -> u8 {
    // Translate screen x,y coords onto the tile map by using scroll registers
    let x = (x + ppu.scx as usize) % 256;
    let y = (y + ppu.scy as usize) % 256;
    let tilemap_base = 0x9800 + 0x0400 * (ppu.control.contains(Control::bg_tile_area) as u16);
    let tile_x = x / 8;
    let tile_y = y / 8;
    ppu.read_vram(tilemap_base + tile_x as u16 + 32 * tile_y as u16)
}

fn get_bg_pixel_id(ppu: &Ppu, x: usize, y: usize, tile_id: u8) -> usize {
    let x = (x % 8) as u16; // x coordinate of current tile
    let y = (y % 8) as u16; // y coordinate of current tile
    let tile_base = if tile_id > 127 {
        0x8800 + 16 * (tile_id as u16 - 128)
    } else {
        0x8000
            + 16 * (tile_id as u16)
            + 0x1000 * (ppu.control.contains(Control::bg_win_mode) as u16)
    };
    let lo = (ppu.read_vram(tile_base + 2 * y) & (1 << x)) > 0;
    let hi = (ppu.read_vram(tile_base + 2 * y + 1) & (1 << x)) > 0;
    match (lo, hi) {
        (false, false) => 0,
        (true, false) => 1,
        (false, true) => 2,
        (true, true) => 3,
    }
}

fn render_pixel(ppu: &mut Ppu, x: usize, y: usize, frame: &mut Frame) {
    // If pixel is in window area, fetch window pixel. Otherwise fetch background pixel
    let tile_id = if ppu.control.contains(Control::window_enable)
        && x + 7 >= ppu.wx as usize
        && y >= ppu.wy as usize
    {
        get_win_tile_id(ppu, x, y)
    } else {
        get_bg_tile_id(ppu, x, y)
    };
    let pixel_id = get_bg_pixel_id(ppu, x, y, tile_id);
    let bg_color = GB_PALETTE[pixel_id];

    // Sprite Pixel

    // Decide which has priority and draw to Frame
}
