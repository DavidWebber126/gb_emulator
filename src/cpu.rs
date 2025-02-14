use bitflags::bitflags;
use std::collections::HashMap;

use crate::bus::Bus;
use crate::opcodes::{self, TargetReg};

bitflags! {
    #[derive(PartialEq, Debug, Clone)]
    struct FlagsReg: u8 {
        // Zero Flag. Also called z flag. This bit is set if and only if the result of an operation is zero.
        const zero = 0b1000_0000;
        // Subtraction Flag. Also called n flag
        const subtraction = 0b0100_0000;
        // Half Carry Flag. Also called h flag
        const half_carry = 0b0010_0000;
        // Carry Flag. Also called c flag. https://gbdev.io/pandocs/CPU_Registers_and_Flags.html
        const carry = 0b0001_0000;
    }
}

pub struct Cpu {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    flags: FlagsReg,
    h: u8,
    l: u8,
    stack_pointer: u16,
    program_counter: u16,
    ime: bool,
    bus: Bus,
    next_op_prefixed: bool,
}

impl Cpu {
    pub fn new(bus: Bus) -> Self {
        Self {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            flags: FlagsReg::empty(),
            h: 0,
            l: 0,
            stack_pointer: 0xfffe,
            program_counter: 0x0100,
            ime: false,
            bus,
            next_op_prefixed: false,
        }
    }

    fn get_bc(&self) -> u16 {
        (self.b as u16) << 8 | self.c as u16
    }

    fn set_bc(&mut self, value: u16) {
        self.c = (value & 0xff) as u8;
        self.b = (value >> 8) as u8;
    }

    fn get_de(&self) -> u16 {
        (self.d as u16) << 8 | self.e as u16
    }

    fn set_de(&mut self, value: u16) {
        self.e = (value & 0xff) as u8;
        self.d = (value >> 8) as u8;
    }

    fn get_hl(&self) -> u16 {
        (self.h as u16) << 8 | self.l as u16
    }

    fn set_hl(&mut self, value: u16) {
        self.l = (value & 0xff) as u8;
        self.h = (value >> 8) as u8;
    }

    fn set_af(&mut self, value: u16) {
        self.a = (value & 0xff) as u8;
        self.flags = FlagsReg::from_bits_retain((value >> 8) as u8);
    }

    fn get_af(&self) -> u16 {
        (self.a as u16) << 8 | self.flags.bits() as u16
    }

    fn push_u8_to_stack(&mut self, val: u8) {
        self.bus.mem_write(self.stack_pointer, val);
        self.stack_pointer -= 1;
    }

    fn push_u16_to_stack(&mut self, addr: u16) {
        let [lo, hi] = addr.to_le_bytes();
        self.push_u8_to_stack(hi);
        self.push_u8_to_stack(lo);
    }

    // fn pop_u8_from_stack(&mut self) -> u8 {
    //     self.stack_pointer += 1;
    //     self.bus.mem_read(self.stack_pointer)
    // }

    fn pop_u16_from_stack(&mut self) -> u16 {
        let val = self.bus.mem_read_u16(self.stack_pointer + 1);
        self.stack_pointer += 2;
        val
    }

    fn reg_read(&mut self, target: &TargetReg) -> Option<u16> {
        match target {
            TargetReg::R8(reg) => Some(self.r8_read(*reg) as u16),
            TargetReg::R16(reg) => Some(self.r16_read(*reg)),
            TargetReg::R16stk(reg) => Some(self.r16stk_read(*reg)),
            TargetReg::R16mem(reg) => Some(self.r16mem_read(*reg)),
            TargetReg::Cond(code) => Some(*code as u16),
            TargetReg::Tgt3(reg) => Some(self.tgt3_read(*reg)),
            TargetReg::B3(bit) => Some(*bit as u16),
            TargetReg::A => Some(self.a as u16),
            TargetReg::SP => Some(self.stack_pointer),
            TargetReg::C => Some(self.bus.mem_read(0xff00 + self.c as u16) as u16),
            TargetReg::Imm16 => Some(self.bus.mem_read_u16(self.program_counter + 1)),
            TargetReg::Imm8 => Some(self.bus.mem_read(self.program_counter + 1) as u16),
            TargetReg::Ptr => {
                let addr = self.bus.mem_read_u16(self.program_counter + 1);
                Some(self.bus.mem_read(addr) as u16)
            }
            _ => panic!("{:?} is not implemented yet", target),
        }
    }

    fn r8_read(&mut self, reg: u8) -> u8 {
        match reg {
            0 => self.b,
            1 => self.c,
            2 => self.d,
            3 => self.e,
            4 => self.h,
            5 => self.l,
            6 => self.bus.mem_read(self.get_hl()),
            7 => self.a,
            _ => panic!("Invalid r8 Register: {}", reg),
        }
    }

