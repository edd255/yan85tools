use crate::isa::InstructionSet;
use crate::yan::{FlagRepr, InstructionRepr, RegisterRepr, SyscallRepr};

pub struct Assembler {}

impl Assembler {
    pub fn new() -> Self {
        Self {}
    }

    pub fn assemble(isa: &InstructionSet, source: &str) -> Result<Vec<u8>, String> {
        let mut code = Vec::new();
        for (line_no, raw_line) in source.lines().enumerate() {
            let line = Self::strip_comments(raw_line);
            for stmt in line.split(';') {
                let stmt = stmt.trim();
                if stmt.is_empty() {
                    continue;
                }
                let bytes = Self::assemble_statement(isa, stmt)
                    .map_err(|err| format!("line {}: {}", line_no + 1, err))?;
                if code.len() + bytes.len() > 0x100 {
                    return Err(format!(
                        "line {}: assembled program exceeds 0x100 bytes",
                        line_no + 1
                    ));
                }
                code.extend_from_slice(&bytes);
            }
        }
        Ok(code)
    }

    fn strip_comments(line: &str) -> &str {
        let hash = line.find('#');
        let slash = line.find("//");
        match (hash, slash) {
            (Some(a), Some(b)) => &line[..a.min(b)],
            (Some(a), None) => &line[..a],
            (None, Some(b)) => &line[..b],
            (None, None) => line,
        }
    }

    fn assemble_statement(isa: &InstructionSet, stmt: &str) -> Result<[u8; 3], String> {
        let normalized = stmt.replace('=', " = ");
        let tokens: Vec<&str> = normalized.split_whitespace().collect();
        if tokens.is_empty() {
            return Err("empty statement".to_string());
        }
        let mnemonic = tokens[0].to_ascii_uppercase();
        match mnemonic.as_str() {
            "IMM" => Self::assemble_imm(isa, &tokens),
            "ADD" => Self::assemble_reg_reg(isa, InstructionRepr::ADD, &tokens),
            "CMP" => Self::assemble_reg_reg(isa, InstructionRepr::CMP, &tokens),
            "STK" => Self::assemble_reg_reg(isa, InstructionRepr::STK, &tokens),
            "STM" => Self::assemble_stm(isa, &tokens),
            "LDM" => Self::assemble_ldm(isa, &tokens),
            "JMP" => Self::assemble_jmp(isa, &tokens),
            "SYS" => Self::assemble_sys(isa, &tokens),
            "PSH" => Self::assemble_psh(isa, &tokens),
            "POP" => Self::assemble_pop(isa, &tokens),
            other => Err(format!("unknown instruction '{other}'")),
        }
    }

    fn assemble_imm(isa: &InstructionSet, tokens: &[&str]) -> Result<[u8; 3], String> {
        let (reg_tok, val_tok) = match tokens {
            ["IMM", reg, "=", val] => (*reg, *val),
            ["IMM", reg, val] => (*reg, *val),
            _ => {
                return Err("expected: IMM <reg> = <value> or IMM <reg> <value>".to_string());
            }
        };
        let reg = Self::parse_register(reg_tok)?;
        let val = Self::parse_u8(val_tok)?;
        Self::encode(isa, InstructionRepr::IMM, Self::reg_enc(isa, reg)?, val)
    }

    fn assemble_reg_reg(
        isa: &InstructionSet,
        op: InstructionRepr,
        tokens: &[&str],
    ) -> Result<[u8; 3], String> {
        let (_, reg1_tok, reg2_tok) = match tokens {
            [mnemonic, reg1, reg2] => (*mnemonic, *reg1, *reg2),
            _ => {
                return Err(format!("expected: {op} <reg> <reg>"));
            }
        };
        let reg1 = Self::parse_register(reg1_tok)?;
        let reg2 = Self::parse_register(reg2_tok)?;
        Self::encode(
            isa,
            op,
            Self::reg_enc(isa, reg1)?,
            Self::reg_enc(isa, reg2)?,
        )
    }

