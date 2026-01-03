use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Write;

#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone, Deserialize)]
pub enum FlagRepr {
    S,
    G,
    E,
    N,
    L,
    Z,
}

impl FlagRepr {
    pub fn as_str(self) -> &'static str {
        match self {
            FlagRepr::S => "*",
            FlagRepr::G => "G",
            FlagRepr::E => "E",
            FlagRepr::N => "N",
            FlagRepr::L => "L",
            FlagRepr::Z => "Z",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "S" => Some(FlagRepr::S),
            "G" => Some(FlagRepr::G),
            "E" => Some(FlagRepr::E),
            "N" => Some(FlagRepr::N),
            "L" => Some(FlagRepr::L),
            "Z" => Some(FlagRepr::Z),
            _ => None,
        }
    }
}

impl fmt::Display for FlagRepr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone, Deserialize)]
pub enum InstructionRepr {
    JMP,
    STK,
    STM,
    ADD,
    IMM,
    LDM,
    CMP,
    SYS,
    INV,
    PSH,
    POP,
}

impl InstructionRepr {
    fn as_str(self) -> &'static str {
        match self {
            InstructionRepr::JMP => "JMP",
            InstructionRepr::STK => "STK",
            InstructionRepr::STM => "STM",
            InstructionRepr::ADD => "ADD",
            InstructionRepr::IMM => "IMM",
            InstructionRepr::LDM => "LDM",
            InstructionRepr::CMP => "CMP",
            InstructionRepr::SYS => "SYS",
            InstructionRepr::INV => "INV",
            InstructionRepr::PSH => "PSH",
            InstructionRepr::POP => "POP",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "JMP" => Some(InstructionRepr::JMP),
            "STK" => Some(InstructionRepr::STK),
            "STM" => Some(InstructionRepr::STM),
            "ADD" => Some(InstructionRepr::ADD),
            "IMM" => Some(InstructionRepr::IMM),
            "LDM" => Some(InstructionRepr::LDM),
            "CMP" => Some(InstructionRepr::CMP),
            "SYS" => Some(InstructionRepr::SYS),
            "INV" => Some(InstructionRepr::INV),
            "PSH" => Some(InstructionRepr::PSH),
            "POP" => Some(InstructionRepr::POP),
            _ => None,
        }
    }

    pub fn get_type(self) -> InstructionType {
        match self {
            InstructionRepr::STK
            | InstructionRepr::STM
            | InstructionRepr::ADD
            | InstructionRepr::PSH
            | InstructionRepr::POP
            | InstructionRepr::LDM
            | InstructionRepr::CMP => InstructionType::Register,
            InstructionRepr::IMM => InstructionType::RegisterImmediate,
            InstructionRepr::SYS => InstructionType::Syscall,
            InstructionRepr::JMP => InstructionType::Flag,
            InstructionRepr::INV => InstructionType::Invalid,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub enum InstructionType {
    RegisterImmediate,
    Register,
    Flag,
    Syscall,
    Invalid,
}

impl fmt::Display for InstructionRepr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub enum Operand {
    Val(u8),
    Reg(RegisterRepr),
    Sys(u8),
    Flg(u8),
}

fn print_operand_kind(op: Operand) {
    match op {
        Operand::Val(_) => println!("Value"),
        Operand::Reg(_) => println!("Register"),
        Operand::Sys(_) => println!("Syscall"),
        Operand::Flg(_) => println!("Flag"),
    }
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Operand::Val(v) => write!(f, "0x{v:02x}"),
            Operand::Reg(r) => write!(f, "{r:?}"),
            Operand::Sys(s) => write!(f, "{s:?}"),
            Operand::Flg(flg) => write!(f, "{flg:?}"),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub struct Instruction {
    pub op: InstructionRepr,
    pub arg1: Operand,
    pub arg2: Operand,
}

impl Instruction {
    pub fn as_str(self) -> String {
        match self.op {
            InstructionRepr::IMM => format!("{} {} = {}", self.op, self.arg1, self.arg2),
            InstructionRepr::ADD
            | InstructionRepr::CMP
            | InstructionRepr::SYS
            | InstructionRepr::JMP => format!("{} {} {}", self.op, self.arg1, self.arg2),
            InstructionRepr::STK => {
                let mut repr = String::new();
                if let Operand::Reg(reg2) = self.arg2
                    && reg2.get_purpose() != RegisterPurpose::None
                {
                    write!(repr, "{} {}", InstructionRepr::PSH, self.arg2);
                }
                if let Operand::Reg(reg1) = self.arg1
                    && reg1.get_purpose() != RegisterPurpose::None
                {
                    if !repr.is_empty() {
                        write!(repr, "; ");
                    }
                    write!(repr, "{} {}", InstructionRepr::POP, self.arg1);
                }
                if repr.is_empty() {
                    format!("{} {} {}", self.op, self.arg1, self.arg2)
                } else {
                    repr
                }
            }
            InstructionRepr::STM => format!("{} *{} = {}", self.op, self.arg1, self.arg2),
            InstructionRepr::LDM => format!("{} {} = *{}", self.op, self.arg1, self.arg2),
            _ => format!("{:?} {} {}", self.op, self.arg1, self.arg2),
        }
    }

    pub fn as_rich_str(
        self,
        sys_map: &HashMap<u8, SyscallRepr>,
        flag_map: &HashMap<u8, FlagRepr>,
    ) -> String {
        match self.op {
            InstructionRepr::IMM => {
                if let Operand::Reg(reg1) = self.arg1 {
                    format!("{} {} = {}", self.op, reg1, self.arg2)
                } else {
                    "INVALID".to_string()
                }
            }
            InstructionRepr::ADD | InstructionRepr::CMP => {
                if let Operand::Reg(reg1) = self.arg1
                    && let Operand::Reg(reg2) = self.arg2
                {
                    format!("{} {} {}", self.op, reg1, reg2)
                } else {
                    "INVALID".to_string()
                }
            }
            InstructionRepr::STK => {
                let mut repr = String::new();
                if let Operand::Reg(reg2) = self.arg2
                    && reg2.get_purpose() != RegisterPurpose::None
                {
                    write!(repr, "{} {}", InstructionRepr::PSH, self.arg2);
                }
                if let Operand::Reg(reg1) = self.arg1
                    && reg1.get_purpose() != RegisterPurpose::None
                {
                    if !repr.is_empty() {
                        write!(repr, "; ");
                    }
                    write!(repr, "{} {}", InstructionRepr::POP, self.arg1);
                }
                if repr.is_empty() {
                    format!("{} {} {}", self.op, self.arg1, self.arg2)
                } else {
                    repr
                }
            }
            InstructionRepr::STM => {
                if let Operand::Reg(reg1) = self.arg1
                    && let Operand::Reg(reg2) = self.arg2
                {
                    format!("{} *{} = {}", self.op, reg1, reg2)
                } else {
                    "INVALID".to_string()
                }
            }
            InstructionRepr::LDM => {
                if let Operand::Reg(reg1) = self.arg1
                    && let Operand::Reg(reg2) = self.arg2
                {
                    format!("{} {} = *{}", self.op, reg1, reg2)
                } else {
                    "INVALID".to_string()
                }
            }
            InstructionRepr::SYS => match (&self.arg1, &self.arg2) {
                (Operand::Sys(syscall), Operand::Reg(reg)) => {
                    let mut parts = vec![self.op.to_string()];
                    for (bit, repr) in sys_map {
                        if syscall & bit != 0 {
                            parts.push(format!("{repr}"));
                        }
                    }
                    parts.push(format!("{reg}"));
                    parts.join(" ")
                }
                _ => format!("INVALID SYSCALL: {}", self.arg1),
            },
            InstructionRepr::JMP => match (&self.arg1, &self.arg2) {
                (Operand::Flg(flag), Operand::Reg(reg)) => {
                    let mut parts = vec![self.op.to_string()];
                    for (bit, repr) in flag_map {
                        if flag & bit != 0 {
                            parts.push(format!("{repr}"));
                        }
                    }
                    parts.push(format!("{reg}"));
                    parts.join(" ")
                }
                _ => format!("INVALID FLAG: {}", self.arg1),
            },
            _ => format!("{:?} {} {}", self.op, self.arg1, self.arg2),
        }
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub enum RegisterPurpose {
    GeneralPurpose,
    StackPointer,
    InstructionPointer,
    Flags,
    None,
}

#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone, Deserialize)]
pub enum RegisterRepr {
    A,
    B,
    C,
    D,
    S,
    P,
    F,
    None,
}

impl RegisterRepr {
    pub fn as_str(self) -> &'static str {
        match self {
            RegisterRepr::A => "a",
            RegisterRepr::B => "b",
            RegisterRepr::C => "c",
            RegisterRepr::D => "d",
            RegisterRepr::S => "s",
            RegisterRepr::P => "i",
            RegisterRepr::F => "f",
            RegisterRepr::None => "none",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "A" => Some(RegisterRepr::A),
            "B" => Some(RegisterRepr::B),
            "C" => Some(RegisterRepr::C),
            "D" => Some(RegisterRepr::D),
            "S" => Some(RegisterRepr::S),
            "P" => Some(RegisterRepr::P),
            "F" => Some(RegisterRepr::F),
            _ => None,
        }
    }

    fn get_purpose(self) -> RegisterPurpose {
        match self {
            RegisterRepr::A | RegisterRepr::B | RegisterRepr::C | RegisterRepr::D => {
                RegisterPurpose::GeneralPurpose
            }
            RegisterRepr::S => RegisterPurpose::StackPointer,
            RegisterRepr::P => RegisterPurpose::InstructionPointer,
            RegisterRepr::F => RegisterPurpose::Flags,
            RegisterRepr::None => RegisterPurpose::None,
        }
    }
}

impl fmt::Display for RegisterRepr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub struct RegisterStub {
    pub purpose: RegisterPurpose,
    pub repr: RegisterRepr,
    pub addr: u16,
}

#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub struct Register {
    pub purpose: RegisterPurpose,
    pub repr: RegisterRepr,
    pub addr: u16,
    pub val: u8,
}

#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone, Deserialize)]
pub enum SyscallRepr {
    #[serde(rename = "OPEN")]
    Open,
    #[serde(rename = "READ_CODE")]
    ReadCode,
    #[serde(rename = "READ_MEM")]
    ReadMemory,
    #[serde(rename = "WRITE")]
    Write,
    #[serde(rename = "SLEEP")]
    Sleep,
    #[serde(rename = "EXIT")]
    Exit,
    #[serde(rename = "INVALID")]
    Invalid,
}

impl SyscallRepr {
    pub fn as_str(self) -> &'static str {
        match self {
            SyscallRepr::Open => "OPEN",
            SyscallRepr::ReadCode => "READ_CODE",
            SyscallRepr::ReadMemory => "READ_MEM",
            SyscallRepr::Write => "WRITE",
            SyscallRepr::Sleep => "SLEEP",
            SyscallRepr::Exit => "EXIT",
            SyscallRepr::Invalid => "INVALID",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "OPEN" => Some(SyscallRepr::Open),
            "READ_CODE" => Some(SyscallRepr::ReadCode),
            "READ_MEM" => Some(SyscallRepr::ReadMemory),
            "WRITE" => Some(SyscallRepr::Write),
            "SLEEP" => Some(SyscallRepr::Sleep),
            "EXIT" => Some(SyscallRepr::Exit),
            "INVALID" => Some(SyscallRepr::Invalid),
            _ => None,
        }
    }
}

impl fmt::Display for SyscallRepr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
