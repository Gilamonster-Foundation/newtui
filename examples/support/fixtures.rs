//! Deterministic, host-owned samples shared by the named demos and live catalog.

#[path = "bsp.rs"]
mod bsp;
pub use bsp::BspPreview;
#[path = "modal.rs"]
mod modal;
pub use modal::ModalPreview;
#[path = "linked.rs"]
mod linked;
pub use linked::LinkedPreview;
#[path = "stream.rs"]
mod stream;
// The named demos and catalog drive the clock; tests include this file too
// but only read samples, so the re-export is unused in that build.
#[allow(unused_imports)]
pub use stream::{DemoStream, TickClock};

use newtui::components::settings_panel::{Backend, Choice, Model, Setting, SettingsSeed};
use newtui::diff::{from_unified, ChangeSet, DiffLine};
use newtui::{
    bar, butterfly, core_grid, gauge, heat_meter, sparkline, ContextRun, CoreSeries, DiffData,
    DiffGeometry, Key, SparkDirection, WidgetOutput,
};

/// One shipped piece in the live catalog.
pub struct Entry {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub data: &'static str,
    pub kind: Kind,
}

/// Display builders, interactive components, and pure layout primitives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Settings,
    Sparkline,
    Butterfly,
    ButterflyHistory,
    HeatMeter,
    Gauge,
    Bar,
    CoreGrid,
    Diff,
    Bsp,
    Modal,
    LinkedPanes,
}

/// The registry intentionally names only shipped library exports.
pub const ENTRIES: &[Entry] = &[
    Entry {
        id: "settings_panel",
        name: "Settings panel",
        description: "A small state machine. Your vocabulary, your effects.",
        data: "Choices, numeric bounds, model availability and backend identity.",
        kind: Kind::Settings,
    },
    Entry {
        id: "sparkline",
        name: "Heat graph",
        description: "A history that keeps its shape, even when space gets tight.",
        data: "Ordered samples, a declared maximum and a growing edge.",
        kind: Kind::Sparkline,
    },
    Entry {
        id: "butterfly",
        name: "Butterfly meter",
        description: "Two directions. One scale. A steady center.",
        data: "Two rates, their labels and a shared maximum.",
        kind: Kind::Butterfly,
    },
    Entry {
        id: "butterfly_history",
        name: "Butterfly history",
        description: "Independent TX and RX wings. A long history around one center.",
        data:
            "Oldest-first TX/RX samples and a shared maximum. The host owns labels, rates and time.",
        kind: Kind::ButterflyHistory,
    },
    Entry {
        id: "heat_meter",
        name: "Heat meter",
        description: "A positional heat ramp with a label and a live value.",
        data: "A percentage plus host-formatted label and value.",
        kind: Kind::HeatMeter,
    },
    Entry {
        id: "gauge",
        name: "Budget gauge",
        description: "See the amount used and the room that remains.",
        data: "A current value, a limit and a caption.",
        kind: Kind::Gauge,
    },
    Entry {
        id: "bar",
        name: "Labeled bar",
        description: "Compact signal. Your units and your wording.",
        data: "A value, a maximum and a host-formatted value label.",
        kind: Kind::Bar,
    },
    Entry {
        id: "core_grid",
        name: "Core histories",
        description: "Independent histories, held in a stable row order.",
        data: "Named series with current values, histories and maxima.",
        kind: Kind::CoreGrid,
    },
    Entry {
        id: "diff",
        name: "Code changes",
        description: "Read a change together, side by side, or as file counts.",
        data: "A parsed change, layout, source window and expanded context runs.",
        kind: Kind::Diff,
    },
    Entry {
        id: "bsp",
        name: "Panel layout",
        description: "Ratios hold their shape. Shrink, restore, and keep your layout.",
        data: "Pane IDs, split ratios and an area. Existing widgets supply the samples.",
        kind: Kind::Bsp,
    },
    Entry {
        id: "modal",
        name: "Modal height",
        description: "Grow and shrink from what is on screen. Zoom, then return to it.",
        data: "The height the host granted. The library answers with the next request.",
        kind: Kind::Modal,
    },
    Entry {
        id: "linked_panes",
        name: "Linked panes",
        description: "Follow a selected source row across unequal regions. Keep each cursor visible.",
        data: "Old/new ASCII text, row counts, monotone ranges and BSP rectangles. Navigation stays in the component.",
        kind: Kind::LinkedPanes,
    },
];