    fn r16_read(&self, reg: u8) -> u16 {
        match reg {
            0 => self.get_bc(),
            1 => self.get_de(),
            2 => self.get_hl(),
            3 => self.stack_pointer,
            _ => panic!("Invalid r16 Register: {}", reg),
        }
    }

    fn r16stk_read(&self, reg: u8) -> u16 {
        match reg {
            0 => self.get_bc(),
            1 => self.get_de(),
            2 => self.get_hl(),
            3 => self.get_af(),
            _ => panic!("Invalid r16 Register: {}", reg),
        }
    }

    fn r16mem_read(&mut self, reg: u8) -> u16 {
        match reg {
            0 => {
                let addr = self.get_bc();
                self.bus.mem_read(addr) as u16
            }
            1 => {
                let addr = self.get_de();
                self.bus.mem_read(addr) as u16
            }
            2 => {
                let addr = self.get_hl();
                self.set_hl(addr.wrapping_add(1));
                self.bus.mem_read(addr) as u16
            }
            3 => {
                let addr = self.get_hl();
                self.set_hl(addr.wrapping_sub(1));
                self.bus.mem_read(addr) as u16
            }
            _ => panic!("Invalid r16 Register: {}", reg),
        }
    }

    fn tgt3_read(&mut self, reg: u8) -> u16 {
        match reg {
            0 => 0,
            1 => 0x0008,
            2 => 0x0010,
            3 => 0x18,
            4 => 0x20,
            5 => 0x28,
            6 => 0x30,
            7 => 0x38,
            _ => panic!("Invalid tgt3 value: {}", reg),
        }
    }

    fn reg_write(&mut self, target: &TargetReg, data: u16) {
        match target {
            TargetReg::R8(reg) => self.r8_write(*reg, (data & 0xff) as u8),
            TargetReg::R16(reg) => self.r16_write(*reg, data),
            TargetReg::R16stk(reg) => self.r16stk_write(*reg, data),
            TargetReg::R16mem(reg) => self.r16mem_write(*reg, data),
            TargetReg::A => self.a = (data & 0xff) as u8,
            TargetReg::SP => self.stack_pointer = data,
            TargetReg::C => self
                .bus
                .mem_write(0xff00 + self.c as u16, (data & 0xff) as u8),
            TargetReg::Ptr => {
                let addr = self.bus.mem_read_u16(self.program_counter + 1);
                self.bus.mem_write(addr, data as u8);
            }
            TargetReg::Imm16 => {
                let addr = self.bus.mem_read_u16(self.program_counter + 1);
                self.bus.mem_write_u16(addr, data);
            }
            _ => panic!("{:?} is not implemented yet", target),
        }
    }

    fn r8_write(&mut self, reg: u8, value: u8) {
        match reg {
            0 => self.b = value,
            1 => self.c = value,
            2 => self.d = value,
            3 => self.e = value,
            4 => self.h = value,
            5 => self.l = value,
            6 => {
                self.bus.mem_write(self.get_hl(), value);
            }
            7 => self.a = value,
            _ => panic!("Impossible State. No reg value {}", reg),
        }
    }

    fn r16_write(&mut self, reg: u8, value: u16) {
        match reg {
            0 => self.set_bc(value),
            1 => self.set_de(value),
            2 => self.set_hl(value),
            3 => self.stack_pointer = value,
            _ => panic!("Invalid State. No r16 value {}", reg),
        }
    }

    fn r16stk_write(&mut self, reg: u8, value: u16) {
        match reg {
            0 => self.set_bc(value),
            1 => self.set_de(value),
            2 => self.set_hl(value),
            3 => self.set_af(value),
            _ => panic!("Invalid State. No r16stk value {}", reg),
        }
    }

    fn r16mem_write(&mut self, reg: u8, value: u16) {
        match reg {
            0 => {
                self.bus.mem_write(self.get_bc(), value as u8);
            }
            1 => {
                self.bus.mem_write(self.get_de(), value as u8);
            }
            2 => {
                let addr = self.get_hl();
                self.bus.mem_write(addr, (value & 0xff) as u8);
                self.set_hl(addr.wrapping_add(1));
            }
            3 => {
                let addr = self.get_hl();
                self.bus.mem_write(addr, (value & 0xff) as u8);
                self.set_hl(addr.wrapping_sub(1));
            }
            _ => panic!("Invalid State. No r16mem value {}", reg),
        }
    }

    // Main CPU loop. Fetch instruction, decode and execute.
    pub fn run(&mut self) {
        loop {
            let (result, _cycles, bytes) = if self.next_op_prefixed {
                let opcodes: &HashMap<u8, opcodes::Opcode> = &opcodes::CPU_PREFIXED_OP_CODES;
                let opcode_num = self.bus.mem_read(self.program_counter);
                let opcode = opcodes.get(&opcode_num).unwrap();

                (
                    self.prefixed_opcodes(opcode_num, opcode),
                    opcode.cycles,
                    opcode.bytes,
                )
            } else {
                let opcodes: &HashMap<u8, opcodes::Opcode> = &opcodes::CPU_OP_CODES;
                let opcode_num = self.bus.mem_read(self.program_counter);
                let opcode = opcodes.get(&opcode_num).unwrap();

                (
                    self.non_prefixed_opcodes(opcode_num, opcode),
                    opcode.cycles,
                    opcode.bytes,
                )
            };

            if result.is_err() {
                break;
            }

            self.program_counter = self.program_counter.wrapping_add(bytes);
        }
    }

