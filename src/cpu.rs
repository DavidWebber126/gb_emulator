use bitflags::bitflags;
use std::collections::HashMap;

use crate::opcodes::{self, TargetReg};
use crate::bus::Bus;


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

pub struct CPU {
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
    bus: Bus,
}

impl CPU {
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
            program_counter: 0,
            bus
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

    fn reg_read(&mut self, target: &TargetReg) -> Option<u16> {
        match target {
            TargetReg::R8(reg) => Some(self.r8_read(*reg) as u16),
            TargetReg::R16(reg) => Some(self.r16_read(*reg)),
            TargetReg::R16stk(reg) => Some(self.r16stk_read(*reg)),
            TargetReg::R16mem(reg) => Some(self.r16mem_read(*reg)),
            TargetReg::A => Some(self.a as u16),
            TargetReg::SP => Some(self.stack_pointer),
            TargetReg::SPimm8 => {
                let offset = self.bus.mem_read(self.program_counter + 1) as i8;
                Some(self.stack_pointer.wrapping_add_signed(offset as i16))
            }
            TargetReg::C => Some(self.bus.mem_read(0xff00 + self.c as u16) as u16),
            TargetReg::Imm16 => Some(self.bus.mem_read_u16(self.program_counter + 1)),
            TargetReg::Imm8 => Some(self.bus.mem_read(self.program_counter + 1) as u16),
            TargetReg::Ptr => {
                let addr = self.bus.mem_read_u16(self.program_counter + 1);
                Some(self.bus.mem_read(addr) as u16)
            }
            _ => panic!("{:?} is not implemented yet", target)
        }
    }

    fn r8_read(&self, reg: u8) -> u8 {
        match reg {
            0 => self.b,
            1 => self.c,
            2 => self.d,
            3 => self.e,
            4 => self.h,
            5 => self.l,
            6 => self.bus.mem_read(self.get_hl()),
            7 => self.a,
            _ => panic!("Invalid r8 Register: {}", reg)
        }
    }

    fn r16_read(&self, reg: u8) -> u16 {
        match reg {
            0 => self.get_bc(),
            1 => self.get_de(),
            2 => self.get_hl(),
            3 => self.stack_pointer,
            _ => panic!("Invalid r16 Register: {}", reg)
        }
    }

    fn r16stk_read(&self, reg: u8) -> u16 {
        match reg {
            0 => self.get_bc(),
            1 => self.get_de(),
            2 => self.get_hl(),
            3 => self.get_af(),
            _ => panic!("Invalid r16 Register: {}", reg)
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
            _ => panic!("Invalid r16 Register: {}", reg)
        }
    }

    fn reg_write(&mut self, target: &TargetReg, data: u16) {
        match target {
            TargetReg::R8(reg) => self.r8_write(*reg, (data & 0xff) as u8 ),
            TargetReg::R16(reg) => self.r16_write(*reg, data),
            TargetReg::R16stk(reg) => self.r16stk_write(*reg, data),
            TargetReg::R16mem(reg) => self.r16mem_write(*reg, data),
            TargetReg::A => self.a = (data & 0xff) as u8,
            TargetReg::SP => self.stack_pointer = data,
            TargetReg::C => self.bus.mem_write(0xff00 + self.c as u16, (data & 0xff) as u8),
            TargetReg::Ptr => {
                let addr = self.bus.mem_read_u16(self.program_counter + 1);
                self.bus.mem_write(addr, data as u8);
            }
            TargetReg::Imm16 => {
                let addr = self.bus.mem_read_u16(self.program_counter + 1);
                self.bus.mem_write_u16(addr, data);
            }
            _ => panic!("{:?} is not implemented yet", target)
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
            _ => panic!("Impossible State. No reg value {}", reg)
        }
    }

    fn r16_write(&mut self, reg: u8, value: u16) {
        match reg {
            0 => self.set_bc(value),
            1 => self.set_de(value),
            2 => self.set_hl(value),
            3 => self.stack_pointer = value,
            _ => panic!("Invalid State. No r16 value {}", reg)
        }
    }

    fn r16stk_write(&mut self, reg: u8, value: u16) {
        match reg {
            0 => self.set_bc(value),
            1 => self.set_de(value),
            2 => self.set_hl(value),
            3 => self.set_af(value),
            _ => panic!("Invalid State. No r16stk value {}", reg)
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
            _ => panic!("Invalid State. No r16mem value {}", reg)
        }
    }

