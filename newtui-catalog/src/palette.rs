use crate::options::Theme;
use newtui::Tone;
use ratatui::style::{Color, Style};

#[derive(Clone, Copy)]
pub struct Palette {
    pub background: Color,
    pub panel: Color,
    pub text: Color,
    pub muted: Color,
    pub border: Color,
    pub accent: Color,
    pub selected: Color,
    pub warm: Color,
    pub danger: Color,
}

impl Palette {
    pub fn for_theme(theme: Theme) -> Self {
        match theme {
            Theme::Dark => Self {
                background: Color::Rgb(15, 23, 24),
                panel: Color::Rgb(21, 34, 35),
                text: Color::Rgb(220, 231, 224),
                muted: Color::Rgb(135, 160, 153),
                border: Color::Rgb(51, 74, 71),
                accent: Color::Rgb(171, 224, 145),
                selected: Color::Rgb(43, 65, 49),
                warm: Color::Rgb(232, 182, 123),
                danger: Color::Rgb(244, 129, 125),
            },
            Theme::Light => Self {
                background: Color::Rgb(247, 246, 239),
                panel: Color::Rgb(237, 238, 226),
                text: Color::Rgb(29, 48, 43),
                muted: Color::Rgb(80, 108, 95),
                border: Color::Rgb(173, 192, 175),
                accent: Color::Rgb(37, 103, 58),
                selected: Color::Rgb(216, 232, 205),
                warm: Color::Rgb(147, 79, 22),
                danger: Color::Rgb(171, 47, 44),
            },
        }
    }

    pub fn tone(self, tone: Tone) -> Style {
        let foreground = match tone {
            Tone::Plain => self.text,
            Tone::Muted => self.muted,
            Tone::Label => self.text,
            Tone::Accent => self.warm,
            Tone::Healthy => self.accent,
            Tone::Caution => self.warm,
            Tone::Critical => self.danger,
        };
        Style::default().fg(foreground).bg(self.panel)
    }
}
