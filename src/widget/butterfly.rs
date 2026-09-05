use super::{
    filled_columns, fitted_line, position_ratio, ramp::intensity_glyph, ratio, single_line,
    tone_for_ratio, Run, Tone, WidgetLine, WidgetOutput,
};

/// Build a two-sided meter around a stable centre marker.
#[must_use]
pub fn butterfly(
    left: f64,
    right: f64,
    maximum: f64,
    left_label: &str,
    right_label: &str,
    width: usize,
    height: usize,
) -> WidgetOutput {
    if width == 0 {
        return WidgetOutput::new(vec![WidgetLine::default(); height]);
    }
    let labels = left_label
        .chars()
        .count()
        .saturating_add(right_label.chars().count());
    let graph = width.saturating_sub(labels).saturating_sub(1);
    let left_width = graph.div_ceil(2);
    let right_width = graph / 2;
    let l = ratio(left, maximum);
    let r = ratio(right, maximum);
    let left_fill = filled_columns(left_width, l);
    let right_fill = filled_columns(right_width, r);
    let left_bar: String = (0..left_width)
        .map(|col| {
            if col + left_fill < left_width {
                ' '
            } else {
                intensity_glyph(position_ratio(col + 1, left_width))
            }
        })
        .collect();
    let right_bar: String = (0..right_width)
        .map(|col| {
            if col < right_fill {
                intensity_glyph(1.0 - position_ratio(col, right_width))
            } else {
                ' '
            }
        })
        .collect();
    let line = fitted_line(
        vec![
            Run::new(left_label, Tone::Label),
            Run::new(left_bar, tone_for_ratio(l)),
            Run::new("|", Tone::Accent),
            Run::new(right_bar, tone_for_ratio(r)),
            Run::new(right_label, Tone::Label),
        ],
        width,
    );
    single_line(line, width, height)
}
