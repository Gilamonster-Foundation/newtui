// Copyright 2026 The Gilamonster Authors
// SPDX-License-Identifier: Apache-2.0

//! ASCII character art for the Monitor Lizard.
//!
//! Art is loaded from config files (one file per state), with hardcoded
//! fallbacks. Each file can have multiple frames separated by `---`.
//!
//! Directory layout:
//!   ~/.config/monitor-lizard/characters/monitor-lizard/
//!     idle.txt          # frame 1\n---\nframe 2 (blink)
//!     sleeping.txt
//!     listening.txt
//!     thinking.txt      # frame 1\n---\nframe 2\n---\nframe 3
//!     active.txt
//!     super-active.txt

use std::collections::HashMap;
use std::path::Path;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

/// Character state — drives which art frame to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CharacterState {
    Sleeping,
    Idle,
    Listening,
    Thinking,
    Active,
    SuperActive,
}

impl CharacterState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Sleeping => "zzz",
            Self::Idle => "idle",
            Self::Listening => "listen",
            Self::Thinking => "think",
            Self::Active => "chat",
            Self::SuperActive => "!!",
        }
    }

    fn color(&self) -> Color {
        match self {
            Self::Sleeping => Color::DarkGray,
            Self::Idle => Color::Green,
            Self::Listening => Color::Cyan,
            Self::Thinking => Color::Yellow,
            Self::Active => Color::Green,
            Self::SuperActive => Color::Red,
        }
    }

    fn filename(&self) -> &'static str {
        match self {
            Self::Sleeping => "sleeping.txt",
            Self::Idle => "idle.txt",
            Self::Listening => "listening.txt",
            Self::Thinking => "thinking.txt",
            Self::Active => "active.txt",
            Self::SuperActive => "super-active.txt",
        }
    }

    pub fn all() -> &'static [CharacterState] {
        &[
            Self::Sleeping,
            Self::Idle,
            Self::Listening,
            Self::Thinking,
            Self::Active,
            Self::SuperActive,
        ]
    }
}

/// Loaded character art — multiple frames per state.
#[derive(Debug, Clone)]
pub struct CharacterArt {
    /// State → vec of frames. Each frame is a vec of lines.
    frames: HashMap<CharacterState, Vec<Vec<String>>>,
}

impl CharacterArt {
    /// Load character art from a directory. Falls back to defaults for missing states.
    pub fn load(dir: &Path) -> Self {
        let mut frames = HashMap::new();

        for state in CharacterState::all() {
            let file_path = dir.join(state.filename());
            let state_frames = if file_path.exists() {
                match std::fs::read_to_string(&file_path) {
                    Ok(content) => parse_frames(&content),
                    Err(_) => default_frames(*state),
                }
            } else {
                default_frames(*state)
            };
            frames.insert(*state, state_frames);
        }

        Self { frames }
    }

    /// Get the art lines for a state at a given tick.
    pub fn get_frame(&self, state: CharacterState, tick: u64) -> &[String] {
        let state_frames = self.frames.get(&state).unwrap();
        let idx = (tick as usize) % state_frames.len();
        &state_frames[idx]
    }

    /// Write default art files to a directory (for --init-config).
    pub fn write_defaults(dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        for state in CharacterState::all() {
            let frames = default_frames(*state);
            let content: Vec<String> = frames.iter().map(|frame| frame.join("\n")).collect();
            let text = content.join("\n---\n");
            std::fs::write(dir.join(state.filename()), text)?;
        }
        Ok(())
    }
}

impl Default for CharacterArt {
    fn default() -> Self {
        let mut frames = HashMap::new();
        for state in CharacterState::all() {
            frames.insert(*state, default_frames(*state));
        }
        Self { frames }
    }
}

/// Parse frames from a text file. Frames are separated by `---` on its own line.
fn parse_frames(content: &str) -> Vec<Vec<String>> {
    let frames: Vec<Vec<String>> = content
        .split("\n---\n")
        .map(|frame| frame.lines().map(String::from).collect())
        .filter(|frame: &Vec<String>| !frame.is_empty())
        .collect();

    if frames.is_empty() {
        vec![vec!["  ?  ".into()]]
    } else {
        frames
    }
}

