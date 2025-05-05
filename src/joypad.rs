// 1: is released, 0: is pressed
pub struct SelectButtons(u8);

pub struct Dpad(u8);

// If dpad_mode is false, then directional buttons can be read
// If select_mode is false, then buttons start, select, a and b can be read
// If both dpad_mode and select_mode are true then lower nibble is $F

pub struct Joypad {
    pub select_mode: bool,
    pub dpad_mode: bool,
    pub select: SelectButtons,
    pub dpad: Dpad,
}

impl Joypad {
    pub fn new() -> Self {
        Self {
            select_mode: false,
            dpad_mode: false,
            select: SelectButtons(0x0f),
            dpad: Dpad(0x0f),
        }
    }

    pub fn read(&self) -> u8 {
        let lo_nib = if !self.select_mode {
            self.select.0 & 0x0f
        } else if !self.dpad_mode {
            self.dpad.0 & 0x0f
        } else {
            0x0f
        };
        ((self.select_mode as u8) << 5) + ((self.dpad_mode as u8) << 4) + lo_nib
    }

    pub fn write(&mut self, val: u8) {
        self.select_mode = val & 0b0010_0000 > 0;
        self.dpad_mode = val & 0b0001_0000 > 0;
    }

    // mode = true => select_mode, mode = false => dpad_mode
    pub fn button_pressed_status(&mut self, mode: bool, button: u8, pressed: bool) {
        match (mode, pressed) {
            (true, true) => self.select.0 &= !button,
            (true, false) => self.select.0 |= button,
            (false, true) => self.dpad.0 &= !button,
            (false, false) => self.dpad.0 |= button,
        }
    }
}