    fn prefixed_opcodes(&mut self, byte: u8, opcode: &opcodes::Opcode) -> Result<(), &str> {
        match byte {
            // bit u3, r8
            0x40..=0x7f => {
                let bit = self.reg_read(&opcode.reg1).unwrap() as u8;
                let val = self.reg_read(&opcode.reg2).unwrap() as u8;
                self.flags.set(FlagsReg::zero, ((val >> bit) & 0b1) == 0);
                self.flags.set(FlagsReg::subtraction, false);
                self.flags.set(FlagsReg::half_carry, true);
            }
            // res u3, r8
            0x80..=0xbf => {
                let bit = self.reg_read(&opcode.reg1).unwrap() as u8;
                let val = self.reg_read(&opcode.reg2).unwrap() as u8;
                self.reg_write(&opcode.reg2, (val & !(0x01 << bit)) as u16);
            }
            // rl r8
            0x10..=0x17 => {
                let mut val = self.reg_read(&opcode.reg1).unwrap() as u8;
                let left_bit = (val & 0x80) != 0x00;
                let carry = self.flags.contains(FlagsReg::carry);
                val <<= 1;
                val += carry as u8;
                self.reg_write(&opcode.reg1, val as u16);
                self.flags.set(FlagsReg::zero, val == 0);
                self.flags.set(FlagsReg::carry, left_bit);
            }
            // rlc r8
            0x00..=0x07 => {
                let mut val = self.reg_read(&opcode.reg1).unwrap() as u8;
                let left_bit = (val & 0x80) != 0x00;
                val <<= 1;
                val += left_bit as u8;
                self.reg_write(&opcode.reg1, val as u16);
                self.flags.set(FlagsReg::zero, val == 0);
                self.flags.set(FlagsReg::carry, left_bit);
            }
            // rr r8
            0x18..=0x1f => {
                let mut val = self.reg_read(&opcode.reg1).unwrap() as u8;
                let right_bit = (val & 0x01) != 0;
                let carry = self.flags.contains(FlagsReg::carry);
                val >>= 1;
                val += (carry as u8) << 7;
                self.reg_write(&opcode.reg1, val as u16);
                self.flags.set(FlagsReg::zero, val == 0);
                self.flags.set(FlagsReg::carry, right_bit);
            }
            // rrc r8
            0x08..=0x0f => {
                let mut val = self.reg_read(&opcode.reg1).unwrap() as u8;
                let right_bit = (val & 0x01) != 0;
                val >>= 1;
                val += (right_bit as u8) << 7;
                self.reg_write(&opcode.reg1, val as u16);
                self.flags.set(FlagsReg::zero, val == 0);
                self.flags.set(FlagsReg::carry, right_bit);
            }
            // set u3, r8
            0xc0..=0xff => {
                let bit = self.reg_read(&opcode.reg1).unwrap() as u8;
                let val = self.reg_read(&opcode.reg2).unwrap() as u8;
                self.reg_write(&opcode.reg2, (val | (0x1 << bit)) as u16);
            }
            // sla r8
            0x20..=0x27 => {
                let mut val = self.reg_read(&opcode.reg1).unwrap() as u8;
                let left_bit = val & 0x80 != 0;
                val <<= 1;
                self.reg_write(&opcode.reg1, val as u16);
                self.flags.set(FlagsReg::zero, val == 0);
                self.flags.set(FlagsReg::subtraction, false);
                self.flags.set(FlagsReg::half_carry, false);
                self.flags.set(FlagsReg::carry, left_bit);
            }
            // sra r8
            0x28..=0x2f => {
                let mut val = self.reg_read(&opcode.reg1).unwrap() as u8;
                let right_bit = val & 0x01 != 0;
                let left_bit = val & 0x80;
                val >>= 1;
                val |= left_bit;
                self.reg_write(&opcode.reg1, val as u16);
                self.flags.set(FlagsReg::zero, val == 0);
                self.flags.set(FlagsReg::subtraction, false);
                self.flags.set(FlagsReg::half_carry, false);
                self.flags.set(FlagsReg::carry, right_bit);
            }
            // srl r8
            0x38..=0x3f => {
                let mut val = self.reg_read(&opcode.reg1).unwrap() as u8;
                let right_bit = val & 0x01 != 0;
                val >>= 1;
                self.reg_write(&opcode.reg1, val as u16);
                self.flags.set(FlagsReg::zero, val == 0);
                self.flags.set(FlagsReg::subtraction, false);
                self.flags.set(FlagsReg::half_carry, false);
                self.flags.set(FlagsReg::carry, right_bit);
            }
            // swap r8
            0x30..=0x37 => {
                let val = self.reg_read(&opcode.reg1).unwrap() as u8;
                let lo = val & 0x0f;
                let hi = val & 0xf0;
                self.reg_write(&opcode.reg1, ((lo << 4) + (hi >> 4)) as u16);
                self.flags.set(FlagsReg::zero, val == 0);
                self.flags.set(FlagsReg::subtraction, false);
                self.flags.set(FlagsReg::half_carry, false);
                self.flags.set(FlagsReg::carry, false);
            }
        };
        Ok(())
    }

