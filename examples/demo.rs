use std::io;
use std::time::Instant;

// Both hosts use one registry and the same fixed inputs. The library itself
// never acquires a dependency on its catalog executable.
#[allow(dead_code)]
#[path = "support/fixtures.rs"]
mod fixtures;
use fixtures::{
    notice_text, settings_seed, BspPreview, DemoStream, DiffPreview, Kind as WidgetKind,
    LinkedPreview, ModalPreview, Scenario, TickClock,
};
use newtui::components::settings_panel::SettingsPanel;
use newtui::{ratatui_lines, Component, Flow, Key, Tone, View, WidgetOutput};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};

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
    let mut clock = TickClock::new(Instant::now());
    let mut dirty = true;
    loop {
        dirty |= demo.advance(clock.take_due(Instant::now()));
        if dirty {
            terminal.draw(|frame| demo.render(frame))?;
            dirty = false;
        }
        if event::poll(clock.timeout(Instant::now()))? {
            match event::read()? {
                Event::Key(event) if event.kind == KeyEventKind::Press => {
                    if demo.handle(event) {
                        return Ok(());
                    }
                    dirty = true;
                }
                Event::Resize(_, _) => dirty = true,
                _ => {}
            }
        }
    }
}

enum Demo {
    Settings(SettingsDemo),
    Widget(Box<WidgetDemo>),
}

impl Demo {
    fn advance(&mut self, ticks: usize) -> bool {
        match self {
            Self::Widget(demo) => demo.advance(ticks),
            Self::Settings(_) => false,
        }
    }
    fn named(name: &str) -> Option<Self> {
        match WidgetKind::from_name(name)? {
            WidgetKind::Settings => Some(Self::Settings(SettingsDemo::new())),
            kind => Some(Self::Widget(Box::new(WidgetDemo::new(
                kind,
                kind.demo_widths(),
            )))),
        }
    }

    fn render(&mut self, frame: &mut Frame<'_>) {
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
        let seed = settings_seed(Scenario::Normal);
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
        let lines = self.lines();
        let area = self.panel_area(frame.area(), lines.len());
        frame.render_widget(Paragraph::new(lines).block(self.panel_block()), area);
    }

    fn lines(&self) -> Vec<Line<'static>> {
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
        lines
    }

    fn panel_area(&self, frame_area: Rect, line_count: usize) -> Rect {
        let height = u16::try_from(line_count.saturating_add(2)).unwrap_or(u16::MAX);
        centered(frame_area, 68, height)
    }

    fn panel_block(&self) -> Block<'static> {
        Block::default().borders(Borders::ALL).title(" settings ")
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

struct WidgetDemo {
    kind: WidgetKind,
    widths: &'static [usize],
    at: usize,
    scenario: Scenario,
    diff: DiffPreview,
    bsp: BspPreview,
    modal: ModalPreview,
    linked: LinkedPreview,
    stream: DemoStream,
    notice_index: usize,
}

impl WidgetDemo {
    fn new(kind: WidgetKind, widths: &'static [usize]) -> Self {
        Self {
            kind,
            widths,
            at: 0,
            scenario: Scenario::Normal,
            diff: DiffPreview::default(),
            bsp: BspPreview::default(),
            modal: ModalPreview::default(),
            linked: LinkedPreview::default(),
            stream: DemoStream::default(),
            notice_index: 0,
        }
    }

