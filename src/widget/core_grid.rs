use super::{fit, ramp::intensity_glyph, ratio, tone_for_ratio, Run, WidgetLine, WidgetOutput};

/// One core's current value and history.
#[derive(Debug, Clone, Copy)]
pub struct CoreSeries<'a> {
    /// Label shown at the left edge.
    pub label: &'a str,
    /// Most recent value.
    pub current: f64,
    /// Oldest-to-newest values.
    pub history: &'a [f64],
    /// Declared maximum for current and history values.
    pub maximum: f64,
}

/// Build one compact history row per visible core.
#[must_use]
pub fn core_grid(cores: &[CoreSeries<'_>], width: usize, height: usize) -> WidgetOutput {
    let mut lines = Vec::with_capacity(height);
    for row in 0..height {
        let Some(core) = cores.get(row) else {
            lines.push(WidgetLine::plain(" ".repeat(width)));
            continue;
        };
        let label_width = core.label.chars().count().min(3).min(width);
        let value = format!("{:>3.0}%", ratio(core.current, core.maximum) * 100.0);
        let graph_width = width.saturating_sub(label_width + value.chars().count() + 2);
        let mut graph = " ".repeat(graph_width.saturating_sub(core.history.len().min(graph_width)));
        for sample in core
            .history
            .iter()
            .rev()
            .take(graph_width)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            graph.push(intensity_glyph(ratio(*sample, core.maximum)));
        }
        let raw = format!("{} {graph} {value}", fit(core.label, label_width));
        lines.push(WidgetLine::new(vec![Run::new(
            fit(&raw, width),
            tone_for_ratio(ratio(core.current, core.maximum)),
        )]));
    }
    WidgetOutput::new(lines)
}