/// Bounded, reproducible input domains; no clocks or metric collectors.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Scenario {
    #[default]
    Normal,
    Narrow,
    Empty,
    Error,
    Long,
}

impl Scenario {
    pub const ALL: [Self; 5] = [
        Self::Normal,
        Self::Narrow,
        Self::Empty,
        Self::Error,
        Self::Long,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Narrow => "narrow",
            Self::Empty => "empty",
            Self::Error => "error",
            Self::Long => "long",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|scenario| scenario.name() == name)
    }

    pub fn next(self) -> Self {
        Self::ALL[(Self::ALL
            .iter()
            .position(|value| *value == self)
            .unwrap_or(0)
            + 1)
            % Self::ALL.len()]
    }

    pub fn note(self, kind: Kind) -> &'static str {
        match (self, kind) {
            (Self::Empty, Kind::LinkedPanes) => "Both sources are empty; neither pane invents a cursor.",
            (Self::Error, Kind::LinkedPanes) => "A crossing map is rejected by the real constructor; source remains available.",
            (_, Kind::LinkedPanes) => "> active cursor, . other cursor, = mapped row. An anchor is a boundary, never a row. @ gives the first visible row.",
            (Self::Empty | Self::Error, Kind::Modal) => {
                "A zero-row request is raised to MIN_ROWS: border, one row, hint, border."
            }
            (Self::Long, Kind::Modal) => {
                "A request taller than the screen; the host grants what fits."
            }
            (_, Kind::Modal) => "Enter to explore. Shift-↑↓ or +/- height; z zoom/restore.",
            (Self::Empty, Kind::Bsp) => "Empty geometry retains every pane ID and split path.",
            (Self::Error, Kind::Bsp) => {
                "A nonfinite ratio edit is rejected without changing the layout."
            }
            (_, Kind::Bsp) => {
                "Enter to explore. Tab divider; up/down ratio; s shrink/restore; x reject NaN."
            }
            (Self::Empty, Kind::Diff) => "An empty change set. No invented source lines.",
            (Self::Error, Kind::Diff) => "A binary-only change has no textual hunks to display.",
            (Self::Long, Kind::Diff) => {
                "Long source and Unicode exercise clipping and reported substitutions."
            }
            (Self::Normal | Self::Narrow, Kind::Diff) => {
                "Enter to explore. g layout, e context, n notice; Home resets scrolling."
            }
            (Self::Empty, Kind::Settings) => {
                "No setting rows supplied; model and backend remain host context."
            }
            (Self::Error, Kind::Settings) => {
                "Backend cannot list models. The active model stays visible and fixed."
            }
            (Self::Long, Kind::Settings) => {
                "Long host labels and values; preview scrolls to keep selection visible."
            }
            (Self::Normal | Self::Narrow, Kind::Settings) => {
                "Try the dial, then the backend door. Esc cancels without an intent."
            }
            (Self::Empty, _) => {
                "No history / zero current signal; the requested rectangle remains defined."
            }
            (Self::Error, _) => {
                "Non-finite samples and invalid maxima become empty signal without panicking."
            }
            (Self::Long, _) => {
                "Long histories or labels exercise clipping inside the requested rectangle."
            }
            (Self::Narrow, _) => {
                "A constrained content area. Labels yield to signal as the viewport contracts."
            }
            (Self::Normal, _) => {
                "Fixed sample data from the named demo. No live services required."
            }
        }
    }
}

