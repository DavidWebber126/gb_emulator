use crate::ppu::{Control, Ppu};

// white, light gray, dark gray, black
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

fn get_sprite(ppu: &Ppu, x: usize, y: usize) -> Option<usize> {
    let mut valid_objs = Vec::new();
    for i in 0..40 {
        let y_byte = ppu.oam[4 * i];
        let x_byte = ppu.oam[4 * i + 1];
        let valid = y + 16 >= y_byte as usize
            && y + 8 *(!ppu.control.contains(Control::obj_size) as usize) < y_byte as usize // If 8x8 we need y < y_byte - 8, in 8x16 just y < y_byte
            && x + 8 >= x_byte as usize
            && x < x_byte as usize;
        if valid {
            valid_objs.push((x_byte, i));
        }
    }
    valid_objs.sort();
    valid_objs.into_iter().map(|(_x, index)| index).next()
}

fn get_pixel_data(ppu: &Ppu, x: usize, y: usize, tile_id: u8, is_obj: bool) -> u8 {
    let x = (x % 8) as u16; // x coordinate of current tile
    let y = (y % 8) as u16; // y coordinate of current tile
                            // if is_obj = true then we want else case base to be 0x8000
                            // if is_obj = false then we need to check
    let adjust = !is_obj && ppu.control.contains(Control::bg_win_mode);
    let tile_base = if tile_id > 127 {
        0x8800 + 16 * (tile_id as u16 - 128)
    } else {
        0x8000 + 16 * (tile_id as u16) + 0x1000 * (adjust as u16)
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
    let pixel_id = get_pixel_data(ppu, x, y, tile_id, false);
    let bg_pixel = ppu.bg_palette & (0b11 << (2 * pixel_id));

    // Sprite Pixel
    let sprite_in_pixel = get_sprite(ppu, x, y);
    let obj_pixel = if let Some(sprite_index) = sprite_in_pixel {
        let mut y_pos = y as u8 + 16 - ppu.oam[4 * sprite_index];
        let mut x_pos = x as u8 + 8 - ppu.oam[4 * sprite_index + 1];
        let tile_index = ppu.oam[4 * sprite_index + 2];
        let sprite_attr = ppu.oam[4 * sprite_index + 3];

        if sprite_attr & 0b0010_0000 > 0 {
            x_pos = 8 - x_pos;
        }
        if sprite_attr & 0b0100_0000 > 0 {
            y_pos = 8 + (8 * ppu.control.contains(Control::obj_size) as u8) - y_pos;
        }

        let obj_id = if ppu.control.contains(Control::obj_size) && y_pos >= 8 {
            get_pixel_data(ppu, x_pos as usize, y_pos as usize, tile_index + 1, true)
        } else {
            get_pixel_data(ppu, x_pos as usize, y_pos as usize, tile_index, true)
        };

        if sprite_attr & 0b1000_0000 > 0 {
            None
        } else if sprite_attr & 0b0001_0000 > 0 {
            Some(ppu.obp1 & (0b11 << (2 * pixel_id)))
        } else {
            Some(ppu.obp0 & (0b11 << (2 * pixel_id)))
        }
    } else {
        None
    };

    // Decide which has priority and draw to Frame
    let pixel = match (ppu.control.contains(Control::obj_enable), obj_pixel) {
        (true, Some(obj_pixel)) => GB_PALETTE[obj_pixel as usize],
        _ => {
            if ppu.control.contains(Control::bg_win_enable) {
                GB_PALETTE[bg_pixel as usize]
            } else {
                GB_PALETTE[0]
            }
        }
    };

    frame.set_pixel(x, y, pixel);
}
