use crate::{cpu::Cpu, opcodes};

use std::collections::HashMap;

pub fn trace_cpu(cpu: &mut Cpu) {
    // Get number of bytes from current opcode
    let pc = cpu.program_counter;
    let opcode_byte = cpu.bus.mem_read(pc);
    let (opcode, opcode_name) = if cpu.prefixed_mode {
        let opcodes: &HashMap<u8, opcodes::Opcode> = &opcodes::CPU_PREFIXED_OP_CODES;
        let opcode = opcodes.get(&opcode_byte).unwrap();
        let actual_op = cpu.bus.mem_read(pc + 1);
        let opcode_name = opcodes.get(&actual_op).unwrap();
        (opcode, opcode_name.name)
    } else {
        let opcodes: &HashMap<u8, opcodes::Opcode> = &opcodes::CPU_OP_CODES;
        let opcode = opcodes
            .get(&opcode_byte)
            .unwrap_or_else(|| panic!("Invalid opcode received: {opcode_byte:02X}"));
        (opcode, opcode.name)
    };

    // Get all bytes involved in the opcode
    let mut opcode_as_bytes = Vec::new();
    for i in 1..opcode.bytes {
        opcode_as_bytes.push(cpu.bus.mem_read(pc.wrapping_add(i)));
    }

    let mut opcode_format = format!("{opcode_byte:02X}");
    // Todo: Add Assembly style format of the opcode and values
    // let mut asm_format = format!("{}", opcode.name);
    if let Some(first_byte) = opcode_as_bytes.first() {
        opcode_format = format!("{opcode_format} {first_byte:02X}");
    }
    if let Some(second_byte) = opcode_as_bytes.get(1) {
        opcode_format = format!("{opcode_format} {second_byte:02X}");
    }

    // Print out formatted log
    let log = format!(
        "{:04X}    {:<8}  {:<5}  AF: {:04X}, BC: {:04X}, DE: {:04X}, HL: {:04X}, SP: {:04X} CB: {}, IME: {}, IE: {:02X}, IF: {:02X}, stat: {:02X} control: {:02X}, cycles: {}, scanline: {}",
        cpu.program_counter,
        opcode_format,
        opcode_name,
        cpu.get_af(),
        cpu.get_bc(),
        cpu.get_de(),
        cpu.get_hl(),
        cpu.stack_pointer,
        cpu.prefixed_mode,
        cpu.ime,
        cpu.bus.interrupt_enable,
        cpu.bus.interrupt_flag,
        cpu.bus.ppu.status,
        cpu.bus.ppu.control,
        cpu.bus.ppu.cycle,
        cpu.bus.ppu.scanline,
    );
    println!("{log}");
}