    fn non_prefixed_opcodes(&mut self, byte: u8, opcode: &opcodes::Opcode) -> Result<(), &str> {
        match byte {
            // 8 bit ADC
            0x88..=0x8f | 0xce => {
                let arg = self.reg_read(&opcode.reg2).unwrap() as u8;
                let sum = self.add_u8(self.a, arg, true);

                self.a = sum;
            }
            // 8 bit ADD
            0x80..=0x87 | 0xc6 | 0xe8 => {
                let arg1 = self.reg_read(&opcode.reg1).unwrap() as u8;
                let arg2 = self.reg_read(&opcode.reg2).unwrap() as u8;
                let sum = self.add_u8(arg1, arg2, false);

                self.reg_write(&opcode.reg1, sum as u16);
            }
            // 16 bit ADD
            0x09 | 0x19 | 0x29 | 0x39 => {
                let arg = self.reg_read(&opcode.reg2).unwrap();
                let sum = self.add_u16(self.get_hl(), arg, false);

                self.set_hl(sum);
            }
            // 8 bit AND
            0xa0..=0xa7 | 0xe6 => {
                let arg = self.reg_read(&opcode.reg2).unwrap() as u8;
                self.a &= arg;

                self.flags.set(FlagsReg::zero, self.a == 0);
                self.flags.remove(FlagsReg::subtraction);
                self.flags.insert(FlagsReg::half_carry);
                self.flags.remove(FlagsReg::carry);
            }
            // CALL
            0xcd => {
                let addr = self.reg_read(&opcode.reg1).unwrap();
                self.call(addr);
            }
            // CALL cc
            0xc4 | 0xcc | 0xd4 | 0xdc => {
                let condition = self.reg_read(&opcode.reg1).unwrap();
                let should_execute = match condition {
                    0 => !self.flags.contains(FlagsReg::zero), // Cond(0) => zero flags is not set
                    1 => self.flags.contains(FlagsReg::zero),  // Cond(1) => zero flag is set
                    2 => !self.flags.contains(FlagsReg::carry), // Cond(3) => carry flag is set
                    3 => self.flags.contains(FlagsReg::carry), // Cond(3) => carry flag is set
                    _ => panic!("Condition Codes are 0-3. Received {}", condition),
                };
                if should_execute {
                    // inc cycle count
                    // self.cycles += 1;
                    let addr = self.reg_read(&opcode.reg2).unwrap();
                    self.call(addr);
                }
            }
            // CCF
            0x3f => {
                self.flags.toggle(FlagsReg::carry);
            }
            // 8 bit CP
            0xb8..=0xbf | 0xfe => {
                let val = (self.reg_read(&opcode.reg2).unwrap() & 0x0f) as u8;
                let _result = self.sub_u8(self.a, val, false);
            }
            // CPL
            0x2f => {
                self.a = !self.a;
                self.flags.insert(FlagsReg::subtraction);
                self.flags.insert(FlagsReg::half_carry);
            }
            // DAA
            0x27 => {
                let mut should_carry = self.flags.contains(FlagsReg::carry);
                let sub_flag = self.flags.contains(FlagsReg::subtraction);
                if sub_flag {
                    let mut adjust = 0;
                    adjust += 0x06 * (self.flags.contains(FlagsReg::half_carry) as u8);
                    adjust += 0x60 * (self.flags.contains(FlagsReg::carry) as u8);
                    self.a = self.sub_u8(self.a, adjust, false);
                } else {
                    let mut adjust = 0;
                    if self.flags.contains(FlagsReg::half_carry) || self.a & 0x0f > 0x09 {
                        adjust += 0x06;
                    }
                    if self.flags.contains(FlagsReg::carry) || self.a > 0x99 {
                        adjust += 0x60;
                        should_carry = true;
                    }
                    self.a += adjust;
                }
                self.flags.set(FlagsReg::zero, self.a == 0);
                self.flags.set(FlagsReg::subtraction, sub_flag);
                self.flags.set(FlagsReg::half_carry, false);
                self.flags.set(FlagsReg::carry, should_carry);
            }
            // 8 bit DEC
            0x05 | 0x0d | 0x15 | 0x1d | 0x25 | 0x2d | 0x35 | 0x3d => {
                let mut val = self.reg_read(&opcode.reg1).unwrap();
                let half_carry = ((val & 0x0f) - 1) & 0x10 > 0;
                val = val.wrapping_sub(1);
                self.reg_write(&opcode.reg1, val);
                self.flags.set(FlagsReg::zero, val == 0);
                self.flags.insert(FlagsReg::subtraction);
                self.flags.set(FlagsReg::half_carry, half_carry);
            }
            // 16 bit DEC
            0x0b | 0x1b | 0x2b | 0x3b => {
                let mut val = self.reg_read(&opcode.reg1).unwrap();
                val = val.wrapping_sub(1);
                self.reg_write(&opcode.reg1, val);
            }
            // DI
            0xf3 => {
                self.ime = false;
            }
            // EI
            0xfb => {
                self.ime = true;
            }
            // HALT
            0x76 => return Err("HALT Opcode Reached"),
            // 8 bit INC
            0x04 | 0x0c | 0x14 | 0x1c | 0x24 | 0x2c | 0x34 | 0x3c => {
                let mut val = self.reg_read(&opcode.reg1).unwrap() as u8;
                let half_carry = val & 0x0f == 0x0f;
                val = val.wrapping_add(1);
                self.reg_write(&opcode.reg1, val as u16);

                self.flags.set(FlagsReg::zero, val == 0);
                self.flags.remove(FlagsReg::subtraction);
                self.flags.set(FlagsReg::half_carry, half_carry);
            }
            // 16 bit INC
            0x03 | 0x13 | 0x23 | 0x33 => {
                let mut val = self.reg_read(&opcode.reg1).unwrap();
                val = val.wrapping_add(1);
                self.reg_write(&opcode.reg1, val);
            }
            // JP
            0xc3 => {
                let addr = self.reg_read(&opcode.reg1).unwrap();
                self.program_counter = addr - 3; // Subtract 3 bytes to account for the addition of 3 bytes from the JP opcode
            }
            // JP cc
            0xc2 | 0xca | 0xd2 | 0xda => {
                let condition = self.reg_read(&opcode.reg1).unwrap();
                let should_execute = match condition {
                    0 => !self.flags.contains(FlagsReg::zero), // Cond(0) => zero flags is not set
                    1 => self.flags.contains(FlagsReg::zero),  // Cond(1) => zero flag is set
                    2 => !self.flags.contains(FlagsReg::carry), // Cond(3) => carry flag is set
                    3 => self.flags.contains(FlagsReg::carry), // Cond(3) => carry flag is set
                    _ => panic!("Condition Codes are 0-3. Received {}", condition),
                };
                if should_execute {
                    // inc cycle count
                    // self.cycles += 1;
                    self.program_counter = self.reg_read(&opcode.reg2).unwrap() - 3;
                }
            }
            // JR imm8
            0x18 => {
                let offset = self.reg_read(&opcode.reg1).unwrap() as u8;
                self.program_counter = self.program_counter.wrapping_add_signed(offset as i16);
                self.program_counter -= 2; // subtract 2 to account for the opcodes bytes
            }
            // JR cc
            0x20 | 0x28 | 0x30 | 0x38 => {
                let offset = self.reg_read(&opcode.reg2).unwrap() as u8;
                let condition = self.reg_read(&opcode.reg1).unwrap();
                let should_execute = match condition {
                    0 => !self.flags.contains(FlagsReg::zero), // Cond(0) => zero flags is not set
                    1 => self.flags.contains(FlagsReg::zero),  // Cond(1) => zero flag is set
                    2 => !self.flags.contains(FlagsReg::carry), // Cond(3) => carry flag is set
                    3 => self.flags.contains(FlagsReg::carry), // Cond(3) => carry flag is set
                    _ => panic!("Condition Codes are 0-3. Received {}", condition),
                };
                if should_execute {
                    // inc cycle count
                    // self.cycles += 1;
                    self.program_counter = self.program_counter.wrapping_add_signed(offset as i16);
                    self.program_counter -= 2; // subtract 2 to account for the opcodes bytes
                }
            }
            // 8 bit LD r8 to r8
            0x40..=0x75 | 0x77..=0x7f => {
                let value = self.reg_read(&opcode.reg2).unwrap();
                self.reg_write(&opcode.reg1, value);
            }
            // 16 bit LD
            0x01 | 0x11 | 0x21 | 0x31 | 0xfa | 0xea | 0x08 | 0xf9 => {
                let value = self.reg_read(&opcode.reg2).unwrap();
                self.reg_write(&opcode.reg1, value);
            }
            // 8 bit LD r16mem
            0x02 | 0x12 | 0x22 | 0x32 | 0x0a | 0x1a | 0x2a | 0x3a => {
                let value = self.reg_read(&opcode.reg2).unwrap();
                self.reg_write(&opcode.reg1, value);
            }

            // 8 bit imm ld
            0x06 | 0x0e | 0x16 | 0x1e | 0x26 | 0x2e | 0x36 | 0x3e => {
                let value = self.reg_read(&opcode.reg2).unwrap();
                self.reg_write(&opcode.reg1, value);
            }
            // ld hl, sp + imm8
            0xf8 => {
                let offset = self.reg_read(&opcode.reg2).unwrap() as u8;
                let sum = self.add_u8(self.stack_pointer as u8, offset, false);
                println!("Sum: {:02X}", sum);
                println!("Val: {:04X}", 0xff00 + sum as u16);
                self.set_hl((self.stack_pointer & 0xff00) + sum as u16);
                self.flags.set(FlagsReg::zero, false);
                self.flags.set(FlagsReg::subtraction, false);
            }
            // 8 bit LDH
            0xe2 | 0xe0 | 0xf0 | 0xf2 => {
                let value = self.reg_read(&opcode.reg2).unwrap();
                self.reg_write(&opcode.reg1, value);
            }
            // NOP
            0x00 => {
                // do nothing
            }
            // 8 bit OR
            0xb0..=0xb7 | 0xf6 => {
                let val = self.reg_read(&opcode.reg2).unwrap() as u8;
                self.a |= val;

                self.flags.set(FlagsReg::zero, self.a == 0);
                self.flags.remove(FlagsReg::subtraction);
                self.flags.remove(FlagsReg::half_carry);
                self.flags.remove(FlagsReg::carry);
            }
            // POP
            0xc1 | 0xd1 | 0xe1 | 0xf1 => {
                let val = self.pop_u16_from_stack();
                self.reg_write(&opcode.reg1, val);
            }
            // PUSH
            0xc5 | 0xd5 | 0xe5 | 0xf5 => {
                let val = self.reg_read(&opcode.reg1).unwrap();
                self.push_u16_to_stack(val);
            }
            // RET
            0xc9 => {
                self.program_counter = self.pop_u16_from_stack() - 1; // minus 1 to account for the added byte
            }
            // RET cc
            0xc0 | 0xc8 | 0xd0 | 0xd8 => {
                let condition = self.reg_read(&opcode.reg1).unwrap();
                let should_execute = match condition {
                    0 => !self.flags.contains(FlagsReg::zero), // Cond(0) => zero flags is not set
                    1 => self.flags.contains(FlagsReg::zero),  // Cond(1) => zero flag is set
                    2 => !self.flags.contains(FlagsReg::carry), // Cond(3) => carry flag is set
                    3 => self.flags.contains(FlagsReg::carry), // Cond(3) => carry flag is set
                    _ => panic!("Condition Codes are 0-3. Received {}", condition),
                };
                if should_execute {
                    // inc cycle count
                    // self.cycles += 1;
                    self.program_counter = self.pop_u16_from_stack() - 1; // minus 1 to account for the added byte
                }
            }
            // RETI
            0xd9 => {
                self.program_counter = self.pop_u16_from_stack() - 1;
                self.ime = true;
            }
            // RLA
            0x17 => {
                let left_bit_set = self.a & 0b1000_0000 != 0;
                self.a <<= 1;
                self.a += self.flags.contains(FlagsReg::carry) as u8; // carry bit goes into bit 0
                self.flags.remove(FlagsReg::zero);
                self.flags.remove(FlagsReg::subtraction);
                self.flags.remove(FlagsReg::half_carry);
                self.flags.set(FlagsReg::carry, left_bit_set);
            }
            // RLCA
            0x07 => {
                let left_bit_set = self.a & 0b1000_0000 != 0;
                self.a <<= 1;
                self.a += left_bit_set as u8; // left bit goes into bit 0
                self.flags.remove(FlagsReg::zero);
                self.flags.remove(FlagsReg::subtraction);
                self.flags.remove(FlagsReg::half_carry);
                self.flags.set(FlagsReg::carry, left_bit_set);
            }
            // RRA
            0x1f => {
                let right_bit_set = self.a & 0b1 != 0;
                self.a >>= 1;
                self.a += (self.flags.contains(FlagsReg::carry) as u8) << 7;
                self.flags.remove(FlagsReg::zero);
                self.flags.remove(FlagsReg::subtraction);
                self.flags.remove(FlagsReg::half_carry);
                self.flags.set(FlagsReg::carry, right_bit_set);
            }
            // RRCA
            0x0f => {
                let right_bit_set = self.a & 0b1 != 0;
                self.a >>= 1;
                self.a += (right_bit_set as u8) << 7;
                self.flags.remove(FlagsReg::zero);
                self.flags.remove(FlagsReg::subtraction);
                self.flags.remove(FlagsReg::half_carry);
                self.flags.set(FlagsReg::carry, right_bit_set);
            }
            // RST
            0xc7 | 0xcf | 0xd7 | 0xdf | 0xe7 | 0xef | 0xf7 | 0xff => {
                let addr = self.reg_read(&opcode.reg1).unwrap();
                self.call(addr);
            }
            // 8 bit SBC
            0x98..=0x9f | 0xde => {
                let reg = self.reg_read(&opcode.reg2).unwrap() as u8;
                self.sub_u8(self.a, reg, true);
            }
            // SCF
            0x37 => {
                self.flags.set(FlagsReg::carry, true);
            }
            // STOP
            0x10 => {
                // does nothing
            }
            // 8 bit SUB
            0x90..=0x97 | 0xd6 => {
                let reg = self.reg_read(&opcode.reg2).unwrap() as u8;
                self.sub_u8(self.a, reg, false);
            }
            // 8 bit XOR
            0xa8..=0xaf | 0xee => {
                let val = self.reg_read(&opcode.reg2).unwrap() as u8;
                self.a ^= val;

                self.flags.set(FlagsReg::zero, self.a == 0);
                self.flags.set(FlagsReg::subtraction, false);
                self.flags.set(FlagsReg::carry, false);
                self.flags.set(FlagsReg::half_carry, false);
            }
            _ => panic!(
                "Opcode: {:02X} '{}' is not implemented yet",
                byte, opcode.name
            ),
        };
        Ok(())
    }

