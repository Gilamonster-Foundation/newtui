//! A settings panel whose vocabulary and effects belong to its host.
//!
//! This module knows the shape of choices, bounded numbers, a model dial, and
//! a backend door. It deliberately knows no product setting names or accepted
//! tokens. Those arrive in [`SettingsSeed`], so adding a host setting cannot
//! make a second vocabulary here drift out of date.

use crate::{Component, Flow, Key, Named, Observation, Property, PropertyOutcome, Row, View};

/// One value a host offers, and the explanation shown beside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    value: String,
    note: String,
}

impl Choice {
    /// Describe an accepted value.
    #[must_use]
    pub fn new(value: impl Into<String>, note: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            note: note.into(),
        }
    }
}

/// A setting row supplied by the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Setting {
    key: String,
    label: String,
    current: String,
    space: ValueSpace,
}

impl Setting {
    /// A row that dials over a closed host vocabulary.
    #[must_use]
    pub fn choice(
        key: impl Into<String>,
        label: impl Into<String>,
        current: impl Into<String>,
        choices: Vec<Choice>,
    ) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            current: current.into(),
            space: ValueSpace::Choice(choices),
        }
    }

    /// A bounded integer row with a token one step below the floor.
    #[must_use]
    pub fn number(
        key: impl Into<String>,
        label: impl Into<String>,
        current: impl Into<String>,
        release: impl Into<String>,
        min: usize,
        max: usize,
    ) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            current: current.into(),
            space: ValueSpace::Number {
                release: release.into(),
                min,
                max,
            },
        }
    }

    /// A value shown here but edited through a host-owned surface.
    #[must_use]
    pub fn fixed(
        key: impl Into<String>,
        label: impl Into<String>,
        current: impl Into<String>,
        note: impl Into<String>,
    ) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            current: current.into(),
            space: ValueSpace::Fixed { note: note.into() },
        }
    }
}

/// The active model and the choices the backend returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    current: String,
    choices: Option<Vec<Choice>>,
}

impl Model {
    /// Seed the model row. `None` means the backend could not be listed.
    #[must_use]
    pub fn new(current: impl Into<String>, choices: Option<Vec<Choice>>) -> Self {
        Self {
            current: current.into(),
            choices,
        }
    }
}

/// The backend door at the bottom of the panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backend {
    current: Option<String>,
}

impl Backend {
    /// Seed the door with the active backend, if one is resolved.
    #[must_use]
    pub fn new(current: Option<impl Into<String>>) -> Self {
        Self {
            current: current.map(Into::into),
        }
    }
}

/// Everything the host knows when the panel opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsSeed {
    settings: Vec<Setting>,
    model: Model,
    backend: Backend,
}

impl SettingsSeed {
    /// Compose the rows in their display order.
    #[must_use]
    pub fn new(settings: Vec<Setting>, model: Model, backend: Backend) -> Self {
        Self {
            settings,
            model,
            backend,
        }
    }
}

/// One changed setting for the host's mutation path to apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingChange {
    /// The opaque key the host supplied.
    pub key: String,
    /// The accepted value the operator selected.
    pub value: String,
}

