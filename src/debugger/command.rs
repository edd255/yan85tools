use super::Debugger;
use crate::debugger::watch::Watchpoint;
use crate::yan::{FlagRepr, Instruction, InstructionRepr, Operand, RegisterRepr, SyscallRepr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParsedCommand {
    Break(u8),
    Delete(u8),
    Watch(Watchpoint),
    Unwatch(usize),
    List,
    Clear,
    Help,
}

impl Debugger {
    pub(super) fn parse_command(&self, input: &str) -> Result<ParsedCommand, String> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return Err("empty command".to_string());
        }
        match parts.as_slice() {
            ["break" | "b", addr] => {
                let addr = parse_u8(addr).ok_or_else(|| "invalid address".to_string())?;
                Ok(ParsedCommand::Break(addr))
            }
            ["delete" | "del", addr] => {
                let addr = parse_u8(addr).ok_or_else(|| "invalid address".to_string())?;
                Ok(ParsedCommand::Delete(addr))
            }
            ["watch", "op", op] => {
                let op = parse_instruction(op).ok_or_else(|| "invalid instruction".to_string())?;
                Ok(ParsedCommand::Watch(Watchpoint::Instruction { op }))
            }
            ["watch", "sys", spec] => {
                let mask = parse_syscall_mask(&self.machine.isa, spec)
                    .ok_or_else(|| "invalid syscall mask".to_string())?;
                Ok(ParsedCommand::Watch(Watchpoint::SyscallMask { mask }))
            }
            ["watch", "reg", reg, "=", value] => {
                let reg = parse_register(reg).ok_or_else(|| "invalid register".to_string())?;
                let value = parse_u8(value).ok_or_else(|| "invalid value".to_string())?;
                Ok(ParsedCommand::Watch(Watchpoint::RegisterEq { reg, value }))
            }
            ["watch", "flags", "any", spec] => {
                let mask = parse_flag_mask(&self.machine.isa, spec)
                    .ok_or_else(|| "invalid flag mask".to_string())?;
                Ok(ParsedCommand::Watch(Watchpoint::FlagsAny { mask }))
            }
            ["watch", "flags", "=", value] => {
                let value = parse_u8(value).ok_or_else(|| "invalid flags value".to_string())?;
                Ok(ParsedCommand::Watch(Watchpoint::FlagsEq { value }))
            }
            ["unwatch", idx] => {
                let idx = idx
                    .parse::<usize>()
                    .map_err(|_| "invalid watchpoint index".to_string())?;
                Ok(ParsedCommand::Unwatch(idx))
            }
            ["list"] => Ok(ParsedCommand::List),
            ["clear"] => Ok(ParsedCommand::Clear),
            ["help"] => Ok(ParsedCommand::Help),
            _ => Err("unknown command".to_string()),
        }
    }

    pub(super) fn apply_command(&mut self, cmd: ParsedCommand) {
        match cmd {
            ParsedCommand::Break(addr) => {
                self.breakpoints.insert(addr);
                self.status = format!("breakpoint set at 0x{addr:02x}");
            }
            ParsedCommand::Delete(addr) => {
                if self.breakpoints.remove(&addr) {
                    self.status = format!("breakpoint removed at 0x{addr:02x}");
                } else {
                    self.status = format!("no breakpoint at 0x{addr:02x}");
                }
            }
            ParsedCommand::Watch(watchpoint) => {
                self.watchpoints.push(watchpoint);
                let idx = self.watchpoints.len() - 1;
                self.status = format!(
                    "watchpoint #{idx} set: {}",
                    watchpoint.describe(&self.machine.isa)
                );
            }
            ParsedCommand::Unwatch(idx) => {
                if idx < self.watchpoints.len() {
                    let watchpoint = self.watchpoints.remove(idx);
                    self.status = format!(
                        "watchpoint #{idx} removed: {}",
                        watchpoint.describe(&self.machine.isa)
                    );
                } else {
                    self.status = format!("no watchpoint #{idx}");
                }
            }
            ParsedCommand::List => {
                self.status = self.list_points();
            }
            ParsedCommand::Clear => {
                self.breakpoints.clear();
                self.watchpoints.clear();
                self.status = "cleared breakpoints and watchpoints".to_string();
            }
            ParsedCommand::Help => {
                self.show_help = true;
                self.status = "help".to_string();
            }
        }
    }
}

pub fn parse_u8(raw: &str) -> Option<u8> {
    if let Some(hex) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
        u8::from_str_radix(hex, 16).ok()
    } else {
        raw.parse::<u8>().ok()
    }
}

pub fn parse_register(raw: &str) -> Option<RegisterRepr> {
    match raw.to_ascii_lowercase().as_str() {
        "a" => Some(RegisterRepr::A),
        "b" => Some(RegisterRepr::B),
        "c" => Some(RegisterRepr::C),
        "d" => Some(RegisterRepr::D),
        "s" => Some(RegisterRepr::S),
        "i" | "p" => Some(RegisterRepr::P),
        "f" => Some(RegisterRepr::F),
        _ => None,
    }
}

pub fn parse_instruction(raw: &str) -> Option<InstructionRepr> {
    match raw.to_ascii_uppercase().as_str() {
        "IMM" => Some(InstructionRepr::IMM),
        "ADD" => Some(InstructionRepr::ADD),
        "CMP" => Some(InstructionRepr::CMP),
        "STM" => Some(InstructionRepr::STM),
        "LDM" => Some(InstructionRepr::LDM),
        "JMP" => Some(InstructionRepr::JMP),
        "SYS" => Some(InstructionRepr::SYS),
        "STK" | "PSH" | "POP" => Some(InstructionRepr::STK),
        "INV" => Some(InstructionRepr::INV),
        _ => None,
    }
}

pub fn parse_syscall_mask(isa: &crate::isa::InstructionSet, raw: &str) -> Option<u8> {
    let mut mask = 0u8;
    for part in raw.split(['|', '+']) {
        let part = part.trim().to_ascii_uppercase();
        if part.is_empty() {
            continue;
        }
        let repr = SyscallRepr::from_name(&part)?;
        let bit = isa.get_syscall(repr)?;
        mask |= bit;
    }
    Some(mask)
}

pub fn parse_flag_mask(isa: &crate::isa::InstructionSet, raw: &str) -> Option<u8> {
    let mut mask = 0u8;
    let normalized = raw.replace(['|', '+', ' '], "");
    if let Some(value) = parse_u8(&normalized) {
        return Some(value);
    }
    for ch in normalized.chars() {
        let repr = match ch.to_ascii_uppercase() {
            'G' => FlagRepr::G,
            'E' => FlagRepr::E,
            'N' => FlagRepr::N,
            'L' => FlagRepr::L,
            'Z' => FlagRepr::Z,
            '*' | 'S' => continue,
            _ => return None,
        };
        let bit = isa.get_flag_enc(repr)?;
        mask |= bit;
    }
    Some(mask)
}
