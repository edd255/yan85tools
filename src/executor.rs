use crate::errors::{ExecutionEngineError, ExecutionError, MachineError};
use crate::isa::InstructionSet;
use crate::kernel::Kernel;
use crate::machine::Machine;
use crate::memory::Memory;
use crate::reg_file::RegisterFile;
use crate::yan::{FlagRepr, Instruction, InstructionRepr, InstructionType, Operand, RegisterRepr};

pub struct Executor {}

impl Executor {
    pub fn new() -> Self {
        Executor {}
    }

    pub fn execute(
        instr: Instruction,
        isa: &InstructionSet,
        kernel: &mut Kernel,
        memory: &mut Memory,
        reg_file: &mut RegisterFile,
    ) -> Result<(), ExecutionError> {
        match instr.op.get_type() {
            InstructionType::Register => {
                Self::execute_register_instruction(instr, memory, reg_file, isa)
            }
            InstructionType::RegisterImmediate => Self::imm(instr, reg_file),
            InstructionType::Syscall => Self::sys(instr, kernel, reg_file, isa, memory),
            InstructionType::Flag => Self::jmp(instr, reg_file),
            InstructionType::Invalid => Err(ExecutionError::Execution(
                ExecutionEngineError::InvalidInstruction(),
            )),
        }
    }

    fn execute_register_instruction(
        instr: Instruction,
        memory: &mut Memory,
        reg_file: &mut RegisterFile,
        isa: &InstructionSet,
    ) -> Result<(), ExecutionError> {
        let Instruction { op, arg1, arg2 } = instr;
        let Operand::Reg(reg1) = arg1 else {
            return Err(ExecutionError::Execution(
                ExecutionEngineError::ExpectationError(
                    1,
                    "register".to_string(),
                    format!("{arg1:?}"),
                ),
            ));
        };

        let Operand::Reg(reg2) = arg2 else {
            return Err(ExecutionError::Execution(
                ExecutionEngineError::ExpectationError(
                    2,
                    "register".to_string(),
                    format!("{arg2:?}"),
                ),
            ));
        };
        if (reg1, reg2) == (RegisterRepr::None, RegisterRepr::None)
            && !matches!(
                op,
                InstructionRepr::STK | InstructionRepr::POP | InstructionRepr::PSH
            )
        {
            return Err(ExecutionError::Execution(
                ExecutionEngineError::ExpectationError(0, "TODO".to_string(), "TODO".to_string()),
            ));
        }
        match instr.op {
            InstructionRepr::STK => Self::stk(reg1, reg2, memory, reg_file),
            InstructionRepr::STM => Self::stm(reg1, reg2, memory, reg_file),
            InstructionRepr::ADD => Self::add(reg1, reg2, memory, reg_file),
            InstructionRepr::PSH => Self::psh(reg1, reg2, memory, reg_file),
            InstructionRepr::POP => Self::pop(reg1, reg2, memory, reg_file),
            InstructionRepr::LDM => Self::ldm(reg1, reg2, memory, reg_file),
            InstructionRepr::CMP => Self::cmp(reg1, reg2, memory, reg_file, isa),
            _ => {
                return Err(ExecutionError::Execution(
                    ExecutionEngineError::InvalidInstruction(),
                ));
            }
        }
        Ok(())
    }

    fn stk(
        reg1: RegisterRepr,
        reg2: RegisterRepr,
        memory: &mut Memory,
        reg_file: &mut RegisterFile,
    ) {
        if reg2 != RegisterRepr::None {
            reg_file.inc_sp();
            memory.write_mem(reg_file.read_sp(), reg_file.read_by_repr(reg2).unwrap());
        }
        if reg1 != RegisterRepr::None {
            reg_file.write_by_repr(reg1, memory.read_mem(reg_file.read_sp()));
            reg_file.dec_sp();
        }
    }

    fn stm(
        reg1: RegisterRepr,
        reg2: RegisterRepr,
        memory: &mut Memory,
        reg_file: &mut RegisterFile,
    ) {
        memory.write_mem(
            reg_file.read_by_repr(reg1).unwrap(),
            reg_file.read_by_repr(reg2).unwrap(),
        );
    }

    fn add(
        reg1: RegisterRepr,
        reg2: RegisterRepr,
        memory: &mut Memory,
        reg_file: &mut RegisterFile,
    ) {
        reg_file.write_by_repr(
            reg1,
            reg_file.read_by_repr(reg1).unwrap() + reg_file.read_by_repr(reg2).unwrap(),
        );
    }

