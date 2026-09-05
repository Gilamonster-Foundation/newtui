use super::{
    cell_intensity, ramp::intensity_glyph, ratio, tone_for_ratio, Run, Tone, WidgetLine,
    WidgetOutput,
};

/// Which edge values grow from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SparkDirection {
    /// Values grow upward from the bottom.
    Up,
    /// Values grow downward from the top.
    Down,
}

/// Build a multi-row history graph.
#[must_use]
pub fn sparkline(
    values: &[f64],
    maximum: f64,
    width: usize,
    height: usize,
    direction: SparkDirection,
) -> WidgetOutput {
    let data: Vec<f64> = values.iter().rev().take(width).rev().copied().collect();
    let mut lines = Vec::with_capacity(height);
    for visible_row in 0..height {
        let row = match direction {
            SparkDirection::Up => height - 1 - visible_row,
            SparkDirection::Down => visible_row,
        };
        let mut text = " ".repeat(width.saturating_sub(data.len()));
        let mut peak = 0.0_f64;
        for value in &data {
            let level = ratio(*value, maximum);
            peak = peak.max(level);
            text.push(intensity_glyph(cell_intensity(level, height, row)));
        }
        lines.push(WidgetLine::new(vec![Run::new(
            text,
            if peak == 0.0 {
                Tone::Muted
            } else {
                tone_for_ratio(peak)
            },
        )]));
    }
    WidgetOutput::new(lines)
}