    fn add_u8(&mut self, arg1: u8, arg2: u8, carry: bool) -> u8 {
        let (sum, c1) = arg1.overflowing_add(arg2);
        let (sum, c2) = sum.overflowing_add(carry as u8); // if either overflows we need to set carry flag
                                                          // set n flag to 0.
        self.flags.remove(FlagsReg::subtraction);
        // set h flag if overflow occured at bit 3
        let half_carry = (arg1 & 0x0f) + (arg2 & 0x0f) + carry as u8;
        self.flags.set(FlagsReg::half_carry, half_carry & 0xf0 > 0);
        // set c flag if overflow occured at bit 15
        self.flags.set(FlagsReg::carry, c1 | c2);

        sum
    }

    fn add_u16(&mut self, arg1: u16, arg2: u16, carry: bool) -> u16 {
        let (sum, c1) = arg1.overflowing_add(arg2);
        let (sum, c2) = sum.overflowing_add(carry as u16); // if either overflows we need to set carry flag
                                                           // set n flag to 0.
        self.flags.remove(FlagsReg::subtraction);
        // set h flag if overflow occured at bit 11
        let half_carry = (arg1 & 0xf00) + (arg2 & 0xf00) + carry as u16;
        self.flags
            .set(FlagsReg::half_carry, half_carry & 0xf000 > 0);
        // set c flag if overflow occured at bit 15
        self.flags.set(FlagsReg::carry, c1 | c2);

        sum
    }

