//! Shared host fixture: real geometry places existing widget output.

use super::{Kind, Scenario};
use newtui::layout::{changed_panes, Direction, LayoutTree, PaneId, Rect, SplitBorder};
use newtui::{Key, Run, Tone, WidgetLine, WidgetOutput};

/// Only demo interaction state; the library's layout owns no keys or content.
#[derive(Clone, Debug, PartialEq)]
pub struct BspPreview {
    tree: LayoutTree,
    previous: Option<(LayoutTree, bool)>,
    selected: usize,
    shrunk: bool,
    rejected: bool,
}

impl Default for BspPreview {
    fn default() -> Self {
        let mut tree = LayoutTree::single(10);
        assert!(tree.split(10, 20, Direction::Horizontal, 0.8));
        assert!(tree.split(10, 30, Direction::Vertical, 0.5));
        Self {
            tree,
            previous: None,
            selected: 0,
            shrunk: false,
            rejected: false,
        }
    }
}

impl BspPreview {
    fn path(&self) -> &[bool] {
        if self.selected == 0 {
            &[]
        } else {
            &[false]
        }
    }

    pub fn ratio(&self) -> f32 {
        self.tree.splits(Rect::default())[self.selected].ratio
    }

    pub fn set_ratio(&mut self, ratio: f32) -> bool {
        self.previous = Some((self.tree.clone(), self.shrunk));
        let path = self.path().to_vec();
        let accepted = self.tree.set_ratio_at(&path, ratio);
        self.rejected = !accepted;
        accepted
    }

    pub fn handle(&mut self, key: Key) {
        match key {
            Key::Up => {
                self.set_ratio(self.ratio() + 0.1);
            }
            Key::Down => {
                self.set_ratio(self.ratio() - 0.1);
            }
            Key::Char('x') => {
                self.set_ratio(f32::NAN);
            }
            Key::Tab | Key::BackTab => {
                self.previous = Some((self.tree.clone(), self.shrunk));
                self.selected = 1 - self.selected;
                self.rejected = false;
            }
            Key::Char('s') => {
                self.previous = Some((self.tree.clone(), self.shrunk));
                self.shrunk = !self.shrunk;
                self.rejected = false;
            }
            Key::Home => *self = Self::default(),
            _ => {}
        }
    }

