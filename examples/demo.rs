use std::io;

use newtui::components::settings_panel::{
    Backend, Choice, Model, Setting, SettingsPanel, SettingsSeed,
};
use newtui::{
    bar, butterfly, core_grid, gauge, heat_meter, ratatui_lines, sparkline, Component, CoreSeries,
    Flow, Key, SparkDirection, Tone, View, WidgetOutput,
};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{DefaultTerminal, Frame};

const CORE_ZERO_HISTORY: &[f64] = &[15.0, 30.0, 65.0, 40.0, 85.0];
const CORE_ONE_HISTORY: &[f64] = &[80.0, 60.0, 25.0, 45.0, 20.0];

fn main() -> io::Result<()> {
    let name = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "settings".to_string());
    let mut demo = Demo::named(&name).unwrap_or_else(|| {
        eprintln!("unknown demo `{name}`");
        std::process::exit(2);
    });

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut demo);
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal, demo: &mut Demo) -> io::Result<()> {
    loop {
        terminal.draw(|frame| demo.render(frame))?;
        if let Event::Key(event) = event::read()? {
            if event.kind == KeyEventKind::Press && demo.handle(event) {
                return Ok(());
            }
        }
    }
}

enum Demo {
    Settings(SettingsDemo),
    Widget(WidgetDemo),
}

impl Demo {
    fn named(name: &str) -> Option<Self> {
        match name {
            "settings" => Some(Self::Settings(SettingsDemo::new())),
            "sparkline" => Some(Self::Widget(WidgetDemo::new(
                WidgetKind::Sparkline,
                &[24, 12, 6],
            ))),
            "butterfly" => Some(Self::Widget(WidgetDemo::new(
                WidgetKind::Butterfly,
                &[24, 8, 1],
            ))),
            "heat_meter" => Some(Self::Widget(WidgetDemo::new(
                WidgetKind::HeatMeter,
                &[24, 10, 4],
            ))),
            "gauge" => Some(Self::Widget(WidgetDemo::new(
                WidgetKind::Gauge,
                &[24, 12, 4],
            ))),
            "bar" => Some(Self::Widget(WidgetDemo::new(WidgetKind::Bar, &[24, 10, 4]))),
            "core_grid" => Some(Self::Widget(WidgetDemo::new(
                WidgetKind::CoreGrid,
                &[24, 12, 6],
            ))),
            _ => None,
        }
    }

    fn render(&self, frame: &mut Frame<'_>) {
        match self {
            Self::Settings(demo) => demo.render(frame),
            Self::Widget(demo) => demo.render(frame),
        }
    }

    /// `true` means the recorder host should exit, not that a component applied.
    fn handle(&mut self, event: KeyEvent) -> bool {
        match self {
            Self::Settings(demo) => demo.handle(event),
            Self::Widget(demo) => demo.handle(event),
        }
    }
}

struct SettingsDemo {
    panel: SettingsPanel,
    outcome: Option<String>,
}

impl SettingsDemo {
    fn new() -> Self {
        let seed = SettingsSeed::new(
            vec![Setting::choice(
                "tenacity",
                "tenacity",
                "auto",
                vec![
                    Choice::new("auto", "inherit host policy"),
                    Choice::new("steady", "keep trying"),
                ],
            )],
            Model::new("qwen-local", None),
            Backend::new(Some("local")),
        );
        Self {
            panel: SettingsPanel::new(seed),
            outcome: None,
        }
    }

    fn handle(&mut self, event: KeyEvent) -> bool {
        if self.outcome.is_some() {
            return matches!(event.code, KeyCode::Char('q'));
        }
        if matches!(event.code, KeyCode::Char('q')) {
            return true;
        }
        let Some(key) = map_key(event) else {
            return false;
        };
        match self.panel.handle(key) {
            Flow::Stay => {}
            Flow::Close(false) => {
                let intent = if self.panel.intent().is_none() {
                    "none"
                } else {
                    "unexpected"
                };
                self.outcome = Some(format!("Esc cancelled: host intent = {intent}"));
            }
            Flow::Close(true) => {
                self.outcome = Some("accepted: host owns the returned intent".to_string());
            }
        }
        false
    }

