//! Pure display widgets and the renderer-neutral data they produce.

mod bar;
mod butterfly;
mod core_grid;
mod gauge;
mod heat_meter;
mod output;
mod ramp;
mod sparkline;

pub use bar::bar;
pub use butterfly::butterfly;
pub use core_grid::{core_grid, CoreSeries};
pub use gauge::gauge;
pub use heat_meter::heat_meter;
pub use output::{Run, Tone, WidgetLine, WidgetOutput, WidgetOutputError};
pub use sparkline::{sparkline, SparkDirection};

#[cfg(feature = "ratatui")]
pub use output::ratatui_lines;

fn ratio(value: f64, maximum: f64) -> f64 {
    if !value.is_finite() || !maximum.is_finite() || maximum <= 0.0 {
        0.0
    } else {
        (value / maximum).clamp(0.0, 1.0)
    }
}

fn tone_for_ratio(ratio: f64) -> Tone {
    if ratio >= 0.9 {
        Tone::Critical
    } else if ratio >= 0.7 {
        Tone::Caution
    } else {
        Tone::Healthy
    }
}

fn filled_columns(width: usize, level: f64) -> usize {
    // `level` is finite and clamped before it arrives. The product is bounded
    // by `width`, so the integer conversion cannot change sign or overflow.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )]
    let filled = (width as f64 * level).round() as usize;
    filled
}

fn position_ratio(position: usize, width: usize) -> f64 {
    // Terminal widths are small even though the API uses usize to compose
    // naturally with strings and vectors; precision loss is unreachable for
    // a real terminal rectangle.
    #[allow(clippy::cast_precision_loss)]
    let result = position as f64 / width.max(1) as f64;
    result
}

fn cell_intensity(level: f64, height: usize, row: usize) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let result = (level * height as f64 - row as f64).clamp(0.0, 1.0);
    result
}

fn fit(text: &str, width: usize) -> String {
    text.chars()
        .filter(|ch| output::is_declared_glyph(*ch))
        .take(width)
        .chain(std::iter::repeat(' '))
        .take(width)
        .collect()
}

fn fitted_line(runs: Vec<Run>, width: usize) -> WidgetLine {
    let mut remaining = width;
    let mut fitted = Vec::with_capacity(runs.len() + 1);
    for run in runs {
        let text: String = run
            .text
            .chars()
            .filter(|glyph| output::is_declared_glyph(*glyph))
            .take(remaining)
            .collect();
        remaining = remaining.saturating_sub(text.chars().count());
        if !text.is_empty() {
            fitted.push(Run::new(text, run.tone));
        }
    }
    if remaining > 0 {
        fitted.push(Run::new(" ".repeat(remaining), Tone::Plain));
    }
    WidgetLine::new(fitted)
}

fn single_line(line: WidgetLine, width: usize, height: usize) -> WidgetOutput {
    let mut lines = Vec::with_capacity(height);
    if height > 0 {
        lines.push(line);
        lines.extend((1..height).map(|_| WidgetLine::plain(" ".repeat(width))));
    }
    WidgetOutput::new(lines)
}
