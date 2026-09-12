//! The catalog is a terminal host, separate from the dependency-free library.
#[path = "../../examples/support/fixtures.rs"]
pub mod fixtures;

pub mod options;
mod palette;

use fixtures::{settings_seed, Entry, Kind, Scenario, ENTRIES};
use newtui::{components::settings_panel::SettingsPanel, ratatui_lines, Component, Flow, Key};
use options::{Options, Theme};
use palette::Palette;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

/// Live catalog state. Fixtures are fixed data; effects are only displayed.
pub struct Catalog {
    selected: usize,
    scenario: Scenario,
    theme: Theme,
    width: u16,
    focused: bool,
    searching: bool,
    query: String,
    panel: SettingsPanel,
    outcome: Option<String>,
}

impl Default for Catalog {
    fn default() -> Self {
        Self::new(Options::default())
    }
}

impl Catalog {
    pub fn new(options: Options) -> Self {
        Self {
            selected: ENTRIES
                .iter()
                .position(|entry| entry.kind == options.item)
                .unwrap_or(0),
            scenario: options.scenario,
            theme: options.theme,
            width: options.width,
            focused: false,
            searching: false,
            query: String::new(),
            panel: SettingsPanel::new(settings_seed(options.scenario)),
            outcome: None,
        }
    }