impl Kind {
    pub fn from_name(name: &str) -> Option<Self> {
        let name = if name == "settings" {
            "settings_panel"
        } else {
            name
        };
        ENTRIES
            .iter()
            .find(|entry| entry.id == name)
            .map(|entry| entry.kind)
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Settings => "settings",
            Self::Sparkline | Self::Bar => "request latency",
            Self::Butterfly => "network tx | rx",
            Self::ButterflyHistory => "TX / RX history",
            Self::HeatMeter => "disk temperature",
            Self::Gauge => "daily budget",
            Self::CoreGrid => "cpu cores",
            Self::Diff => "code changes",
            Self::Bsp => "panel layout",
            Self::Modal => "modal size",
            Self::LinkedPanes => "linked panes",
        }
    }

    pub fn demo_widths(self) -> &'static [usize] {
        match self {
            Self::Modal => &[48, 24, 12, 1],
            Self::Diff
            | Self::Bsp
            | Self::LinkedPanes
            | Self::Butterfly
            | Self::ButterflyHistory => &[88, 44, 12, 1],
            Self::Sparkline | Self::CoreGrid => &[72, 36, 12, 1],
            Self::HeatMeter | Self::Bar => &[24, 10, 4],
            Self::Gauge => &[24, 12, 4],
            _ => &[24, 12, 6],
        }
    }

    /// Call the shipped builder; this host does not implement chart logic.
    pub fn output(self, scenario: Scenario, width: usize) -> Option<WidgetOutput> {
        let maximum = if scenario == Scenario::Error {
            f64::NAN
        } else {
            100.0
        };
        let current = if scenario == Scenario::Empty {
            0.0
        } else if scenario == Scenario::Error {
            f64::INFINITY
        } else {
            83.0
        };
        let samples: &[f64] = match scenario {
            Scenario::Empty => &[],
            Scenario::Error => &[f64::NAN, f64::INFINITY, f64::NEG_INFINITY],
            Scenario::Long => &[
                10.0, 35.0, 80.0, 20.0, 100.0, 45.0, 70.0, 10.0, 35.0, 80.0, 20.0, 100.0, 45.0,
                70.0, 24.0, 50.0, 99.0, 36.0, 12.0, 18.0, 48.0, 80.0, 63.0, 92.0, 44.0, 57.0, 90.0,
                64.0, 34.0, 16.0, 30.0, 88.0,
            ],
            _ => &[10.0, 35.0, 80.0, 20.0, 100.0, 45.0, 70.0],
        };
        let label = |ordinary| {
            if scenario == Scenario::Long {
                "a deliberately long host supplied metric label"
            } else {
                ordinary
            }
        };
        Some(match self {
            Self::Settings => return None,
            Self::ButterflyHistory => {
                let history = DemoStream::default().histories();
                let (tx, rx): (&[f64], &[f64]) = match scenario {
                    Scenario::Empty => (&[], &[]),
                    Scenario::Error => (&[f64::NAN, f64::INFINITY], &[f64::NEG_INFINITY, f64::NAN]),
                    _ => (&history[0], &history[1]),
                };
                newtui::butterfly_history(tx, rx, maximum, width, 20)
            }
            Self::Diff => DiffPreview::default().output(scenario, width, 16),
            Self::Bsp => BspPreview::default().output(scenario, width, 16),
            Self::Modal => {
                ModalPreview::new(scenario).output(width, usize::from(modal::SCREEN_ROWS))
            }
            Self::LinkedPanes => {
                let mut preview = LinkedPreview::new(scenario);
                preview.resize(u16::try_from(width).expect("host width fits u16"), 16);
                preview.output(width, 16)
            }
            Self::Sparkline => sparkline(samples, maximum, width, 4, SparkDirection::Up),
            Self::Butterfly => butterfly(
                if scenario == Scenario::Normal
                    || scenario == Scenario::Narrow
                    || scenario == Scenario::Long
                {
                    28.0
                } else {
                    current
                },
                if scenario == Scenario::Normal
                    || scenario == Scenario::Narrow
                    || scenario == Scenario::Long
                {
                    74.0
                } else {
                    current
                },
                maximum,
                label("TX"),
                "RX",
                width,
                1,
            ),
            Self::HeatMeter => heat_meter(
                label("disk temperature"),
                if matches!(
                    scenario,
                    Scenario::Normal | Scenario::Narrow | Scenario::Long
                ) {
                    72.0
                } else {
                    current
                },
                if scenario == Scenario::Empty {
                    "0%"
                } else if scenario == Scenario::Error {
                    "unavailable"
                } else {
                    "72%"
                },
                width,
                1,
            ),
            Self::Gauge => gauge(
                label("daily budget"),
                if matches!(
                    scenario,
                    Scenario::Normal | Scenario::Narrow | Scenario::Long
                ) {
                    7.5
                } else {
                    current
                },
                if scenario == Scenario::Error {
                    maximum
                } else {
                    10.0
                },
                width,
                1,
            ),
            Self::Bar => bar(
                label("request latency"),
                current,
                maximum,
                if scenario == Scenario::Empty {
                    "0 ms"
                } else if scenario == Scenario::Error {
                    "unavailable"
                } else {
                    "83 ms"
                },
                width,
                1,
            ),
            Self::CoreGrid => {
                let cores = [
                    CoreSeries {
                        label: label("0"),
                        current: if matches!(
                            scenario,
                            Scenario::Normal | Scenario::Narrow | Scenario::Long
                        ) {
                            85.0
                        } else {
                            current
                        },
                        history: if scenario == Scenario::Normal || scenario == Scenario::Narrow {
                            &[15.0, 30.0, 65.0, 40.0, 85.0]
                        } else {
                            samples
                        },
                        maximum,
                    },
                    CoreSeries {
                        label: "1",
                        current: if matches!(
                            scenario,
                            Scenario::Normal | Scenario::Narrow | Scenario::Long
                        ) {
                            20.0
                        } else {
                            current
                        },
                        history: if scenario == Scenario::Normal || scenario == Scenario::Narrow {
                            &[80.0, 60.0, 25.0, 45.0, 20.0]
                        } else {
                            samples
                        },
                        maximum,
                    },
                ];
                core_grid(
                    if scenario == Scenario::Empty {
                        &[]
                    } else {
                        &cores
                    },
                    width,
                    4,
                )
            }
        })
    }
}