    fn handle(&mut self, event: KeyEvent) -> bool {
        if event
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return event.code == KeyCode::Char('c')
                && event.modifiers.contains(KeyModifiers::CONTROL);
        }
        if self.kind.supports_stream() {
            match event.code {
                KeyCode::Char(' ') => self.stream.toggle(),
                KeyCode::Char('.') => self.stream.step(),
                KeyCode::Char('r') | KeyCode::F(4) => {
                    self.stream.reset();
                    self.at = 0;
                }
                KeyCode::Char('f') | KeyCode::F(2) if self.kind != WidgetKind::Bsp => {
                    self.scenario = self.scenario.next();
                    self.stream.reset();
                    self.at = if self.scenario == Scenario::Narrow {
                        self.widths.len().saturating_sub(2)
                    } else {
                        0
                    };
                }
                _ => {}
            }
        }
        if self.is_host_driven() {
            match event.code {
                KeyCode::Left | KeyCode::Right
                    if self.kind == WidgetKind::Diff
                        && event.modifiers.contains(KeyModifiers::SHIFT) =>
                {
                    self.diff
                        .handle(map_key(event).expect("an arrow maps to a key"));
                    return false;
                }
                KeyCode::Char('f') | KeyCode::F(2) => {
                    self.scenario = self.scenario.next();
                    self.at = if self.scenario == Scenario::Narrow {
                        2
                    } else {
                        0
                    };
                    self.diff = DiffPreview::default();
                    self.bsp = BspPreview::default();
                    self.modal = ModalPreview::new(self.scenario);
                    self.linked = LinkedPreview::new(self.scenario);
                    self.notice_index = 0;
                }
                KeyCode::Char('r') | KeyCode::F(4) => {
                    self.diff = DiffPreview::default();
                    self.bsp = BspPreview::default();
                    self.modal = ModalPreview::new(self.scenario);
                    self.linked = LinkedPreview::new(self.scenario);
                    self.notice_index = 0;
                    self.at = 0;
                }
                KeyCode::Char('n') if self.kind == WidgetKind::Diff => {
                    self.notice_index = self.notice_index.wrapping_add(1)
                }
                KeyCode::Left | KeyCode::Right => {}
                _ => {
                    if let Some(key) = map_key(event) {
                        if self.kind == WidgetKind::Bsp {
                            self.bsp.handle(key);
                        } else if self.kind == WidgetKind::Modal {
                            self.modal
                                .handle(key, event.modifiers.contains(KeyModifiers::SHIFT));
                        } else if self.kind == WidgetKind::LinkedPanes {
                            self.linked.handle(key);
                        } else {
                            self.diff.handle(key);
                        }
                    }
                }
            }
        }
        match event.code {
            KeyCode::Left => self.at = self.at.saturating_add(1).min(self.widths.len() - 1),
            KeyCode::Right => self.at = self.at.saturating_sub(1),
            KeyCode::Esc if self.kind == WidgetKind::LinkedPanes => {}
            KeyCode::Char('q') | KeyCode::Esc => return true,
            _ => {}
        }
        false
    }

    fn advance(&mut self, ticks: usize) -> bool {
        self.kind.supports_stream()
            && !matches!(self.scenario, Scenario::Empty | Scenario::Error)
            && self.stream.advance(ticks)
    }

    fn render(&mut self, frame: &mut Frame<'_>) {
        let width = self.widths[self.at];
        let mut output = self.output(width);
        let chart_height = u16::try_from(output.lines.len()).unwrap_or(u16::MAX);
        let chart_area = self.chart_area(frame.area(), width, chart_height);
        if self.kind == WidgetKind::LinkedPanes {
            let inner = self.chart_block().inner(chart_area);
            self.linked.resize(inner.width, inner.height);
            output = self
                .linked
                .output(usize::from(inner.width), usize::from(inner.height));
        }
        let lines = ratatui_lines(&output, tone_style);
        // Header and footer each draw one line. Giving either a padding row
        // would make the recorder steal the only content row from a short widget.
        let layout = self.layout(frame.area(), chart_height);
        let state = if matches!(self.scenario, Scenario::Empty | Scenario::Error) {
            self.scenario.name()
        } else if self.stream.paused() {
            "paused"
        } else {
            "live"
        };
        frame.render_widget(
            Paragraph::new(if self.kind == WidgetKind::Diff {
                Line::from(format!(
                    "{} / {} · {width} columns",
                    self.diff.geometry_name(),
                    self.scenario.name()
                ))
            } else if self.kind == WidgetKind::Bsp {
                Line::from(format!(
                    "panel layout / {state} · {} / {} · {width} columns",
                    self.bsp.size_name(),
                    self.scenario.name()
                ))
            } else if self.kind == WidgetKind::Modal {
                Line::from(format!(
                    "modal size / {} / {} · {width} columns",
                    self.modal.zoom_name(),
                    self.scenario.name()
                ))
            } else if self.kind == WidgetKind::LinkedPanes {
                Line::from(format!(
                    "{} / {} · {width} columns",
                    self.linked.mode_name(),
                    self.scenario.name()
                ))
            } else if self.kind.supports_stream() {
                Line::from(vec![
                    Span::styled(
                        format!("{} / synthetic {state}", self.kind.title()),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" · {width} columns"),
                        Style::default().fg(Color::Gray),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::styled("requested width: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        width.to_string(),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            })
            .alignment(Alignment::Center),
            layout[0],
        );
        frame.render_widget(Paragraph::new(lines).block(self.chart_block()), chart_area);
        if self.is_host_driven() || self.kind.supports_stream() {
            frame.render_widget(
                Paragraph::new(if self.kind == WidgetKind::LinkedPanes {
                    self.linked.status()
                } else if self.kind == WidgetKind::Modal {
                    self.modal.status()
                } else if self.kind == WidgetKind::Bsp {
                    format!(
                        "{}\n{}",
                        self.stream.caption(true),
                        self.bsp
                            .status(self.scenario, u16::try_from(width).unwrap(), chart_height)
                    )
                } else if self.kind.supports_stream() {
                    self.kind.stream_caption(self.scenario, &self.stream, true)
                } else {
                    notice_text(&output, self.notice_index)
                        .unwrap_or_else(|| self.scenario.note(self.kind).to_string())
                })
                .style(Style::default().fg(Color::Yellow))
                .wrap(Wrap { trim: false }),
                layout[2],
            );
        }
        frame.render_widget(
            Paragraph::new(if self.kind == WidgetKind::Diff {
                "←→ size · ↑↓ rows · Shift-←→ columns · g layout · e context\nf fixture · n notice · Home scroll reset · r reset · q quit"
            } else if self.kind == WidgetKind::Bsp {
                "Tab divider · ↑↓ ratio · s shrink/restore · Space pause · . step\n←→ size · f fixture · r reset · x reject NaN · q quit"
            } else if self.kind == WidgetKind::Modal {
                "Shift-↑↓ or +/- height from the granted rows · z zoom/restore\n←→ size · f fixture · r reset · q quit"
            } else if self.kind == WidgetKind::LinkedPanes {
                "↑↓/Pg/Home/End cursor · Tab focus · l mode · Esc cancel\n> cursor · = mapped row · @ window top · ←→ size · f fixture · r reset · q quit"
            } else if self.kind.supports_stream() {
                "Space pause/resume · . single step · r reset · f fixture\n←→ size · q quit"
            } else {
                "← narrower   → wider   q quit"
            })
                .style(Style::default().fg(if self.kind.supports_stream() { Color::Gray } else { Color::DarkGray }))
                .alignment(Alignment::Center),
            layout[3],
        );
    }

    fn output(&self, width: usize) -> WidgetOutput {
        if self.kind == WidgetKind::Butterfly {
            let mut compact = self
                .kind
                .stream_output(self.scenario, &self.stream, width, 1);
            compact
                .lines
                .push(newtui::WidgetLine::new(vec![newtui::Run::new(
                    " ".repeat(width),
                    Tone::Muted,
                )]));
            compact.lines.extend(
                WidgetKind::ButterflyHistory
                    .stream_output(self.scenario, &self.stream, width, 24)
                    .lines,
            );
            compact
        } else if self.kind == WidgetKind::LinkedPanes {
            self.linked.output(width, 16)
        } else if self.kind == WidgetKind::Modal {
            self.modal.output(width, 16)
        } else if self.kind == WidgetKind::Bsp {
            self.bsp
                .output_with_stream(self.scenario, width, 16, Some(&self.stream))
        } else if self.kind.supports_stream() {
            let height = match self.kind {
                WidgetKind::Sparkline => 8,
                WidgetKind::ButterflyHistory => 24,
                WidgetKind::CoreGrid => 12,
                _ => 1,
            };
            self.kind
                .stream_output(self.scenario, &self.stream, width, height)
        } else if self.kind == WidgetKind::Diff {
            self.diff.output(self.scenario, width, 16)
        } else {
            self.kind
                .output(self.scenario, width)
                .expect("a widget demo has output")
        }
    }

    fn layout(&self, area: Rect, chart_height: u16) -> [Rect; 4] {
        let is_rich = self.is_host_driven() || self.kind.supports_stream();
        let host = centered(
            area,
            if is_rich { 106 } else { 42 },
            chart_height.saturating_add(if self.kind == WidgetKind::Bsp {
                9
            } else if is_rich {
                8
            } else {
                4
            }),
        );
        let parts = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(chart_height.saturating_add(2)),
                Constraint::Length(if self.kind == WidgetKind::Bsp {
                    4
                } else if is_rich {
                    3
                } else {
                    0
                }),
                Constraint::Length(if is_rich { 2 } else { 1 }),
            ])
            .split(host);
        [parts[0], parts[1], parts[2], parts[3]]
    }

    fn chart_area(&self, area: Rect, width: usize, chart_height: u16) -> Rect {
        let layout = self.layout(area, chart_height);
        centered(
            layout[1],
            u16::try_from(width.saturating_add(2)).unwrap_or(u16::MAX),
            chart_height.saturating_add(2),
        )
    }

    /// Pieces whose presentation state the host keeps between key presses.
    fn is_host_driven(&self) -> bool {
        matches!(
            self.kind,
            WidgetKind::Diff | WidgetKind::Bsp | WidgetKind::Modal | WidgetKind::LinkedPanes
        )
    }

    fn chart_block(&self) -> Block<'static> {
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", self.kind.title()))
    }
}

#[cfg(test)]
pub(crate) fn assert_recorded_demos_render_content() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    for name in [
        "sparkline",
        "butterfly",
        "butterfly_history",
        "heat_meter",
        "gauge",
        "bar",
        "core_grid",
        "diff",
        "bsp",
        "modal",
        "linked_panes",
    ] {
        let Some(Demo::Widget(mut demo)) = Demo::named(name) else {
            panic!("the `{name}` recording names a widget demo");
        };
        for at in 0..demo.widths.len() {
            demo.at = at;
            let width = demo.widths[at];
            let output = demo.output(width);
            let height = u16::try_from(output.lines.len()).expect("demo output height fits a u16");
            // The short tapes expose six rows; the taller recordings have room
            // for their four-row widgets. Keeping the short case constrained is
            // what exercises the release artifact instead of a roomier fiction.
            let recorder_height = if demo.kind.supports_stream() {
                height + if demo.kind == WidgetKind::Bsp { 9 } else { 8 }
            } else if matches!(name, "diff" | "bsp" | "modal" | "linked_panes") {
                28
            } else if height == 1 {
                6
            } else {
                12
            };
            let frame_area = Rect::new(
                0,
                0,
                if matches!(name, "diff" | "bsp" | "modal" | "linked_panes")
                    || demo.kind.supports_stream()
                {
                    120
                } else {
                    64
                },
                recorder_height,
            );
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

            let has_content = area_has_content(terminal.backend().buffer(), inner);
            assert!(
                has_content,
                "`{name}` at requested width {width} renders a blank interior"
            );
        }
    }

    let Some(Demo::Settings(settings)) = Demo::named("settings") else {
        panic!("the `settings` recording names its component demo");
    };
    let frame_area = Rect::new(0, 0, 80, 20);
    let backend = TestBackend::new(frame_area.width, frame_area.height);
    let mut terminal = Terminal::new(backend).expect("the test terminal is available");
    terminal
        .draw(|frame| settings.render(frame))
        .expect("the settings demo renders into its recorder-sized terminal");
    let inner = settings
        .panel_block()
        .inner(settings.panel_area(frame_area, settings.lines().len()));
    assert!(
        area_has_content(terminal.backend().buffer(), inner),
        "`settings` renders a blank interior"
    );
}