    fn visible(&self) -> Vec<usize> {
        let query = self.query.to_lowercase();
        ENTRIES
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                format!("{} {} {}", entry.id, entry.name, entry.description)
                    .to_lowercase()
                    .contains(&query)
            })
            .map(|(index, _)| index)
            .collect()
    }

    pub fn selected_entry(&self) -> Option<&'static Entry> {
        self.visible()
            .contains(&self.selected)
            .then(|| &ENTRIES[self.selected])
    }

    fn reset(&mut self) {
        self.panel = SettingsPanel::new(settings_seed(self.scenario));
        self.outcome = None;
    }

    fn cycle_scenario(&mut self) {
        self.scenario = self.scenario.next();
        self.width = if self.scenario == Scenario::Narrow {
            8
        } else {
            48
        };
        self.reset();
    }

    fn resize(&mut self, grow: bool) {
        self.width = if grow {
            self.width.saturating_add(4).min(200)
        } else {
            self.width.saturating_sub(4).max(1)
        };
    }

    fn move_selection(&mut self, forward: bool) {
        let visible = self.visible();
        if visible.is_empty() {
            return;
        }
        let at = visible
            .iter()
            .position(|index| *index == self.selected)
            .unwrap_or(0);
        self.selected = visible[if forward {
            at.saturating_add(1).min(visible.len() - 1)
        } else {
            at.saturating_sub(1)
        }];
        self.reset();
    }

    /// Returns true only when the catalog host should exit.
    pub fn handle(&mut self, event: KeyEvent) -> bool {
        if event.code == KeyCode::Char('c') && event.modifiers.contains(KeyModifiers::CONTROL) {
            return true;
        }
        if self.searching {
            match event.code {
                KeyCode::Esc => {
                    self.query.clear();
                    self.searching = false;
                }
                KeyCode::Enter => self.searching = false,
                KeyCode::Backspace => {
                    self.query.pop();
                }
                KeyCode::Char(character)
                    if !event
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    self.query.push(character)
                }
                _ => return false,
            }
            if !self.visible().contains(&self.selected) {
                self.selected = self.visible().first().copied().unwrap_or(0);
                self.reset();
            }
            return false;
        }
        match event.code {
            KeyCode::F(1) => {
                self.focused = false;
                return false;
            }
            KeyCode::F(2) => {
                self.cycle_scenario();
                return false;
            }
            KeyCode::F(3) => {
                self.theme = self.theme.next();
                return false;
            }
            KeyCode::F(4) => {
                self.reset();
                return false;
            }
            _ => {}
        }
        if self.focused {
            if self
                .selected_entry()
                .is_some_and(|entry| entry.kind == Kind::Settings)
            {
                if self.outcome.is_none() {
                    if let Some(key) = map_key(event) {
                        match self.panel.handle(key) {
                            Flow::Stay => {}
                            Flow::Close(false) => {
                                self.outcome = Some(format!(
                                    "Cancelled · host intent: {}",
                                    if self.panel.intent().is_none() {
                                        "none"
                                    } else {
                                        "unexpected"
                                    }
                                ))
                            }
                            Flow::Close(true) => {
                                self.outcome = Some(format!("Accepted · {:?}", self.panel.intent()))
                            }
                        }
                    }
                }
            } else {
                match event.code {
                    KeyCode::Left => self.resize(false),
                    KeyCode::Right => self.resize(true),
                    KeyCode::Esc => self.focused = false,
                    _ => {}
                }
            }
            return false;
        }
        if matches!(event.code, KeyCode::Char(_))
            && event
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return false;
        }
        match event.code {
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(false),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(true),
            KeyCode::Left => self.resize(false),
            KeyCode::Right => self.resize(true),
            KeyCode::Enter => self.focused = self.selected_entry().is_some(),
            KeyCode::Char('/') => self.searching = true,
            KeyCode::Char('f') => self.cycle_scenario(),
            KeyCode::Char('t') => self.theme = self.theme.next(),
            KeyCode::Char('r') => self.reset(),
            KeyCode::Char('q') | KeyCode::Esc => return true,
            _ => {}
        }
        false
    }

    /// Draw the same host that is used for live browsing and recorded screenshots.
    pub fn render(&self, frame: &mut Frame<'_>) {
        let palette = Palette::for_theme(self.theme);
        frame.render_widget(
            Block::default().style(Style::default().bg(palette.background).fg(palette.text)),
            frame.area(),
        );
        if frame.area().width < 54 || frame.area().height < 28 {
            frame.render_widget(Paragraph::new("newtui / live catalog\nResize to at least 54 × 28 cells.\nCtrl-C exits any mode").style(Style::default().fg(palette.accent)).wrap(Wrap { trim: false }), frame.area());
            return;
        }
        let regions = Regions::new(frame.area());
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(vec![
                    Span::styled(
                        "newtui",
                        Style::default()
                            .fg(palette.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("   /   LIVE CATALOG", Style::default().fg(palette.muted)),
                ]),
                Line::from(Span::styled(
                    "Shawn's custom TUI widgets",
                    Style::default().fg(palette.text),
                )),
            ]),
            regions.header,
        );
        self.render_sidebar(frame, regions.sidebar, palette);
        if let Some(entry) = self.selected_entry() {
            self.render_detail(frame, &regions, entry, palette);
        } else {
            frame.render_widget(
                Paragraph::new(
                    "No matching pieces.\nPress / to edit the search, or / then Esc to clear it.",
                )
                .style(Style::default().fg(palette.muted))
                .wrap(Wrap { trim: false }),
                regions.detail,
            );
        }
        let hint = if self.searching {
            "SEARCH   type to filter · Enter keep · Esc clear"
        } else if self.focused {
            "INTERACT   F1 catalog · F2 fixture · F3 theme · F4 reset · Ctrl-C quit"
        } else if frame.area().width < 80 {
            "BROWSE ↑↓ select · Enter interact · / search\n←→ size · f fixture · t theme · r reset · q quit"
        } else {
            "BROWSE   ↑↓ select · Enter interact · / search · ←→ size · f fixture · t theme · r reset · q quit"
        };
        let mut footer: Vec<_> = hint
            .lines()
            .map(|line| Line::styled(line, Style::default().fg(palette.accent)))
            .collect();
        footer.push(Line::styled(
            if frame.area().width < 80 {
                "Live library fixtures."
            } else {
                "Fixed samples. Real library code. Your terminal is the canvas."
            },
            Style::default().fg(palette.muted),
        ));
        frame.render_widget(
            Paragraph::new(footer).wrap(Wrap { trim: false }),
            regions.footer,
        );
    }

    fn render_sidebar(&self, frame: &mut Frame<'_>, area: Rect, palette: Palette) {
        let outer = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(palette.border))
            .title(" COLLECTION ");
        let inner = outer.inner(area);
        frame.render_widget(outer, area);
        let parts = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(4),
            Constraint::Length(3),
        ])
        .split(inner);
        let search = if self.query.is_empty() {
            if self.searching {
                "/ _".to_string()
            } else {
                "/  Find a piece".to_string()
            }
        } else {
            format!("/ {}{}", self.query, if self.searching { "_" } else { "" })
        };
        frame.render_widget(
            Paragraph::new(search).style(Style::default().fg(if self.searching {
                palette.accent
            } else {
                palette.muted
            })),
            parts[0].inner(Margin {
                horizontal: 1,
                vertical: 1,
            }),
        );
        let visible = self.visible();
        let items: Vec<ListItem<'_>> = visible
            .iter()
            .map(|index| {
                let entry = &ENTRIES[*index];
                ListItem::new(vec![
                    Line::from(entry.name),
                    Line::styled(
                        if entry.kind == Kind::Settings {
                            "  INTERACTIVE"
                        } else {
                            "  DISPLAY WIDGET"
                        },
                        Style::default().fg(palette.muted),
                    ),
                ])
            })
            .collect();
        let list = List::new(items)
            .style(Style::default().fg(palette.text))
            .highlight_symbol("▸ ")
            .highlight_style(
                Style::default()
                    .bg(palette.selected)
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            );
        let mut state = ListState::default()
            .with_selected(visible.iter().position(|index| *index == self.selected));
        frame.render_stateful_widget(list, parts[1], &mut state);
        frame.render_widget(
            Paragraph::new(format!(
                "{} of {} pieces\nzero core deps",
                visible.len(),
                ENTRIES.len()
            ))
            .style(Style::default().fg(palette.muted)),
            parts[2].inner(Margin {
                horizontal: 1,
                vertical: 0,
            }),
        );
    }

    fn render_detail(
        &self,
        frame: &mut Frame<'_>,
        regions: &Regions,
        entry: &Entry,
        palette: Palette,
    ) {
        frame.render_widget(
            Paragraph::new(vec![
                Line::styled(
                    entry.name,
                    Style::default()
                        .fg(palette.text)
                        .add_modifier(Modifier::BOLD),
                ),
                Line::from(vec![
                    Span::styled(
                        format!("{}  ", entry.id),
                        Style::default().fg(palette.muted),
                    ),
                    Span::styled(
                        format!("{} / {}", self.scenario.name(), self.theme.name()),
                        Style::default().fg(palette.warm),
                    ),
                ]),
            ]),
            regions.title,
        );
        frame.render_widget(
            Paragraph::new(entry.description)
                .style(Style::default().fg(palette.muted))
                .wrap(Wrap { trim: false }),
            regions.description,
        );
        let block = preview_block(self.focused, palette);
        let inside = block.inner(regions.preview);
        frame.render_widget(block, regions.preview);
        if entry.kind == Kind::Settings {
            self.render_settings(frame, inside, palette);
        } else if let Some(content) = self.preview_content_rect(frame.area()) {
            let output = entry
                .kind
                .output(self.scenario, usize::from(content.width))
                .expect("a display entry produces widget output");
            frame.render_widget(
                Paragraph::new(ratatui_lines(&output, |tone| palette.tone(tone))),
                content,
            );
            let caption = Rect::new(inside.x, inside.bottom().saturating_sub(1), inside.width, 1);
            frame.render_widget(
                Paragraph::new(format!(
                    "{}×{} cells{}",
                    content.width,
                    content.height,
                    if content.width < self.width {
                        " · limited"
                    } else {
                        ""
                    }
                ))
                .alignment(ratatui::layout::Alignment::Center)
                .style(Style::default().fg(palette.muted)),
                caption,
            );
        }
        let note = self
            .outcome
            .as_deref()
            .unwrap_or_else(|| self.scenario.note(entry.kind));
        frame.render_widget(
            Paragraph::new(note)
                .style(Style::default().fg(if self.outcome.is_some() {
                    palette.warm
                } else {
                    palette.muted
                }))
                .wrap(Wrap { trim: false }),
            regions.note,
        );
        frame.render_widget(
            Paragraph::new(vec![
                Line::styled("HOST SUPPLIES", Style::default().fg(palette.accent)),
                Line::styled(entry.data, Style::default().fg(palette.muted)),
            ])
            .wrap(Wrap { trim: false }),
            regions.data,
        );
    }

    fn render_settings(&self, frame: &mut Frame<'_>, area: Rect, palette: Palette) {
        let content = area.inner(Margin {
            horizontal: 1,
            vertical: 1,
        });
        let width = self.width.min(content.width);
        let target = Rect::new(
            content.x + (content.width - width) / 2,
            content.y,
            width,
            content.height,
        );
        let view = self.panel.view();
        let capacity = usize::from(target.height.saturating_sub(2)) / 3;
        let capacity = capacity.max(1);
        let selected = view.selected().unwrap_or(0);
        let start = selected.saturating_add(1).saturating_sub(capacity);
        let mut lines = Vec::new();
        for row in view.rows.iter().skip(start).take(capacity) {
            let style = Style::default()
                .fg(if row.selected {
                    palette.accent
                } else {
                    palette.text
                })
                .bg(if row.selected {
                    palette.selected
                } else {
                    palette.panel
                });
            lines.push(Line::styled(
                format!("{} {}", if row.selected { "▸" } else { " " }, row.label),
                style,
            ));
            lines.push(Line::styled(
                format!(
                    "  {}{}",
                    row.value,
                    if row.adjustable { "   ‹ ›" } else { "" }
                ),
                Style::default().fg(palette.warm),
            ));
            lines.push(Line::styled(
                format!("  {}", row.note),
                Style::default().fg(palette.muted),
            ));
        }
        let clipped = lines.iter().any(|line| line.width() > usize::from(width));
        frame.render_widget(Paragraph::new(lines), target);
        let status = if view.rows.len() > capacity {
            format!(
                "rows {}–{} / {} · ↑↓ scroll",
                start + 1,
                (start + capacity).min(view.rows.len()),
                view.rows.len()
            )
        } else if self.focused {
            "↑↓ select · ←→ dial · Enter apply · Esc cancel".to_string()
        } else {
            "Enter to try · F1 returns to catalog".to_string()
        };
        frame.render_widget(
            Paragraph::new(status)
                .style(Style::default().fg(palette.muted))
                .alignment(ratatui::layout::Alignment::Center),
            Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1),
        );
        if width < self.width || clipped {
            frame.render_widget(
                Paragraph::new(format!(
                    "{} columns · {}",
                    width,
                    if clipped {
                        "text clipped"
                    } else {
                        "limited by terminal"
                    }
                ))
                .style(Style::default().fg(palette.warm)),
                Rect::new(area.x, area.y, area.width, 1),
            );
        }
    }

    /// The actual widget content rectangle, excluding borders and caption.
    /// Tests compare the library output with this region in the rendered host.
    pub fn preview_content_rect(&self, area: Rect) -> Option<Rect> {
        if area.width < 54 || area.height < 28 {
            return None;
        }
        let entry = self.selected_entry()?;
        if entry.kind == Kind::Settings {
            return None;
        }
        let regions = Regions::new(area);
        let inner = preview_block(self.focused, Palette::for_theme(self.theme))
            .inner(regions.preview)
            .inner(Margin {
                horizontal: 1,
                vertical: 1,
            });
        let width = self.width.min(inner.width);
        let height = entry
            .kind
            .output(self.scenario, usize::from(width))?
            .lines
            .len();
        let height = u16::try_from(height)
            .unwrap_or(u16::MAX)
            .min(inner.height.saturating_sub(1));
        Some(Rect::new(
            inner.x + (inner.width - width) / 2,
            inner.y + inner.height.saturating_sub(height + 1) / 2,
            width,
            height,
        ))
    }
}

