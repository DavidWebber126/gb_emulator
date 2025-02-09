use lazy_static::lazy_static;
use std::collections::HashMap;

#[derive(Debug)]
pub enum TargetReg {
    None,
    R8(u8),     // 0: b, 1: c, 2: d, 3: e, 4: h, 5: l, 6: [hl], 7: a
    R16(u8),    // 0: bc, 1: de, 2: hl, 3: sp
    R16stk(u8), // 0: bc, 1: de, 2: hl, 3: af
    R16mem(u8), // 0: bc, 1: de, 2: hl+, 3: hl-
    Cond(u8),   // 0: nz, 1: z, 2: nc, 3: c
    B3(u8),
    Tgt3(u8),
    A,
    C,
    SP,
    Imm8,
    Imm16,
    Ptr,
}

pub struct Opcode {
    pub name: &'static str,
    pub reg1: TargetReg,
    pub reg2: TargetReg,
    pub bytes: u16,
    pub cycles: u8,
}

impl Opcode {
    pub fn new(
        name: &'static str,
        reg1: TargetReg,
        reg2: TargetReg,
        bytes: u16,
        cycles: u8,
    ) -> Self {
        Self {
            name,
            reg1,
            reg2,
            bytes,
            cycles,
        }
    }
}

lazy_static! {
    pub static ref CPU_OP_CODES: HashMap<u8, Opcode> = {
        let mut map = HashMap::new();

        // adc a, r8
        map.insert(0x88, Opcode::new("ADC", TargetReg::A, TargetReg::R8(0), 1, 1));
        map.insert(0x89, Opcode::new("ADC", TargetReg::A, TargetReg::R8(1), 1, 1));
        map.insert(0x8a, Opcode::new("ADC", TargetReg::A, TargetReg::R8(2), 1, 1));
        map.insert(0x8b, Opcode::new("ADC", TargetReg::A, TargetReg::R8(3), 1, 1));
        map.insert(0x8c, Opcode::new("ADC", TargetReg::A, TargetReg::R8(4), 1, 1));
        map.insert(0x8d, Opcode::new("ADC", TargetReg::A, TargetReg::R8(5), 1, 1));
        map.insert(0x8e, Opcode::new("ADC", TargetReg::A, TargetReg::R8(6), 1, 2)); // adc a, [hl]
        map.insert(0x8f, Opcode::new("ADC", TargetReg::A, TargetReg::R8(7), 1, 1));

        // adc a, n8
        map.insert(0xce, Opcode::new("ADC", TargetReg::A, TargetReg::Imm8, 2, 2));

        // add a, r8
        map.insert(0x80, Opcode::new("ADD", TargetReg::A, TargetReg::R8(0), 1, 1));
        map.insert(0x81, Opcode::new("ADD", TargetReg::A, TargetReg::R8(1), 1, 1));
        map.insert(0x82, Opcode::new("ADD", TargetReg::A, TargetReg::R8(2), 1, 1));
        map.insert(0x83, Opcode::new("ADD", TargetReg::A, TargetReg::R8(3), 1, 1));
        map.insert(0x84, Opcode::new("ADD", TargetReg::A, TargetReg::R8(4), 1, 1));
        map.insert(0x85, Opcode::new("ADD", TargetReg::A, TargetReg::R8(5), 1, 1));
        map.insert(0x86, Opcode::new("ADD", TargetReg::A, TargetReg::R8(6), 1, 2)); // add a, [hl]
        map.insert(0x87, Opcode::new("ADD", TargetReg::A, TargetReg::R8(7), 1, 1));

        // add a, r8
        map.insert(0xc6, Opcode::new("ADD", TargetReg::A, TargetReg::Imm8, 2, 2));

        // add hl, r16
        map.insert(0x09, Opcode::new("ADD", TargetReg::R16(2), TargetReg::R16(0), 2, 1));
        map.insert(0x19, Opcode::new("ADD", TargetReg::R16(2), TargetReg::R16(1), 2, 1));
        map.insert(0x29, Opcode::new("ADD", TargetReg::R16(2), TargetReg::R16(2), 2, 1));
        map.insert(0x39, Opcode::new("ADD", TargetReg::R16(2), TargetReg::R16(3), 2, 1));

        // add sp, r8
        map.insert(0xe8, Opcode::new("ADD", TargetReg::SP, TargetReg::Imm8, 2, 4));

        // and a, r8
        map.insert(0xa0, Opcode::new("AND", TargetReg::A, TargetReg::R8(0), 1, 1));
        map.insert(0xa1, Opcode::new("AND", TargetReg::A, TargetReg::R8(1), 1, 1));
        map.insert(0xa2, Opcode::new("AND", TargetReg::A, TargetReg::R8(2), 1, 1));
        map.insert(0xa3, Opcode::new("AND", TargetReg::A, TargetReg::R8(3), 1, 1));
        map.insert(0xa4, Opcode::new("AND", TargetReg::A, TargetReg::R8(4), 1, 1));
        map.insert(0xa5, Opcode::new("AND", TargetReg::A, TargetReg::R8(5), 1, 1));
        map.insert(0xa6, Opcode::new("AND", TargetReg::A, TargetReg::R8(6), 1, 2)); // and a, hl
        map.insert(0xa7, Opcode::new("AND", TargetReg::A, TargetReg::R8(7), 1, 1));

        // and a, r8
        map.insert(0xe6, Opcode::new("AND", TargetReg::A, TargetReg::Imm8, 2, 2));

        // call r16
        map.insert(0xcd, Opcode::new("CALL", TargetReg::Imm16, TargetReg::None, 3, 6));

        // call cond, r16
        map.insert(0xc4, Opcode::new("CALL", TargetReg::Cond(0), TargetReg::Imm16, 3, 3));
        map.insert(0xcc, Opcode::new("CALL", TargetReg::Cond(1), TargetReg::Imm16, 3, 3));
        map.insert(0xd4, Opcode::new("CALL", TargetReg::Cond(2), TargetReg::Imm16, 3, 3));
        map.insert(0xdc, Opcode::new("CALL", TargetReg::Cond(3), TargetReg::Imm16, 3, 3));

        // ccf
        map.insert(0x3f, Opcode::new("CCF", TargetReg::None, TargetReg::None, 1, 1));

        // cp a, r8
        map.insert(0xb8, Opcode::new("CP", TargetReg::A, TargetReg::R8(0), 1, 1));
        map.insert(0xb9, Opcode::new("CP", TargetReg::A, TargetReg::R8(1), 1, 1));
        map.insert(0xba, Opcode::new("CP", TargetReg::A, TargetReg::R8(2), 1, 1));
        map.insert(0xbb, Opcode::new("CP", TargetReg::A, TargetReg::R8(3), 1, 1));
        map.insert(0xbc, Opcode::new("CP", TargetReg::A, TargetReg::R8(4), 1, 1));
        map.insert(0xbd, Opcode::new("CP", TargetReg::A, TargetReg::R8(5), 1, 1));
        map.insert(0xbe, Opcode::new("CP", TargetReg::A, TargetReg::R8(6), 1, 2)); // cp a, [hl]
        map.insert(0xbf, Opcode::new("CP", TargetReg::A, TargetReg::R8(7), 1, 1));

        // cp a, n8
        map.insert(0xfe, Opcode::new("CP", TargetReg::A, TargetReg::Imm8, 2, 2));

        // cpl
        map.insert(0x2f, Opcode::new("CPL", TargetReg::None, TargetReg::None, 2, 2));

        // daa
        map.insert(0x27, Opcode::new("DAA", TargetReg::None, TargetReg::None, 1, 1));

        // dec r8
        map.insert(0x05, Opcode::new("DEC", TargetReg::R8(0), TargetReg::None, 1, 1));
        map.insert(0x0d, Opcode::new("DEC", TargetReg::R8(1), TargetReg::None, 1, 1));
        map.insert(0x15, Opcode::new("DEC", TargetReg::R8(2), TargetReg::None, 1, 1));
        map.insert(0x1d, Opcode::new("DEC", TargetReg::R8(3), TargetReg::None, 1, 1));
        map.insert(0x25, Opcode::new("DEC", TargetReg::R8(4), TargetReg::None, 1, 1));
        map.insert(0x2d, Opcode::new("DEC", TargetReg::R8(5), TargetReg::None, 1, 1));
        map.insert(0x35, Opcode::new("DEC", TargetReg::R8(6), TargetReg::None, 1, 3)); // dec [hl]
        map.insert(0x3d, Opcode::new("DEC", TargetReg::R8(7), TargetReg::None, 1, 1));

        // dec r16
        map.insert(0x0b, Opcode::new("DEC", TargetReg::R16(0), TargetReg::None, 1, 2));
        map.insert(0x1b, Opcode::new("DEC", TargetReg::R16(1), TargetReg::None, 1, 2));
        map.insert(0x2b, Opcode::new("DEC", TargetReg::R16(2), TargetReg::None, 1, 2));
        map.insert(0x3b, Opcode::new("DEC", TargetReg::R16(3), TargetReg::None, 1, 2));

        // di
        map.insert(0xf3, Opcode::new("DI", TargetReg::None, TargetReg::None, 1, 1));

        // ei
        map.insert(0xfb, Opcode::new("EI", TargetReg::None, TargetReg::None, 1, 1));

        // halt
        map.insert(0x76, Opcode::new("HALT", TargetReg::None, TargetReg::None, 1, 0));

        // inc r8
        map.insert(0x04, Opcode::new("INC", TargetReg::R8(0), TargetReg::None, 1, 1));
        map.insert(0x0c, Opcode::new("INC", TargetReg::R8(1), TargetReg::None, 1, 1));
        map.insert(0x14, Opcode::new("INC", TargetReg::R8(2), TargetReg::None, 1, 1));
        map.insert(0x1c, Opcode::new("INC", TargetReg::R8(3), TargetReg::None, 1, 1));
        map.insert(0x24, Opcode::new("INC", TargetReg::R8(4), TargetReg::None, 1, 1));
        map.insert(0x2c, Opcode::new("INC", TargetReg::R8(5), TargetReg::None, 1, 1));
        map.insert(0x34, Opcode::new("INC", TargetReg::R8(6), TargetReg::None, 1, 3)); // inc [hl]
        map.insert(0x3c, Opcode::new("INC", TargetReg::R8(7), TargetReg::None, 1, 1));

        // inc r16
        map.insert(0x03, Opcode::new("INC", TargetReg::R16(0), TargetReg::None, 1, 2));
        map.insert(0x13, Opcode::new("INC", TargetReg::R16(1), TargetReg::None, 1, 2));
        map.insert(0x23, Opcode::new("INC", TargetReg::R16(2), TargetReg::None, 1, 2));
        map.insert(0x33, Opcode::new("INC", TargetReg::R16(3), TargetReg::None, 1, 2));

        // jp n16
        map.insert(0xc3, Opcode::new("JP", TargetReg::Imm16, TargetReg::None, 3, 4));

        // jp cc, n16
        map.insert(0xc2, Opcode::new("JP", TargetReg::Cond(0), TargetReg::Imm16, 3, 3));
        map.insert(0xca, Opcode::new("JP", TargetReg::Cond(1), TargetReg::Imm16, 3, 3));
        map.insert(0xd2, Opcode::new("JP", TargetReg::Cond(2), TargetReg::Imm16, 3, 3));
        map.insert(0xda, Opcode::new("JP", TargetReg::Cond(2), TargetReg::Imm16, 3, 3));

        // jp hl
        map.insert(0xd9, Opcode::new("JP", TargetReg::None, TargetReg::None, 1, 1));

        // jr n8
        map.insert(0x18, Opcode::new("JR", TargetReg::Imm8, TargetReg::None, 2, 3));

        // jr cc, n8
        map.insert(0x20, Opcode::new("JR", TargetReg::Cond(0), TargetReg::Imm8, 2, 2));
        map.insert(0x28, Opcode::new("JR", TargetReg::Cond(1), TargetReg::Imm8, 2, 2));
        map.insert(0x30, Opcode::new("JR", TargetReg::Cond(2), TargetReg::Imm8, 2, 2));
        map.insert(0x38, Opcode::new("JR", TargetReg::Cond(3), TargetReg::Imm8, 2, 2));

        // ld r8, r8
        map.insert(0x40, Opcode::new("LD", TargetReg::R8(0), TargetReg::R8(0), 1, 1));
        map.insert(0x41, Opcode::new("LD", TargetReg::R8(0), TargetReg::R8(1), 1, 1));
        map.insert(0x42, Opcode::new("LD", TargetReg::R8(0), TargetReg::R8(2), 1, 1));
        map.insert(0x43, Opcode::new("LD", TargetReg::R8(0), TargetReg::R8(3), 1, 1));
        map.insert(0x44, Opcode::new("LD", TargetReg::R8(0), TargetReg::R8(4), 1, 1));
        map.insert(0x45, Opcode::new("LD", TargetReg::R8(0), TargetReg::R8(5), 1, 1));
        map.insert(0x46, Opcode::new("LD", TargetReg::R8(0), TargetReg::R8(6), 1, 2));
        map.insert(0x47, Opcode::new("LD", TargetReg::R8(0), TargetReg::R8(7), 1, 1));

        map.insert(0x48, Opcode::new("LD", TargetReg::R8(1), TargetReg::R8(0), 1, 1));
        map.insert(0x49, Opcode::new("LD", TargetReg::R8(1), TargetReg::R8(1), 1, 1));
        map.insert(0x4a, Opcode::new("LD", TargetReg::R8(1), TargetReg::R8(2), 1, 1));
        map.insert(0x4b, Opcode::new("LD", TargetReg::R8(1), TargetReg::R8(3), 1, 1));
        map.insert(0x4c, Opcode::new("LD", TargetReg::R8(1), TargetReg::R8(4), 1, 1));
        map.insert(0x4d, Opcode::new("LD", TargetReg::R8(1), TargetReg::R8(5), 1, 1));
        map.insert(0x4e, Opcode::new("LD", TargetReg::R8(1), TargetReg::R8(6), 1, 2));
        map.insert(0x4f, Opcode::new("LD", TargetReg::R8(1), TargetReg::R8(7), 1, 1));

        map.insert(0x50, Opcode::new("LD", TargetReg::R8(2), TargetReg::R8(0), 1, 1));
        map.insert(0x51, Opcode::new("LD", TargetReg::R8(2), TargetReg::R8(1), 1, 1));
        map.insert(0x52, Opcode::new("LD", TargetReg::R8(2), TargetReg::R8(2), 1, 1));
        map.insert(0x53, Opcode::new("LD", TargetReg::R8(2), TargetReg::R8(3), 1, 1));
        map.insert(0x54, Opcode::new("LD", TargetReg::R8(2), TargetReg::R8(4), 1, 1));
        map.insert(0x55, Opcode::new("LD", TargetReg::R8(2), TargetReg::R8(5), 1, 1));
        map.insert(0x56, Opcode::new("LD", TargetReg::R8(2), TargetReg::R8(6), 1, 2));
        map.insert(0x57, Opcode::new("LD", TargetReg::R8(2), TargetReg::R8(7), 1, 1));

        map.insert(0x58, Opcode::new("LD", TargetReg::R8(3), TargetReg::R8(0), 1, 1));
        map.insert(0x59, Opcode::new("LD", TargetReg::R8(3), TargetReg::R8(1), 1, 1));
        map.insert(0x5a, Opcode::new("LD", TargetReg::R8(3), TargetReg::R8(2), 1, 1));
        map.insert(0x5b, Opcode::new("LD", TargetReg::R8(3), TargetReg::R8(3), 1, 1));
        map.insert(0x5c, Opcode::new("LD", TargetReg::R8(3), TargetReg::R8(4), 1, 1));
        map.insert(0x5d, Opcode::new("LD", TargetReg::R8(3), TargetReg::R8(5), 1, 1));
        map.insert(0x5e, Opcode::new("LD", TargetReg::R8(3), TargetReg::R8(6), 1, 2));
        map.insert(0x5f, Opcode::new("LD", TargetReg::R8(3), TargetReg::R8(7), 1, 1));

        map.insert(0x60, Opcode::new("LD", TargetReg::R8(4), TargetReg::R8(0), 1, 1));
        map.insert(0x61, Opcode::new("LD", TargetReg::R8(4), TargetReg::R8(1), 1, 1));
        map.insert(0x62, Opcode::new("LD", TargetReg::R8(4), TargetReg::R8(2), 1, 1));
        map.insert(0x63, Opcode::new("LD", TargetReg::R8(4), TargetReg::R8(3), 1, 1));
        map.insert(0x64, Opcode::new("LD", TargetReg::R8(4), TargetReg::R8(4), 1, 1));
        map.insert(0x65, Opcode::new("LD", TargetReg::R8(4), TargetReg::R8(5), 1, 1));
        map.insert(0x66, Opcode::new("LD", TargetReg::R8(4), TargetReg::R8(6), 1, 2));
        map.insert(0x67, Opcode::new("LD", TargetReg::R8(4), TargetReg::R8(7), 1, 1));

        map.insert(0x68, Opcode::new("LD", TargetReg::R8(5), TargetReg::R8(0), 1, 1));
        map.insert(0x69, Opcode::new("LD", TargetReg::R8(5), TargetReg::R8(1), 1, 1));
        map.insert(0x6a, Opcode::new("LD", TargetReg::R8(5), TargetReg::R8(2), 1, 1));
        map.insert(0x6b, Opcode::new("LD", TargetReg::R8(5), TargetReg::R8(3), 1, 1));
        map.insert(0x6c, Opcode::new("LD", TargetReg::R8(5), TargetReg::R8(4), 1, 1));
        map.insert(0x6d, Opcode::new("LD", TargetReg::R8(5), TargetReg::R8(5), 1, 1));
        map.insert(0x6e, Opcode::new("LD", TargetReg::R8(5), TargetReg::R8(6), 1, 2));
        map.insert(0x6f, Opcode::new("LD", TargetReg::R8(5), TargetReg::R8(7), 1, 1));

        map.insert(0x70, Opcode::new("LD", TargetReg::R8(6), TargetReg::R8(0), 1, 2));
        map.insert(0x71, Opcode::new("LD", TargetReg::R8(6), TargetReg::R8(1), 1, 2));
        map.insert(0x72, Opcode::new("LD", TargetReg::R8(6), TargetReg::R8(2), 1, 2));
        map.insert(0x73, Opcode::new("LD", TargetReg::R8(6), TargetReg::R8(3), 1, 2));
        map.insert(0x74, Opcode::new("LD", TargetReg::R8(6), TargetReg::R8(4), 1, 2));
        map.insert(0x75, Opcode::new("LD", TargetReg::R8(6), TargetReg::R8(5), 1, 2));
        //map.insert(0x76, Opcode::new("LD", TargetReg::R8(6), TargetReg::R8(6), 1, 2)); 0x76 is halt opcode
        map.insert(0x77, Opcode::new("LD", TargetReg::R8(6), TargetReg::R8(7), 1, 2));

        map.insert(0x78, Opcode::new("LD", TargetReg::R8(7), TargetReg::R8(0), 1, 1));
        map.insert(0x79, Opcode::new("LD", TargetReg::R8(7), TargetReg::R8(1), 1, 1));
        map.insert(0x7a, Opcode::new("LD", TargetReg::R8(7), TargetReg::R8(2), 1, 1));
        map.insert(0x7b, Opcode::new("LD", TargetReg::R8(7), TargetReg::R8(3), 1, 1));
        map.insert(0x7c, Opcode::new("LD", TargetReg::R8(7), TargetReg::R8(4), 1, 1));
        map.insert(0x7d, Opcode::new("LD", TargetReg::R8(7), TargetReg::R8(5), 1, 1));
        map.insert(0x7e, Opcode::new("LD", TargetReg::R8(7), TargetReg::R8(6), 1, 2));
        map.insert(0x7f, Opcode::new("LD", TargetReg::R8(7), TargetReg::R8(7), 1, 1));

        // ld r8, imm8
        map.insert(0x06, Opcode::new("LD", TargetReg::R8(0), TargetReg::Imm8, 2, 2));
        map.insert(0x0e, Opcode::new("LD", TargetReg::R8(1), TargetReg::Imm8, 2, 2));
        map.insert(0x16, Opcode::new("LD", TargetReg::R8(2), TargetReg::Imm8, 2, 2));
        map.insert(0x1e, Opcode::new("LD", TargetReg::R8(3), TargetReg::Imm8, 2, 2));
        map.insert(0x26, Opcode::new("LD", TargetReg::R8(4), TargetReg::Imm8, 2, 2));
        map.insert(0x2e, Opcode::new("LD", TargetReg::R8(5), TargetReg::Imm8, 2, 2));
        map.insert(0x36, Opcode::new("LD", TargetReg::R8(6), TargetReg::Imm8, 2, 3));
        map.insert(0x3e, Opcode::new("LD", TargetReg::R8(7), TargetReg::Imm8, 2, 2));

        // ld r16, imm16
        map.insert(0x01, Opcode::new("LD", TargetReg::R16(0), TargetReg::Imm16, 3, 3));
        map.insert(0x11, Opcode::new("LD", TargetReg::R16(1), TargetReg::Imm16, 3, 3));
        map.insert(0x21, Opcode::new("LD", TargetReg::R16(2), TargetReg::Imm16, 3, 3));
        map.insert(0x31, Opcode::new("LD", TargetReg::R16(3), TargetReg::Imm16, 3, 3));

        // ld [r16mem], a
        map.insert(0x02, Opcode::new("LD", TargetReg::R16mem(0), TargetReg::A, 1, 2));
        map.insert(0x12, Opcode::new("LD", TargetReg::R16mem(1), TargetReg::A, 1, 2));
        map.insert(0x22, Opcode::new("LD", TargetReg::R16mem(2), TargetReg::A, 1, 2));
        map.insert(0x32, Opcode::new("LD", TargetReg::R16mem(3), TargetReg::A, 1, 2));

        // ldh [c], a
        map.insert(0xe2, Opcode::new("LDH", TargetReg::C, TargetReg::A, 1, 2));

        // ld a, [r16mem]
        map.insert(0x0a, Opcode::new("LD", TargetReg::A, TargetReg::R16mem(0), 1, 2));
        map.insert(0x1a, Opcode::new("LD", TargetReg::A, TargetReg::R16mem(1), 1, 2));
        map.insert(0x2a, Opcode::new("LD", TargetReg::A, TargetReg::R16mem(2), 1, 2));
        map.insert(0x3a, Opcode::new("LD", TargetReg::A, TargetReg::R16mem(3), 1, 2));

        // ld a, [imm16]
        map.insert(0xfa, Opcode::new("LD", TargetReg::A, TargetReg::Ptr, 3, 4));

        // ldh [imm8], a
        map.insert(0xe0, Opcode::new("LDH", TargetReg::Imm8, TargetReg::A, 2, 3));

        // ld [imm16], a
        map.insert(0xea, Opcode::new("LD", TargetReg::Ptr, TargetReg::A, 3, 4));

        // ldh a, [imm8]
        map.insert(0xf0, Opcode::new("LDH", TargetReg::A, TargetReg::Imm8, 2, 3));

        // ldh a, [c]
        map.insert(0xf2, Opcode::new("LDH", TargetReg::A, TargetReg::C, 1, 2));

        // ld [imm16], sp
        map.insert(0x08, Opcode::new("LD", TargetReg::Imm16, TargetReg::SP, 3, 5));

        // ld hl, sp + imm8
        map.insert(0xf8, Opcode::new("LD", TargetReg::R16(2), TargetReg::Imm8, 2, 3));

        // ld sp, hl
        map.insert(0xf9, Opcode::new("LD", TargetReg::SP, TargetReg::R16(2), 1, 2));

        // NOP
        map.insert(0x00, Opcode::new("NOP", TargetReg::None, TargetReg::None, 1, 1));

        // or a, r8
        map.insert(0xb0, Opcode::new("OR", TargetReg::A, TargetReg::R8(0), 1, 1));
        map.insert(0xb1, Opcode::new("OR", TargetReg::A, TargetReg::R8(1), 1, 1));
        map.insert(0xb2, Opcode::new("OR", TargetReg::A, TargetReg::R8(2), 1, 1));
        map.insert(0xb3, Opcode::new("OR", TargetReg::A, TargetReg::R8(3), 1, 1));
        map.insert(0xb4, Opcode::new("OR", TargetReg::A, TargetReg::R8(4), 1, 1));
        map.insert(0xb5, Opcode::new("OR", TargetReg::A, TargetReg::R8(5), 1, 1));
        map.insert(0xb6, Opcode::new("OR", TargetReg::A, TargetReg::R8(6), 1, 2)); // or a, [hl]
        map.insert(0xb7, Opcode::new("OR", TargetReg::A, TargetReg::R8(7), 1, 1));

        // or a, n8
        map.insert(0xf6, Opcode::new("OR", TargetReg::A, TargetReg::Imm8, 2, 2));

        // pop r16stk
        map.insert(0xc1, Opcode::new("POP", TargetReg::R16stk(0), TargetReg::None, 1, 3));
        map.insert(0xd1, Opcode::new("POP", TargetReg::R16stk(1), TargetReg::None, 1, 3));
        map.insert(0xe1, Opcode::new("POP", TargetReg::R16stk(2), TargetReg::None, 1, 3));
        map.insert(0xf1, Opcode::new("POP", TargetReg::R16stk(3), TargetReg::None, 1, 3));

        // push r16stk
        map.insert(0xc5, Opcode::new("PUSH", TargetReg::R16stk(0), TargetReg::None, 1, 4));
        map.insert(0xd5, Opcode::new("PUSH", TargetReg::R16stk(1), TargetReg::None, 1, 4));
        map.insert(0xe5, Opcode::new("PUSH", TargetReg::R16stk(2), TargetReg::None, 1, 4));
        map.insert(0xf5, Opcode::new("PUSH", TargetReg::R16stk(3), TargetReg::None, 1, 4));

        // ret
        map.insert(0xc9, Opcode::new("RET", TargetReg::None, TargetReg::None, 1, 4));

        // ret cc
        map.insert(0xc0, Opcode::new("RET", TargetReg::Cond(0), TargetReg::None, 1, 2));
        map.insert(0xc8, Opcode::new("RET", TargetReg::Cond(1), TargetReg::None, 1, 2));
        map.insert(0xd0, Opcode::new("RET", TargetReg::Cond(2), TargetReg::None, 1, 2));
        map.insert(0xd8, Opcode::new("RET", TargetReg::Cond(3), TargetReg::None, 1, 2));

        // reti
        map.insert(0xd9, Opcode::new("RETI", TargetReg::None, TargetReg::None, 1, 4));

        // rla
        map.insert(0x17, Opcode::new("RLA", TargetReg::None, TargetReg::None, 1, 1));

        // rlca
        map.insert(0x07, Opcode::new("RLCA", TargetReg::None, TargetReg::None, 1, 1));

        // rra
        map.insert(0x1f, Opcode::new("RRA", TargetReg::None, TargetReg::None, 1, 1));

        // rrca
        map.insert(0x0f, Opcode::new("RRCA", TargetReg::None, TargetReg::None, 1, 1));

        // rst tgt3
        map.insert(0xc7, Opcode::new("RST", TargetReg::Tgt3(0), TargetReg::None, 1, 4));
        map.insert(0xcf, Opcode::new("RST", TargetReg::Tgt3(1), TargetReg::None, 1, 4));
        map.insert(0xd7, Opcode::new("RST", TargetReg::Tgt3(2), TargetReg::None, 1, 4));
        map.insert(0xdf, Opcode::new("RST", TargetReg::Tgt3(3), TargetReg::None, 1, 4));
        map.insert(0xe7, Opcode::new("RST", TargetReg::Tgt3(4), TargetReg::None, 1, 4));
        map.insert(0xef, Opcode::new("RST", TargetReg::Tgt3(5), TargetReg::None, 1, 4));
        map.insert(0xf7, Opcode::new("RST", TargetReg::Tgt3(6), TargetReg::None, 1, 4));
        map.insert(0xff, Opcode::new("RST", TargetReg::Tgt3(7), TargetReg::None, 1, 4));

        // sbc a, r8
        map.insert(0x98, Opcode::new("SBC", TargetReg::A, TargetReg::R8(0), 1, 1));
        map.insert(0x99, Opcode::new("SBC", TargetReg::A, TargetReg::R8(1), 1, 1));
        map.insert(0x9a, Opcode::new("SBC", TargetReg::A, TargetReg::R8(2), 1, 1));
        map.insert(0x9b, Opcode::new("SBC", TargetReg::A, TargetReg::R8(3), 1, 1));
        map.insert(0x9c, Opcode::new("SBC", TargetReg::A, TargetReg::R8(4), 1, 1));
        map.insert(0x9d, Opcode::new("SBC", TargetReg::A, TargetReg::R8(5), 1, 1));
        map.insert(0x9e, Opcode::new("SBC", TargetReg::A, TargetReg::R8(6), 1, 2)); // sbc a, [hl]
        map.insert(0x9f, Opcode::new("SBC", TargetReg::A, TargetReg::R8(7), 1, 1));

        // sbc a, imm8
        map.insert(0xde, Opcode::new("SBC", TargetReg::A, TargetReg::Imm8, 2, 2));

        // scf
        map.insert(0x37, Opcode::new("SCF", TargetReg::None, TargetReg::None, 1, 1));

        // stop
        map.insert(0x10, Opcode::new("STOP", TargetReg::None, TargetReg::None, 2, 0));

        // sub a, r8
        map.insert(0x90, Opcode::new("SUB", TargetReg::A, TargetReg::R8(0), 1, 1));
        map.insert(0x91, Opcode::new("SUB", TargetReg::A, TargetReg::R8(1), 1, 1));
        map.insert(0x92, Opcode::new("SUB", TargetReg::A, TargetReg::R8(2), 1, 1));
        map.insert(0x93, Opcode::new("SUB", TargetReg::A, TargetReg::R8(3), 1, 1));
        map.insert(0x94, Opcode::new("SUB", TargetReg::A, TargetReg::R8(4), 1, 1));
        map.insert(0x95, Opcode::new("SUB", TargetReg::A, TargetReg::R8(5), 1, 1));
        map.insert(0x96, Opcode::new("SUB", TargetReg::A, TargetReg::R8(6), 1, 2)); // sub a, [hl]
        map.insert(0x97, Opcode::new("SUB", TargetReg::A, TargetReg::R8(7), 1, 1));

        // sub a, imm8
        map.insert(0xd6, Opcode::new("SUB", TargetReg::A, TargetReg::Imm8, 2, 2));

        // xor a, r8
        map.insert(0xa8, Opcode::new("XOR", TargetReg::A, TargetReg::R8(0), 1, 1));
        map.insert(0xa9, Opcode::new("XOR", TargetReg::A, TargetReg::R8(1), 1, 1));
        map.insert(0xaa, Opcode::new("XOR", TargetReg::A, TargetReg::R8(2), 1, 1));
        map.insert(0xab, Opcode::new("XOR", TargetReg::A, TargetReg::R8(3), 1, 1));
        map.insert(0xac, Opcode::new("XOR", TargetReg::A, TargetReg::R8(4), 1, 1));
        map.insert(0xad, Opcode::new("XOR", TargetReg::A, TargetReg::R8(5), 1, 1));
        map.insert(0xae, Opcode::new("XOR", TargetReg::A, TargetReg::R8(6), 1, 2)); // xor a, [hl]
        map.insert(0xaf, Opcode::new("XOR", TargetReg::A, TargetReg::R8(7), 1, 1));

        // xor a, n8
        map.insert(0xee, Opcode::new("XOR", TargetReg::A, TargetReg::Imm8, 2, 2));

        map
    };
}