#[cfg(test)]
#[test]
fn diff_demo_keys_keep_the_real_widget_visible_across_layouts_and_fixtures() {
    use newtui::DiffGeometry;
    use ratatui::{backend::TestBackend, Terminal};

    let mut demo = WidgetDemo::new(WidgetKind::Diff, WidgetKind::Diff.demo_widths());
    let press = |demo: &mut WidgetDemo, code| demo.handle(KeyEvent::new(code, KeyModifiers::NONE));
    for scenario in Scenario::ALL {
        assert_eq!(demo.scenario, scenario);
        for _ in 0..4 {
            press(&mut demo, KeyCode::Right);
        }
        for geometry in [
            DiffGeometry::Unified,
            DiffGeometry::Split,
            DiffGeometry::Stat,
        ] {
            assert_eq!(demo.diff.geometry, geometry);
            for width in WidgetKind::Diff.demo_widths() {
                assert_eq!(demo.widths[demo.at], *width);
                let output = demo.output(*width);
                let mut terminal = Terminal::new(TestBackend::new(120, 28)).unwrap();
                terminal.draw(|frame| demo.render(frame)).unwrap();
                let buffer = terminal.backend().buffer();
                let area = demo
                    .chart_block()
                    .inner(demo.chart_area(buffer.area, *width, 16));
                for (row, expected) in output.lines.iter().enumerate() {
                    let actual: String = (area.x..area.right())
                        .map(|column| {
                            buffer[(column, area.y + u16::try_from(row).unwrap())].symbol()
                        })
                        .collect();
                    assert_eq!(actual, expected.text(), "{geometry:?}/{scenario:?}/{width}");
                }
                press(&mut demo, KeyCode::Left);
            }
            for _ in 0..4 {
                press(&mut demo, KeyCode::Right);
            }
            press(&mut demo, KeyCode::Char('g'));
        }
        press(&mut demo, KeyCode::Char('f'));
    }
    press(&mut demo, KeyCode::Char('e'));
    press(&mut demo, KeyCode::Down);
    demo.handle(KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT));
    assert!(demo.diff.expanded);
    assert_eq!((demo.diff.row_offset, demo.diff.column_offset), (1, 4));
    press(&mut demo, KeyCode::Char('n'));
    assert_eq!(demo.notice_index, 1);
    press(&mut demo, KeyCode::Home);
    assert_eq!((demo.diff.row_offset, demo.diff.column_offset), (0, 0));
    press(&mut demo, KeyCode::Char('r'));
    assert_eq!(demo.diff, DiffPreview::default());
    assert_eq!(demo.notice_index, 0);
    demo.handle(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL));
    assert_eq!(demo.diff.geometry, DiffGeometry::Unified);
    assert!(demo.handle(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)));
    assert!(press(&mut demo, KeyCode::Char('q')));
}