/// Default hardcoded frames for a character state.
/// Each frame is padded with blank lines at top so the character
/// sits in the middle of the 7-line drawing area, not stuck to ceiling.
fn default_frames(state: CharacterState) -> Vec<Vec<String>> {
    match state {
        CharacterState::Sleeping => vec![vec![
            "           ".into(),
                "           ".into(),
            "  ( -.- )  ".into(),
            "  /|   |\\  ".into(),
            " / |   | \\ ".into(),
            "   d   b   ".into(),
            "    zzz    ".into(),
        ]],
        CharacterState::Idle => vec![
            vec![
                "           ".into(),
                "  ( •ᴗ• )  ".into(),
                "  /|   |\\  ".into(),
                " / |   | \\ ".into(),
                "   d   b   ".into(),
                "           ".into(),
            ],
            vec![
                "           ".into(),
                "  ( •ᴗ• )  ".into(),
                "  /|   |\\  ".into(),
                " / |   | \\ ".into(),
                "   d   b   ".into(),
                "           ".into(),
            ],
            vec![
                "           ".into(),
                "  ( •ᴗ• )  ".into(),
                "  /|   |\\  ".into(),
                " / |   | \\ ".into(),
                "   d   b   ".into(),
                "           ".into(),
            ],
            // blink frame
            vec![
                "           ".into(),
                "  ( -ᴗ- )  ".into(),
                "  /|   |\\  ".into(),
                " / |   | \\ ".into(),
                "   d   b   ".into(),
                "           ".into(),
            ],
        ],
        CharacterState::Listening => vec![vec![
                "           ".into(),
            "  ( ◉◡◉ )  ".into(),
            " \\|     |/ ".into(),
            "  |     |  ".into(),
            "  d     b  ".into(),
            "    🎤     ".into(),
        ]],
        CharacterState::Thinking => vec![
            vec![
                "           ".into(),
                "  ( •_• )  ".into(),
                "  /|   |\\  ".into(),
                " / |   | \\ ".into(),
                "   d   b   ".into(),
                "    .      ".into(),
            ],
            vec![
                "           ".into(),
                "  ( •_• )  ".into(),
                "  /|   |\\  ".into(),
                " / |   | \\ ".into(),
                "   d   b   ".into(),
                "    ..     ".into(),
            ],
            vec![
                "           ".into(),
                "  ( •_• )  ".into(),
                "  /|   |\\  ".into(),
                " / |   | \\ ".into(),
                "   d   b   ".into(),
                "    ...    ".into(),
            ],
        ],
        CharacterState::Active => vec![vec![
                "           ".into(),
            "  ( •ᴗ• )  ".into(),
            "  \\|   |/  ".into(),
            "   |   |   ".into(),
            "   d   b   ".into(),
            "    💬     ".into(),
        ]],
        CharacterState::SuperActive => vec![
            vec![
                "           ".into(),
                " \\( °□° )/ ".into(),
                "   |   |   ".into(),
                "  /|   |\\  ".into(),
                "  d     b  ".into(),
                "    ⚡     ".into(),
            ],
            vec![
                "           ".into(),
                " \\( >□< )/ ".into(),
                "   |   |   ".into(),
                "  /|   |\\  ".into(),
                "  d     b  ".into(),
                "    🔥     ".into(),
            ],
        ],
    }
}

