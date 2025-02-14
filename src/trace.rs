use crate::{
    cpu::Cpu,
    opcodes,
};

use std::collections::HashMap;

pub fn trace_cpu(cpu: &mut Cpu) {
    // Get number of bytes from current opcode
    let pc = cpu.program_counter;
    let opcode_byte = cpu.bus.mem_read(pc);
    let bytes = if cpu.prefixed_mode {
        let opcodes: &HashMap<u8, opcodes::Opcode> = &opcodes::CPU_PREFIXED_OP_CODES;
        let opcode = opcodes.get(&opcode_byte).unwrap();
        opcode.bytes
    } else {
        let opcodes: &HashMap<u8, opcodes::Opcode> = &opcodes::CPU_OP_CODES;
        let opcode = opcodes.get(&opcode_byte).unwrap();
        opcode.bytes
    };

    // Get all bytes involved in the opcode
    let mut opcode_as_bytes = Vec::new();
    for i in 1..bytes {
        opcode_as_bytes.push(cpu.bus.mem_read(pc + i));
    }

    let mut opcode_format = format!("{:02X}", opcode_byte);
    if let Some(first_byte) = opcode_as_bytes.get(0) {
        opcode_format = format!("{} {:02X}", opcode_format, first_byte);
    }
    if let Some(second_byte) = opcode_as_bytes.get(1) {
        opcode_format = format!("{} {:02X}", opcode_format, second_byte);
    }

    // Print out formatted log
    let log = format!(
        "PC: {:04X}    OP: {:<12}    AF: {:04X}, BC: {:04X}, DE: {:04X}, HL: {:04X}, SP: {:04X} CB: {}",
        cpu.program_counter,
        opcode_format,
        cpu.get_af(),
        cpu.get_bc(),
        cpu.get_de(),
        cpu.get_hl(),
        cpu.stack_pointer,
        cpu.prefixed_mode,
    );
    println!("{}", log);
}