    fn assemble_stm(isa: &InstructionSet, tokens: &[&str]) -> Result<[u8; 3], String> {
        let (dst_tok, src_tok) = match tokens {
            ["STM", dst, "=", src] => (*dst, *src),
            ["STM", dst, src] => (*dst, *src),
            _ => {
                return Err("expected: STM *<reg> = <reg> or STM *<reg> <reg>".to_string());
            }
        };
        let dst_reg = Self::parse_deref_register(dst_tok)?;
        let src_reg = Self::parse_register(src_tok)?;
        Self::encode(
            isa,
            InstructionRepr::STM,
            Self::reg_enc(isa, dst_reg)?,
            Self::reg_enc(isa, src_reg)?,
        )
    }

    fn assemble_ldm(isa: &InstructionSet, tokens: &[&str]) -> Result<[u8; 3], String> {
        let (dst_tok, src_tok) = match tokens {
            ["LDM", dst, "=", src] => (*dst, *src),
            ["LDM", dst, src] => (*dst, *src),
            _ => {
                return Err("expected: LDM <reg> = *<reg> or LDM <reg> *<reg>".to_string());
            }
        };
        let dst_reg = Self::parse_register(dst_tok)?;
        let src_reg = Self::parse_deref_register(src_tok)?;
        Self::encode(
            isa,
            InstructionRepr::LDM,
            Self::reg_enc(isa, dst_reg)?,
            Self::reg_enc(isa, src_reg)?,
        )
    }

    fn assemble_jmp(isa: &InstructionSet, tokens: &[&str]) -> Result<[u8; 3], String> {
        if tokens.len() < 2 {
            return Err("expected: JMP <reg> or JMP <flags> <reg>".to_string());
        }
        let reg = Self::parse_register(tokens[tokens.len() - 1])?;
        let mut flag_mask = 0u8;
        for tok in &tokens[1..tokens.len() - 1] {
            flag_mask |= Self::parse_flag_token(isa, tok)?;
        }
        Self::encode(
            isa,
            InstructionRepr::JMP,
            flag_mask,
            Self::reg_enc(isa, reg)?,
        )
    }

    fn assemble_sys(isa: &InstructionSet, tokens: &[&str]) -> Result<[u8; 3], String> {
        if tokens.len() < 3 {
            return Err("expected: SYS <syscall>... <reg>".to_string());
        }
        let reg = Self::parse_register(tokens[tokens.len() - 1])?;
        let mut syscall_mask = 0u8;
        for tok in &tokens[1..tokens.len() - 1] {
            syscall_mask |= Self::parse_syscall_token(isa, tok)?;
        }
        Self::encode(
            isa,
            InstructionRepr::SYS,
            syscall_mask,
            Self::reg_enc(isa, reg)?,
        )
    }

    fn assemble_psh(isa: &InstructionSet, tokens: &[&str]) -> Result<[u8; 3], String> {
        let reg_tok = match tokens {
            ["PSH", reg] => *reg,
            _ => return Err("expected: PSH <reg>".to_string()),
        };
        let reg = Self::parse_register(reg_tok)?;
        Self::encode(isa, InstructionRepr::STK, 0x00, Self::reg_enc(isa, reg)?)
    }

    fn assemble_pop(isa: &InstructionSet, tokens: &[&str]) -> Result<[u8; 3], String> {
        let reg_tok = match tokens {
            ["POP", reg] => *reg,
            _ => return Err("expected: POP <reg>".to_string()),
        };
        let reg = Self::parse_register(reg_tok)?;
        Self::encode(isa, InstructionRepr::STK, Self::reg_enc(isa, reg)?, 0x00)
    }

    fn parse_register(token: &str) -> Result<RegisterRepr, String> {
        match token.to_ascii_lowercase().as_str() {
            "a" => Ok(RegisterRepr::A),
            "b" => Ok(RegisterRepr::B),
            "c" => Ok(RegisterRepr::C),
            "d" => Ok(RegisterRepr::D),
            "s" => Ok(RegisterRepr::S),
            "i" | "p" => Ok(RegisterRepr::P),
            "f" => Ok(RegisterRepr::F),
            _ => Err(format!("invalid register '{token}'")),
        }
    }