    fn call(&mut self, addr: u16) {
        self.push_u16_to_stack(self.program_counter + 3);
        self.program_counter = addr - 3; // Subtract 3 to account for the three bytes of opcode
    }

    fn sub_u8(&mut self, arg1: u8, arg2: u8, carry: bool) -> u8 {
        let result = self.add_u8(arg1, (!arg2).wrapping_add(1), carry);
        self.flags.set(FlagsReg::subtraction, true);
        self.flags.toggle(FlagsReg::carry);
        self.flags.toggle(FlagsReg::half_carry);
        result
    }
}

#[cfg(test)]
mod tests {
    use crate::cartridge::Cartridge;

    use super::*;
    use rand::prelude::*;
    use std::vec;

    fn setup(program: Vec<u8>) -> Cpu {
        let cartridge = Cartridge::new(&program).unwrap();
        let bus = Bus::new(cartridge);
        let cpu = Cpu::new(bus);
        cpu
    }

    #[test]
    fn test_ld_r8_r8() {
        let mut rng = rand::thread_rng();
        for i in 0..8 {
            for j in 0..8 {
                // skip opcode 0x76
                if (i != 6) && (j != 6) {
                    let prg = vec![64 + 8 * i + j, 0x00, 0x76];
                    let mut cpu = setup(prg);
                    let mut value = rng.gen::<u8>();
                    let status = cpu.flags.clone();
                    // set hl to addr 2 so that Reg 6 does not affect program run.
                    // Also need to set h and l registers to values within our program (i.e not random).
                    cpu.set_hl(2);
                    if j == 4 {
                        cpu.r8_write(4, 0x00);
                        value = 0;
                    } else if j == 5 {
                        cpu.r8_write(5, 0x02);
                        value = 2;
                    } else {
                        cpu.r8_write(j, value);
                    }
                    cpu.run();

                    assert_eq!(cpu.r8_read(i), value);
                    assert_eq!(cpu.flags, status);
                }
            }
        }
    }

