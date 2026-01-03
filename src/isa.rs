use crate::errors::ConfigError;
use crate::yan::{
    FlagRepr, InstructionRepr, RegisterPurpose, RegisterRepr, RegisterStub, SyscallRepr,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::hash::Hash;

#[derive(Deserialize)]
struct RawInstructionSet {
    syscalls: HashMap<SyscallRepr, u8>,
    flags: HashMap<FlagRepr, u8>,
    registers: HashMap<RegisterRepr, RawRegister>,
    instructions: HashMap<InstructionRepr, u8>,
    encoding: RawEncoding,
}

#[derive(Deserialize)]
struct RawRegister {
    enc: u8,
    addr: u16,
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
struct RawEncoding {
    LEN: u8,
    ARG1: u8,
    OP: u8,
    ARG2: u8,
    MAX_VAL: u8,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct InstructionSet {
    pub flag_map: HashMap<u8, FlagRepr>,
    pub instr_map: HashMap<u8, InstructionRepr>,
    pub reg_map: HashMap<u8, RegisterStub>,
    pub syscall_map: HashMap<u8, SyscallRepr>,
    pub max_val: u8,
    pub instr_len: u8,
    pub arg1: u8,
    pub arg2: u8,
    pub op: u8,
}

impl InstructionSet {
    pub fn parse(config_path: &str) -> Result<InstructionSet, ConfigError> {
        let contents = fs::read_to_string(config_path).map_err(|_| ConfigError::FileNotFound())?;
        let raw: RawInstructionSet =
            toml::from_str(&contents).map_err(|_| ConfigError::ParserError())?;
        Ok(InstructionSet {
            flag_map: Self::reverse_map(raw.flags),
            instr_map: Self::reverse_map(raw.instructions),
            reg_map: Self::build_reg_map(raw.registers),
            syscall_map: Self::reverse_map(raw.syscalls),
            max_val: raw.encoding.MAX_VAL,
            instr_len: raw.encoding.LEN,
            arg1: raw.encoding.ARG1,
            arg2: raw.encoding.ARG2,
            op: raw.encoding.OP,
        })
    }

    fn reverse_map<K>(map: HashMap<K, u8>) -> HashMap<u8, K>
    where
        K: Eq + Hash,
    {
        let mut out = HashMap::with_capacity(map.len());
        for (repr, enc) in map {
            out.insert(enc, repr);
        }
        out
    }

    fn build_reg_map(raw: HashMap<RegisterRepr, RawRegister>) -> HashMap<u8, RegisterStub> {
        let mut out = HashMap::with_capacity(raw.len());
        for (repr, reg) in raw {
            out.insert(
                reg.enc,
                RegisterStub {
                    purpose: Self::register_purpose(repr),
                    repr,
                    addr: reg.addr,
                },
            );
        }
        out
    }

    fn register_purpose(repr: RegisterRepr) -> RegisterPurpose {
        match repr {
            RegisterRepr::A | RegisterRepr::B | RegisterRepr::C | RegisterRepr::D => {
                RegisterPurpose::GeneralPurpose
            }
            RegisterRepr::S => RegisterPurpose::StackPointer,
            RegisterRepr::P => RegisterPurpose::InstructionPointer,
            RegisterRepr::F => RegisterPurpose::Flags,
            RegisterRepr::None => RegisterPurpose::None,
        }
    }

    pub fn get_reg_repr_by_enc(&self, enc: u8) -> Option<RegisterRepr> {
        if (enc == 0x00) {
            return Some(RegisterRepr::None);
        }
        let reg = self.reg_map.get(&enc)?;
        Some(reg.repr)
    }

    pub fn get_reg_repr_by_addr(&self, addr: u16) -> Option<RegisterRepr> {
        for reg in self.reg_map.values() {
            if reg.addr == addr {
                return Some(reg.repr);
            }
        }
        None
    }

    pub fn get_flag_enc(&self, flag: FlagRepr) -> Option<u8> {
        for enc in self.flag_map.keys() {
            if self.flag_map[enc] == flag {
                return Some(*enc);
            }
        }
        None
    }

    pub fn get_syscall(&self, syscall: SyscallRepr) -> Option<u8> {
        for enc in self.syscall_map.keys() {
            if self.syscall_map[enc] == syscall {
                return Some(*enc);
            }
        }
        None
    }
}
