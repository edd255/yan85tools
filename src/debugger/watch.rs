use super::Debugger;
use crate::machine::Machine;
use crate::yan::{FlagRepr, Instruction, InstructionRepr, Operand, RegisterRepr, SyscallRepr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Watchpoint {
    Instruction { op: InstructionRepr },
    SyscallMask { mask: u8 },
    RegisterEq { reg: RegisterRepr, value: u8 },
    FlagsAny { mask: u8 },
    FlagsEq { value: u8 },
}

impl Debugger {
    pub(super) fn hit_watchpoint(&self, instr: Instruction) -> Option<String> {
        for (idx, watchpoint) in self.watchpoints.iter().enumerate() {
            if watchpoint.matches(&self.machine, instr) {
                return Some(format!(
                    "watchpoint #{idx} hit: {}",
                    watchpoint.describe(&self.machine.isa)
                ));
            }
        }
        None
    }

    pub(super) fn toggle_breakpoint(&mut self, pc: u8) {
        if self.breakpoints.remove(&pc) {
            self.status = format!("breakpoint removed at 0x{pc:02x}");
        } else {
            self.breakpoints.insert(pc);
            self.status = format!("breakpoint set at 0x{pc:02x}");
        }
    }
}

impl Watchpoint {
    pub fn describe(&self, isa: &crate::isa::InstructionSet) -> String {
        match *self {
            Watchpoint::Instruction { op } => format!("op {op}"),
            Watchpoint::SyscallMask { mask } => {
                let parts = sorted_syscalls(&isa.syscall_map)
                    .into_iter()
                    .filter_map(|(bit, repr)| (mask & bit != 0).then_some(repr.to_string()))
                    .collect::<Vec<_>>();
                if parts.is_empty() {
                    format!("sys 0x{mask:02x}")
                } else {
                    format!("sys {}", parts.join("|"))
                }
            }
            Watchpoint::RegisterEq { reg, value } => format!("reg {reg} = 0x{value:02x}"),
            Watchpoint::FlagsAny { mask } => {
                let parts = sorted_flags(&isa.flag_map)
                    .into_iter()
                    .filter_map(|(bit, repr)| {
                        (bit != 0 && (mask & bit) != 0).then_some(repr.to_string())
                    })
                    .collect::<Vec<_>>();
                if parts.is_empty() {
                    format!("flags any 0x{mask:02x}")
                } else {
                    format!("flags any {}", parts.join(""))
                }
            }
            Watchpoint::FlagsEq { value } => format!("flags = 0x{value:02x}"),
        }
    }

    pub fn matches(&self, machine: &Machine, instr: Instruction) -> bool {
        match *self {
            Watchpoint::Instruction { op } => instr.op == op,
            Watchpoint::SyscallMask { mask } => match instr {
                Instruction {
                    op: InstructionRepr::SYS,
                    arg1: Operand::Sys(sysmask),
                    ..
                } => sysmask & mask != 0,
                _ => false,
            },
            Watchpoint::RegisterEq { reg, value } => {
                machine.reg_file.read_by_repr(reg) == Some(value)
            }
            Watchpoint::FlagsAny { mask } => machine.reg_file.read_flags() & mask != 0,
            Watchpoint::FlagsEq { value } => machine.reg_file.read_flags() == value,
        }
    }
}

pub fn sorted_syscalls(map: &std::collections::HashMap<u8, SyscallRepr>) -> Vec<(u8, SyscallRepr)> {
    let mut values = map
        .iter()
        .map(|(bit, repr)| (*bit, *repr))
        .collect::<Vec<_>>();
    values.sort_by_key(|(bit, _)| *bit);
    values
}

pub fn sorted_flags(map: &std::collections::HashMap<u8, FlagRepr>) -> Vec<(u8, FlagRepr)> {
    let mut values = map
        .iter()
        .map(|(bit, repr)| (*bit, *repr))
        .collect::<Vec<_>>();
    values.sort_by_key(|(bit, _)| *bit);
    values
}

pub fn active_flags(map: &std::collections::HashMap<u8, FlagRepr>, value: u8) -> String {
    let mut parts = Vec::new();
    for (bit, repr) in sorted_flags(map) {
        if bit != 0 && (value & bit) != 0 {
            parts.push(repr.to_string());
        }
    }
    parts.join("")
}
