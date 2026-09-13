//! Finite reference fixture matching the Python face's dial example.

use newtui::{properties, Component, Flow, Key, Property, Row, View};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::model::{Artifact, Domain, Input};

pub const DIAL: &str = "newtui.example-dial/v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Seed {
    level: u8,
    maximum: u8,
}

pub struct Dial {
    level: u8,
    maximum: u8,
    accepted: Option<u8>,
}

impl Dial {
    pub fn from_seed(seed: &Value) -> Result<Self, String> {
        let seed: Seed = serde_json::from_value(seed.clone()).map_err(|error| error.to_string())?;
        if seed.maximum > 20 || seed.level > seed.maximum {
            return Err("dial seed requires 0 <= level <= maximum <= 20".into());
        }
        Ok(Self {
            level: seed.level,
            maximum: seed.maximum,
            accepted: None,
        })
    }

    pub fn intent(&self) -> Value {
        self.accepted
            .map_or(Value::Null, |level| json!({"accepted": level}))
    }
}

impl Component for Dial {
    fn handle(&mut self, key: Key) -> Flow {
        match key {
            Key::Right => self.level = (self.level + 1).min(self.maximum),
            Key::Left => self.level = self.level.saturating_sub(1),
            Key::Enter => {
                self.accepted = Some(self.level);
                return Flow::Close(true);
            }
            Key::Esc => return Flow::Close(false),
            _ => {}
        }
        Flow::Stay
    }

    fn view(&self) -> View {
        View::titled("dial")
            .row(
                Row::new("level", self.level.to_string())
                    .selected()
                    .adjustable(),
            )
            .footer("arrows change")
    }
}

pub fn claims() -> Vec<Box<dyn Property>> {
    vec![
        Box::new(properties::selection_is_always_in_range()),
        Box::new(properties::escape_always_closes_without_applying()),
        Box::new(properties::only_adjustable_rows_move()),
    ]
}

pub fn dial() -> Result<Artifact, String> {
    let seed = json!({"level": 0, "maximum": 3});
    let properties = claims();
    let properties: Vec<&dyn Property> = properties
        .iter()
        .map(|property| property.as_ref())
        .collect();
    crate::export(
        DIAL,
        seed.clone(),
        Domain {
            alphabet: vec![
                Input::Up,
                Input::Down,
                Input::Left,
                Input::Right,
                Input::Enter,
                Input::Esc,
            ],
            max_states: 32,
            max_depth: 8,
        },
        || Dial::from_seed(&seed).expect("the reference seed is valid"),
        &properties,
        Dial::intent,
    )
}
