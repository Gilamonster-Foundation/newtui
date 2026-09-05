use super::{
    filled_columns, fit, ratio, single_line, tone_for_ratio, Run, WidgetLine, WidgetOutput,
};

/// Build a labelled bar; `value_label` is host-formatted to keep units out of the widget.
#[must_use]
pub fn bar(
    label: &str,
    value: f64,
    maximum: f64,
    value_label: &str,
    width: usize,
    height: usize,
) -> WidgetOutput {
    let level = ratio(value, maximum);
    let label_width = label.chars().count().min(4);
    let value_width = value_label.chars().count();
    let bar_width = width.saturating_sub(label_width.saturating_add(value_width).saturating_add(2));
    let filled = filled_columns(bar_width, level);
    let raw = format!(
        "{} {}{} {}",
        fit(label, label_width),
        "\u{2588}".repeat(filled),
        "\u{00b7}".repeat(bar_width.saturating_sub(filled)),
        value_label
    );
    single_line(
        WidgetLine::new(vec![Run::new(fit(&raw, width), tone_for_ratio(level))]),
        width,
        height,
    )
}
