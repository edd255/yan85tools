use super::Debugger;
use super::command::InputMode;
use super::style::{
    breakpoint_style, dim_style, flag_style, immediate_style, invalid_style, opcode_style,
    pc_style, punctuation_style, register_style, sp_style, syscall_style,
};
use super::watch::{active_flags, sorted_flags, sorted_syscalls};
use crate::decoder::Decoder;
use crate::yan::{FlagRepr, Instruction, InstructionRepr, Operand, RegisterRepr, SyscallRepr};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

impl Debugger {
    pub(super) fn draw(&self, frame: &mut Frame<'_>) {
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(9),
                Constraint::Min(10),
                Constraint::Length(8),
                Constraint::Length(3),
            ])
            .split(frame.area());
        let top = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(28), Constraint::Min(20)])
            .split(outer[0]);
        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(48), Constraint::Percentage(52)])
            .split(outer[1]);
        let console = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(outer[2]);
        let registers = Paragraph::new(self.registers_text())
            .block(Block::default().title("Registers").borders(Borders::ALL));
        let state = Paragraph::new(self.state_text())
            .block(Block::default().title("State").borders(Borders::ALL))
            .wrap(Wrap { trim: false });
        let code_height = body[0].height.saturating_sub(2) as usize;
        let code = Paragraph::new(self.code_text(code_height))
            .block(Block::default().title("Code").borders(Borders::ALL))
            .wrap(Wrap { trim: false });
        let memory = Paragraph::new(self.memory_text())
            .block(Block::default().title("Memory").borders(Borders::ALL))
            .wrap(Wrap { trim: false });
        let stdout = Paragraph::new(self.stdout_text())
            .block(Block::default().title("stdout").borders(Borders::ALL))
            .wrap(Wrap { trim: false });
        let stderr = Paragraph::new(self.stderr_text())
            .block(Block::default().title("stderr").borders(Borders::ALL))
            .wrap(Wrap { trim: false });
        let command = Paragraph::new(self.command_text())
            .block(Block::default().title("Command").borders(Borders::ALL))
            .wrap(Wrap { trim: false });
        frame.render_widget(registers, top[0]);
        frame.render_widget(state, top[1]);
        frame.render_widget(code, body[0]);
        frame.render_widget(memory, body[1]);
        frame.render_widget(stdout, console[0]);
        frame.render_widget(stderr, console[1]);
        frame.render_widget(command, outer[3]);
        if self.show_help {
            let area = centered_rect(70, 65, frame.area());
            let help = Paragraph::new(self.help_text())
                .block(Block::default().title("Help").borders(Borders::ALL))
                .wrap(Wrap { trim: false });
            frame.render_widget(Clear, area);
            frame.render_widget(help, area);
        }
    }

    pub(super) fn stdout_text(&self) -> Text<'static> {
        let text = String::from_utf8_lossy(&self.machine.kernel.guest_stdout);
        Text::from(text.to_string())
    }

    pub(super) fn stderr_text(&self) -> Text<'static> {
        let text = String::from_utf8_lossy(&self.machine.kernel.guest_stderr);
        Text::from(text.to_string())
    }

    pub(super) fn console_text(&self) -> Text<'static> {
        let stdout = String::from_utf8_lossy(&self.machine.kernel.guest_stdout);
        let stderr = String::from_utf8_lossy(&self.machine.kernel.guest_stderr);
        let mut lines = Vec::new();
        lines.push(Line::from(vec![Span::styled("stdout", dim_style())]));
        for line in stdout.lines() {
            lines.push(Line::from(line.to_string()));
        }
        if !stderr.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled("stderr", dim_style())]));
            for line in stderr.lines() {
                lines.push(Line::from(Span::styled(
                    line.to_string(),
                    Style::default().fg(Color::Red),
                )));
            }
        }
        Text::from(lines)
    }

    pub(super) fn state_text(&self) -> Text<'static> {
        let pc = self.machine.reg_file.read_pc();
        let sp = self.machine.reg_file.read_sp();
        let flags = self.machine.reg_file.read_flags();
        let flags = active_flags(
            &self.machine.isa.flag_map,
            self.machine.reg_file.read_flags(),
        );
        let mode = if self.running { "running" } else { "paused" };
        let mut instr_line = vec![Span::raw("next:  ")];
        instr_line.extend(self.current_instruction_line().spans);
        let mut lines = vec![
            Line::from(format!("mode:  {mode}")),
            Line::from(format!("sp:    0x{sp:02x}")),
            Line::from(format!("flags: {flags}")),
            Line::from(instr_line),
            Line::from(format!("bp:    {}", self.breakpoints_summary())),
            Line::from(format!("wp:    {}", self.watchpoints_summary())),
        ];
        if lines.len() > 9 {
            lines.truncate(9);
        }
        Text::from(lines)
    }

    pub(super) fn command_text(&self) -> Text<'static> {
        match self.input_mode {
            InputMode::Normal => Text::from(vec![
                Line::from("s step  c continue  b toggle-breakpoint  : command  ? help  q quit"),
                Line::from(format!("status: {}", self.status)),
            ]),
            InputMode::Command => Text::from(vec![
                Line::from(vec![
                    Span::styled(":", Style::default().fg(Color::Yellow)),
                    Span::raw(self.command.clone()),
                ]),
                Line::from(format!("status: {}", self.status)),
            ]),
        }
    }

    pub(super) fn help_text(&self) -> Text<'static> {
        Text::from(vec![
            Line::from("Keys"),
            Line::from("  s              step one instruction"),
            Line::from("  c              continue until breakpoint/watchpoint/error"),
            Line::from("  b              toggle breakpoint at current pc"),
            Line::from("  :              enter command mode"),
            Line::from("  ?              toggle this help"),
            Line::from("  esc            close help / exit command mode / pause"),
            Line::from("  q              quit"),
            Line::from(""),
            Line::from("Commands"),
            Line::from("  break 0x12"),
            Line::from("  delete 0x12"),
            Line::from("  watch op SYS"),
            Line::from("  watch sys OPEN"),
            Line::from("  watch sys OPEN|READ_MEM"),
            Line::from("  watch reg a = 0x2f"),
            Line::from("  watch flags any GE"),
            Line::from("  watch flags = 0x11"),
            Line::from("  unwatch 0"),
            Line::from("  list"),
            Line::from("  clear"),
            Line::from("  help"),
        ])
    }

    pub(super) fn code_text(&self, visible_rows: usize) -> Text<'static> {
        let mut lines = Vec::new();
        let pc = self.machine.reg_file.read_pc();
        if self.machine.memory.code_len == 0 {
            return Text::from(vec![Line::from("no code loaded")]);
        }
        let last_addr = ((self.machine.memory.code_len - 1) / 3) as u8;
        let window = visible_rows.max(1).min((last_addr as usize) + 1);
        let half = window / 2;
        let mut start = (pc as usize).saturating_sub(half);
        let mut end = (start + window - 1).min(last_addr as usize);
        if end - start + 1 < window {
            start = end.saturating_sub(window - 1);
        }
        for addr in start..=end {
            let addr = addr as u8;
            let raw = self.machine.memory.read_instruction(addr.wrapping_mul(3));
            let active = addr == pc;
            let mut spans = Vec::new();
            if self.breakpoints.contains(&addr) {
                spans.push(Span::styled("● ", breakpoint_style()));
            } else {
                spans.push(Span::raw("  "));
            }
            let addr_style = if active { pc_style() } else { dim_style() };
            spans.push(Span::styled(format!("0x{addr:02x}"), addr_style));
            spans.push(Span::raw("  "));
            match Decoder::decode(&self.machine.isa, raw) {
                Some(instr) => spans.extend(self.instruction_spans(instr, active)),
                None => spans.push(Span::styled("INVALID", invalid_style(active))),
            }
            lines.push(Line::from(spans));
        }
        Text::from(lines)
    }

    pub(super) fn memory_text(&self) -> Text<'static> {
        let pc = (self.machine.reg_file.read_pc() as usize) * 3;
        let sp = 0x300 + self.machine.reg_file.read_sp() as usize;
        let mut lines = Vec::new();
        let mut previous_zero_row = false;
        for base in (0..self.machine.memory.memory.len()).step_by(0x10) {
            let end = (base + 0x10).min(self.machine.memory.memory.len());
            let chunk = &self.machine.memory.memory[base..end];
            let is_zero = chunk.iter().all(|&b| b == 0);
            if is_zero {
                if !previous_zero_row {
                    lines.push(Line::from(Span::styled("...", dim_style())));
                    previous_zero_row = true;
                }
                continue;
            }
            previous_zero_row = false;
            let mut spans = Vec::new();
            spans.push(Span::styled(
                format!("0x{:03x}: ", 0x400 + base),
                dim_style(),
            ));
            for (off, byte) in chunk.iter().enumerate() {
                let idx = base + off;
                let style = self.memory_byte_style(*byte, idx);
                if idx == pc {
                    spans.push(Span::styled("[".to_string(), pc_style()));
                    spans.push(Span::styled(format!("{byte:02x}"), style));
                    spans.push(Span::styled("]".to_string(), pc_style()));
                } else if idx == sp {
                    spans.push(Span::styled("[".to_string(), sp_style()));
                    spans.push(Span::styled(format!("{byte:02x}"), style));
                    spans.push(Span::styled("]".to_string(), sp_style()));
                } else {
                    spans.push(Span::styled(format!(" {byte:02x} "), style));
                }
            }
            spans.push(Span::raw(" "));
            for (off, byte) in chunk.iter().enumerate() {
                let idx = base + off;
                let style = self.memory_byte_style(*byte, idx);
                let ch = if byte.is_ascii_graphic() || *byte == b' ' {
                    *byte as char
                } else {
                    '.'
                };
                spans.push(Span::styled(ch.to_string(), style));
            }
            lines.push(Line::from(spans));
        }
        Text::from(lines)
    }

    pub(super) fn current_instruction_line(&self) -> Line<'static> {
        match self.current_instruction() {
            Ok(instr) => Line::from(self.instruction_spans(instr, true)),
            Err(_) => Line::from(Span::styled("INVALID", invalid_style(true))),
        }
    }

    pub(super) fn instruction_spans(&self, instr: Instruction, active: bool) -> Vec<Span<'static>> {
        let base = if active {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        fn styled_span(text: impl Into<String>, style: Style) -> Span<'static> {
            Span::styled(text.into(), style)
        }

        let opcode = |text: &str| styled_span(text, base.patch(opcode_style()));
        let reg = |r: RegisterRepr| styled_span(r.to_string(), base.patch(register_style()));
        let imm = |v: u8| styled_span(format!("0x{v:02x}"), base.patch(immediate_style()));
        let sys = |s: SyscallRepr| styled_span(s.to_string(), base.patch(syscall_style()));
        let flg = |f: FlagRepr| styled_span(f.to_string(), base.patch(flag_style()));
        let punc = |text: &str| styled_span(text, base.patch(punctuation_style()));
        match instr.op {
            InstructionRepr::IMM => match (instr.arg1, instr.arg2) {
                (Operand::Reg(dst), Operand::Val(val)) => {
                    vec![opcode("IMM"), punc(" "), reg(dst), punc(" = "), imm(val)]
                }
                _ => vec![styled_span("INVALID", invalid_style(active))],
            },
            InstructionRepr::ADD | InstructionRepr::CMP => match (instr.arg1, instr.arg2) {
                (Operand::Reg(a), Operand::Reg(b)) => vec![
                    styled_span(instr.op.to_string(), base.patch(opcode_style())),
                    punc(" "),
                    reg(a),
                    punc(" "),
                    reg(b),
                ],
                _ => vec![styled_span("INVALID", invalid_style(active))],
            },
            InstructionRepr::STM => match (instr.arg1, instr.arg2) {
                (Operand::Reg(addr), Operand::Reg(src)) => {
                    vec![opcode("STM"), punc(" *"), reg(addr), punc(" = "), reg(src)]
                }
                _ => vec![styled_span("INVALID", invalid_style(active))],
            },
            InstructionRepr::LDM => match (instr.arg1, instr.arg2) {
                (Operand::Reg(dst), Operand::Reg(addr)) => {
                    vec![opcode("LDM"), punc(" "), reg(dst), punc(" = *"), reg(addr)]
                }
                _ => vec![styled_span("INVALID", invalid_style(active))],
            },
            InstructionRepr::SYS => match (instr.arg1, instr.arg2) {
                (Operand::Sys(mask), Operand::Reg(reg_dst)) => {
                    let mut spans = vec![opcode("SYS")];
                    let mut any = false;
                    for (_, repr) in sorted_syscalls(&self.machine.isa.syscall_map) {
                        let bit = self.machine.isa.get_syscall(repr).unwrap_or(0);
                        if mask & bit != 0 {
                            spans.push(punc(" "));
                            spans.push(sys(repr));
                            any = true;
                        }
                    }
                    if !any {
                        spans.push(punc(" "));
                        spans.push(imm(mask));
                    }
                    spans.push(punc(" "));
                    spans.push(reg(reg_dst));
                    spans
                }
                _ => vec![styled_span("INVALID", invalid_style(active))],
            },
            InstructionRepr::JMP => match (instr.arg1, instr.arg2) {
                (Operand::Flg(mask), Operand::Reg(reg_dst)) => {
                    let mut spans = vec![opcode("JMP")];
                    let mut any = false;
                    for (_, repr) in sorted_flags(&self.machine.isa.flag_map) {
                        let bit = self.machine.isa.get_flag_enc(repr).unwrap_or(0);
                        if bit != 0 && (mask & bit) != 0 {
                            spans.push(punc(" "));
                            spans.push(flg(repr));
                            any = true;
                        }
                    }
                    if !any && mask != 0 {
                        spans.push(punc(" "));
                        spans.push(imm(mask));
                    }
                    spans.push(punc(" "));
                    spans.push(reg(reg_dst));
                    spans
                }
                _ => vec![styled_span("INVALID", invalid_style(active))],
            },
            InstructionRepr::STK => {
                let mut spans = Vec::new();
                if let Operand::Reg(src) = instr.arg2
                    && src != RegisterRepr::None
                {
                    spans.push(opcode("PSH"));
                    spans.push(punc(" "));
                    spans.push(reg(src));
                }
                if let Operand::Reg(dst) = instr.arg1
                    && dst != RegisterRepr::None
                {
                    if !spans.is_empty() {
                        spans.push(punc("; "));
                    }
                    spans.push(opcode("POP"));
                    spans.push(punc(" "));
                    spans.push(reg(dst));
                }
                if spans.is_empty() {
                    vec![opcode("STK")]
                } else {
                    spans
                }
            }
            _ => vec![styled_span(
                instr.op.to_string(),
                base.patch(opcode_style()),
            )],
        }
    }

    pub(super) fn memory_byte_style(&self, byte: u8, idx: usize) -> Style {
        Style::default().fg(self.ccs[byte as usize])
    }

    pub(super) fn registers_text(&self) -> Text<'static> {
        let mut lines = Vec::new();
        for reg in [
            RegisterRepr::A,
            RegisterRepr::B,
            RegisterRepr::C,
            RegisterRepr::D,
            RegisterRepr::S,
            RegisterRepr::P,
            RegisterRepr::F,
        ] {
            let value = self.machine.reg_file.read_by_repr(reg).unwrap_or(0);
            let mut spans = vec![
                Span::styled(format!("{reg}: "), dim_style()),
                Span::styled(format!("0x{value:02x}"), register_style()),
            ];
            if reg == RegisterRepr::F {
                let flags = active_flags(&self.machine.isa.flag_map, value);
                if !flags.is_empty() {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(format!("({flags})"), flag_style()));
                }
            }
            lines.push(Line::from(spans));
        }
        Text::from(lines)
    }

    pub(super) fn breakpoints_summary(&self) -> String {
        if self.breakpoints.is_empty() {
            "none".to_string()
        } else {
            self.breakpoints
                .iter()
                .map(|addr| format!("0x{addr:02x}"))
                .collect::<Vec<_>>()
                .join(", ")
        }
    }

    pub(super) fn watchpoints_summary(&self) -> String {
        if self.watchpoints.is_empty() {
            "none".to_string()
        } else {
            self.watchpoints
                .iter()
                .enumerate()
                .map(|(idx, wp)| format!("#{idx} {}", wp.describe(&self.machine.isa)))
                .collect::<Vec<_>>()
                .join(", ")
        }
    }

    pub(super) fn list_points(&self) -> String {
        let mut parts = Vec::new();
        if self.breakpoints.is_empty() {
            parts.push("bp: none".to_string());
        } else {
            let bp = self
                .breakpoints
                .iter()
                .map(|addr| format!("0x{addr:02x}"))
                .collect::<Vec<_>>()
                .join(", ");
            parts.push(format!("bp: {bp}"));
        }
        if self.watchpoints.is_empty() {
            parts.push("watch: none".to_string());
        } else {
            let wp = self
                .watchpoints
                .iter()
                .enumerate()
                .map(|(idx, wp)| format!("#{idx} {}", wp.describe(&self.machine.isa)))
                .collect::<Vec<_>>()
                .join(", ");
            parts.push(format!("watch: {wp}"));
        }
        parts.join(" | ")
    }
}

pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup[1])[1]
}