    #[test]
    fn test_ld_r8_imm8() {
        let mut rng = rand::thread_rng();
        for i in 0..8 {
            let value = rng.gen::<u8>();
            let prg = vec![8 * i + 6, value, 0x76];
            let mut cpu = setup(prg);
            cpu.set_hl(3); // set HL reg to point to an addr in program
            let status = cpu.flags.bits();
            cpu.run();

            assert_eq!(cpu.r8_read(i), value);
            assert_eq!(cpu.flags.bits(), status);
        }
    }

    #[test]
    fn test_ld_r16_imm16() {
        let mut rng = rand::thread_rng();
        for i in 0..4 {
            let lo = rng.gen::<u8>();
            let hi = rng.gen::<u8>();
            let prg = vec![16 * i + 1, lo, hi, 0x76];
            println!("program: {:?}", prg);
            let mut cpu = setup(prg);
            let status = cpu.flags.bits();
            cpu.run();

            assert_eq!(cpu.r16_read(i), u16::from_le_bytes([lo, hi]));
            assert_eq!(cpu.flags.bits(), status);
        }
    }

    #[test]
    fn test_ld_r16_a() {
        let mut rng = rand::thread_rng();
        for i in 0..4 {
            let value = rng.gen::<u8>();
            // 0x3e loads A with an imm8
            let prg = vec![0x3e, value, 16 * i + 2, 0x76, 0x76, 0x76, 0x76];
            println!("program: {:?}", prg);
            let mut cpu = setup(prg);
            cpu.set_hl(5);
            let status = cpu.flags.bits();
            cpu.run();

            // Since HL+ and HL- change HL, we cannot use r16mem_read to see the change
            // we need to go back to the addr.
            let target = if i == 2 {
                cpu.bus.mem_read(cpu.get_hl() - 1)
            } else if i == 3 {
                cpu.bus.mem_read(cpu.get_hl() + 1)
            } else {
                cpu.r16mem_read(i) as u8
            };

            assert_eq!(target, value);
            assert_eq!(cpu.flags.bits(), status);
        }
    }