/// What an accepted close asks the host to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsIntent {
    /// Apply the changed settings and optional model selection.
    Apply {
        /// Settings whose value differs from when the panel opened.
        changes: Vec<SettingChange>,
        /// A changed model, for the host's validated model switch path.
        model: Option<String>,
    },
    /// Apply pending changes, then open the host's backend chooser.
    OpenBackends {
        /// Settings changed before walking through the door.
        changes: Vec<SettingChange>,
        /// A changed model selected before walking through the door.
        model: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ValueSpace {
    Choice(Vec<Choice>),
    Number {
        release: String,
        min: usize,
        max: usize,
    },
    Fixed {
        note: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SettingRow {
    key: String,
    label: String,
    value: String,
    opened_as: String,
    space: ValueSpace,
}

impl SettingRow {
    fn cycle(&mut self, direction: Direction) {
        match &self.space {
            ValueSpace::Choice(choices) => {
                if choices.is_empty() {
                    return;
                }
                let at = choices
                    .iter()
                    .position(|choice| choice.value == self.value)
                    .unwrap_or(0);
                self.value = choices[step_index(at, direction, choices.len())]
                    .value
                    .clone();
            }
            ValueSpace::Number { release, min, max } => {
                self.value = match self.value.parse::<usize>() {
                    Ok(value) if direction == Direction::Left && value <= *min => release.clone(),
                    Ok(value) if direction == Direction::Left => {
                        value.saturating_sub(1).to_string()
                    }
                    Ok(value) => value.saturating_add(1).min(*max).to_string(),
                    Err(_) if direction == Direction::Right => min.to_string(),
                    Err(_) => release.clone(),
                };
            }
            ValueSpace::Fixed { .. } => {}
        }
    }

    fn dialable(&self) -> bool {
        match &self.space {
            ValueSpace::Choice(choices) => choices.len() > 1,
            ValueSpace::Number { .. } => true,
            ValueSpace::Fixed { .. } => false,
        }
    }

    fn meaning(&self) -> String {
        match &self.space {
            ValueSpace::Choice(choices) => choices
                .iter()
                .find(|choice| choice.value == self.value)
                .map_or_else(String::new, |choice| choice.note.clone()),
            ValueSpace::Number { .. } => String::new(),
            ValueSpace::Fixed { note } => note.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModelRow {
    choices: Option<Vec<Choice>>,
    at: usize,
    opened_as: String,
}

impl ModelRow {
    fn new(model: Model) -> Self {
        // The live choice remains visible when a stale served-list omits it;
        // silently opening on another model would turn Enter into an edit the
        // operator did not make.
        let choices = model.choices.map(|mut choices| {
            if !model.current.is_empty()
                && !choices.iter().any(|choice| choice.value == model.current)
            {
                choices.push(Choice::new(model.current.clone(), "(not served)"));
            }
            choices
        });
        let at = choices
            .as_ref()
            .and_then(|choices| {
                choices
                    .iter()
                    .position(|choice| choice.value == model.current)
            })
            .unwrap_or(0);
        Self {
            choices,
            at,
            opened_as: model.current,
        }
    }

    fn value(&self) -> String {
        self.choices
            .as_ref()
            .and_then(|choices| choices.get(self.at))
            .map_or_else(|| self.opened_as.clone(), |choice| choice.value.clone())
    }

    fn note(&self) -> String {
        self.choices
            .as_ref()
            .and_then(|choices| choices.get(self.at))
            .map_or_else(String::new, |choice| choice.note.clone())
    }

    fn dialable(&self) -> bool {
        self.choices
            .as_ref()
            .is_some_and(|choices| choices.len() > 1)
    }

    fn cycle(&mut self, direction: Direction) {
        if let Some(choices) = self.choices.as_ref().filter(|choices| !choices.is_empty()) {
            self.at = step_index(self.at, direction, choices.len());
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Left,
    Right,
}

fn step_index(at: usize, direction: Direction, len: usize) -> usize {
    match direction {
        Direction::Left => at.saturating_sub(1),
        Direction::Right => at.saturating_add(1).min(len.saturating_sub(1)),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PanelRow {
    Setting(SettingRow),
    Model(ModelRow),
    Backend(String),
}

/// The pure state machine for a settings panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsPanel {
    rows: Vec<PanelRow>,
    selected: usize,
    intent: Option<SettingsIntent>,
}

impl SettingsPanel {
    /// Open a panel from host-owned data.
    #[must_use]
    pub fn new(seed: SettingsSeed) -> Self {
        let mut rows: Vec<PanelRow> = seed
            .settings
            .into_iter()
            .map(|setting| {
                PanelRow::Setting(SettingRow {
                    key: setting.key,
                    label: setting.label,
                    value: setting.current.clone(),
                    opened_as: setting.current,
                    space: setting.space,
                })
            })
            .collect();
        rows.push(PanelRow::Model(ModelRow::new(seed.model)));
        rows.push(PanelRow::Backend(
            seed.backend.current.unwrap_or_else(|| "(none)".to_string()),
        ));
        Self {
            rows,
            selected: 0,
            intent: None,
        }
    }

    /// The host action reported by the last accepted close.
    #[must_use]
    pub fn intent(&self) -> Option<&SettingsIntent> {
        self.intent.as_ref()
    }

    fn finish(&mut self, apply: bool, open_backends: bool) -> Flow {
        if !apply {
            self.intent = None;
            return Flow::Close(false);
        }
        let changes = self
            .rows
            .iter()
            .filter_map(|row| match row {
                PanelRow::Setting(row) if row.value != row.opened_as => Some(SettingChange {
                    key: row.key.clone(),
                    value: row.value.clone(),
                }),
                _ => None,
            })
            .collect();
        let model = self.rows.iter().find_map(|row| match row {
            PanelRow::Model(row) if row.value() != row.opened_as => Some(row.value()),
            _ => None,
        });
        self.intent = Some(if open_backends {
            SettingsIntent::OpenBackends { changes, model }
        } else {
            SettingsIntent::Apply { changes, model }
        });
        Flow::Close(true)
    }
}

impl Component for SettingsPanel {
    fn handle(&mut self, key: Key) -> Flow {
        match key {
            Key::Up => self.selected = self.selected.saturating_sub(1),
            Key::Down => {
                self.selected = self
                    .selected
                    .saturating_add(1)
                    .min(self.rows.len().saturating_sub(1));
            }
            Key::Left | Key::Right => {
                let direction = if key == Key::Left {
                    Direction::Left
                } else {
                    Direction::Right
                };
                match self.rows.get_mut(self.selected) {
                    Some(PanelRow::Setting(row)) => row.cycle(direction),
                    Some(PanelRow::Model(row)) => row.cycle(direction),
                    Some(PanelRow::Backend(_)) | None => {}
                }
            }
            Key::Enter => {
                let open_backends =
                    matches!(self.rows.get(self.selected), Some(PanelRow::Backend(_)));
                return self.finish(true, open_backends);
            }
            Key::Esc | Key::Char('q') => return self.finish(false, false),
            _ => {}
        }
        Flow::Stay
    }

    fn view(&self) -> View {
        let mut view = View::titled("settings");
        for (index, row) in self.rows.iter().enumerate() {
            let selected = index == self.selected;
            let rendered = match row {
                PanelRow::Setting(row) => {
                    let note = if selected {
                        row.meaning()
                    } else if row.value != row.opened_as {
                        format!("was {}", row.opened_as)
                    } else {
                        String::new()
                    };
                    let mut rendered = Row::new(&row.label, &row.value).note(note);
                    if row.dialable() {
                        rendered = rendered.adjustable();
                    }
                    rendered
                }
                PanelRow::Model(row) => {
                    let note = if selected && !row.dialable() {
                        if row.choices.is_none() {
                            "the active backend could not be listed".to_string()
                        } else {
                            "the backend serves only this one".to_string()
                        }
                    } else if selected {
                        row.note()
                    } else if row.value() != row.opened_as {
                        format!("was {}", row.opened_as)
                    } else {
                        row.note()
                    };
                    let mut rendered = Row::new("model", row.value()).note(note);
                    if row.dialable() {
                        rendered = rendered.adjustable();
                    }
                    rendered
                }
                PanelRow::Backend(value) => Row::new("backend", value).note(if selected {
                    "Enter: choose, edit, add or remove a backend"
                } else {
                    ""
                }),
            };
            view = view.row(if selected {
                rendered.selected()
            } else {
                rendered
            });
        }
        view.footer("↑↓ select · ←→ change · Enter apply · Esc cancel")
    }
}

#[derive(Debug, Clone)]
enum Accepted {
    Choice(Vec<String>),
    Number {
        release: String,
        min: usize,
        max: usize,
    },
    None,
}

impl Accepted {
    fn contains(&self, value: &str) -> bool {
        match self {
            Self::Choice(choices) => choices.iter().any(|choice| choice == value),
            Self::Number { release, min, max } => {
                value == release
                    || value
                        .parse::<usize>()
                        .is_ok_and(|number| (*min..=*max).contains(&number))
            }
            Self::None => false,
        }
    }
}

fn accepted_rows(seed: &SettingsSeed) -> Vec<Accepted> {
    let mut accepted: Vec<Accepted> = seed
        .settings
        .iter()
        .map(|setting| match &setting.space {
            ValueSpace::Choice(choices) => {
                Accepted::Choice(choices.iter().map(|choice| choice.value.clone()).collect())
            }
            ValueSpace::Number { release, min, max } => Accepted::Number {
                release: release.clone(),
                min: *min,
                max: *max,
            },
            ValueSpace::Fixed { .. } => Accepted::None,
        })
        .collect();
    accepted.push(match &seed.model.choices {
        Some(choices) => {
            let mut values: Vec<String> =
                choices.iter().map(|choice| choice.value.clone()).collect();
            if !seed.model.current.is_empty() && !values.contains(&seed.model.current) {
                values.push(seed.model.current.clone());
            }
            Accepted::Choice(values)
        }
        None => Accepted::None,
    });
    accepted.push(Accepted::None);
    accepted
}

/// The settings-panel-specific claim: a moved dial lands in host-supplied data.
#[must_use]
pub fn dial_values_are_accepted(
    seed: &SettingsSeed,
) -> Named<impl Fn(&Observation<'_>) -> PropertyOutcome> {
    let accepted = accepted_rows(seed);
    Named::new("dial values are accepted", move |observation| {
        let Observation::Transition { from, key, to, .. } = observation else {
            return PropertyOutcome::NotApplicable;
        };
        if !matches!(key, Key::Left | Key::Right) {
            return PropertyOutcome::NotApplicable;
        }
        let Some(at) = from.selected() else {
            return PropertyOutcome::NotApplicable;
        };
        let (Some(before), Some(after), Some(space)) =
            (from.rows.get(at), to.rows.get(at), accepted.get(at))
        else {
            return PropertyOutcome::NotApplicable;
        };
        if before.value == after.value {
            return PropertyOutcome::NotApplicable;
        }
        if space.contains(&after.value) {
            PropertyOutcome::Held
        } else {
            PropertyOutcome::Violated(format!(
                "`{}` produced `{}`, which the host vocabulary refuses",
                after.label, after.value
            ))
        }
    })
}

/// The acceptance set every settings-panel implementation must satisfy.
#[must_use]
pub fn acceptance(seed: &SettingsSeed) -> Vec<Box<dyn Property>> {
    vec![
        Box::new(crate::properties::selection_is_always_in_range()),
        Box::new(crate::properties::escape_always_closes_without_applying()),
        Box::new(crate::properties::only_adjustable_rows_move()),
        Box::new(dial_values_are_accepted(seed)),
    ]
}