    fn render(&self, frame: &mut Frame<'_>) {
        let view = self.panel.view();
        let mut lines = view_lines(&view);
        if let Some(outcome) = &self.outcome {
            lines.push(Line::default());
            lines.push(Line::styled(
                outcome.clone(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ));
            lines.push(Line::styled(
                "press q to close",
                Style::default().fg(Color::DarkGray),
            ));
        }
        let height = u16::try_from(lines.len().saturating_add(2)).unwrap_or(u16::MAX);
        let area = centered(frame.area(), 68, height);
        frame.render_widget(
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" settings ")),
            area,
        );
    }
}

fn view_lines(view: &View) -> Vec<Line<'static>> {
    let mut lines = Vec::with_capacity(view.rows.len().saturating_mul(2) + 2);
    for row in &view.rows {
        let marker = if row.selected { ">" } else { " " };
        let dial = if row.adjustable { " ‹ ›" } else { "" };
        let text = format!("{marker} {:<14} {:<12}{dial}", row.label, row.value);
        let style = if row.selected {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        lines.push(Line::styled(text, style));
        if !row.note.is_empty() {
            lines.push(Line::styled(
                format!("    └ {}", row.note),
                Style::default().fg(Color::DarkGray),
            ));
        }
    }
    lines.push(Line::default());
    lines.push(Line::styled(
        view.footer.clone(),
        Style::default().fg(Color::DarkGray),
    ));
    lines
}

#[derive(Clone, Copy)]
enum WidgetKind {
    Sparkline,
    Butterfly,
    HeatMeter,
    Gauge,
    Bar,
    CoreGrid,
}

impl WidgetKind {
    fn title(self) -> &'static str {
        match self {
            Self::Sparkline => "request latency",
            Self::Butterfly => "network tx | rx",
            Self::HeatMeter => "disk temperature",
            Self::Gauge => "daily budget",
            Self::Bar => "request latency",
            Self::CoreGrid => "cpu cores",
        }
    }

    fn output(self, width: usize) -> WidgetOutput {
        match self {
            Self::Sparkline => sparkline(
                &[10.0, 35.0, 80.0, 20.0, 100.0, 45.0, 70.0],
                100.0,
                width,
                4,
                SparkDirection::Up,
            ),
            Self::Butterfly => butterfly(28.0, 74.0, 100.0, "TX", "RX", width, 1),
            Self::HeatMeter => heat_meter("disk temperature", 72.0, "72%", width, 1),
            Self::Gauge => gauge("daily budget", 7.5, 10.0, width, 1),
            Self::Bar => bar("request latency", 83.0, 100.0, "83 ms", width, 1),
            Self::CoreGrid => {
                let cores = [
                    CoreSeries {
                        label: "0",
                        current: 85.0,
                        history: CORE_ZERO_HISTORY,
                        maximum: 100.0,
                    },
                    CoreSeries {
                        label: "1",
                        current: 20.0,
                        history: CORE_ONE_HISTORY,
                        maximum: 100.0,
                    },
                ];
                core_grid(&cores, width, 4)
            }
        }
    }
}

struct WidgetDemo {
    kind: WidgetKind,
    widths: &'static [usize],
    at: usize,
}

impl WidgetDemo {
    fn new(kind: WidgetKind, widths: &'static [usize]) -> Self {
        Self {
            kind,
            widths,
            at: 0,
        }
    }

    fn handle(&mut self, event: KeyEvent) -> bool {
        match event.code {
            KeyCode::Left => self.at = self.at.saturating_add(1).min(self.widths.len() - 1),
            KeyCode::Right => self.at = self.at.saturating_sub(1),
            KeyCode::Char('q') | KeyCode::Esc => return true,
            _ => {}
        }
        false
    }