    fn psh(
        reg1: RegisterRepr,
        reg2: RegisterRepr,
        memory: &mut Memory,
        reg_file: &mut RegisterFile,
    ) {
        Self::stk(reg1, reg2, memory, reg_file);
    }

    fn pop(
        reg1: RegisterRepr,
        reg2: RegisterRepr,
        memory: &mut Memory,
        reg_file: &mut RegisterFile,
    ) {
        Self::stk(reg1, reg2, memory, reg_file);
    }

    fn ldm(
        reg1: RegisterRepr,
        reg2: RegisterRepr,
        memory: &mut Memory,
        reg_file: &mut RegisterFile,
    ) {
        reg_file.write_by_repr(reg1, memory.read_mem(reg_file.read_by_repr(reg2).unwrap()));
    }

    fn cmp(
        reg1: RegisterRepr,
        reg2: RegisterRepr,
        memory: &mut Memory,
        reg_file: &mut RegisterFile,
        isa: &InstructionSet,
    ) {
        let val1 = reg_file.read_by_repr(reg1).unwrap();
        let val2 = reg_file.read_by_repr(reg2).unwrap();
        reg_file.write_flags(0x00);
        if val1 < val2 {
            reg_file.set_flag_bit(isa.get_flag_enc(FlagRepr::L).unwrap());
        }
        if val1 > val2 {
            reg_file.set_flag_bit(isa.get_flag_enc(FlagRepr::G).unwrap());
        }
        if val1 == val2 {
            reg_file.set_flag_bit(isa.get_flag_enc(FlagRepr::E).unwrap());
        }
        let result = val1;
        if result != val2 {
            reg_file.set_flag_bit(isa.get_flag_enc(FlagRepr::N).unwrap());
        }
        if val1 == 0 && val2 == 0 {
            reg_file.set_flag_bit(isa.get_flag_enc(FlagRepr::Z).unwrap());
        }
    }

    fn imm(instr: Instruction, reg_file: &mut RegisterFile) -> Result<(), ExecutionError> {
        let Instruction { op, arg1, arg2 } = instr;
        assert!(op == InstructionRepr::IMM);
        let Operand::Reg(reg) = arg1 else {
            return Err(ExecutionError::Execution(
                ExecutionEngineError::ExpectationError(
                    1,
                    "register".to_string(),
                    format!("{arg1:?}"),
                ),
            ));
        };
        let Operand::Val(val) = arg2 else {
            return Err(ExecutionError::Execution(
                ExecutionEngineError::ExpectationError(2, "value".to_string(), format!("{arg2:?}")),
            ));
        };
        reg_file.write_by_repr(reg, val);
        Ok(())
    }

    fn sys(
        instr: Instruction,
        kernel: &mut Kernel,
        reg_file: &mut RegisterFile,
        isa: &InstructionSet,
        memory: &mut Memory,
    ) -> Result<(), ExecutionError> {
        let Instruction { op, arg1, arg2 } = instr;
        assert!(op == InstructionRepr::SYS);
        let Operand::Sys(syscall) = arg1 else {
            return Err(ExecutionError::Execution(
                ExecutionEngineError::ExpectationError(1, "value".to_string(), format!("{arg1:?}")),
            ));
        };
        let Operand::Reg(reg) = arg2 else {
            return Err(ExecutionError::Execution(
                ExecutionEngineError::ExpectationError(
                    2,
                    "register".to_string(),
                    format!("{arg2:?}"),
                ),
            ));
        };
        if let Err(e) = kernel.sys(syscall, reg, reg_file, isa, memory) {
            Err(ExecutionError::Kernel(e))
        } else {
            Ok(())
        }
    }

    fn jmp(instr: Instruction, reg_file: &mut RegisterFile) -> Result<(), ExecutionError> {
        let Instruction { op, arg1, arg2 } = instr;
        assert!(op == InstructionRepr::JMP);
        let Operand::Flg(flags) = arg1 else {
            return Err(ExecutionError::Execution(
                ExecutionEngineError::ExpectationError(1, "flags".to_string(), format!("{arg1:?}")),
            ));
        };
        let Operand::Reg(reg) = arg2 else {
            return Err(ExecutionError::Execution(
                ExecutionEngineError::ExpectationError(
                    2,
                    "register".to_string(),
                    format!("{arg2:?}"),
                ),
            ));
        };
        if flags != 0x00 && reg_file.read_flags() & flags == 0 {
            return Ok(());
        }
        reg_file.write_pc(reg_file.read_by_repr(reg).unwrap());
        Ok(())
    }
}
