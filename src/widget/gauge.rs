use super::{
    filled_columns, fit, ratio, single_line, tone_for_ratio, Run, WidgetLine, WidgetOutput,
};

/// Build a gauge with a caller-owned label and numeric limit.
#[must_use]
pub fn gauge(label: &str, value: f64, maximum: f64, width: usize, height: usize) -> WidgetOutput {
    let level = ratio(value, maximum);
    let caption = format!("{label} {value:.1}/{maximum:.1}");
    let filled = filled_columns(width, level);
    let bar = format!(
        "{}{}",
        "\u{2588}".repeat(filled),
        "\u{00b7}".repeat(width.saturating_sub(filled))
    );
    let text = if width >= caption.chars().count() {
        fit(&caption, width)
    } else {
        fit(&bar, width)
    };
    single_line(
        WidgetLine::new(vec![Run::new(text, tone_for_ratio(level))]),
        width,
        height,
    )
}
