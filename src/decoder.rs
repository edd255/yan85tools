use crate::isa::InstructionSet;
use crate::yan::{Instruction, InstructionType, Operand};

pub struct Decoder {}

impl Decoder {
    pub fn new() -> Self {
        Decoder {}
    }

    pub fn decode(isa: &InstructionSet, instr: u32) -> Option<Instruction> {
        let bytes = [
            ((instr >> 16) & 0xFF) as u8,
            ((instr >> 8) & 0xFF) as u8,
            (instr & 0xFF) as u8,
        ];
        let op = bytes[isa.op as usize];
        let arg1 = bytes[isa.arg1 as usize];
        let arg2 = bytes[isa.arg2 as usize];
        let instr = isa.instr_map.get(&op)?;

        match instr.get_type() {
            InstructionType::Register => {
                if isa.get_reg_repr_by_enc(arg1).is_none()
                    || isa.get_reg_repr_by_enc(arg2).is_none()
                {
                    return None;
                }
                Some(Instruction {
                    op: *instr,
                    arg1: Operand::Reg(isa.get_reg_repr_by_enc(arg1).unwrap()),
                    arg2: Operand::Reg(isa.get_reg_repr_by_enc(arg2).unwrap()),
                })
            }
            InstructionType::RegisterImmediate => {
                isa.get_reg_repr_by_enc(arg1)?;
                Some(Instruction {
                    op: *instr,
                    arg1: Operand::Reg(isa.get_reg_repr_by_enc(arg1).unwrap()),
                    arg2: Operand::Val(arg2),
                })
            }
            InstructionType::Syscall => {
                isa.get_reg_repr_by_enc(arg2)?;
                if !Self::valid_mask(&isa.syscall_map, arg1) {
                    return None;
                }
                Some(Instruction {
                    op: *instr,
                    arg1: Operand::Sys(arg1),
                    arg2: Operand::Reg(isa.get_reg_repr_by_enc(arg2).unwrap()),
                })
            }
            InstructionType::Flag => {
                isa.get_reg_repr_by_enc(arg2)?;
                if !Self::valid_mask(&isa.flag_map, arg1) {
                    return None;
                }
                Some(Instruction {
                    op: *instr,
                    arg1: Operand::Flg(arg1),
                    arg2: Operand::Reg(isa.get_reg_repr_by_enc(arg2).unwrap()),
                })
            }
            InstructionType::Invalid => None,
        }
    }

    fn valid_mask(map: &std::collections::HashMap<u8, impl Copy>, mask: u8) -> bool {
        let allowed = map.keys().copied().fold(0u8, |acc, bit| acc | bit);
        mask & !allowed == 0
    }
}
