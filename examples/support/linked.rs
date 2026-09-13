//! The host owns source text and BSP drawing; the component owns navigation.

use super::Scenario;
use newtui::components::linked_panes::{
    Correspondence, CorrespondenceError, LinkMode, LinkedPanes, MappedLocation, MatchKind,
    PaneSide, Region,
};
use newtui::layout::{Direction, LayoutTree, Rect};
use newtui::{Component, Flow, Key, Run, Tone, WidgetLine, WidgetOutput};

#[derive(Clone, Debug)]
pub struct LinkedPreview {
    source: [Vec<String>; 2],
    component: Result<LinkedPanes, CorrespondenceError>,
    layout: LayoutTree,
    cancelled: bool,
}

impl Default for LinkedPreview {
    fn default() -> Self {
        Self::new(Scenario::Normal)
    }
}

impl LinkedPreview {
    pub fn new(scenario: Scenario) -> Self {
        let mut source = [
            vec![
                "fn render(status: &str) {",
                "    let width = 80;",
                "    draw_plain(status);",
                "    cache.flush();",
                "    metrics.tick();",
                "    log_status(status);",
                "    obsolete_banner();",
                "    legacy_footer();",
                "    // unchanged gap",
                "    finish();",
                "}",
            ]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>(),
            vec![
                "fn render(status: &str) {",
                "    let width = viewport.width();",
                "    let theme = theme::current();",
                "    let badge = \"ready\";",
                "    draw_changes(status, width);",
                "    cache.flush();",
                "    metrics.tick();",
                "    log_status(status);",
                "    // unchanged gap",
                "    finish();",
                "}",
            ]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>(),
        ];
        let mut regions = vec![
            Region {
                first: 0..1,
                second: 0..1,
            },
            Region {
                first: 1..3,
                second: 1..5,
            },
            Region {
                first: 3..6,
                second: 5..8,
            },
            Region {
                first: 6..8,
                second: 8..8,
            },
            Region {
                first: 9..11,
                second: 9..11,
            },
        ];
        match scenario {
            Scenario::Empty => {
                source = [vec![], vec![]];
                regions.clear();
            }
            Scenario::Error => {
                // The error is produced by the actual constructor. No fake state
                // replaces the rejected crossing correspondence.
                regions.push(Region {
                    first: 0..1,
                    second: 0..1,
                });
            }
            Scenario::Long => {
                for row in 11..51 {
                    source[0].push(format!("// old source row {row}: a deliberately long host-owned ASCII line that remains available beyond the viewport"));
                    source[1].push(format!("// new preview row {row}: a deliberately long host-owned ASCII line with a corresponding numeric location"));
                }
                regions.push(Region {
                    first: 11..51,
                    second: 11..51,
                });
            }
            Scenario::Normal | Scenario::Narrow => {}
        }
        let component = Correspondence::new([source[0].len(), source[1].len()], regions)
            .map(|map| LinkedPanes::new(map, [0, 0]));
        let mut layout = LayoutTree::single(0);
        assert!(layout.split(0, 1, Direction::Horizontal, 0.5));
        Self {
            source,
            component,
            layout,
            cancelled: false,
        }
    }

    pub fn component(&self) -> Option<&LinkedPanes> {
        self.component.as_ref().ok()
    }

    pub fn error(&self) -> Option<&CorrespondenceError> {
        self.component.as_ref().err()
    }

    pub fn source(&self, side: PaneSide) -> &[String] {
        &self.source[index(side)]
    }

    pub fn panes(&self, width: u16, height: u16) -> Vec<(usize, Rect)> {
        self.layout.rects(Rect {
            x: 0,
            y: 0,
            width,
            height,
        })
    }

    /// Update the actual component before input: page movement uses these rows.
    pub fn resize(&mut self, width: u16, height: u16) {
        let mut heights = [0; 2];
        for (id, pane) in self.panes(width, height) {
            heights[id] = if pane.width == 0 {
                0
            } else {
                usize::from(pane.height.saturating_sub(u16::from(pane.height > 1)))
            };
        }
        if let Ok(component) = &mut self.component {
            component.resize(heights);
        }
    }

    pub fn handle(&mut self, key: Key) -> Flow {
        if self.cancelled {
            return Flow::Stay;
        }
        let flow = match &mut self.component {
            Ok(component) => component.handle(key),
            Err(_) if key == Key::Esc => Flow::Close(false),
            Err(_) => Flow::Stay,
        };
        if flow == Flow::Close(false) {
            self.cancelled = true;
        }
        flow
    }

