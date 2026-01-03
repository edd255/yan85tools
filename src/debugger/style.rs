use palette::{IntoColor, LinSrgb, Mix};
use ratatui::style::{Color, Modifier, Style};

pub fn dim_style() -> Style {
    Style::default().fg(Color::Rgb(130, 130, 130))
}

pub fn opcode_style() -> Style {
    Style::default()
        .fg(Color::LightCyan)
        .add_modifier(Modifier::BOLD)
}

pub fn register_style() -> Style {
    Style::default().fg(Color::LightGreen)
}

pub fn immediate_style() -> Style {
    Style::default().fg(Color::LightYellow)
}

pub fn syscall_style() -> Style {
    Style::default().fg(Color::LightMagenta)
}

pub fn flag_style() -> Style {
    Style::default().fg(Color::LightBlue)
}

pub fn punctuation_style() -> Style {
    dim_style()
}

pub fn breakpoint_style() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

pub fn pc_style() -> Style {
    Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

pub fn sp_style() -> Style {
    Style::default()
        .fg(Color::Magenta)
        .add_modifier(Modifier::BOLD)
}

pub fn invalid_style(active: bool) -> Style {
    let style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);
    if active {
        style.add_modifier(Modifier::REVERSED)
    } else {
        style
    }
}

pub fn generate_tui_ccs() -> Vec<Color> {
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
        let seg = (t * segments as f64).floor().min(segments as f64 - 1.0) as usize;
        let seg_t = (t * segments as f64) - seg as f64;
        let color = stops[seg].mix(stops[seg + 1], seg_t).into_format::<u8>();
        ccs.push(Color::Rgb(color.red, color.green, color.blue));
    }
    ccs
}