#[cfg(test)]
fn area_has_content(buffer: &ratatui::buffer::Buffer, area: Rect) -> bool {
    (area.y..area.bottom()).any(|y| (area.x..area.right()).any(|x| buffer[(x, y)].symbol() != " "))
}

#[cfg(test)]
#[test]
fn linked_demo_forwards_navigation_and_renders_the_component_window() {
    use newtui::components::linked_panes::{LinkMode, PaneSide};
    use ratatui::{backend::TestBackend, Terminal};
    let mut demo = WidgetDemo::new(
        WidgetKind::LinkedPanes,
        WidgetKind::LinkedPanes.demo_widths(),
    );
    let press = |demo: &mut WidgetDemo, code| demo.handle(KeyEvent::new(code, KeyModifiers::NONE));
    for scenario in Scenario::ALL {
        assert_eq!(demo.scenario, scenario);
        for _ in 0..4 {
            press(&mut demo, KeyCode::Right);
        }
        for width in WidgetKind::LinkedPanes.demo_widths() {
            assert_eq!(demo.widths[demo.at], *width);
            let mut terminal = Terminal::new(TestBackend::new(120, 28)).unwrap();
            terminal.draw(|frame| demo.render(frame)).unwrap();
            let buffer = terminal.backend().buffer();
            let area = demo
                .chart_block()
                .inner(demo.chart_area(buffer.area, *width, 16));
            let output = demo
                .linked
                .output(usize::from(area.width), usize::from(area.height));
            for (row, expected) in output.lines.iter().enumerate() {
                let actual: String = (area.x..area.right())
                    .map(|x| buffer[(x, area.y + u16::try_from(row).unwrap())].symbol())
                    .collect();
                assert_eq!(actual, expected.text(), "{scenario:?}/{width}");
            }
            let note = demo.layout(buffer.area, 16)[2];
            let text: String = (note.y..note.bottom())
                .flat_map(|y| (note.x..note.right()).map(move |x| (x, y)))
                .map(|point| buffer[point].symbol())
                .collect();
            for line in demo.linked.status().lines() {
                assert!(text.contains(line), "missing {line:?}");
            }
            press(&mut demo, KeyCode::Left);
        }
        press(&mut demo, KeyCode::Char('f'));
    }
    let mut terminal = Terminal::new(TestBackend::new(80, 14)).unwrap();
    terminal.draw(|frame| demo.render(frame)).unwrap();
    let area = demo.chart_block().inner(demo.chart_area(
        terminal.backend().buffer().area,
        demo.widths[demo.at],
        16,
    ));
    let page = usize::from(area.height.saturating_sub(u16::from(area.height > 1)));
    assert_eq!(
        demo.linked
            .component()
            .unwrap()
            .position(PaneSide::First)
            .height,
        page
    );
    press(&mut demo, KeyCode::PageDown);
    assert_eq!(
        demo.linked
            .component()
            .unwrap()
            .position(PaneSide::First)
            .cursor,
        Some(page.clamp(1, 10))
    );
    let before = demo.linked.component().unwrap().clone();
    press(&mut demo, KeyCode::Tab);
    assert_eq!(
        demo.linked.component().unwrap().relation(),
        before.relation()
    );
    press(&mut demo, KeyCode::Char('l'));
    assert_eq!(
        demo.linked.component().unwrap().mode(),
        LinkMode::Proportional
    );
    press(&mut demo, KeyCode::Char('l'));
    assert_eq!(demo.linked.component().unwrap().mode(), LinkMode::Unlinked);
    let before = demo.linked.component().unwrap().view();
    press(&mut demo, KeyCode::Enter);
    press(&mut demo, KeyCode::Char('x'));
    demo.handle(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::ALT));
    assert_eq!(demo.linked.component().unwrap().view(), before);
    assert!(!press(&mut demo, KeyCode::Esc));
    assert!(demo.linked.status().contains("Esc cancelled"));
    press(&mut demo, KeyCode::Down);
    assert_eq!(demo.linked.component().unwrap().view(), before);
    press(&mut demo, KeyCode::Char('r'));
    assert_eq!(demo.linked.component().unwrap().mode(), LinkMode::Locked);
    assert!(press(&mut demo, KeyCode::Char('q')));
}

