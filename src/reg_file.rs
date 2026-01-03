use crate::errors::RegisterFileError;
use crate::yan::{Register, RegisterPurpose, RegisterRepr, RegisterStub};
use std::collections::HashMap;

pub struct RegisterFile {
    pub reg_map: HashMap<u8, Register>,
    sp_enc: u8,
    pc_enc: u8,
    flags_enc: u8,
}

impl RegisterFile {
    pub fn new(reg_stub_map: &HashMap<u8, RegisterStub>) -> Result<Self, RegisterFileError> {
        let mut sp_enc_opt: Option<u8> = None;
        let mut pc_enc_opt: Option<u8> = None;
        let mut flags_enc_opt: Option<u8> = None;
        for (enc, reg) in reg_stub_map {
            match reg.purpose {
                RegisterPurpose::StackPointer => sp_enc_opt = Some(*enc),
                RegisterPurpose::InstructionPointer => pc_enc_opt = Some(*enc),
                RegisterPurpose::Flags => flags_enc_opt = Some(*enc),
                _ => {}
            }
            if sp_enc_opt.is_some() && pc_enc_opt.is_some() && flags_enc_opt.is_some() {
                break;
            }
        }
        let sp_enc = sp_enc_opt.ok_or(RegisterFileError::RegisterNotFound(
            "Stack Pointer".to_string(),
        ))?;
        let pc_enc = pc_enc_opt.ok_or(RegisterFileError::RegisterNotFound(
            "Instruction Pointer".to_string(),
        ))?;
        let flags_enc = flags_enc_opt.ok_or(RegisterFileError::RegisterNotFound(
            "Flags Register".to_string(),
        ))?;
        let mut reg_map = HashMap::new();
        for (enc, reg_stub) in reg_stub_map {
            reg_map.insert(
                *enc,
                Register {
                    purpose: reg_stub.purpose,
                    repr: reg_stub.repr,
                    addr: reg_stub.addr,
                    val: 0x00,
                },
            );
        }
        Ok(RegisterFile {
            reg_map,
            sp_enc,
            pc_enc,
            flags_enc,
        })
    }

    pub fn read_by_enc(&self, enc: u8) -> Option<u8> {
        let reg = self.reg_map.get(&enc)?;
        Some(reg.val)
    }

    pub fn read_by_addr(&self, addr: u16) -> Option<u8> {
        for reg in self.reg_map.values() {
            if reg.addr == addr {
                return Some(reg.val);
            }
        }
        None
    }

    pub fn read_by_repr(&self, repr: RegisterRepr) -> Option<u8> {
        for reg in self.reg_map.values() {
            if reg.repr == repr {
                return Some(reg.val);
            }
        }
        None
    }

    pub fn write_by_enc(&mut self, enc: u8, val: u8) -> Option<u8> {
        let reg = self.reg_map.get_mut(&enc)?;
        reg.val = val;
        Some(reg.val)
    }

    pub fn read_sp(&self) -> u8 {
        self.reg_map[&self.sp_enc].val
    }

    pub fn read_pc(&self) -> u8 {
        self.reg_map[&self.pc_enc].val
    }

    pub fn read_flags(&self) -> u8 {
        self.reg_map[&self.flags_enc].val
    }

    pub fn write_by_repr(&mut self, repr: RegisterRepr, val: u8) -> Option<u8> {
        for reg in self.reg_map.values_mut() {
            if reg.repr == repr {
                reg.val = val;
                return Some(reg.val);
            }
        }
        None
    }

    pub fn write_sp(&mut self, val: u8) -> Option<u8> {
        let reg = self.reg_map.get_mut(&self.sp_enc)?;
        reg.val = val;
        Some(reg.val)
    }

    pub fn inc_sp(&mut self) -> Option<u8> {
        let reg = self.reg_map.get_mut(&self.sp_enc)?;
        reg.val += 1;
        Some(reg.val)
    }

    pub fn dec_sp(&mut self) -> Option<u8> {
        let reg = self.reg_map.get_mut(&self.sp_enc)?;
        reg.val -= 1;
        Some(reg.val)
    }

    pub fn write_pc(&mut self, val: u8) -> Option<u8> {
        let reg = self.reg_map.get_mut(&self.pc_enc)?;
        reg.val = val;
        Some(reg.val)
    }

    pub fn inc_pc(&mut self) -> Option<u8> {
        let reg = self.reg_map.get_mut(&self.pc_enc)?;
        reg.val += 1;
        Some(reg.val)
    }

    pub fn write_flags(&mut self, val: u8) -> Option<u8> {
        let reg = self.reg_map.get_mut(&self.flags_enc)?;
        reg.val = val;
        Some(reg.val)
    }

    pub fn set_flag_bit(&mut self, flag: u8) -> Option<u8> {
        let reg = self.reg_map.get_mut(&self.flags_enc)?;
        reg.val |= flag;
        Some(reg.val)
    }
}
