use crate::decoder::Decoder;
use crate::errors::MachineError;
use crate::executor::Executor;
use crate::isa::InstructionSet;
use crate::kernel::Kernel;
use crate::memory::Memory;
use crate::reg_file::RegisterFile;
use crate::yan::{Instruction, RegisterPurpose, RegisterRepr};
use palette::{IntoColor, LinSrgb, Mix};
use std::collections::HashMap;
use std::fmt::Write;

pub struct Machine {
    pub ccs: Vec<String>,
    pub isa: InstructionSet,
    pub kernel: Kernel,
    pub memory: Memory,
    pub reg_file: RegisterFile,
}

const RESET_COLOR: &str = "\x1b[0m";

pub fn generate_ccs() -> Vec<String> {
    let stops = [
        LinSrgb::new(0.2, 0.4, 1.0),
        LinSrgb::new(0.4, 0.9, 0.6),
        LinSrgb::new(1.0, 0.9, 0.3),
        LinSrgb::new(1.0, 0.4, 0.4),
    ];
    let segments = stops.len() - 1;
    let mut ccs = Vec::with_capacity(256);
    for i in 0..=255 {
        let t = f64::from(i) / 255.0;
        let seg = ((t * segments as f64).floor().min(segments as f64 - 1.0));
        let seg_t = ((t * segments as f64) - seg);
        let start = stops[seg as usize];
        let end = stops[seg as usize + 1];
        let color = start.mix(end, seg_t).into_format::<u8>();
        let ansi = format!("\x1b[38;2;{};{};{}m", color.red, color.green, color.blue);
        ccs.push(ansi);
    }
    ccs
}

impl Machine {
    pub fn new(
        isa: InstructionSet,
        kernel: Kernel,
        memory: Memory,
        reg_file: RegisterFile,
    ) -> Self {
        let ccs = generate_ccs();
        Machine {
            ccs,
            isa,
            kernel,
            memory,
            reg_file,
        }
    }

    pub fn run(&mut self) -> Result<(), MachineError> {
        loop {
            // println!("{}", self.dump_registers());
            // println!("{}", self.dump_memory());
            let instr = self.get_current_instruction()?;
            println!(
                "{}",
                instr.as_rich_str(&self.isa.syscall_map, &self.isa.flag_map)
            );
            self.execute_quiet()?;
        }
    }

    pub fn execute(&mut self) -> Result<(), MachineError> {
        let Ok(instr) = self.get_current_instruction() else {
            return Err(MachineError::InvalidInstruction());
        };
        println!(
            "{}",
            instr.as_rich_str(&self.isa.syscall_map, &self.isa.flag_map)
        );
        match Executor::execute(
            instr,
            &self.isa,
            &mut self.kernel,
            &mut self.memory,
            &mut self.reg_file,
        ) {
            Ok(()) => (),
            Err(e) => return Err(e.into()),
        }
        self.reg_file.inc_pc();
        Ok(())
    }

    pub fn execute_quiet(&mut self) -> Result<(), MachineError> {
        let instr = self.get_current_instruction()?;
        self.reg_file.inc_pc();
        match Executor::execute(
            instr,
            &self.isa,
            &mut self.kernel,
            &mut self.memory,
            &mut self.reg_file,
        ) {
            Ok(()) => (),
            Err(e) => return Err(e.into()),
        }
        Ok(())
    }

    pub fn get_current_instruction(&self) -> Result<Instruction, MachineError> {
        let pc = self.isa.instr_len * self.reg_file.read_pc();
        let bytes = self.memory.read_instruction(pc);
        let Some(instr) = Decoder::decode(&self.isa, bytes) else {
            return Err(MachineError::InvalidInstruction());
        };
        Ok(instr)
    }

    pub fn dump_memory(&self) -> String {
        let pc = (self.isa.instr_len as usize) * self.reg_file.read_pc().wrapping_sub(1) as usize;
        let sp = 0x300 + self.reg_file.read_sp() as usize;
        let mut output = String::new();
        let mut previous_is_zero = false;
        for i in (0..self.memory.memory.len()).step_by(0x20) {
            if i + 0x20 > self.memory.memory.len() {
                continue;
            }
            let chunk = &self.memory.memory[i..i + 0x20];
            let address = format!("0x{:03x}", 0x400 + i);
            let mut values = String::new();
            let mut reprs = String::new();
            let mut has_pc = false;
            let mut has_sp = false;
            let mut is_zero = true;
            for (j, b) in chunk.iter().enumerate().take(0x20) {
                let idx = i + j;
                let color = &self.ccs[*b as usize];
                if *b != 0 {
                    is_zero = false;
                }
                if idx == pc {
                    has_pc = true;
                    write!(values, "[{color}{b:02x}{RESET_COLOR}]").unwrap();
                } else if idx == sp {
                    has_sp = true;
                    write!(values, "[{color}{b:02x}{RESET_COLOR}]").unwrap();
                } else {
                    write!(values, " {color}{b:02x}{RESET_COLOR} ").unwrap();
                }
                let ascii = if b.is_ascii_graphic() || *b == b' ' {
                    *b as char
                } else {
                    '.'
                };
                write!(reprs, "{color}{ascii}{RESET_COLOR}").unwrap();
            }
            let mut annotation = String::new();
            if has_pc {
                annotation += "| pc ";
            }
            if has_sp {
                annotation += "| sp ";
            }
            if !is_zero {
                writeln!(output, "{address}  {values}  {reprs}  {annotation}").unwrap();
                previous_is_zero = false;
            } else if !previous_is_zero {
                writeln!(output, "...").unwrap();
                previous_is_zero = true;
            }
        }
        output
    }

    pub fn dump_registers(&self) -> String {
        let reg_order = vec![
            RegisterRepr::A,
            RegisterRepr::B,
            RegisterRepr::C,
            RegisterRepr::D,
            RegisterRepr::S,
            RegisterRepr::P,
            RegisterRepr::F,
        ];
        let mut output = String::new();
        for repr in reg_order {
            let reg = self.reg_file.reg_map.values().find(|r| r.repr == repr);
            if let Some(reg) = reg {
                match reg.purpose {
                    RegisterPurpose::Flags => {
                        let mut set_flags = vec![];
                        for (bitmask, repr) in &self.isa.flag_map {
                            if reg.val & bitmask != 0x00 {
                                set_flags.push(format!("{repr:?}").to_uppercase());
                            }
                        }
                        write!(
                            output,
                            "{}: 0x{:02x} ({})",
                            format!("{:?}", reg.repr).to_uppercase(),
                            reg.val,
                            set_flags.join("")
                        )
                        .unwrap();
                    }
                    _ => {
                        write!(
                            output,
                            "{}: 0x{:02x} ",
                            format!("{:?}", reg.repr).to_uppercase(),
                            reg.val
                        )
                        .unwrap();
                    }
                }
            }
        }
        output
    }
}