#[cfg(test)]
#[test]
fn numeric_named_demos_advance_source_cells_and_honor_pause_step_reset() {
    use ratatui::{backend::TestBackend, Terminal};
    use std::collections::BTreeSet;
    for entry in fixtures::ENTRIES
        .iter()
        .filter(|entry| entry.kind.supports_stream())
    {
        let mut demo = Demo::named(entry.id).unwrap();
        let content = |demo: &mut Demo| {
            let Demo::Widget(widget) = demo else {
                panic!("numeric entry uses the widget host");
            };
            let output = widget.output(widget.widths[0]);
            let height = u16::try_from(output.lines.len()).unwrap();
            let mut terminal = Terminal::new(TestBackend::new(120, height + 10)).unwrap();
            terminal.draw(|frame| demo.render(frame)).unwrap();
            let buffer = terminal.backend().buffer();
            let Demo::Widget(widget) = demo else {
                unreachable!("render preserves the selected demo");
            };
            let area = widget.chart_block().inner(widget.chart_area(
                buffer.area,
                widget.widths[0],
                height,
            ));
            (area.y..area.bottom())
                .map(|y| {
                    (area.x..area.right())
                        .map(|x| buffer[(x, y)].symbol())
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
        };
        let mut frames = BTreeSet::new();
        for _ in 0..24 {
            frames.insert(content(&mut demo));
            assert!(demo.advance(1));
        }
        assert!(
            frames.len() >= 8,
            "{} named recording must move actual chart cells",
            entry.id
        );
        let press = |demo: &mut Demo, code| demo.handle(KeyEvent::new(code, KeyModifiers::NONE));
        press(&mut demo, KeyCode::Char(' '));
        let paused = content(&mut demo);
        assert!(!demo.advance(50));
        assert_eq!(content(&mut demo), paused);
        press(&mut demo, KeyCode::Char('.'));
        assert_ne!(content(&mut demo), paused);
        let Demo::Widget(widget) = &demo else {
            unreachable!();
        };
        assert!(widget.stream.paused());
        assert_eq!(widget.stream.tick(), 25);
        press(&mut demo, KeyCode::Char('r'));
        let Demo::Widget(widget) = &demo else {
            unreachable!();
        };
        assert_eq!(widget.stream.tick(), 0);
        assert!(widget.stream.paused());
        assert!(press(&mut demo, KeyCode::Char('q')));
    }
}

#[cfg(test)]
#[test]
fn bsp_demo_keys_restore_the_same_cells_and_keep_geometry_status_visible() {
    use ratatui::{backend::TestBackend, Terminal};
    let mut demo = WidgetDemo::new(WidgetKind::Bsp, WidgetKind::Bsp.demo_widths());
    let press = |demo: &mut WidgetDemo, code| demo.handle(KeyEvent::new(code, KeyModifiers::NONE));
    for scenario in Scenario::ALL {
        assert_eq!(demo.scenario, scenario);
        for _ in 0..4 {
            press(&mut demo, KeyCode::Right);
        }
        for width in WidgetKind::Bsp.demo_widths() {
            assert_eq!(demo.widths[demo.at], *width);
            let original = demo.output(*width);
            press(&mut demo, KeyCode::Char('s'));
            assert_eq!(demo.bsp.size_name(), "half width");
            press(&mut demo, KeyCode::Char('s'));
            assert_eq!(demo.output(*width), original);
            let mut terminal = Terminal::new(TestBackend::new(120, 28)).unwrap();
            terminal.draw(|frame| demo.render(frame)).unwrap();
            let buffer = terminal.backend().buffer();
            let area = demo
                .chart_block()
                .inner(demo.chart_area(buffer.area, *width, 16));
            for (row, expected) in original.lines.iter().enumerate() {
                let actual: String = (area.x..area.right())
                    .map(|column| buffer[(column, area.y + u16::try_from(row).unwrap())].symbol())
                    .collect();
                assert_eq!(actual, expected.text());
            }
            let note = demo.layout(buffer.area, 16)[2];
            let actual: String = (note.y..note.bottom())
                .flat_map(|row| {
                    (note.x..note.right()).map(move |column| buffer[(column, row)].symbol())
                })
                .collect();
            for line in demo
                .bsp
                .status(scenario, u16::try_from(*width).unwrap(), 16)
                .lines()
            {
                assert!(
                    actual.contains(line),
                    "status stays outside even a one-cell preview: {actual:?}"
                );
            }
            press(&mut demo, KeyCode::Left);
        }
        press(&mut demo, KeyCode::Char('f'));
    }
    press(&mut demo, KeyCode::Tab);
    press(&mut demo, KeyCode::Up);
    assert_eq!(demo.bsp.changed(Scenario::Normal, 88, 16), vec![10, 30]);
    press(&mut demo, KeyCode::Char('x'));
    assert!(demo.bsp.changed(Scenario::Normal, 88, 16).is_empty());
    let before = demo.bsp.clone();
    assert!(!demo.handle(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::ALT)));
    assert_eq!(demo.bsp, before);
    press(&mut demo, KeyCode::Char('r'));
    assert_eq!(demo.bsp, BspPreview::default());
    assert!(demo.handle(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)));
    assert!(press(&mut demo, KeyCode::Char('q')));
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
        Tone::Added => Color::Green,
        Tone::Removed => Color::Red,
        Tone::Context => Color::White,
        Tone::Hunk => Color::Cyan,
        _ => Color::White,
    };
    let style = Style::default().fg(color);
    match tone {
        Tone::Added => style.bg(Color::Rgb(22, 54, 34)),
        Tone::Removed => style.bg(Color::Rgb(65, 28, 29)),
        _ => style,
    }
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