    pub fn size_name(&self) -> &'static str {
        if self.shrunk {
            "half width"
        } else {
            "full width"
        }
    }

    fn area(scenario: Scenario, width: u16, height: u16, shrunk: bool) -> Rect {
        if scenario == Scenario::Empty {
            Rect::default()
        } else {
            Rect {
                x: 0,
                y: 0,
                width: if shrunk { width / 2 } else { width },
                height,
            }
        }
    }

    pub fn panes(&self, scenario: Scenario, width: u16, height: u16) -> Vec<(PaneId, Rect)> {
        self.tree
            .rects(Self::area(scenario, width, height, self.shrunk))
    }

    pub fn borders(&self, scenario: Scenario, width: u16, height: u16) -> Vec<SplitBorder> {
        self.tree
            .splits(Self::area(scenario, width, height, self.shrunk))
    }

    /// Compare the last demo edit at the current size, without inspecting content.
    pub fn changed(&self, scenario: Scenario, width: u16, height: u16) -> Vec<PaneId> {
        self.previous
            .as_ref()
            .map_or_else(Vec::new, |(before, shrunk)| {
                changed_panes(
                    &before.rects(Self::area(scenario, width, height, *shrunk)),
                    &self.panes(scenario, width, height),
                )
            })
    }

    /// Complete geometry status stays outside the preview, including at one cell.
    pub fn status(&self, scenario: Scenario, width: u16, height: u16) -> String {
        let borders = self.tree.splits(Rect::default());
        let changed = self.changed(scenario, width, height);
        let affected = if changed.is_empty() {
            "none".to_string()
        } else {
            changed
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        };
        let state = if scenario == Scenario::Error || self.rejected {
            let mut probe = self.tree.clone();
            if probe.set_ratio_at(self.path(), f32::NAN) {
                "ERROR: NaN accepted".to_string()
            } else {
                "NaN rejected; unchanged".to_string()
            }
        } else if scenario == Scenario::Empty {
            "empty area; 3 pane IDs".to_string()
        } else {
            let area = Self::area(scenario, width, height, self.shrunk);
            format!(
                "{}x{}; s {}",
                area.width,
                area.height,
                if self.shrunk { "restore" } else { "shrink" }
            )
        };
        format!(
            "root{} {:.0}% / first{} {:.0}%\nedit affects {affected}\n{state}",
            if self.selected == 0 { "*" } else { "" },
            borders[0].ratio * 100.0,
            if self.selected == 1 { "*" } else { "" },
            borders[1].ratio * 100.0
        )
    }

    /// Compose the existing builders inside the exact library pane rectangles.
    pub fn output(&self, scenario: Scenario, width: usize, height: usize) -> WidgetOutput {
        let columns = u16::try_from(width).expect("catalog width fits terminal coordinates");
        let rows = u16::try_from(height).expect("catalog height fits terminal coordinates");
        let area = Self::area(scenario, columns, rows, self.shrunk);
        let mut cells = vec![vec![(' ', Tone::Plain); width]; height];
        let sample = if scenario == Scenario::Error {
            Scenario::Normal
        } else {
            scenario
        };
        for (id, pane) in self.tree.rects(area) {
            if pane.width == 0 || pane.height == 0 {
                continue;
            }
            let (kind, label) = match id {
                10 => (Kind::Sparkline, "HEAT"),
                20 => (Kind::Gauge, "BUDGET"),
                _ => (Kind::Butterfly, "NETWORK"),
            };
            let output = kind
                .output(sample, usize::from(pane.width))
                .expect("a pane owns a real widget");
            // At one row, keep the widget's signal rather than spending it on a title.
            let header = usize::from(pane.height > 1);
            if header > 0 {
                let title = format!("{id} {label} {}x{}", pane.width, pane.height);
                for (offset, glyph) in title.chars().take(usize::from(pane.width)).enumerate() {
                    cells[usize::from(pane.y)][usize::from(pane.x) + offset] = (glyph, Tone::Label);
                }
            }
            for (row, line) in output
                .lines
                .iter()
                .take(usize::from(pane.height) - header)
                .enumerate()
            {
                let glyphs = line
                    .runs
                    .iter()
                    .flat_map(|run| run.text.chars().map(move |glyph| (glyph, run.tone)));
                for (column, cell) in glyphs.take(usize::from(pane.width)).enumerate() {
                    cells[usize::from(pane.y) + header + row][usize::from(pane.x) + column] = cell;
                }
            }
        }
        for (index, border) in self.tree.splits(area).iter().enumerate() {
            if border.area.width == 0 || border.area.height == 0 {
                continue;
            }
            let tone = if index == self.selected {
                Tone::Accent
            } else {
                Tone::Muted
            };
            match border.direction {
                Direction::Horizontal => {
                    for row in border.area.y..border.area.y + border.area.height {
                        cells[usize::from(row)][usize::from(border.pos)] = ('|', tone);
                    }
                }
                Direction::Vertical => {
                    for column in border.area.x..border.area.x + border.area.width {
                        cells[usize::from(border.pos)][usize::from(column)] = ('-', tone);
                    }
                }
            }
        }
        WidgetOutput::new(
            cells
                .into_iter()
                .map(|row| {
                    let mut runs: Vec<Run> = Vec::new();
                    for (glyph, tone) in row {
                        if let Some(last) = runs.last_mut().filter(|last| last.tone == tone) {
                            last.text.push(glyph);
                        } else {
                            runs.push(Run::new(glyph.to_string(), tone));
                        }
                    }
                    WidgetLine::new(runs)
                })
                .collect(),
        )
    }
}