    // Main CPU loop. Fetch instruction, decode and execute.
    pub fn run(&mut self) {
        loop {
            let opcodes: &HashMap<u8, opcodes::Opcode> = &opcodes::CPU_OP_CODES;

            let opcode_num = self.bus.mem_read(self.program_counter);
            let opcode = opcodes.get(&opcode_num).unwrap();

            match opcode_num {
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
                    self.a = self.a & arg;
                }
                "CALL" => {
                    todo!()
                }
                // CCF
                0x3f => {
                    self.flags.toggle(FlagsReg::carry);
                }
                // 8 bit CP
                0xb8..=0xbf | 0xfe => {
                    todo!()
                }
                // CPL
                0x2f => {
                    self.a = !self.a;
                    self.flags.insert(FlagsReg::subtraction);
                    self.flags.insert(FlagsReg::half_carry);
                }
                // DAA
                0x27 => {
                    todo!()
                }
                // 8 bit DEC
                0x05 | 0x0d | 0x15 | 0x1d | 0x25 | 0x2d | 0x35 | 0x3d => {
                    let mut val = self.reg_read(&opcode.reg1).unwrap();
                    val.wrapping_sub(1);
                    self.reg_write(&opcode.reg1, val);
                    self.flags.set(FlagsReg::zero, val == 0);
                    self.flags.insert(FlagsReg::subtraction);
                    todo!("Need to implement half carry")
                }
                // 16 bit DEC
                0x0b | 0x1b | 0x2b | 0x3b => {
                    let mut val = self.reg_read(&opcode.reg1).unwrap();
                    val.wrapping_sub(1);
                    self.reg_write(&opcode.reg1, val);
                }
                "DI" => {
                    todo!()
                }
                "EI" => {
                    todo!()
                }
                "HALT" => {
                    todo!()
                }
                "INC" => {
                    todo!("Flags on r8 but not on r16")
                }
                "JP" => {
                    todo!()
                }
                "JR" => {
                    todo!()
                }
                "LD" | "LDH" => {
                    let value = self.reg_read(&opcode.reg2).unwrap();
                    self.reg_write(&opcode.reg1, value);
                }
                "NOP" => {
                    // do nothing
                }
                "OR" => {
                    todo!()
                }
                "POP" => {
                    todo!()
                }
                "PUSH" => {
                    todo!()
                }
                _ => panic!("Opcode: {} is not implemented yet", opcode.name)
            }

            self.program_counter = self.program_counter.wrapping_add(opcode.bytes);
            //println!("PC now: {}. Num bytes {}", self.program_counter, opcode.bytes);
        }
    }

    fn add_u8(&mut self, arg1: u8, arg2: u8, carry: bool) -> u8 {
        let (sum, carry) = arg1.overflowing_add(arg2);
        // set n flag to 0.
        self.flags.remove(FlagsReg::subtraction);
        // set h flag if overflow occured at bit 3
        let half_carry = (arg1 & 0x0f) + (arg2 & 0x0f);
        self.flags.set(FlagsReg::half_carry, half_carry & 0xf0 > 0);
        // set c flag if overflow occured at bit 15
        self.flags.set(FlagsReg::carry, carry);

        sum
    }


    fn add_u16(&mut self, arg1: u16, arg2: u16, carry: bool) -> u16 {
        let (sum, carry) = arg1.overflowing_add(arg2);
        // set n flag to 0.
        self.flags.remove(FlagsReg::subtraction);
        // set h flag if overflow occured at bit 11
        let half_carry = (arg1 & 0xf00) + (arg2 & 0xf00);
        self.flags.set(FlagsReg::half_carry, half_carry & 0xf000 > 0);
        // set c flag if overflow occured at bit 15
        self.flags.set(FlagsReg::carry, carry);

        sum
    }
}

#[cfg(test)]
mod tests {
    use std::vec;
    use super::*;
    use rand::prelude::*;

    fn setup(program: Vec<u8>) -> CPU {
        let bus = Bus::new(program);
        let cpu = CPU::new(bus);
        cpu
    }