impl Kind {
    pub fn supports_stream(self) -> bool {
        matches!(
            self,
            Self::Sparkline
                | Self::Butterfly
                | Self::ButterflyHistory
                | Self::HeatMeter
                | Self::Gauge
                | Self::Bar
                | Self::CoreGrid
                | Self::Bsp
        )
    }

    /// Pure projection of a host tick; neither rendering nor keys advance it.
    pub fn stream_output(
        self,
        scenario: Scenario,
        stream: &DemoStream,
        width: usize,
        height: usize,
    ) -> WidgetOutput {
        let history = stream.histories();
        let (mut tx, mut rx) = stream.rates();
        let empty = scenario == Scenario::Empty;
        let invalid = scenario == Scenario::Error;
        if empty {
            tx = 0.0;
            rx = 0.0;
        }
        if invalid {
            tx = f64::NAN;
            rx = f64::INFINITY;
        }
        let tx_history: &[f64] = if empty {
            &[]
        } else if invalid {
            &[f64::NAN]
        } else {
            &history[0]
        };
        let rx_history: &[f64] = if empty {
            &[]
        } else if invalid {
            &[f64::INFINITY]
        } else {
            &history[1]
        };
        match self {
            Self::Sparkline => sparkline(tx_history, 100.0, width, height, SparkDirection::Up),
            Self::Butterfly => butterfly(tx, rx, 100.0, "TX", "RX", width, height),
            Self::ButterflyHistory => {
                newtui::butterfly_history(tx_history, rx_history, 100.0, width, height)
            }
            Self::HeatMeter => heat_meter("utilization", tx, &format!("{tx:.0}%"), width, height),
            Self::Gauge => gauge("quota", tx / 10.0, 10.0, width, height),
            Self::Bar => bar("latency", tx, 100.0, &format!("{tx:.0} ms"), width, height),
            Self::CoreGrid => {
                let histories = stream.core_histories();
                let labels: Vec<_> = (0..histories.len())
                    .map(|core| format!("{core:02}"))
                    .collect();
                let cores: Vec<_> = histories
                    .iter()
                    .zip(&labels)
                    .map(|(history, label)| CoreSeries {
                        label,
                        current: if invalid {
                            f64::NAN
                        } else {
                            *history.last().unwrap()
                        },
                        history: if invalid { &[f64::NAN] } else { history },
                        maximum: 100.0,
                    })
                    .collect();
                core_grid(if empty { &[] } else { &cores }, width, height)
            }
            _ => self
                .output(scenario, width)
                .expect("only display pieces have stream output"),
        }
    }