    fn parse_deref_register(token: &str) -> Result<RegisterRepr, String> {
        let reg = token
            .strip_prefix('*')
            .ok_or_else(|| format!("expected dereferenced register, got '{token}'"))?;
        Self::parse_register(reg)
    }

    fn parse_u8(token: &str) -> Result<u8, String> {
        if let Some(hex) = token
            .strip_prefix("0x")
            .or_else(|| token.strip_prefix("0X"))
        {
            u8::from_str_radix(hex, 16).map_err(|_| format!("invalid byte literal '{token}'"))
        } else {
            token
                .parse::<u8>()
                .map_err(|_| format!("invalid byte literal '{token}'"))
        }
    }

    fn parse_flag_token(isa: &InstructionSet, token: &str) -> Result<u8, String> {
        let mut out = 0u8;
        for part in token.split('+') {
            for ch in part.chars() {
                match ch.to_ascii_uppercase() {
                    '*' | 'S' => {}
                    'G' => out |= Self::flag_enc(isa, FlagRepr::G)?,
                    'E' => out |= Self::flag_enc(isa, FlagRepr::E)?,
                    'N' => out |= Self::flag_enc(isa, FlagRepr::N)?,
                    'L' => out |= Self::flag_enc(isa, FlagRepr::L)?,
                    'Z' => out |= Self::flag_enc(isa, FlagRepr::Z)?,
                    other => return Err(format!("invalid flag '{other}' in '{token}'")),
                }
            }
        }
        Ok(out)
    }

    fn parse_syscall_token(isa: &InstructionSet, token: &str) -> Result<u8, String> {
        let mut out = 0u8;
        for part in token.split('+') {
            let syscall = match part.to_ascii_uppercase().as_str() {
                "OPEN" => SyscallRepr::Open,
                "READ_CODE" => SyscallRepr::ReadCode,
                "READ_MEM" => SyscallRepr::ReadMemory,
                "WRITE" => SyscallRepr::Write,
                "SLEEP" => SyscallRepr::Sleep,
                "EXIT" => SyscallRepr::Exit,
                _ => return Err(format!("invalid syscall '{part}'")),
            };
            out |= Self::syscall_enc(isa, syscall)?;
        }
        Ok(out)
    }

    fn encode(
        isa: &InstructionSet,
        op: InstructionRepr,
        arg1: u8,
        arg2: u8,
    ) -> Result<[u8; 3], String> {
        let mut bytes = [0u8; 3];
        bytes[isa.arg1 as usize] = arg1;
        bytes[isa.op as usize] = Self::instr_enc(isa, op)?;
        bytes[isa.arg2 as usize] = arg2;
        Ok(bytes)
    }

    fn instr_enc(isa: &InstructionSet, target: InstructionRepr) -> Result<u8, String> {
        isa.instr_map
            .iter()
            .find_map(|(enc, repr)| (*repr == target).then_some(*enc))
            .ok_or_else(|| format!("missing encoding for instruction '{target}'"))
    }

    fn reg_enc(isa: &InstructionSet, target: RegisterRepr) -> Result<u8, String> {
        isa.reg_map
            .iter()
            .find_map(|(enc, reg)| (reg.repr == target).then_some(*enc))
            .ok_or_else(|| format!("missing encoding for register '{target}'"))
    }

    fn syscall_enc(isa: &InstructionSet, target: SyscallRepr) -> Result<u8, String> {
        isa.syscall_map
            .iter()
            .find_map(|(enc, repr)| (*repr == target).then_some(*enc))
            .ok_or_else(|| format!("missing encoding for syscall '{target}'"))
    }

    fn flag_enc(isa: &InstructionSet, target: FlagRepr) -> Result<u8, String> {
        isa.flag_map
            .iter()
            .find_map(|(enc, repr)| (*repr == target).then_some(*enc))
            .ok_or_else(|| format!("missing encoding for flag '{target}'"))
    }
}