fn preview_block(focused: bool, palette: Palette) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(palette.panel))
        .border_style(Style::default().fg(if focused {
            palette.accent
        } else {
            palette.border
        }))
        .title(if focused {
            " LIVE PREVIEW / INTERACT "
        } else {
            " LIVE PREVIEW "
        })
}

struct Regions {
    header: Rect,
    sidebar: Rect,
    detail: Rect,
    title: Rect,
    description: Rect,
    preview: Rect,
    note: Rect,
    data: Rect,
    footer: Rect,
}

impl Regions {
    fn new(area: Rect) -> Self {
        let inset = area.inner(Margin {
            horizontal: 2,
            vertical: 1,
        });
        let main = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(12),
            Constraint::Length(3),
        ])
        .split(inset);
        let body = Layout::horizontal([
            Constraint::Length(if area.width < 80 { 20 } else { 26 }),
            Constraint::Length(2),
            Constraint::Min(20),
        ])
        .split(main[1]);
        let detail = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Min(4),
            Constraint::Length(if area.height >= 30 { 4 } else { 3 }),
            Constraint::Length(if area.height >= 30 { 4 } else { 3 }),
        ])
        .split(body[2]);
        Self {
            header: main[0],
            sidebar: body[0],
            detail: body[2],
            title: detail[0],
            description: detail[1],
            preview: detail[2],
            note: detail[3],
            data: detail[4],
            footer: main[2],
        }
    }
}

