#![allow(unused)]
mod assembler;
mod debugger;
mod decoder;
mod disassembler;
mod errors;
mod executor;
mod isa;
mod kernel;
mod machine;
mod memory;
mod reg_file;
mod yan;
use crate::assembler::Assembler;
use crate::debugger::Debugger;
use crate::executor::Executor;
use crate::isa::InstructionSet;
use crate::kernel::Kernel;
use crate::machine::Machine;
use crate::memory::Memory;
use crate::reg_file::RegisterFile;
use crate::{decoder::Decoder, disassembler::Disassembler};
use clap::{Parser, Subcommand};
use std::fs;

#[derive(Parser)]
#[command(author, version, about, long_about)]
struct Cli {
    #[arg(short, long)]
    filename: String,

    #[arg(short, long)]
    config: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Disassemble {},
    Assemble {
        #[arg(short, long)]
        output: String,
    },
    Debug {},
    Emulate {},
}

fn main() {
    let cli = Cli::parse();
    let isa = InstructionSet::parse(&cli.config).unwrap_or_else(|e| panic!("{}", e));
    let reg_file = RegisterFile::new(&isa.reg_map).unwrap_or_else(|e| panic!("{}", e));
    match &cli.command {
        Some(Commands::Disassemble {}) => {
            let code = match fs::read(&cli.filename) {
                Ok(str) => str,
                Err(err) => panic!("{err:?}"),
            };
            assert!(code.len() < 0x100);
            let mut memory = Memory::new(&code);
            let disassembler = Disassembler::new();
            let disassembled_lines = Disassembler::disassemble(&isa, &memory);
            for line in disassembled_lines {
                println!("{line}");
            }
        }
        Some(Commands::Assemble { output }) => {
            let source = match fs::read_to_string(&cli.filename) {
                Ok(src) => src,
                Err(err) => panic!("{err:?}"),
            };
            let code = Assembler::assemble(&isa, &source).unwrap_or_else(|e| panic!("{e}"));
            if let Err(err) = fs::write(output, &code) {
                panic!("{err:?}");
            }
        }
        Some(Commands::Debug {}) => {
            let code = match fs::read(&cli.filename) {
                Ok(bytes) => bytes,
                Err(err) => panic!("{err:?}"),
            };
            assert!(code.len() < 0x100);
            let memory = Memory::new(&code);
            let kernel = Kernel::new();
            let machine = Machine::new(isa, kernel, memory, reg_file);
            let mut debugger = Debugger::new(machine);
            debugger.machine.kernel.console_mode = crate::kernel::ConsoleMode::Buffered;
            if let Err(e) = debugger.run() {
                eprintln!("{e}");
            }
        }
        Some(Commands::Emulate {}) => {
            let code = match fs::read(&cli.filename) {
                Ok(str) => str,
                Err(err) => panic!("{err:?}"),
            };
            assert!(code.len() < 0x100);
            let mut memory = Memory::new(&code);
            let kernel = Kernel::new();
            let mut machine = Machine::new(isa, kernel, memory, reg_file);
            if let Err(e) = machine.run() {
                eprintln!("{e}");
            }
        }
        None => {
            eprintln!("No command provided. Use --help for more information.");
        }
    }
}