    pub fn stream_caption(self, scenario: Scenario, stream: &DemoStream, animated: bool) -> String {
        if matches!(scenario, Scenario::Empty | Scenario::Error) {
            return format!("SYNTHETIC {} fixture / no live samples", scenario.name());
        }
        let (tx, rx) = stream.rates();
        let reading = match self {
            Self::Butterfly | Self::ButterflyHistory => format!("TX {tx:.0} / RX {rx:.0} MiB/s"),
            Self::Sparkline | Self::Bar => format!("current {tx:.0} ms"),
            Self::Gauge => format!("used {:.1} / 10", tx / 10.0),
            Self::CoreGrid => format!(
                "12 cores / peak {:.0}%",
                stream
                    .core_histories()
                    .iter()
                    .map(|history| *history.last().unwrap())
                    .fold(0.0_f64, f64::max)
            ),
            Self::Bsp => format!("pane samples {tx:.0}% / {rx:.0}%"),
            _ => format!("current {tx:.0}%"),
        };
        format!(
            "SYNTHETIC {} / tick {}\n{reading}\n250 ms ticks / Space pause / . step",
            if !animated {
                "STILL"
            } else if stream.paused() {
                "PAUSED"
            } else {
                "LIVE"
            },
            stream.tick()
        )
    }
}

/// Presentation choices belong to the demo host; the widget stays stateless.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiffPreview {
    pub geometry: DiffGeometry,
    pub row_offset: usize,
    pub column_offset: usize,
    pub expanded: bool,
}

impl DiffPreview {
    pub fn geometry_name(self) -> &'static str {
        match self.geometry {
            DiffGeometry::Unified => "unified",
            DiffGeometry::Split => "split",
            DiffGeometry::Stat => "stat",
            _ => "other",
        }
    }

    /// Hosts map shifted horizontal arrows here and keep plain arrows for size.
    pub fn handle(&mut self, key: Key) {
        match key {
            Key::Up => self.row_offset = self.row_offset.saturating_sub(1),
            Key::Down => self.row_offset = self.row_offset.saturating_add(1),
            Key::PageUp => self.row_offset = self.row_offset.saturating_sub(8),
            Key::PageDown => self.row_offset = self.row_offset.saturating_add(8),
            Key::Left => self.column_offset = self.column_offset.saturating_sub(4),
            Key::Right => self.column_offset = self.column_offset.saturating_add(4),
            Key::Home => {
                self.row_offset = 0;
                self.column_offset = 0;
            }
            Key::Char('g') => {
                self.geometry = match self.geometry {
                    DiffGeometry::Unified => DiffGeometry::Split,
                    DiffGeometry::Split => DiffGeometry::Stat,
                    _ => DiffGeometry::Unified,
                };
                self.row_offset = 0;
                self.column_offset = 0;
            }
            Key::Char('e') => self.expanded = !self.expanded,
            _ => {}
        }
    }

    pub fn output(self, scenario: Scenario, width: usize, height: usize) -> WidgetOutput {
        let changes = diff_fixture(scenario);
        let mut expanded = Vec::new();
        if self.expanded {
            for (file, change) in changes.files().iter().enumerate() {
                for (hunk, section) in change.hunks().iter().enumerate() {
                    for (line, source) in section.lines().iter().enumerate() {
                        if matches!(source, DiffLine::Context(_))
                            && !matches!(
                                line.checked_sub(1).and_then(|at| section.lines().get(at)),
                                Some(DiffLine::Context(_))
                            )
                        {
                            expanded.push(ContextRun { file, hunk, line });
                        }
                    }
                }
            }
        }
        newtui::diff(
            DiffData::new(&changes)
                .geometry(self.geometry)
                .row_offset(self.row_offset)
                .column_offset(self.column_offset)
                .context(1)
                .expanded(&expanded),
            width,
            height,
        )
    }
}

