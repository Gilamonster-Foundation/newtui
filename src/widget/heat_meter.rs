use super::{
    filled_columns, fit, ratio, single_line, tone_for_ratio, Run, WidgetLine, WidgetOutput,
};

/// Build a labelled heat meter; labels are clipped to the requested width.
#[must_use]
pub fn heat_meter(
    label: &str,
    percent: f64,
    right_label: &str,
    width: usize,
    height: usize,
) -> WidgetOutput {
    let left = fit(label, label.chars().count().min(4));
    let right = fit(right_label, right_label.chars().count());
    let chrome = left
        .chars()
        .count()
        .saturating_add(right.chars().count())
        .saturating_add(2)
        .min(width);
    let bar_width = width.saturating_sub(chrome);
    let level = ratio(percent, 100.0);
    let filled = filled_columns(bar_width, level);
    let raw = format!(
        "{left} {}{} {right}",
        "\u{2588}".repeat(filled),
        "\u{00b7}".repeat(bar_width.saturating_sub(filled))
    );
    single_line(
        WidgetLine::new(vec![Run::new(fit(&raw, width), tone_for_ratio(level))]),
        width,
        height,
    )
}