fn map_key(event: KeyEvent) -> Option<Key> {
    Some(match event.code {
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Enter => Key::Enter,
        KeyCode::Esc => Key::Esc,
        KeyCode::Tab => Key::Tab,
        KeyCode::BackTab => Key::BackTab,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Char(character) if event.modifiers.contains(KeyModifiers::CONTROL) => {
            Key::Ctrl(character)
        }
        KeyCode::Char(character) => Key::Char(character),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

    fn press(catalog: &mut Catalog, code: KeyCode) -> bool {
        catalog.handle(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn render(catalog: &Catalog, width: u16, height: u16) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| catalog.render(frame)).unwrap();
        terminal.backend().buffer().clone()
    }

    fn text(buffer: &Buffer) -> String {
        buffer.content.iter().map(|cell| cell.symbol()).collect()
    }

    fn preview_matches(catalog: &Catalog, buffer: &Buffer) -> bool {
        let Some(area) = catalog.preview_content_rect(buffer.area) else {
            return false;
        };
        let output = catalog
            .selected_entry()
            .unwrap()
            .kind
            .output(catalog.scenario, usize::from(area.width))
            .unwrap();
        if output.lines.len() != usize::from(area.height) {
            return false;
        }
        output.lines.iter().enumerate().all(|(y, line)| {
            let rendered: String = (area.x..area.right())
                .map(|x| buffer[(x, area.y + u16::try_from(y).unwrap())].symbol())
                .collect();
            rendered == line.text()
        })
    }

    #[test]
    fn catalog_renders_real_fixture_content() {
        let mut terminal = Terminal::new(TestBackend::new(120, 36)).unwrap();
        terminal
            .draw(|frame| Catalog::default().render(frame))
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(
            text.contains("tenacity"),
            "the real settings fixture must be visible, not just catalog chrome"
        );
    }

    // GUARD: tests::actual_host_preserves_widget_cells_across_fixtures_and_viewports
    #[test]
    fn actual_host_preserves_widget_cells_across_fixtures_and_viewports() {
        for entry in ENTRIES.iter().filter(|entry| entry.kind != Kind::Settings) {
            for scenario in Scenario::ALL {
                for theme in [Theme::Dark, Theme::Light] {
                    for (width, height, requested) in
                        [(120, 36, 48), (80, 28, 8), (54, 28, 1), (120, 36, 200)]
                    {
                        let catalog = Catalog::new(Options {
                            item: entry.kind,
                            scenario,
                            theme,
                            width: requested,
                            ..Options::default()
                        });
                        let buffer = render(&catalog, width, height);
                        assert!(preview_matches(&catalog, &buffer), "{} {scenario:?} {theme:?} {width}x{height}/{requested} loses widget cells", entry.id);
                        if catalog.preview_content_rect(buffer.area).unwrap().width < requested {
                            assert!(
                                text(&buffer).contains("· limited"),
                                "the width notice must fit too"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn preview_guard_rejects_blank_content_even_when_catalog_chrome_survives() {
        let catalog = Catalog::new(Options {
            item: Kind::HeatMeter,
            ..Options::default()
        });
        let mut buffer = render(&catalog, 120, 36);
        assert!(preview_matches(&catalog, &buffer));
        let area = catalog.preview_content_rect(buffer.area).unwrap();
        for y in area.y..area.bottom() {
            for x in area.x..area.right() {
                buffer[(x, y)].set_symbol(" ");
            }
        }
        assert!(text(&buffer).contains("LIVE PREVIEW"));
        assert!(
            !preview_matches(&catalog, &buffer),
            "a blank chart must fail despite intact chrome"
        );
    }

    #[test]
    fn themes_change_real_widget_styles_without_changing_content() {
        let dark = Catalog::new(Options {
            item: Kind::HeatMeter,
            ..Options::default()
        });
        let light = Catalog::new(Options {
            item: Kind::HeatMeter,
            theme: Theme::Light,
            ..Options::default()
        });
        let dark_buffer = render(&dark, 120, 36);
        let light_buffer = render(&light, 120, 36);
        let area = dark.preview_content_rect(dark_buffer.area).unwrap();
        assert!(preview_matches(&dark, &dark_buffer) && preview_matches(&light, &light_buffer));
        assert_ne!(
            dark_buffer[(area.x, area.y)].fg,
            light_buffer[(area.x, area.y)].fg
        );
    }

    #[test]
    fn catalog_escape_and_component_escape_have_separate_meanings() {
        let mut catalog = Catalog::default();
        press(&mut catalog, KeyCode::Enter);
        press(&mut catalog, KeyCode::Right);
        assert_eq!(catalog.panel.view().rows[0].value, "steady");
        assert!(!press(&mut catalog, KeyCode::Esc));
        assert!(catalog.focused);
        assert!(catalog.panel.intent().is_none());
        assert!(catalog.outcome.as_deref().unwrap().contains("Cancelled"));
        press(&mut catalog, KeyCode::Enter);
        assert!(
            catalog.panel.intent().is_none(),
            "a closed component is not driven again"
        );
        press(&mut catalog, KeyCode::F(4));
        assert!(catalog.outcome.is_none());
        assert_eq!(catalog.panel.view().rows[0].value, "auto");
        press(&mut catalog, KeyCode::F(1));
        assert!(!catalog.focused);
        assert!(press(&mut catalog, KeyCode::Esc));
    }

    #[test]
    fn returning_to_catalog_never_accepts_or_cancels_the_component() {
        let mut catalog = Catalog::default();
        press(&mut catalog, KeyCode::Enter);
        press(&mut catalog, KeyCode::Right);
        press(&mut catalog, KeyCode::F(1));
        assert!(catalog.outcome.is_none());
        assert!(catalog.panel.intent().is_none());
        press(&mut catalog, KeyCode::Enter);
        assert_eq!(catalog.panel.view().rows[0].value, "steady");
        press(&mut catalog, KeyCode::Enter);
        assert!(catalog.panel.intent().is_some());
        assert!(catalog.outcome.as_deref().unwrap().contains("Accepted"));
        assert!(text(&render(&catalog, 120, 36)).contains("Accepted"));
    }

    #[test]
    fn filtering_is_case_insensitive_and_no_results_cannot_enter_stale_preview() {
        let mut catalog = Catalog::default();
        press(&mut catalog, KeyCode::Char('/'));
        assert!(text(&render(&catalog, 120, 36)).contains("SEARCH"));
        for ch in "HEAT".chars() {
            press(&mut catalog, KeyCode::Char(ch));
        }
        assert_eq!(catalog.visible().len(), 2);
        press(&mut catalog, KeyCode::Enter);
        press(&mut catalog, KeyCode::Down);
        assert_eq!(catalog.selected_entry().unwrap().kind, Kind::HeatMeter);
        press(&mut catalog, KeyCode::Up);
        assert_eq!(catalog.selected_entry().unwrap().kind, Kind::Sparkline);
        press(&mut catalog, KeyCode::Char('/'));
        press(&mut catalog, KeyCode::Char('z'));
        assert!(catalog.selected_entry().is_none());
        press(&mut catalog, KeyCode::Down);
        press(&mut catalog, KeyCode::Enter);
        press(&mut catalog, KeyCode::Down);
        press(&mut catalog, KeyCode::Enter);
        assert!(!catalog.focused);
        assert!(text(&render(&catalog, 120, 36)).contains("No matching pieces"));
        press(&mut catalog, KeyCode::Char('/'));
        press(&mut catalog, KeyCode::Backspace);
        assert!(catalog.selected_entry().is_some());
        press(&mut catalog, KeyCode::Esc);
        assert_eq!(catalog.visible().len(), ENTRIES.len());
    }

    #[test]
    fn host_controls_reset_fixture_resize_preview_and_switch_palettes() {
        let mut catalog = Catalog::default();
        for _ in 0..30 {
            press(&mut catalog, KeyCode::Down);
        }
        assert_eq!(catalog.selected_entry().unwrap().kind, Kind::CoreGrid);
        for _ in 0..60 {
            press(&mut catalog, KeyCode::Left);
        }
        assert_eq!(catalog.width, 1);
        for _ in 0..60 {
            press(&mut catalog, KeyCode::Right);
        }
        assert_eq!(catalog.width, 200);
        press(&mut catalog, KeyCode::Char('f'));
        assert_eq!((catalog.scenario, catalog.width), (Scenario::Narrow, 8));
        press(&mut catalog, KeyCode::Char('t'));
        assert_eq!(catalog.theme, Theme::Light);
        press(&mut catalog, KeyCode::Char('r'));
        press(&mut catalog, KeyCode::Enter);
        press(&mut catalog, KeyCode::Left);
        press(&mut catalog, KeyCode::Right);
        press(&mut catalog, KeyCode::F(2));
        press(&mut catalog, KeyCode::F(3));
        press(&mut catalog, KeyCode::Char('x'));
        assert_eq!(
            (catalog.scenario, catalog.theme),
            (Scenario::Empty, Theme::Dark)
        );
        assert!(!press(&mut catalog, KeyCode::Esc));
        assert!(!catalog.focused);
        press(&mut catalog, KeyCode::F(9));
        assert!(press(&mut catalog, KeyCode::Char('q')));
    }

    #[test]
    fn long_settings_follow_selection_and_tiny_terminals_report_their_limit() {
        let mut catalog = Catalog::new(Options {
            scenario: Scenario::Long,
            ..Options::default()
        });
        press(&mut catalog, KeyCode::Enter);
        for _ in 0..10 {
            press(&mut catalog, KeyCode::Down);
        }
        let buffer = render(&catalog, 120, 36);
        assert!(text(&buffer).contains("setting number 10"));
        assert!(text(&buffer).contains("rows 9–11 / 14"));
        for (width, height) in [(0, 0), (1, 1), (8, 4), (53, 27), (120, 19)] {
            let buffer = render(&catalog, width, height);
            assert!(catalog.preview_content_rect(buffer.area).is_none());
            if width == 53 {
                assert!(text(&buffer).contains("Resize to at least"));
            }
        }
        for scenario in [Scenario::Empty, Scenario::Error, Scenario::Narrow] {
            let catalog = Catalog::new(Options {
                scenario,
                width: 8,
                ..Options::default()
            });
            assert!(text(&render(&catalog, 54, 28)).contains("text clipped"));
        }
    }

    #[test]
    fn component_key_alphabet_reaches_host_decoder_without_global_letter_shortcuts() {
        let mut catalog = Catalog::default();
        press(&mut catalog, KeyCode::Enter);
        for code in [
            KeyCode::Up,
            KeyCode::Down,
            KeyCode::Left,
            KeyCode::Right,
            KeyCode::Tab,
            KeyCode::BackTab,
            KeyCode::Home,
            KeyCode::End,
            KeyCode::PageUp,
            KeyCode::PageDown,
            KeyCode::Backspace,
            KeyCode::Char('q'),
            KeyCode::F(10),
        ] {
            assert!(!press(&mut catalog, code));
        }
        assert!(!catalog.handle(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL)));
        assert!(catalog.handle(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)));
    }

    #[test]
    fn modified_browse_letters_never_alias_plain_catalog_actions() {
        let mut catalog = Catalog::default();
        press(&mut catalog, KeyCode::Enter);
        press(&mut catalog, KeyCode::Right);
        press(&mut catalog, KeyCode::F(1));
        let opening_view = catalog.panel.view();
        for modifiers in [
            KeyModifiers::CONTROL,
            KeyModifiers::ALT,
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        ] {
            for character in ['q', 'r', 'f', 't', 'j', 'k', '/'] {
                assert!(!catalog.handle(KeyEvent::new(KeyCode::Char(character), modifiers)));
                assert_eq!(catalog.selected, 0);
                assert_eq!(catalog.scenario, Scenario::Normal);
                assert_eq!(catalog.theme, Theme::Dark);
                assert!(!catalog.searching);
                assert_eq!(catalog.panel.view(), opening_view);
            }
        }
    }
}
