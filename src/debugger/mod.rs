use crate::debugger::command::{
    InputMode, ParsedCommand, parse_flag_mask, parse_instruction, parse_register,
    parse_syscall_mask, parse_u8,
};
use crate::debugger::style::{
    breakpoint_style, dim_style, flag_style, generate_tui_ccs, immediate_style, invalid_style,
    opcode_style, pc_style, punctuation_style, register_style, sp_style, syscall_style,
};
use crate::debugger::ui::centered_rect;
use crate::debugger::watch::{Watchpoint, active_flags, sorted_flags, sorted_syscalls};
use crate::decoder::Decoder;
use crate::errors::MachineError;
use crate::executor::Executor;
use crate::machine::Machine;
use crate::yan::{FlagRepr, Instruction, InstructionRepr, Operand, RegisterRepr, SyscallRepr};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use palette::{IntoColor, LinSrgb, Mix};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use std::collections::BTreeSet;
use std::error::Error;
use std::io;
use std::time::Duration;
pub mod command;
pub mod style;
pub mod ui;
pub mod watch;

type TuiTerminal = Terminal<CrosstermBackend<io::Stdout>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Breakpoint {
    Pc(u8),
}

pub struct Debugger {
    pub machine: Machine,
    breakpoints: BTreeSet<u8>,
    watchpoints: Vec<Watchpoint>,
    running: bool,
    skip_break_once: bool,
    status: String,
    ccs: Vec<Color>,
    input_mode: InputMode,
    command: String,
    show_help: bool,
}

impl Debugger {
    pub fn new(machine: Machine) -> Self {
        Self {
            machine,
            breakpoints: BTreeSet::new(),
            watchpoints: Vec::new(),
            running: false,
            skip_break_once: false,
            status: "ready".to_string(),
            ccs: generate_tui_ccs(),
            input_mode: InputMode::Normal,
            command: String::new(),
            show_help: false,
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        let result = self.run_loop(&mut terminal);
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;
        result
    }

    fn run_loop(&mut self, terminal: &mut TuiTerminal) -> Result<(), Box<dyn Error>> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;
            if self.running {
                self.tick();
            }
            if event::poll(Duration::from_millis(30))?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
                && !self.handle_key(key)
            {
                break;
            }
        }
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match self.input_mode {
            InputMode::Normal => self.handle_normal_key(key.code),
            InputMode::Command => self.handle_command_key(key.code),
        }
    }

    fn handle_normal_key(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Char('q') => return false,
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
            }
            KeyCode::Char(':') => {
                self.input_mode = InputMode::Command;
                self.command.clear();
                self.show_help = false;
            }
            KeyCode::Char('s') => {
                self.running = false;
                self.step_once();
            }
            KeyCode::Char('c') => {
                self.running = true;
                self.skip_break_once = true;
                self.status = "running".to_string();
            }
            KeyCode::Char('b') => {
                let pc = self.machine.reg_file.read_pc();
                self.toggle_breakpoint(pc);
            }
            KeyCode::Esc => {
                self.running = false;
                self.show_help = false;
                self.status = "paused".to_string();
            }
            _ => {}
        }
        true
    }

    fn handle_command_key(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.command.clear();
                self.status = "command cancelled".to_string();
            }
            KeyCode::Enter => {
                let input = self.command.trim().to_string();
                self.input_mode = InputMode::Normal;
                self.command.clear();
                self.execute_command(&input);
            }
            KeyCode::Backspace => {
                self.command.pop();
            }
            KeyCode::Char(ch) => {
                self.command.push(ch);
            }
            _ => {}
        }
        true
    }

    fn tick(&mut self) {
        let pc = self.machine.reg_file.read_pc();
        if self.breakpoints.contains(&pc) && !self.skip_break_once {
            self.running = false;
            self.status = format!("hit breakpoint at 0x{pc:02x}");
            return;
        }
        let instr = match self.current_instruction() {
            Ok(instr) => instr,
            Err(err) => {
                self.running = false;
                self.status = err.to_string();
                return;
            }
        };
        if let Some(reason) = self.hit_watchpoint(instr) {
            self.running = false;
            self.status = reason;
            return;
        }
        self.skip_break_once = false;
        self.step_once();
    }

    fn step_once(&mut self) {
        let pc = self.machine.reg_file.read_pc();
        match self.execute_current_instruction() {
            Ok(()) => {
                self.status = format!(
                    "stepped 0x{pc:02x} -> 0x{:02x}",
                    self.machine.reg_file.read_pc()
                );
            }
            Err(err) => {
                self.running = false;
                self.status = err.to_string();
            }
        }
    }

    fn execute_current_instruction(&mut self) -> Result<(), MachineError> {
        let instr = self.current_instruction()?;
        self.machine.reg_file.inc_pc();
        Executor::execute(
            instr,
            &self.machine.isa,
            &mut self.machine.kernel,
            &mut self.machine.memory,
            &mut self.machine.reg_file,
        )?;
        Ok(())
    }

    fn current_instruction(&self) -> Result<Instruction, MachineError> {
        let pc = self.machine.reg_file.read_pc().wrapping_mul(3);
        let bytes = self.machine.memory.read_instruction(pc);
        Decoder::decode(&self.machine.isa, bytes).ok_or(MachineError::InvalidInstruction())
    }

    fn execute_command(&mut self, input: &str) {
        match self.parse_command(input) {
            Ok(cmd) => self.apply_command(cmd),
            Err(err) => self.status = err,
        }
    }
}
