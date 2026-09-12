//! Deterministic, host-owned samples shared by the named demos and live catalog.

use newtui::components::settings_panel::{Backend, Choice, Model, Setting, SettingsSeed};
use newtui::{
    bar, butterfly, core_grid, gauge, heat_meter, sparkline, CoreSeries, SparkDirection,
    WidgetOutput,
};

/// One shipped piece in the live catalog.
pub struct Entry {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub data: &'static str,
    pub kind: Kind,
}

/// Display builders and the one interactive component currently shipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Settings,
    Sparkline,
    Butterfly,
    HeatMeter,
    Gauge,
    Bar,
    CoreGrid,
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
            Self::HeatMeter => "disk temperature",
            Self::Gauge => "daily budget",
            Self::CoreGrid => "cpu cores",
        }
    }

    pub fn demo_widths(self) -> &'static [usize] {
        match self {
            Self::Butterfly => &[24, 8, 1],
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
