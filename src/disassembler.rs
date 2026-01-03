use crate::decoder::Decoder;
use crate::isa::InstructionSet;
use crate::memory::Memory;
use crate::yan::{Instruction, InstructionType, Operand};

pub struct Disassembler {}

impl Disassembler {
    pub fn new() -> Self {
        Disassembler {}
    }

    pub fn disassemble(isa: &InstructionSet, memory: &Memory) -> Vec<String> {
        let mut out = Vec::new();
        let mut pc = 0_u8;
        while pc != 252 {
            let bytes = memory.read_instruction(pc);
            if bytes == 0x00 {
                break;
            }
            let instr = match Decoder::decode(isa, bytes) {
                Some(i) => i.as_rich_str(&isa.syscall_map, &isa.flag_map),
                None => "INVALID".to_string(),
            };
            out.push(instr);
            pc += 3;
        }
        out
    }
}