    #[test]
    fn test_ld_a_r16() {
        let mut rng = rand::thread_rng();
        for i in 0..4 {
            let value = rng.gen::<u8>();
            let prg = vec![16 * i + 10, 0x76, 0x76, value, 0x76];
            println!("program: {:?}", prg);
            let mut cpu = setup(prg);
            cpu.set_bc(3);
            cpu.set_de(3);
            cpu.set_hl(3);
            let status = cpu.flags.bits();
            cpu.run();

            assert_eq!(cpu.a, value);
            assert_eq!(cpu.flags.bits(), status);
        }
    }

    #[test]
    fn test_ld_a_imm16() {
        let mut rng = rand::thread_rng();
        let value = rng.gen::<u8>();
        let prg = vec![0xfa, 0x05, 0x00, 0x00, 0x76, value];
        let mut cpu = setup(prg);
        let status = cpu.flags.bits();
        cpu.run();

        assert_eq!(cpu.a, value);
        assert_eq!(cpu.flags.bits(), status);
    }

    #[test]
    fn test_ld_imm16_a() {
        let mut rng = rand::thread_rng();
        let value = rng.gen::<u8>();
        // 0x3e loads a with imm8
        let prg = vec![0x3e, value, 0xea, 0x06, 0x00, 0x76, 0x76];
        let mut cpu = setup(prg);
        let status = cpu.flags.bits();
        cpu.run();

        assert_eq!(cpu.bus.mem_read(0x0006), value);
        assert_eq!(cpu.flags.bits(), status);
    }

    #[test]
    fn test_ld_imm16_sp() {
        let mut rng = rand::thread_rng();
        let value1 = rng.gen::<u8>();
        let value2 = rng.gen::<u8>();
        let prg = vec![0x08, 0x04, 0x00, 0x76, value1, value2];
        let mut cpu = setup(prg);
        let status = cpu.flags.bits();
        cpu.run();

        assert_eq!(cpu.bus.mem_read_u16(0x04), 0xfffe);
        assert_eq!(cpu.flags.bits(), status);
    }

    #[test]
    fn test_ld_hl_spimm8() {
        let prg = vec![0xf8, 0x01, 0x76];
        let mut cpu = setup(prg);
        let status = cpu.flags.bits();
        println!("SP: {}", cpu.stack_pointer);
        cpu.run();

        assert_eq!(cpu.get_hl(), 0xffff);
        assert_eq!(cpu.flags.bits(), status);

        // test negative behavior
        let prg = vec![0xf8, 0xf1, 0x76]; // offset = -0x0f
        let mut cpu = setup(prg);
        let status = cpu.flags.bits();
        cpu.run();

        assert_eq!(cpu.get_hl(), 0xffef);
        assert_eq!(cpu.flags.bits(), status | 0b0001_0000); // There is a carry in the sum
    }

    #[test]
    fn test_ld_sp_hl() {
        let mut rng = rand::thread_rng();
        let value1 = rng.gen::<u8>();
        let value2 = rng.gen::<u8>();
        // 0x21 loads imm16 into Reg HL.
        let prg = vec![0x21, value1, value2, 0xf9, 0x76];
        let mut cpu = setup(prg);
        let status = cpu.flags.bits();
        cpu.run();

        assert_eq!(cpu.stack_pointer, u16::from_le_bytes([value1, value2]));
        assert_eq!(cpu.flags.bits(), status);
    }
}
