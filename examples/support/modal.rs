//! Shared host fixture: a modal whose height follows the library's policy.

use super::Scenario;
use newtui::layout::{ModalSize, SizeKey, FILL};
use newtui::{Key, Run, Tone, WidgetLine, WidgetOutput};

/// The preview is the modal's screen; the host grants at most this many rows.
pub const SCREEN_ROWS: u16 = 16;

/// Only host state: the library owns the policy, the host owns the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModalPreview {
    size: ModalSize,
    screen: u16,
}

impl Default for ModalPreview {
    fn default() -> Self {
        Self::new(Scenario::Normal)
    }
}

impl ModalPreview {
    pub fn new(scenario: Scenario) -> Self {
        Self {
            size: ModalSize::new(match scenario {
                Scenario::Empty | Scenario::Error => 0,
                Scenario::Long => 40,
                Scenario::Normal | Scenario::Narrow => 8,
            }),
            screen: SCREEN_ROWS,
        }
    }

    /// The host learns its screen height at draw time, as a real modal does.
    pub fn resize(&mut self, rows: u16) {
        self.screen = rows;
    }

    pub fn granted(&self) -> u16 {
        self.size.requested().min(self.screen)
    }

    /// Host key decoding: Shift-Up/Down step, `z` zooms. `+`/`-` also step,
    /// because the tape recorder cannot send a shifted arrow. A pointer drag
    /// would produce `SizeKey::To`; this host has no pointer.
    pub fn handle(&mut self, key: Key, shifted: bool) {
        let intent = match key {
            Key::Up if shifted => SizeKey::Grow,
            Key::Char('+') => SizeKey::Grow,
            Key::Down if shifted => SizeKey::Shrink,
            Key::Char('-') => SizeKey::Shrink,
            Key::Char('z') => SizeKey::Zoom,
            _ => return,
        };
        self.size.apply(intent, self.granted());
    }

    pub fn zoom_name(&self) -> &'static str {
        if self.size.zoomed() {
            "zoomed"
        } else {
            "sized"
        }
    }

    fn requested_name(&self) -> String {
        match self.size.requested() {
            FILL => "fill".into(),
            rows => rows.to_string(),
        }
    }

    pub fn status(&self) -> String {
        format!(
            "requested {} / granted {} of {} rows · {}\nShift-↑↓ or +/- one row from the granted height · z zoom/restore",
            self.requested_name(),
            self.granted(),
            self.screen,
            self.zoom_name()
        )
    }

    /// A `height`-row screen with the modal docked at its bottom edge.
    pub fn output(&self, width: usize, height: usize) -> WidgetOutput {
        let granted = usize::from(self.size.requested()).min(height);
        let inside = width.saturating_sub(2);
        let boxed = |text: &str, tone| {
            if width < 2 {
                WidgetLine::new(vec![Run::new(fit_to(text, width), tone)])
            } else {
                WidgetLine::new(vec![
                    Run::new("|", Tone::Accent),
                    Run::new(fit_to(text, inside), tone),
                    Run::new("|", Tone::Accent),
                ])
            }
        };
        let edge = |title: &str| {
            let rule = format!("+{:-<inside$.inside$}+", format!("-{title}"));
            WidgetLine::new(vec![Run::new(fit_to(&rule, width), Tone::Accent)])
        };
        let mut lines: Vec<WidgetLine> = (0..height - granted)
            .map(|_| WidgetLine::new(vec![Run::new(".".repeat(width), Tone::Muted)]))
            .collect();
        let body = [
            format!("requested {} / granted {granted}", self.requested_name()),
            format!("screen {height} rows · {}", self.zoom_name()),
        ];
        for row in 0..granted {
            lines.push(if row == 0 {
                edge(" modal ")
            } else if row + 1 == granted {
                edge("")
            } else if row + 2 == granted {
                boxed("S-↑↓ +/- size · z zoom", Tone::Muted)
            } else {
                boxed(body.get(row - 1).map_or("", String::as_str), Tone::Label)
            });
        }
        WidgetOutput::new(lines)
    }
}

/// Exactly `width` cells: clip, then pad.
fn fit_to(text: &str, width: usize) -> String {
    format!("{text:<width$.width$}")
}