    pub fn mode_name(&self) -> &'static str {
        match self.component().map(LinkedPanes::mode) {
            Some(LinkMode::Locked) => "Locked",
            Some(LinkMode::Proportional) => "Proportional",
            Some(LinkMode::Unlinked) => "Unlinked",
            None => "rejected map",
        }
    }

    /// Complete short status is outside the panes, even at zero/one cell.
    /// Displayed row numbers are one-based; anchors are before that row or EOF.
    pub fn status(&self) -> String {
        if self.cancelled {
            return "Esc cancelled\nNo host effect requested\nr resets the fixture".into();
        }
        let component = match &self.component {
            Ok(component) => component,
            Err(error) => {
                let reason = match error {
                    CorrespondenceError::Range { region, side } => {
                        format!("{} #{} invalid range", name(*side), region + 1)
                    }
                    CorrespondenceError::EmptyRegion { region } => {
                        format!("#{} both ranges empty", region + 1)
                    }
                    CorrespondenceError::Order { region, side } => {
                        format!("{} #{} order rejected", name(*side), region + 1)
                    }
                    _ => "unknown validation error".into(),
                };
                return format!("Rejected correspondence\n{reason}\nSource kept; no selection");
            }
        };
        let position = |side| {
            let p = component.position(side);
            p.cursor.map_or_else(
                || "empty".into(),
                |row| format!("{}@{}", row + 1, p.offset + 1),
            )
        };
        let relation = component.relation().map_or_else(
            || "No linked counterpart".into(),
            |relation| {
                let target = relation.source.other();
                let location = match relation.mapping.target {
                    MappedLocation::Row(row) => format!("row {}", row + 1),
                    MappedLocation::Boundary(at)
                        if at == component.correspondence().len(target) =>
                    {
                        "anchor EOF".into()
                    }
                    MappedLocation::Boundary(at) => format!("anchor {}", at + 1),
                };
                let kind = match relation.mapping.kind {
                    MatchKind::Region => format!("#{}", relation.mapping.region.unwrap() + 1),
                    MatchKind::Gap => format!("gap #{}", relation.mapping.region.unwrap() + 1),
                    MatchKind::Proportional => "proportional".into(),
                    MatchKind::NoCorrespondence => "fallback".into(),
                };
                format!(
                    "{}->{} {kind} {location}",
                    name(relation.source),
                    name(target)
                )
            },
        );
        format!(
            "{} | focus {}\nold {} | new {}\n{relation}",
            self.mode_name(),
            name(component.focus()),
            position(PaneSide::First),
            position(PaneSide::Second)
        )
    }

    pub fn output(&self, width: usize, height: usize) -> WidgetOutput {
        let area = Rect {
            x: 0,
            y: 0,
            width: u16::try_from(width).expect("host width fits u16"),
            height: u16::try_from(height).expect("host height fits u16"),
        };
        let mut cells = vec![vec![(' ', Tone::Plain); width]; height];
        for (id, pane) in self.panes(area.width, area.height) {
            let side = if id == 0 {
                PaneSide::First
            } else {
                PaneSide::Second
            };
            let mut y = usize::from(pane.y);
            if pane.height > 1 {
                put(
                    &mut cells,
                    pane,
                    y,
                    &format!("{} / {} rows", name(side), self.source[id].len()),
                    Tone::Label,
                );
                y += 1;
            }
            let Some(component) = self.component() else {
                put(&mut cells, pane, y, "! rejected map", Tone::Critical);
                continue;
            };
            let position = component.position(side);
            if self.source[id].is_empty() {
                put(&mut cells, pane, y, "(empty)", Tone::Muted);
            }
            let mapped = component.relation().and_then(|relation| {
                if relation.source.other() != side {
                    return None;
                }
                match relation.mapping.target {
                    MappedLocation::Row(row) => Some(row),
                    MappedLocation::Boundary(_) => None,
                }
            });
            for (row, text) in self.source[id].iter().enumerate().skip(position.offset) {
                if y >= usize::from(pane.y) + usize::from(pane.height) {
                    break;
                }
                let selected = position.cursor == Some(row);
                let active = selected && component.focus() == side;
                let marker = if active {
                    '>'
                } else if selected {
                    '.'
                } else {
                    ' '
                };
                let matched = mapped == Some(row);
                let tone = if active {
                    Tone::Label
                } else if matched {
                    Tone::Accent
                } else {
                    Tone::Context
                };
                put(
                    &mut cells,
                    pane,
                    y,
                    &format!(
                        "{marker}{}{:>2} {text}",
                        if matched { '=' } else { ' ' },
                        row + 1
                    ),
                    tone,
                );
                y += 1;
            }
        }
        for border in self.layout.splits(area) {
            for row in &mut cells {
                if let Some(cell) = row.get_mut(usize::from(border.pos)) {
                    *cell = ('|', Tone::Accent);
                }
            }
        }
        WidgetOutput::new(
            cells
                .into_iter()
                .map(|row| {
                    let mut runs: Vec<Run> = Vec::new();
                    for (ch, tone) in row {
                        if let Some(run) = runs.last_mut().filter(|run| run.tone == tone) {
                            run.text.push(ch);
                        } else {
                            runs.push(Run::new(ch.to_string(), tone));
                        }
                    }
                    WidgetLine::new(runs)
                })
                .collect(),
        )
    }
}

fn index(side: PaneSide) -> usize {
    if side == PaneSide::First {
        0
    } else {
        1
    }
}
fn name(side: PaneSide) -> &'static str {
    if side == PaneSide::First {
        "old"
    } else {
        "new"
    }
}

fn put(cells: &mut [Vec<(char, Tone)>], pane: Rect, y: usize, text: &str, tone: Tone) {
    if y >= usize::from(pane.y) + usize::from(pane.height) {
        return;
    }
    if let Some(row) = cells.get_mut(y) {
        for (at, ch) in text.chars().take(usize::from(pane.width)).enumerate() {
            row[usize::from(pane.x) + at] = (ch, tone);
        }
    }
}
