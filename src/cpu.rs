use bitflags::bitflags;
use std::collections::HashMap;

use crate::bus::{Bus, Interrupt};
use crate::opcodes::{self, Opcode, TargetReg};
use crate::render;
use crate::trace;

bitflags! {
    #[derive(PartialEq, Debug, Clone)]
    pub struct CpuFlag: u8 {
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
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub flags: CpuFlag,
    pub h: u8,
    pub l: u8,
    pub stack_pointer: u16,
    pub program_counter: u16,
    pub ime: bool,
    pub bus: Bus,
    pub prefixed_mode: bool,
    pub halted: bool,
    pub frame_ready: bool,
    cycles: u8,
}

impl Cpu {
    pub fn new(bus: Bus) -> Self {
        Self {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            flags: CpuFlag::empty(),
            h: 0,
            l: 0,
            stack_pointer: 0xfffe,
            program_counter: 0x0100,
            ime: false,
            bus,
            halted: false,
            prefixed_mode: false,
            frame_ready: false,
            cycles: 0,
        }
    }

    pub fn get_bc(&self) -> u16 {
        ((self.b as u16) << 8) | self.c as u16
    }

    pub fn set_bc(&mut self, value: u16) {
        self.c = (value & 0xff) as u8;
        self.b = (value >> 8) as u8;
    }

    pub fn get_de(&self) -> u16 {
        ((self.d as u16) << 8) | self.e as u16
    }

    pub fn set_de(&mut self, value: u16) {
        self.e = (value & 0xff) as u8;
        self.d = (value >> 8) as u8;
    }

    pub fn get_hl(&self) -> u16 {
        ((self.h as u16) << 8) | self.l as u16
    }

    pub fn set_hl(&mut self, value: u16) {
        self.l = (value & 0xff) as u8;
        self.h = (value >> 8) as u8;
    }

    pub fn set_af(&mut self, value: u16) {
        let [lo, hi] = value.to_le_bytes();
        self.a = hi;
        self.flags = CpuFlag::from_bits_retain(lo);
    }

    pub fn get_af(&self) -> u16 {
        ((self.a as u16) << 8) | self.flags.bits() as u16
    }

    fn push_u8_to_stack(&mut self, val: u8) {
        self.stack_pointer -= 1;
        self.bus.mem_write(self.stack_pointer, val);
    }

    fn push_u16_to_stack(&mut self, val: u16) {
        let [lo, hi] = val.to_le_bytes();
        self.push_u8_to_stack(hi);
        self.push_u8_to_stack(lo);
    }

    // fn pop_u8_from_stack(&mut self) -> u8 {
    //     self.stack_pointer += 1;
    //     self.bus.mem_read(self.stack_pointer)
    // }

    fn pop_u16_from_stack(&mut self) -> u16 {
        let val = self.bus.mem_read_u16(self.stack_pointer);
        self.stack_pointer = self.stack_pointer.wrapping_add(2);
        val
    }

    // fn reg_read(&mut self, target: &TargetReg) -> Option<u16> {
    //     match target {
    //         TargetReg::R8(reg) => Some(self.r8_read(*reg) as u16),
    //         TargetReg::R16(reg) => Some(self.r16_read(*reg)),
    //         TargetReg::R16stk(reg) => Some(self.r16stk_read(*reg)),
    //         TargetReg::R16mem(reg) => Some(self.r16mem_read(*reg) as u16),
    //         TargetReg::Cond(code) => Some(*code as u16),
    //         TargetReg::Tgt3(reg) => Some(self.tgt3_read(*reg)),
    //         TargetReg::B3(bit) => Some(*bit as u16),
    //         TargetReg::A => Some(self.a as u16),
    //         TargetReg::SP => Some(self.stack_pointer),
    //         TargetReg::C => Some(self.bus.mem_read(0xff00 + self.c as u16) as u16),
    //         TargetReg::Imm16 => Some(self.bus.mem_read_u16(self.program_counter + 1)),
    //         TargetReg::Imm8 => Some(self.bus.mem_read(self.program_counter + 1) as u16),
    //         TargetReg::Ptr => {
    //             let addr = self.bus.mem_read_u16(self.program_counter + 1);
    //             Some(self.bus.mem_read(addr) as u16)
    //         }
    //         _ => panic!("{:?} is not implemented yet", target),
    //     }
    // }

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

    fn r16mem_read(&mut self, reg: u8) -> u8 {
        match reg {
            0 => {
                let addr = self.get_bc();
                self.bus.mem_read(addr)
            }
            1 => {
                let addr = self.get_de();
                self.bus.mem_read(addr)
            }
            2 => {
                let addr = self.get_hl();
                let val = self.bus.mem_read(addr);
                self.set_hl(addr.wrapping_add(1));
                val
            }
            3 => {
                let addr = self.get_hl();
                let val = self.bus.mem_read(addr);
                self.set_hl(addr.wrapping_sub(1));
                val
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

    // fn reg_write(&mut self, target: &TargetReg, data: u16) {
    //     match target {
    //         TargetReg::R8(reg) => self.r8_write(*reg, (data & 0xff) as u8),
    //         TargetReg::R16(reg) => self.r16_write(*reg, data),
    //         TargetReg::R16stk(reg) => self.r16stk_write(*reg, data),
    //         TargetReg::R16mem(reg) => self.r16mem_write(*reg, data),
    //         TargetReg::A => self.a = (data & 0xff) as u8,
    //         TargetReg::SP => self.stack_pointer = data,
    //         TargetReg::C => self
    //             .bus
    //             .mem_write(0xff00 + self.c as u16, (data & 0xff) as u8),
    //         TargetReg::Ptr => {
    //             let addr = self.bus.mem_read_u16(self.program_counter + 1);
    //             self.bus.mem_write(addr, data as u8);
    //         }
    //         TargetReg::Imm8 => {
    //             let addr = self.bus.mem_read(self.program_counter + 1);
    //             self.bus.mem_write(0xff00 + addr as u16, data as u8);
    //         }
    //         TargetReg::Imm16 => {
    //             let addr = self.bus.mem_read_u16(self.program_counter + 1);
    //             self.bus.mem_write_u16(addr, data);
    //         }
    //         _ => panic!("{:?} is not implemented yet", target),
    //     }
    // }

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

    fn interrupt_check(&mut self) {
        // Interrupt is serviced is IME is set, bit is set in both IE and IF flags
        let vblank_interrupt = self.bus.vblank_flag() && self.bus.vblank_enabled();
        let lcd_interrupt = self.bus.lcd_flag() && self.bus.lcd_enabled();
        let timer_interrupt = self.bus.timer_flag() && self.bus.timer_enabled();
        let serial_interrupt = self.bus.serial_flag() && self.bus.serial_enabled();
        let joypad_interrupt = self.bus.joypad_flag() && self.bus.joypad_enabled();

        let interrupt_pending = vblank_interrupt
            || lcd_interrupt
            || timer_interrupt
            || serial_interrupt
            || joypad_interrupt;

        // Vblank has highest priority, Joypad has lowest priority. Only handle one interrupt at a time
        // Turn off interrupts then handle the current interrupt by priority
        match (self.halted, self.ime, interrupt_pending) {
            (_, _, false) => {}
            (false, false, true) => {
                return; // return early to avoid interrupt handling this case
            }
            (true, true, true) => {
                self.ime = false;
                self.halted = false;
                self.push_u16_to_stack(self.program_counter + 1);
            }
            (false, true, true) => {
                self.ime = false;
                self.push_u16_to_stack(self.program_counter);
            }
            (true, false, true) => {
                self.halted = false;
                self.program_counter += 1;
                return; // return early to avoid interrupt handling this case
            }
        }

        // Interrupt handler
        if vblank_interrupt {
            self.bus.interrupt_flag.set(Interrupt::vblank, false);
            self.program_counter = 0x0040;
        } else if lcd_interrupt {
            self.bus.interrupt_flag.set(Interrupt::lcd, false);
            self.program_counter = 0x0048;
        } else if timer_interrupt {
            self.bus.interrupt_flag.set(Interrupt::timer, false);
            self.program_counter = 0x0050;
        } else if serial_interrupt {
            self.bus.interrupt_flag.set(Interrupt::serial, false);
            self.program_counter = 0x0058;
        } else if joypad_interrupt {
            self.bus.interrupt_flag.set(Interrupt::joypad, false);
            self.program_counter = 0x0060;
        }
    }

    // Main CPU step. Fetch instruction, decode and execute.
    // Tell bus how much to step the ppu and apu.
    pub fn step<F>(&mut self, mut callback: F) -> Option<&render::Frame>
    where
        F: FnMut(&mut Cpu),
    {
        // check for interrupts or halt
        self.interrupt_check();

        callback(self);

        // Get opcode from prefixed or regular
        let (cycles, bytes) = if self.prefixed_mode {
            let opcodes: &HashMap<u8, Opcode> = &opcodes::CPU_PREFIXED_OP_CODES;
            let opcode_num = self.bus.mem_read(self.program_counter + 1);
            let opcode = opcodes.get(&opcode_num).unwrap();

            self.prefixed_mode = false;
            self.prefixed_opcodes(opcode_num, opcode);
            (opcode.cycles, opcode.bytes)
        } else {
            let opcodes: &HashMap<u8, Opcode> = &opcodes::CPU_OP_CODES;
            let opcode_num = self.bus.mem_read(self.program_counter);
            let opcode = opcodes
                .get(&opcode_num)
                .unwrap_or_else(|| panic!("Invalid opcode received: {:02X}", opcode_num));

            self.non_prefixed_opcodes(opcode_num, opcode);
            (opcode.cycles, opcode.bytes)
        };

        self.frame_ready = self.bus.tick(cycles + self.cycles);
        self.cycles = 0;
        self.program_counter = self.program_counter.wrapping_add(bytes);

        // check if frame is ready to display
        let mut output = None;
        if self.frame_ready {
            output = Some(&self.bus.frame);
        }
        output
    }

    pub fn run(&mut self) {
        loop {
            let _ = self.step(|_| {});
        }
    }

    pub fn step_with_trace(&mut self) -> Option<&render::Frame> {
        self.step(|cpu| {
            trace::trace_cpu(cpu);
        })
    }

    fn prefixed_opcodes(&mut self, byte: u8, opcode: &Opcode) {
        match byte {
            // bit u3, r8
            0x40..=0x7f => {
                let TargetReg::B3(bit) = &opcode.reg1 else {
                    panic!("BIT Opcode needs U3 in first Register but it is not")
                };
                let TargetReg::R8(reg) = &opcode.reg2 else {
                    panic!("BIT Opcode needs R8 in second Register but it is not")
                };
                let val = self.r8_read(*reg);
                self.flags.set(CpuFlag::zero, ((val >> bit) & 0b1) == 0);
                self.flags.set(CpuFlag::subtraction, false);
                self.flags.set(CpuFlag::half_carry, true);
            }
            // res u3, r8
            0x80..=0xbf => {
                let TargetReg::B3(bit) = &opcode.reg1 else {
                    panic!("RES Opcode needs U3 in first Register but it is not")
                };
                let TargetReg::R8(reg) = &opcode.reg2 else {
                    panic!("RES Opcode needs R8 in second Register but it is not")
                };
                let val = self.r8_read(*reg);
                self.r8_write(*reg, val & !(0x01 << bit));
            }
            // rl r8
            0x10..=0x17 => {
                let TargetReg::R8(reg) = &opcode.reg1 else {
                    panic!("Opcode needs R8 but it is not")
                };
                let mut val = self.r8_read(*reg);
                let left_bit = (val & 0x80) > 0x00;
                let carry = self.flags.contains(CpuFlag::carry);
                val <<= 1;
                val += carry as u8;
                self.r8_write(*reg, val);
                self.flags.set(CpuFlag::zero, val == 0);
                self.flags.remove(CpuFlag::subtraction);
                self.flags.remove(CpuFlag::half_carry);
                self.flags.set(CpuFlag::carry, left_bit);
            }
            // rlc r8
            0x00..=0x07 => {
                let TargetReg::R8(reg) = &opcode.reg1 else {
                    panic!("Opcode needs R8 but it is not")
                };
                let mut val = self.r8_read(*reg);
                let left_bit = (val & 0x80) != 0x00;
                val <<= 1;
                val += left_bit as u8;
                self.r8_write(*reg, val);
                self.flags.set(CpuFlag::zero, val == 0);
                self.flags.remove(CpuFlag::subtraction);
                self.flags.remove(CpuFlag::half_carry);
                self.flags.set(CpuFlag::carry, left_bit);
            }
            // rr r8
            0x18..=0x1f => {
                let TargetReg::R8(reg) = &opcode.reg1 else {
                    panic!("Opcode needs R8 but it is not")
                };
                let mut val = self.r8_read(*reg);
                let right_bit = (val & 0x01) != 0;
                let carry = self.flags.contains(CpuFlag::carry);
                val >>= 1;
                val += (carry as u8) << 7;
                self.r8_write(*reg, val);
                self.flags.set(CpuFlag::zero, val == 0);
                self.flags.remove(CpuFlag::subtraction);
                self.flags.remove(CpuFlag::half_carry);
                self.flags.set(CpuFlag::carry, right_bit);
            }
            // rrc r8
            0x08..=0x0f => {
                let TargetReg::R8(reg) = &opcode.reg1 else {
                    panic!("Opcode needs R8 but it is not")
                };
                let mut val = self.r8_read(*reg);
                let right_bit = (val & 0x01) != 0;
                val >>= 1;
                val += (right_bit as u8) << 7;
                self.r8_write(*reg, val);
                self.flags.set(CpuFlag::zero, val == 0);
                self.flags.remove(CpuFlag::subtraction);
                self.flags.remove(CpuFlag::half_carry);
                self.flags.set(CpuFlag::carry, right_bit);
            }
            // set u3, r8
            0xc0..=0xff => {
                let TargetReg::B3(bit) = &opcode.reg1 else {
                    panic!("BIT Opcode needs U3 in first Register but it is not")
                };
                let TargetReg::R8(reg) = &opcode.reg2 else {
                    panic!("Opcode needs R8 but it is not")
                };
                let val = self.r8_read(*reg);
                self.r8_write(*reg, val | (0x1 << bit));
            }
            // sla r8
            0x20..=0x27 => {
                let TargetReg::R8(reg) = &opcode.reg1 else {
                    panic!("Opcode needs R8 but it is not")
                };
                let mut val = self.r8_read(*reg);
                let left_bit = val & 0x80 != 0;
                val <<= 1;
                self.r8_write(*reg, val);
                self.flags.set(CpuFlag::zero, val == 0);
                self.flags.set(CpuFlag::subtraction, false);
                self.flags.set(CpuFlag::half_carry, false);
                self.flags.set(CpuFlag::carry, left_bit);
            }
            // sra r8
            0x28..=0x2f => {
                let TargetReg::R8(reg) = &opcode.reg1 else {
                    panic!("Opcode needs R8 but it is not")
                };
                let mut val = self.r8_read(*reg);
                let right_bit = val & 0x01 != 0;
                let left_bit = val & 0x80;
                val >>= 1;
                val |= left_bit;
                self.r8_write(*reg, val);
                self.flags.set(CpuFlag::zero, val == 0);
                self.flags.set(CpuFlag::subtraction, false);
                self.flags.set(CpuFlag::half_carry, false);
                self.flags.set(CpuFlag::carry, right_bit);
            }
            // srl r8
            0x38..=0x3f => {
                let TargetReg::R8(reg) = &opcode.reg1 else {
                    panic!("Opcode needs R8 but it is not")
                };
                let mut val = self.r8_read(*reg);
                let right_bit = val & 0x01 != 0;
                val >>= 1;
                self.r8_write(*reg, val);
                self.flags.set(CpuFlag::zero, val == 0);
                self.flags.set(CpuFlag::subtraction, false);
                self.flags.set(CpuFlag::half_carry, false);
                self.flags.set(CpuFlag::carry, right_bit);
            }
            // swap r8
            0x30..=0x37 => {
                let TargetReg::R8(reg) = &opcode.reg1 else {
                    panic!("Opcode needs R8 but it is not")
                };
                let val = self.r8_read(*reg);
                let lo = val & 0x0f;
                let hi = val & 0xf0;
                self.r8_write(*reg, (lo << 4) + (hi >> 4));
                self.flags.set(CpuFlag::zero, val == 0);
                self.flags.set(CpuFlag::subtraction, false);
                self.flags.set(CpuFlag::half_carry, false);
                self.flags.set(CpuFlag::carry, false);
            }
        };
    }

    fn non_prefixed_opcodes(&mut self, byte: u8, opcode: &Opcode) {
        match byte {
            // ADC A, r8
            0x88..=0x8f => {
                let TargetReg::R8(reg) = &opcode.reg2 else {
                    panic!("Opcode needs R8 but it is not")
                };
                let arg = self.r8_read(*reg);
                let sum = self.add_u8(self.a, arg, true);

                self.a = sum;
            }
            // ADC A, imm8
            0xce => {
                let arg = self.bus.mem_read(self.program_counter + 1);
                let sum = self.add_u8(self.a, arg, true);

                self.a = sum;
            }
            // ADD A, r8
            0x80..=0x87 => {
                let TargetReg::R8(reg) = &opcode.reg2 else {
                    panic!("Opcode needs R8 but it is not")
                };
                let arg = self.r8_read(*reg);
                let sum = self.add_u8(self.a, arg, false);

                self.a = sum;
            }
            // ADD A, imm8
            0xc6 => {
                let arg = self.bus.mem_read(self.program_counter + 1);
                let sum = self.add_u8(self.a, arg, false);

                self.a = sum;
            }
            // ADD SP, e8
            0xe8 => {
                let arg = self.bus.mem_read(self.program_counter + 1);
                self.stack_pointer = self.add_e8(self.stack_pointer, arg);
                self.flags.remove(CpuFlag::zero);
                self.flags.remove(CpuFlag::subtraction);
            }
            // ADD HL, r16
            0x09 | 0x19 | 0x29 | 0x39 => {
                let TargetReg::R16(reg) = &opcode.reg2 else {
                    panic!("Opcode needs R16 but it is not")
                };
                let arg = self.r16_read(*reg);
                let sum = self.add_u16(self.get_hl(), arg, false);

                self.set_hl(sum);
            }
            // AND A, r8
            0xa0..=0xa7 => {
                let TargetReg::R8(reg) = &opcode.reg2 else {
                    panic!("Opcode needs R8 but it is not")
                };
                let arg = self.r8_read(*reg);
                self.a &= arg;

                self.flags.set(CpuFlag::zero, self.a == 0);
                self.flags.remove(CpuFlag::subtraction);
                self.flags.insert(CpuFlag::half_carry);
                self.flags.remove(CpuFlag::carry);
            }
            // AND A, imm8
            0xe6 => {
                let arg = self.bus.mem_read(self.program_counter + 1);
                self.a &= arg;

                self.flags.set(CpuFlag::zero, self.a == 0);
                self.flags.remove(CpuFlag::subtraction);
                self.flags.insert(CpuFlag::half_carry);
                self.flags.remove(CpuFlag::carry);
            }
            // CALL
            0xcd => {
                let addr = self.bus.mem_read_u16(self.program_counter + 1);
                self.push_u16_to_stack(self.program_counter.wrapping_add(3));
                self.program_counter = addr.wrapping_sub(3);
            }
            // CALL cc
            0xc4 | 0xcc | 0xd4 | 0xdc => {
                let TargetReg::Cond(condition) = &opcode.reg1 else {
                    panic!("Expected Cond register")
                };
                let should_execute = match condition {
                    0 => !self.flags.contains(CpuFlag::zero), // Cond(0) => zero flags is not set
                    1 => self.flags.contains(CpuFlag::zero),  // Cond(1) => zero flag is set
                    2 => !self.flags.contains(CpuFlag::carry), // Cond(3) => carry flag is set
                    3 => self.flags.contains(CpuFlag::carry), // Cond(3) => carry flag is set
                    _ => panic!("Condition Codes are 0-3. Received {}", condition),
                };
                if should_execute {
                    // inc cycle count
                    self.cycles += 3;
                    let addr = self.bus.mem_read_u16(self.program_counter + 1);
                    self.push_u16_to_stack(self.program_counter.wrapping_add(3));
                    self.program_counter = addr.wrapping_sub(3);
                }
            }
            // CCF
            0x3f => {
                self.flags.remove(CpuFlag::subtraction);
                self.flags.remove(CpuFlag::half_carry);
                self.flags.toggle(CpuFlag::carry);
            }
            // CP A, r8
            0xb8..=0xbf => {
                let TargetReg::R8(reg) = &opcode.reg2 else {
                    panic!("Opcode needs R8 but it is not")
                };
                let val = self.r8_read(*reg);
                let _result = self.sub_u8(self.a, val, false);
            }
            // CP A, imm8
            0xfe => {
                let val = self.bus.mem_read(self.program_counter + 1);
                let _result = self.sub_u8(self.a, val, false);
            }
            // CPL
            0x2f => {
                self.a = !self.a;
                self.flags.insert(CpuFlag::subtraction);
                self.flags.insert(CpuFlag::half_carry);
            }
            // DAA
            0x27 => {
                let mut adjustment = 0;
                if self.flags.contains(CpuFlag::subtraction) {
                    if self.flags.contains(CpuFlag::half_carry) {
                        adjustment += 0x06;
                    }
                    if self.flags.contains(CpuFlag::carry) {
                        adjustment += 0x60;
                    };
                    self.a = self.a.wrapping_sub(adjustment);
                } else {
                    if self.flags.contains(CpuFlag::half_carry) || self.a & 0x0f > 0x09 {
                        adjustment += 0x06;
                    }
                    if self.flags.contains(CpuFlag::carry) || self.a > 0x99 {
                        adjustment += 0x60;
                        self.flags.set(CpuFlag::carry, true);
                    }
                    self.a = self.a.wrapping_add(adjustment);
                }

                self.flags.set(CpuFlag::zero, self.a == 0);
                self.flags.set(CpuFlag::half_carry, false);
            }
            // DEC r8
            0x05 | 0x0d | 0x15 | 0x1d | 0x25 | 0x2d | 0x35 | 0x3d => {
                let TargetReg::R8(reg) = &opcode.reg1 else {
                    panic!("Opcode needs R8 but it is not")
                };
                let mut val = self.r8_read(*reg);
                let half_carry = ((val & 0x0f).wrapping_sub(1)) & 0x10 > 0;
                val = val.wrapping_sub(1);
                self.r8_write(*reg, val);
                self.flags.set(CpuFlag::zero, val == 0);
                self.flags.insert(CpuFlag::subtraction);
                self.flags.set(CpuFlag::half_carry, half_carry);
            }
            // DEC r16
            0x0b | 0x1b | 0x2b | 0x3b => {
                let TargetReg::R16(reg) = &opcode.reg1 else {
                    panic!("Opcode needs R16 but it is not")
                };
                let mut val = self.r16_read(*reg);
                val = val.wrapping_sub(1);
                self.r16_write(*reg, val);
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
            0x76 => {
                self.halted = true;
            }
            // INC r8
            0x04 | 0x0c | 0x14 | 0x1c | 0x24 | 0x2c | 0x34 | 0x3c => {
                let TargetReg::R8(reg) = &opcode.reg1 else {
                    panic!("Opcode needs R8 but it is not")
                };
                let mut val = self.r8_read(*reg);
                let half_carry = val & 0x0f == 0x0f;
                val = val.wrapping_add(1);
                self.r8_write(*reg, val);

                self.flags.set(CpuFlag::zero, val == 0);
                self.flags.remove(CpuFlag::subtraction);
                self.flags.set(CpuFlag::half_carry, half_carry);
            }
            // INC r16
            0x03 | 0x13 | 0x23 | 0x33 => {
                let TargetReg::R16(reg) = &opcode.reg1 else {
                    panic!("Opcode needs R16 but it is not")
                };
                let mut val = self.r16_read(*reg);
                val = val.wrapping_add(1);
                self.r16_write(*reg, val);
            }
            // JP
            0xc3 => {
                let addr = self.bus.mem_read_u16(self.program_counter + 1);
                self.program_counter = addr.wrapping_sub(3); // Subtract 3 bytes to account for the addition of 3 bytes from the JP opcode
            }
            // JP HL
            0xe9 => {
                self.program_counter = (self.get_hl()).wrapping_sub(1);
            }
            // JP cc
            0xc2 | 0xca | 0xd2 | 0xda => {
                let TargetReg::Cond(condition) = &opcode.reg1 else {
                    panic!("Expected Cond register")
                };
                let should_execute = match condition {
                    0 => !self.flags.contains(CpuFlag::zero), // Cond(0) => zero flags is not set
                    1 => self.flags.contains(CpuFlag::zero),  // Cond(1) => zero flag is set
                    2 => !self.flags.contains(CpuFlag::carry), // Cond(3) => carry flag is set
                    3 => self.flags.contains(CpuFlag::carry), // Cond(3) => carry flag is set
                    _ => panic!("Condition Codes are 0-3. Received {}", condition),
                };
                if should_execute {
                    // inc cycle count
                    self.cycles += 1;
                    self.program_counter = self.bus.mem_read_u16(self.program_counter + 1) - 3;
                }
            }
            // JR imm8
            0x18 => {
                let offset = self.bus.mem_read(self.program_counter + 1) as i8;
                self.program_counter = self.program_counter.wrapping_add_signed(offset as i16);
            }
            // JR cc, imm8
            0x20 | 0x28 | 0x30 | 0x38 => {
                let offset = self.bus.mem_read(self.program_counter + 1) as i8;
                let TargetReg::Cond(condition) = &opcode.reg1 else {
                    panic!("Expected Cond register")
                };
                let should_execute = match condition {
                    0 => !self.flags.contains(CpuFlag::zero), // Cond(0) => zero flags is not set
                    1 => self.flags.contains(CpuFlag::zero),  // Cond(1) => zero flag is set
                    2 => !self.flags.contains(CpuFlag::carry), // Cond(3) => carry flag is not set
                    3 => self.flags.contains(CpuFlag::carry), // Cond(3) => carry flag is set
                    _ => panic!("Condition Codes are 0-3. Received {}", condition),
                };
                if should_execute {
                    // inc cycle count
                    self.cycles += 1;
                    self.program_counter = self.program_counter.wrapping_add_signed(offset as i16);
                }
            }
            // LD r8, r8
            0x40..=0x75 | 0x77..=0x7f => {
                let TargetReg::R8(reg2) = &opcode.reg2 else {
                    panic!("Opcode needs R8 but it is not")
                };
                let val = self.r8_read(*reg2);
                let TargetReg::R8(reg1) = &opcode.reg1 else {
                    panic!("Opcode needs R8 but it is not")
                };
                self.r8_write(*reg1, val);
            }
            // LD r16, imm16
            0x01 | 0x11 | 0x21 | 0x31 => {
                let val = self.bus.mem_read_u16(self.program_counter + 1);
                let TargetReg::R16(reg) = &opcode.reg1 else {
                    panic!("Opcode needs R16 but it is not")
                };
                self.r16_write(*reg, val);
            }
            // LD A, imm16
            0xfa => {
                let addr = self.bus.mem_read_u16(self.program_counter + 1);
                let val = self.bus.mem_read(addr);
                self.a = val;
            }
            // LD imm16, A
            0xea => {
                let addr = self.bus.mem_read_u16(self.program_counter + 1);
                self.bus.mem_write(addr, self.a);
            }
            // LD imm16, SP
            0x08 => {
                let addr = self.bus.mem_read_u16(self.program_counter + 1);
                self.bus.mem_write_u16(addr, self.stack_pointer);
            }
            // LD SP, HL
            0xf9 => {
                self.stack_pointer = self.get_hl();
            }
            // LD r16mem, A
            0x02 | 0x12 | 0x22 | 0x32 => {
                let TargetReg::R16mem(reg) = &opcode.reg1 else {
                    panic!("Opcode needs R16mem but it is not")
                };
                self.r16mem_write(*reg, self.a as u16);
            }
            // LD A, r16mem
            0x0a | 0x1a | 0x2a | 0x3a => {
                let TargetReg::R16mem(reg) = &opcode.reg2 else {
                    panic!("Opcode needs R16mem but it is not")
                };
                let val = self.r16mem_read(*reg);
                self.a = val;
            }
            // LD r8, imm8
            0x06 | 0x0e | 0x16 | 0x1e | 0x26 | 0x2e | 0x36 | 0x3e => {
                let val = self.bus.mem_read(self.program_counter + 1);
                let TargetReg::R8(reg) = &opcode.reg1 else {
                    panic!("Opcode needs R8 but it is not")
                };
                self.r8_write(*reg, val);
            }
            // ld hl, sp + imm8
            0xf8 => {
                let offset = self.bus.mem_read(self.program_counter + 1);
                let sum = self.add_e8(self.stack_pointer, offset);
                self.set_hl(sum);
                self.flags.set(CpuFlag::zero, false);
                self.flags.set(CpuFlag::subtraction, false);
            }
            // LDH [C], A
            0xe2 => {
                let addr = 0xff00 + self.c as u16;
                self.bus.mem_write(addr, self.a);
            }
            // LDH A, [C]
            0xf2 => {
                let val = self.bus.mem_read(0xff00 + self.c as u16);
                self.a = val;
            }
            // LDH imm8, A
            0xe0 => {
                let addr_lo = self.bus.mem_read(self.program_counter + 1) as u16;
                self.bus.mem_write(0xff00 + (addr_lo & 0x00ff), self.a);
            }
            // LDH A, imm8
            0xf0 => {
                let addr_lo = self.bus.mem_read(self.program_counter + 1) as u16;
                let val = self.bus.mem_read(0xff00 + (addr_lo & 0x00ff));
                self.a = val;
            }
            // NOP
            0x00 => {
                // do nothing
            }
            // OR A, r8
            0xb0..=0xb7 => {
                let TargetReg::R8(reg) = &opcode.reg2 else {
                    panic!("Opcode needs R8 but it is not")
                };
                let val = self.r8_read(*reg);
                self.a |= val;

                self.flags.set(CpuFlag::zero, self.a == 0);
                self.flags.remove(CpuFlag::subtraction);
                self.flags.remove(CpuFlag::half_carry);
                self.flags.remove(CpuFlag::carry);
            }
            // OR A, imm8
            0xf6 => {
                let val = self.bus.mem_read(self.program_counter + 1);
                self.a |= val;
            }
            // POP r16stk
            0xc1 | 0xd1 | 0xe1 => {
                let val = self.pop_u16_from_stack();
                let TargetReg::R16stk(reg) = &opcode.reg1 else {
                    panic!("Opcode needs R16stk but it is not")
                };
                self.r16stk_write(*reg, val);
            }
            // POP AF
            0xf1 => {
                let val = self.pop_u16_from_stack();
                self.set_af(val & 0xfff0);
            }
            // PUSH
            0xc5 | 0xd5 | 0xe5 | 0xf5 => {
                let TargetReg::R16stk(reg) = &opcode.reg1 else {
                    panic!("Opcode needs R16stk but it is not")
                };
                let val = self.r16stk_read(*reg);
                self.push_u16_to_stack(val);
            }
            // RET
            0xc9 => {
                self.program_counter = self.pop_u16_from_stack() - 1; // minus 1 to account for the added byte
            }
            // RET cc
            0xc0 | 0xc8 | 0xd0 | 0xd8 => {
                let TargetReg::Cond(condition) = &opcode.reg1 else {
                    panic!("Expected Cond register")
                };
                let should_execute = match condition {
                    0 => !self.flags.contains(CpuFlag::zero), // Cond(0) => zero flags is not set
                    1 => self.flags.contains(CpuFlag::zero),  // Cond(1) => zero flag is set
                    2 => !self.flags.contains(CpuFlag::carry), // Cond(3) => carry flag is not set
                    3 => self.flags.contains(CpuFlag::carry), // Cond(3) => carry flag is set
                    _ => panic!("Condition Codes are 0-3. Received {}", condition),
                };
                if should_execute {
                    // inc cycle count
                    self.cycles += 3;
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
                self.a += self.flags.contains(CpuFlag::carry) as u8; // carry bit goes into bit 0
                self.flags.remove(CpuFlag::zero);
                self.flags.remove(CpuFlag::subtraction);
                self.flags.remove(CpuFlag::half_carry);
                self.flags.set(CpuFlag::carry, left_bit_set);
            }
            // RLCA
            0x07 => {
                let left_bit_set = self.a & 0b1000_0000 != 0;
                self.a <<= 1;
                self.a += left_bit_set as u8; // left bit goes into bit 0
                self.flags.remove(CpuFlag::zero);
                self.flags.remove(CpuFlag::subtraction);
                self.flags.remove(CpuFlag::half_carry);
                self.flags.set(CpuFlag::carry, left_bit_set);
            }
            // RRA
            0x1f => {
                let right_bit_set = self.a & 0b1 > 0;
                self.a >>= 1;
                self.a += (self.flags.contains(CpuFlag::carry) as u8) << 7;
                self.flags.remove(CpuFlag::zero);
                self.flags.remove(CpuFlag::subtraction);
                self.flags.remove(CpuFlag::half_carry);
                self.flags.set(CpuFlag::carry, right_bit_set);
            }
            // RRCA
            0x0f => {
                let right_bit_set = self.a & 0b1 != 0;
                self.a >>= 1;
                self.a += (right_bit_set as u8) << 7;
                self.flags.remove(CpuFlag::zero);
                self.flags.remove(CpuFlag::subtraction);
                self.flags.remove(CpuFlag::half_carry);
                self.flags.set(CpuFlag::carry, right_bit_set);
            }
            // RST
            0xc7 | 0xcf | 0xd7 | 0xdf | 0xe7 | 0xef | 0xf7 | 0xff => {
                let TargetReg::Tgt3(tgt) = &opcode.reg1 else {
                    panic!("Expected Tgt register")
                };
                let addr = self.tgt3_read(*tgt);
                // push next instruction onto the stack
                self.push_u16_to_stack(self.program_counter + 1);
                self.program_counter = addr.wrapping_sub(1); // -1 since rst instruction is one byte long
            }
            // SBC A, r8
            0x98..=0x9f => {
                let TargetReg::R8(reg) = &opcode.reg2 else {
                    panic!("Opcode needs R8 but it is not")
                };
                let val = self.r8_read(*reg);
                self.a = self.sub_u8(self.a, val, true);
            }
            // SBC A, imm8
            0xde => {
                let val = self.bus.mem_read(self.program_counter + 1);
                self.a = self.sub_u8(self.a, val, true);
            }
            // SCF
            0x37 => {
                self.flags.remove(CpuFlag::subtraction);
                self.flags.remove(CpuFlag::half_carry);
                self.flags.set(CpuFlag::carry, true);
            }
            // STOP
            0x10 => {
                // does nothing
            }
            // SUB A, r8
            0x90..=0x97 => {
                let TargetReg::R8(reg) = &opcode.reg2 else {
                    panic!("Opcode needs R8 but it is not")
                };
                let val = self.r8_read(*reg);
                self.a = self.sub_u8(self.a, val, false);
            }
            // SUB A, imm8
            0xd6 => {
                let val = self.bus.mem_read(self.program_counter + 1);
                self.a = self.sub_u8(self.a, val, false);
            }
            // XOR A, r8
            0xa8..=0xaf => {
                let TargetReg::R8(reg) = &opcode.reg2 else {
                    panic!("Opcode needs R8 but it is not")
                };
                let val = self.r8_read(*reg);
                self.a ^= val;

                self.flags.set(CpuFlag::zero, self.a == 0);
                self.flags.set(CpuFlag::subtraction, false);
                self.flags.set(CpuFlag::carry, false);
                self.flags.set(CpuFlag::half_carry, false);
            }
            // XOR A, imm8
            0xee => {
                let val = self.bus.mem_read(self.program_counter + 1);
                self.a ^= val;

                self.flags.set(CpuFlag::zero, self.a == 0);
                self.flags.set(CpuFlag::subtraction, false);
                self.flags.set(CpuFlag::carry, false);
                self.flags.set(CpuFlag::half_carry, false);
            }
            // Prefixed
            0xcb => {
                self.prefixed_mode = true;
            }
            _ => panic!(
                "Opcode: {:02X} '{}' is not implemented yet",
                byte, opcode.name
            ),
        };
    }

    fn add_u8(&mut self, arg1: u8, arg2: u8, carry: bool) -> u8 {
        let c = (carry && self.flags.contains(CpuFlag::carry)) as u8;
        let (sum, c1) = arg1.overflowing_add(arg2);
        let (sum, c2) = sum.overflowing_add(c); // if either overflows we need to set carry flag

        // Set zero flags if sum is 0
        self.flags.set(CpuFlag::zero, sum == 0);
        // set n flag to 0.
        self.flags.remove(CpuFlag::subtraction);
        // set h flag if overflow occured at bit 3
        let half_carry = (arg1 & 0x0f) + (arg2 & 0x0f) + c;
        self.flags.set(CpuFlag::half_carry, half_carry & 0xf0 > 0);
        // set c flag if overflow occured at bit 7
        self.flags.set(CpuFlag::carry, c1 | c2);

        sum
    }

    fn add_u16(&mut self, arg1: u16, arg2: u16, carry: bool) -> u16 {
        let zero_flag = self.flags.contains(CpuFlag::zero);
        // sum the lower 8 bits first
        let lo_sum = self.add_u8(arg1 as u8, arg2 as u8, carry);
        // sum the upper 8 bits. Carry from lower sum must be used
        let hi_sum = self.add_u8(
            (arg1 >> 8) as u8,
            (arg2 >> 8) as u8,
            self.flags.contains(CpuFlag::carry),
        );

        // Set zero flags back to original
        self.flags.set(CpuFlag::zero, zero_flag);
        // subtraction flag already set to 0 by add_u8.
        // half_carry flag set from add_u8
        // carry flag set from add_u8

        // Convert into u16 and return
        (hi_sum as u16) << 8 | lo_sum as u16
    }

    fn sub_u8(&mut self, arg1: u8, arg2: u8, carry: bool) -> u8 {
        let c = (carry && self.flags.contains(CpuFlag::carry)) as u8;
        let (sum, c1) = arg1.overflowing_sub(arg2);
        let (sum, c2) = sum.overflowing_sub(c);

        self.flags.set(CpuFlag::zero, sum == 0);
        self.flags.set(CpuFlag::subtraction, true);
        self.flags.set(CpuFlag::carry, c1 | c2);
        let half_carry = (arg1 & 0x0f).wrapping_sub(arg2 & 0x0f).wrapping_sub(c) & 0x10 > 0;
        self.flags.set(CpuFlag::half_carry, half_carry);

        sum
    }

    fn add_e8(&mut self, arg1: u16, arg2: u8) -> u16 {
        // Carry and Half carry flags generated by unsigned addition of lower byte
        let lo = self.add_u8(arg1 as u8, arg2, false) as u16;
        let hi = match (self.flags.contains(CpuFlag::carry), arg2 & 0x80 > 0) {
            // No carry and e8 is positive
            (false, false) => arg1,
            // Is carry and e8 is positive
            (true, false) => arg1.wrapping_add(0x0100),
            // No carry (so is carry for subtraction) and e8 is negative
            (false, true) => arg1.wrapping_sub(0x0100),
            // Is carry and e8 is negative
            (true, true) => arg1,
        };
        (hi & 0xff00) + lo
    }
}

#[cfg(test)]
mod tests {
    use crate::cartridge::get_mapper;
    use crate::sdl2_setup;

    use super::*;
    use rand::prelude::*;
    use std::vec;

    fn setup(program: Vec<u8>) -> Cpu {
        let cartridge = get_mapper(&program);
        let (_canvas, _event_pump) = sdl2_setup::setup();
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
