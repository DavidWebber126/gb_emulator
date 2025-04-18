// 1: is released, 0: is pressed
// If bit 4 is 0, then directional buttons can be read
// If bit 5 is 0, then buttons start, select, a and b can be read
// If both dpad and select (bits 4 and 5) are 1 then lower nibble is $F
pub struct Joypad(pub u8);

impl Joypad {
    pub fn new() -> Self {
        Joypad(0)
    }
}
