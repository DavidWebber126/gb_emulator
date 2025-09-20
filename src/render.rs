use crate::ppu::{Control, Ppu, ScreenOptions};
use eframe::egui::{self, Color32};

// white, light gray, dark gray, black
const GB_PALETTE: [(u8, u8, u8); 4] = [(155, 188, 15), (139, 172, 15), (48, 98, 48), (15, 56, 15)];

#[derive(Clone)]
pub struct Frame {
    pub data: Vec<egui::Color32>,
}

impl Frame {
    const WIDTH: usize = 160;
    const HEIGHT: usize = 144;

    pub fn new() -> Frame {
        Self {
            data: vec![Color32::PLACEHOLDER; Frame::WIDTH * Frame::HEIGHT],
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, rgb: (u8, u8, u8)) {
        let color = egui::Color32::from_rgb(rgb.0, rgb.1, rgb.2);
        let base = y * Frame::WIDTH + x;
        self.data[base] = color;
    }

    // pub fn _get_pixel(&self, x: usize, y: usize) -> (u8, u8, u8) {
    //     let base = y * Frame::WIDTH + x;
    //     base = self.data[base];
    // }
}

// returns (tile_id, x_pos, y_pos)
fn get_win_tile_id(ppu: &Ppu, x: usize, y: usize) -> (u8, u8, u8, bool) {
    // Translate screen x, y coords onto window tile map by subtracting WX/WY
    let x_pos = x + 7 - ppu.wx as usize; // Plus 7 since WX is corner upper left + 7 pixels for some reason
    let y_pos = y;
    let tilemap_base = if ppu.control.contains(Control::window_map_area) {
        0x9c00
    } else {
        0x9800
    };
    let tile_x = x_pos / 8;
    let tile_y = y_pos / 8;
    let x_p = (x_pos % 8) as u8;
    let y_p = (y_pos % 8) as u8;
    (
        ppu.read_vram(tilemap_base + tile_x as u16 + 32 * tile_y as u16),
        x_p,
        y_p,
        true,
    )
}

// x,y are screen coordinates i.e 0 <= x < 160 and 0 <= y < 144
fn get_bg_tile_id(ppu: &Ppu, x: usize, y: usize) -> (u8, u8, u8, bool) {
    // Translate screen x,y coords onto the tile map by using scroll registers
    let x_pos = (x + ppu.scx as usize) % 256;
    let y_pos = (y + ppu.scy as usize) % 256;
    let tilemap_base = if ppu.control.contains(Control::bg_tile_area) {
        0x9c00
    } else {
        0x9800
    };
    let tile_x = x_pos / 8;
    let tile_y = y_pos / 8;
    let x_p = (x_pos % 8) as u8;
    let y_p = (y_pos % 8) as u8;
    (
        ppu.read_vram(tilemap_base + tile_x as u16 + 32 * tile_y as u16),
        x_p,
        y_p,
        false,
    )
}

fn get_sprite(ppu: &Ppu, x: usize, y: usize) -> (u8, bool) {
    let mut valid_objs = Vec::new();
    for i in ppu.scanline_oams.iter() {
        let x_byte = ppu.oam[4 * i + 1];
        let valid = x + 8 >= x_byte as usize && x < x_byte as usize;
        if valid {
            valid_objs.push((x_byte, *i));
        }
    }
    valid_objs.sort();
    let sprites: Vec<usize> = valid_objs.into_iter().map(|(_x, id)| id).collect();
    resolve_sprite_overlap(ppu, x, y, &sprites)
}

fn resolve_sprite_overlap(ppu: &Ppu, x: usize, y: usize, sprites: &[usize]) -> (u8, bool) {
    for sprite_index in sprites {
        let mut y_pos = y as u8 + 16 - ppu.oam[4 * sprite_index];
        let mut x_pos = x as u8 + 8 - ppu.oam[4 * sprite_index + 1];
        let tile_index = ppu.oam[4 * sprite_index + 2];
        let sprite_attr = ppu.oam[4 * sprite_index + 3];

        if sprite_attr & 0b0010_0000 > 0 {
            x_pos = 7 - x_pos;
        }
        if sprite_attr & 0b0100_0000 > 0 {
            y_pos = 7 + (8 * ppu.control.contains(Control::obj_size) as u8) - y_pos;
        }

        let obj_id = if ppu.control.contains(Control::obj_size) && y_pos >= 8 {
            get_pixel_data(ppu, x_pos, y_pos - 8, tile_index | 0x01, true)
        } else if ppu.control.contains(Control::obj_size) {
            get_pixel_data(ppu, x_pos, y_pos, tile_index & 0xfe, true)
        } else {
            get_pixel_data(ppu, x_pos, y_pos, tile_index, true)
        };

        if obj_id != 0 {
            let color = if sprite_attr & 0b0001_0000 > 0 {
                (ppu.obp1 & (0b11 << (2 * obj_id))) >> (2 * obj_id)
            } else {
                (ppu.obp0 & (0b11 << (2 * obj_id))) >> (2 * obj_id)
            };
            return (color, sprite_attr & 0b1000_0000 > 0);
        }
    }
    // Return 0xff if obj_id is 0 for all previous sprites.
    // This means pixel is transparent for all the sprites.
    (0xff, true)
}

// Need a relative x and y to the upper left pixel of tile/obj
fn get_pixel_data(ppu: &Ppu, x: u8, y: u8, tile_id: u8, is_obj: bool) -> u8 {
    let x = x as u16; // x coordinate of current tile
    let y = y as u16; // y coordinate of current tile

    // if is_obj = true then we want else case base to be 0x8000
    // if is_obj = false then we need to check
    let adjust = !is_obj && !ppu.control.contains(Control::bg_win_mode);
    let tile_base = if tile_id > 127 {
        0x8800 + 16 * (tile_id as u16 - 128)
    } else {
        0x8000 + 16 * (tile_id as u16) + 0x1000 * (adjust as u16)
    };
    let inverted_x = 7 - x; // Invert so that x=0 corresponds to bit 7 of color index
    let lo = (ppu.read_vram(tile_base + 2 * y) & (1 << inverted_x)) > 0;
    let hi = (ppu.read_vram(tile_base + 2 * y + 1) & (1 << inverted_x)) > 0;
    match (lo, hi) {
        (false, false) => 0,
        (true, false) => 1,
        (false, true) => 2,
        (true, true) => 3,
    }
}

fn render_pixel(ppu: &Ppu, x: usize, y: usize, frame: &mut Frame) {
    // If pixel is in window area, fetch window pixel. Otherwise fetch background pixel
    let (tile_id, x_pos, y_pos, is_window) = if ppu.control.contains(Control::window_enable)
        && x + 7 >= ppu.wx as usize
        && y >= ppu.wy as usize
    {
        //eprintln!("Scanline: {}, window: {}, wy: {}", ppu.scanline, ppu.window_counter, ppu.wy);
        get_win_tile_id(ppu, x, ppu.window_counter)
    } else {
        get_bg_tile_id(ppu, x, y)
    };
    let pixel_id = get_pixel_data(ppu, x_pos, y_pos, tile_id, false);
    let bg_pixel = (ppu.bg_palette & (0b11 << (2 * pixel_id))) >> (2 * pixel_id);

    // Sprite Pixel
    let (obj_color, bg_over_obj) = get_sprite(ppu, x, y);
    let obj_pixel = if (bg_over_obj && pixel_id > 0) || obj_color == 0xff {
        None
    } else {
        Some(obj_color)
    };

    // Decide which has priority and draw to Frame
    let pixel = match ppu.screen_options {
        ScreenOptions::All => match (ppu.control.contains(Control::obj_enable), obj_pixel) {
            (true, Some(obj_pixel)) => GB_PALETTE[obj_pixel as usize],
            _ => {
                if ppu.control.contains(Control::bg_win_enable) {
                    GB_PALETTE[bg_pixel as usize]
                } else {
                    GB_PALETTE[0]
                }
            }
        },
        ScreenOptions::BackgroundOnly => {
            if !is_window {
                GB_PALETTE[bg_pixel as usize]
            } else {
                (0, 0, 0)
            }
        }
        ScreenOptions::SpritesOnly => match obj_pixel {
            Some(pixel) => GB_PALETTE[pixel as usize],
            None => (0, 0, 0),
        },
        ScreenOptions::WindowOnly => {
            if is_window {
                GB_PALETTE[bg_pixel as usize]
            } else {
                (0, 0, 0)
            }
        }
    };

    frame.set_pixel(x, y, pixel);
}

pub fn render_scanline(ppu: &Ppu, frame: &mut Frame) {
    let current_scanline = ppu.scanline as usize;
    for i in 0..Frame::WIDTH {
        render_pixel(ppu, i, current_scanline, frame);
    }
}