pub fn draw(
    frame: &mut Frame,
    art: &CharacterArt,
    state: CharacterState,
    tick: u64,
    reaction_text: &str,
    volume_percent: Option<u32>,
    area: Rect,
) {
    let label = format!(" [{}] ", state.label());
    let mut block = Block::default()
        .borders(Borders::ALL)
        .title(label)
        .border_style(Style::default().fg(state.color()));

    // Volume indicator in the bottom border (mirrors [state] in the top border)
    if let Some(vol) = volume_percent {
        let vol_color = if vol > 70 {
            Color::Red
        } else if vol > 40 {
            Color::Yellow
        } else {
            Color::DarkGray
        };
        let vol_label = Line::from(Span::styled(
            format!(" Vol {vol}% "),
            Style::default().fg(vol_color),
        ));
        block = block.title_bottom(vol_label);
    }

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Reserve 3 lines at bottom for reaction text
    let reaction_lines: u16 = 3;
    let art_area = Rect {
        height: inner.height.saturating_sub(reaction_lines),
        ..inner
    };
    let reaction_area = Rect {
        y: inner.y + art_area.height,
        height: reaction_lines.min(inner.height),
        ..inner
    };

    // Draw character art
    let art_lines = art.get_frame(state, tick);
    let lines: Vec<Line> = art_lines
        .iter()
        .map(|l| Line::from(Span::styled(l.as_str(), Style::default().fg(state.color()))))
        .collect();
    frame.render_widget(Paragraph::new(lines), art_area);

    // Draw reaction text (word-wrapped into 3 lines)
    if !reaction_text.is_empty() {
        let width = reaction_area.width as usize;
        let mut wrapped: Vec<Line> = Vec::new();
        let mut remaining = reaction_text;

        while !remaining.is_empty() && wrapped.len() < reaction_lines as usize {
            let chunk_len = remaining.len().min(width);
            let break_at = if chunk_len < remaining.len() {
                remaining[..chunk_len]
                    .rfind(' ')
                    .map(|p| p + 1)
                    .unwrap_or(chunk_len)
            } else {
                chunk_len
            };
            let chunk = &remaining[..break_at];
            remaining = remaining[break_at..].trim_start();
            wrapped.push(Line::from(Span::styled(
                chunk,
                Style::default().fg(Color::Yellow),
            )));
        }

        frame.render_widget(Paragraph::new(wrapped), reaction_area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_art_all_states() {
        let art = CharacterArt::default();
        for state in CharacterState::all() {
            let frame = art.get_frame(*state, 0);
            assert!(!frame.is_empty(), "state {:?} has no frames", state);
        }
    }

    #[test]
    fn test_frame_cycling() {
        let art = CharacterArt::default();
        // Idle has 4 frames (3 normal + 1 blink)
        let f0 = art.get_frame(CharacterState::Idle, 0);
        let f3 = art.get_frame(CharacterState::Idle, 3);
        // Frame 3 is the blink frame
        assert!(f3[0].contains("-ᴗ-"), "frame 3 should be blink");
        assert!(f0[0].contains("•ᴗ•"), "frame 0 should be open eyes");
    }

    #[test]
    fn test_parse_frames_single() {
        let content = "  ( •ᴗ• )  \n  /|   |\\  ";
        let frames = parse_frames(content);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].len(), 2);
    }

    #[test]
    fn test_parse_frames_multiple() {
        let content = "frame1 line1\nframe1 line2\n---\nframe2 line1\nframe2 line2";
        let frames = parse_frames(content);
        assert_eq!(frames.len(), 2);
    }

    #[test]
    fn test_load_from_dir() {
        // Default loads when dir doesn't exist
        let art = CharacterArt::load(Path::new("/nonexistent"));
        let frame = art.get_frame(CharacterState::Idle, 0);
        assert!(!frame.is_empty());
    }

    #[test]
    fn test_write_and_load_defaults() {
        let dir = std::env::temp_dir().join("test-monitor-art");
        let _ = std::fs::remove_dir_all(&dir);
        CharacterArt::write_defaults(&dir).unwrap();

        // Verify files exist
        for state in CharacterState::all() {
            assert!(dir.join(state.filename()).exists(), "missing {:?}", state);
        }

        // Load and verify
        let art = CharacterArt::load(&dir);
        for state in CharacterState::all() {
            let frame = art.get_frame(*state, 0);
            assert!(!frame.is_empty());
        }

        let _ = std::fs::remove_dir_all(&dir);
    }
}