pub fn diff_fixture(scenario: Scenario) -> ChangeSet {
    const NORMAL: &str = r#"diff --git a/src/session.rs b/src/session.rs
--- a/src/session.rs
+++ b/src/session.rs
@@ -1,14 +1,15 @@
 use crate::Session;
 // Render the current session.
 fn render(status: &str) {
-    let width = 80;
+    let width = viewport.width();
+    let badge = "ready";
     let title = "Session";
     let theme = theme::current();
     let border = theme.border();
     let padding = 1;
     let context = 3;
     let mut rows = Vec::new();
     rows.reserve(32);
     rows.push(title);
-    draw_plain(status);
+    draw_changes(status, width);
 }
"#;
    let source = match scenario {
        Scenario::Empty => String::new(),
        Scenario::Error => "diff --git a/assets/logo.bin b/assets/logo.bin\nBinary files a/assets/logo.bin and b/assets/logo.bin differ\n".to_string(),
        Scenario::Long => format!(
            "{NORMAL}diff --git a/docs/status.md b/docs/status.md\n--- /dev/null\n+++ b/docs/status.md\n@@ -0,0 +1,2 @@\n+# Status\n+Ready – café 🦎: this deliberately long line keeps its original bytes in the model while the viewport reports substitutions and clipping.\n"
        ),
        Scenario::Normal | Scenario::Narrow => NORMAL.to_string(),
    };
    from_unified(&source).expect("the checked-in diff fixture parses")
}

/// A host caption makes diagnostics available even outside a one-cell widget.
pub fn notice_text(output: &WidgetOutput, index: usize) -> Option<String> {
    if output.notices.is_empty() {
        return None;
    }
    let at = index % output.notices.len();
    Some(format!(
        "Notice {}/{}: {} · n next",
        at + 1,
        output.notices.len(),
        output.notices[at].message()
    ))
}

pub fn settings_seed(scenario: Scenario) -> SettingsSeed {
    let settings = match scenario {
        Scenario::Empty => vec![],
        Scenario::Long => (0..12)
            .map(|index| {
                Setting::choice(
                    format!("field-{index}"),
                    format!("host supplied setting number {index:02}"),
                    "auto",
                    vec![
                        Choice::new(
                            "auto",
                            "a long description supplied by the host application",
                        ),
                        Choice::new("steady", "keep trying"),
                    ],
                )
            })
            .collect(),
        _ => vec![Setting::choice(
            "tenacity",
            "tenacity",
            "auto",
            vec![
                Choice::new("auto", "inherit host policy"),
                Choice::new("steady", "keep trying"),
            ],
        )],
    };
    let (model, backend) = if scenario == Scenario::Error {
        ("last-known-model", "offline")
    } else {
        ("qwen-local", "local")
    };
    SettingsSeed::new(
        settings,
        Model::new(model, None),
        Backend::new(Some(backend)),
    )
}