    fn render(&self, frame: &mut Frame<'_>) {
        let width = self.widths[self.at];
        let output = self.kind.output(width);
        let lines = ratatui_lines(&output, tone_style);
        let chart_height = u16::try_from(output.lines.len()).unwrap_or(u16::MAX);
        let chart_area = self.chart_area(frame.area(), width, chart_height);
        // Header and footer each draw one line. Giving either a padding row
        // would make the recorder steal the only content row from a short widget.
        let host = centered(frame.area(), 42, chart_height.saturating_add(4));
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(chart_height.saturating_add(2)),
                Constraint::Length(1),
            ])
            .split(host);
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("requested width: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    width.to_string(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ]))
            .alignment(Alignment::Center),
            layout[0],
        );
        frame.render_widget(Paragraph::new(lines).block(self.chart_block()), chart_area);
        frame.render_widget(
            Paragraph::new("← narrower   → wider   q quit")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center),
            layout[2],
        );
    }

    fn chart_area(&self, area: Rect, width: usize, chart_height: u16) -> Rect {
        let host = centered(area, 42, chart_height.saturating_add(4));
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(chart_height.saturating_add(2)),
                Constraint::Length(1),
            ])
            .split(host);
        centered(
            layout[1],
            u16::try_from(width.saturating_add(2)).unwrap_or(u16::MAX),
            chart_height.saturating_add(2),
        )
    }

    fn chart_block(&self) -> Block<'static> {
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", self.kind.title()))
    }
}

#[cfg(test)]
pub(crate) fn assert_recorded_widgets_render_content() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    for name in [
        "sparkline",
        "butterfly",
        "heat_meter",
        "gauge",
        "bar",
        "core_grid",
    ] {
        let Some(Demo::Widget(mut demo)) = Demo::named(name) else {
            panic!("the `{name}` recording names a widget demo");
        };
        for at in 0..demo.widths.len() {
            demo.at = at;
            let width = demo.widths[at];
            let output = demo.kind.output(width);
            let height = u16::try_from(output.lines.len()).expect("demo output height fits a u16");
            // The short tapes expose six rows; the taller recordings have room
            // for their four-row widgets. Keeping the short case constrained is
            // what exercises the release artifact instead of a roomier fiction.
            let recorder_height = if height == 1 { 6 } else { 12 };
            let frame_area = Rect::new(0, 0, 64, recorder_height);
            let backend = TestBackend::new(frame_area.width, frame_area.height);
            let mut terminal = Terminal::new(backend).expect("the test terminal is available");
            terminal
                .draw(|frame| demo.render(frame))
                .expect("the demo renders into its recorder-sized terminal");

            let outer = demo.chart_area(frame_area, width, height);
            let inner = demo.chart_block().inner(outer);
            assert_eq!(
                usize::from(inner.width),
                width,
                "`{name}` says requested width {width}, but receives {} columns",
                inner.width
            );
            assert_eq!(
                inner.height, height,
                "`{name}` loses content rows inside its border"
            );

            let buffer = terminal.backend().buffer();
            let has_content = (inner.y..inner.bottom())
                .any(|y| (inner.x..inner.right()).any(|x| buffer[(x, y)].symbol() != " "));
            assert!(
                has_content,
                "`{name}` at requested width {width} renders a blank interior"
            );
        }
    }
}

fn tone_style(tone: Tone) -> Style {
    let color = match tone {
        Tone::Plain => Color::White,
        Tone::Muted => Color::DarkGray,
        Tone::Label => Color::Cyan,
        Tone::Accent => Color::Magenta,
        Tone::Healthy => Color::Green,
        Tone::Caution => Color::Yellow,
        Tone::Critical => Color::Red,
    };
    Style::default().fg(color)
}

fn map_key(event: KeyEvent) -> Option<Key> {
    match event.code {
        KeyCode::Up => Some(Key::Up),
        KeyCode::Down => Some(Key::Down),
        KeyCode::Left => Some(Key::Left),
        KeyCode::Right => Some(Key::Right),
        KeyCode::Enter => Some(Key::Enter),
        KeyCode::Esc => Some(Key::Esc),
        KeyCode::Backspace => Some(Key::Backspace),
        KeyCode::Tab => Some(Key::Tab),
        KeyCode::BackTab => Some(Key::BackTab),
        KeyCode::Home => Some(Key::Home),
        KeyCode::End => Some(Key::End),
        KeyCode::PageUp => Some(Key::PageUp),
        KeyCode::PageDown => Some(Key::PageDown),
        KeyCode::Char(character) if event.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Key::Ctrl(character))
        }
        KeyCode::Char(character) => Some(Key::Char(character)),
        _ => None,
    }
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}
