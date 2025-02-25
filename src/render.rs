use crate::ppu::{Control, Ppu};

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

// x,y are screen coordinates i.e 0 <= x < 160 and 0 <= y < 144
fn get_bg_tile_id(ppu: &Ppu, x: usize, y: usize) -> u8 {
    let x = x % 8; // x coordinate of current tile
    let y = y % 8; // y coordinate of current tile
    let tilemap_base = 0x9800 + 0x0400 * (ppu.control.contains(Control::bg_tile_area) as u16);
    let tile_x = ((ppu.scx / 8).wrapping_add(x as u8)) & 0x1F;
    let tile_y = (y as u8 + ppu.scy) & 0xFF;
    ppu.read_vram(tilemap_base + tile_x as u16 + 32 * tile_y as u16)
}

fn get_bg_tile_data(ppu: &Ppu, x: usize, y: usize, tile_id: u8) -> u8 {
    let x = (x % 8) as u16; // x coordinate of current tile
    let y = (y % 8) as u16; // y coordinate of current tile
    let tile_base = if tile_id > 127 {
        0x8800 + 16 * (tile_id as u16 - 128)
    } else {
        0x8000
            + 16 * (tile_id as u16)
            + 0x1000 * (ppu.control.contains(Control::bg_tile_mode) as u16)
    };
    let lo = ppu.read_vram(tile_base + 2 * y);
    let hi = ppu.read_vram(tile_base + 2 * y + 1);
    (lo & (1 << x) > 0) as u8 + 2 * ((hi & (1 << x) > 0) as u8)
}

fn get_obj_tile_id(ppu: &Ppu, x: usize, y: usize) -> u8 {
    let obj_on_scanline = ppu.oam.iter().step_by(4).any(|&y_pos| y_pos as usize == y);
    
}

fn render_pixel(ppu: &mut Ppu, x: usize, y: usize, frame: &mut Frame) {
    // Background or Window pixel
    let bg_tile_id = get_bg_tile_id(ppu, x, y);
    let bg_pixel = get_bg_tile_data(ppu, x, y, bg_tile_id);

    // Sprite Pixel

    // Decide which has priority and draw to Frame
}