    #[test]
    fn test_ld_r8_r8() {
        let mut rng = rand::thread_rng();
        for i in 0..8 {
            for j in 0..8 {
                // skip opcode 0x76
                if (i != 6) && (j != 6) {
                    let prg = vec![64 + 8*i + j, 0x00, 0x00];
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
            let prg = vec![8*i + 6, value, 0x00, 0x00];
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
            let prg = vec![16*i + 1, lo, hi, 0x00];
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
            let prg = vec![0x3e, value, 16*i + 2, 0x00, 0x00, 0x00, 0x00];
            println!("program: {:?}", prg);
            let mut cpu = setup(prg);
            cpu.set_hl(5);
            let status = cpu.flags.bits();
            cpu.run();

            // Since HL+ and HL- change HL, we cannot use r16mem_read to see the change
            // we need to go back to the addr.
            let target = if i == 2 {
                cpu.bus.mem_read(cpu.get_hl() - 1 )
            } else if i == 3 {
                cpu.bus.mem_read(cpu.get_hl() + 1 )
            } else {
                cpu.r16mem_read(i) as u8
            };

            assert_eq!(target, value);
            assert_eq!(cpu.flags.bits(), status);
        }
    }

    #[test]
    fn test_ldh_c_a() {
        todo!()
    }

    #[test]
    fn test_ld_a_r16() {
        let mut rng = rand::thread_rng();
        for i in 0..4 {
            let value = rng.gen::<u8>();
            let prg = vec![16*i + 10, 0x00, 0x00, value, 0x00];
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
        let prg = vec![0xfa, 0x05, 0x00, 0x00, 0x00, value];
        let mut cpu = setup(prg);
        let status = cpu.flags.bits();
        cpu.run();

        assert_eq!(cpu.a, value);
        assert_eq!(cpu.flags.bits(), status);
    }

    #[test]
    fn test_ldh_imm8_a() {
        todo!()
    }

    #[test]
    fn test_ld_imm16_a() {
        let mut rng = rand::thread_rng();
        let value = rng.gen::<u8>();
        // 0x3e loads a with imm8
        let prg = vec![0x3e, value, 0xea, 0x06, 0x00, 0x00, 0x00];
        let mut cpu = setup(prg);
        let status = cpu.flags.bits();
        cpu.run();

        assert_eq!(cpu.bus.mem_read(0x0006), value);
        assert_eq!(cpu.flags.bits(), status);
    }

    #[test]
    fn test_ldh_a_imm8() {
        todo!()
    }

    #[test]
    fn test_ldh_a_c() {
        todo!()
    }

    #[test]
    fn test_ld_imm16_sp() {
        let mut rng = rand::thread_rng();
        let value1 = rng.gen::<u8>();
        let value2 = rng.gen::<u8>();
        let prg = vec![0x08, 0x04, 0x00, 0x00, value1, value2];
        let mut cpu = setup(prg);
        let status = cpu.flags.bits();
        cpu.run();

        assert_eq!(cpu.bus.mem_read_u16(0x04), 0xfffe);
        assert_eq!(cpu.flags.bits(), status);
    }

    #[test]
    fn test_ld_hl_spimm8() {
        let prg = vec![0xf8, 0x01, 0x00];
        let mut cpu = setup(prg);
        let status = cpu.flags.bits();
        cpu.run();

        assert_eq!(cpu.get_hl(), 0xffff);
        assert_eq!(cpu.flags.bits(), status);

        // test flags
        let prg = vec![0xf8, 0xf1, 0x00]; // offset = -0x0f
        let mut cpu = setup(prg);
        let status = cpu.flags.bits();
        cpu.run();

        assert_eq!(cpu.get_hl(), 0xffef);
        assert_eq!(cpu.flags.bits(), status);
    }

    #[test]
    fn test_ld_sp_hl() {
        let mut rng = rand::thread_rng();
        let value1 = rng.gen::<u8>();
        let value2 = rng.gen::<u8>();
        // 0x21 loads imm16 into Reg HL.
        let prg = vec![0x21, value1, value2, 0xf9, 0x00];
        let mut cpu = setup(prg);
        let status = cpu.flags.bits();
        cpu.run();

        assert_eq!(cpu.stack_pointer, u16::from_le_bytes([value1, value2]));
        assert_eq!(cpu.flags.bits(), status);
    }
}