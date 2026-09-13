use super::{butterfly, WidgetOutput};

/// Build a vertical history of bars growing away from a stable centre marker.
///
/// Both slices run oldest to newest. Their newest samples share the bottom
/// row; older samples beyond `height` are omitted, and a missing sample on
/// either side leaves that bar empty. The host aligns sampling times before
/// calling this builder and owns clocks, labels, units, and current rates.
///
/// Each row uses the compact [`butterfly`] meter's shared `maximum`, glyphs,
/// and semantic tones. Non-finite samples, negative values, and invalid maxima
/// render empty signal; finite samples above the maximum fill their side.
/// One column is reserved for `|`; an even width gives the extra bar column
/// to the left. Zero width or height produces the corresponding empty rectangle.
///
/// ```
/// let history = newtui::butterfly_history(
///     &[10.0, 50.0, 90.0], &[80.0, 40.0, 20.0], 100.0, 41, 12,
/// );
/// assert!(history.validate(41, 12).is_ok());
/// ```
#[must_use]
pub fn butterfly_history(
    tx: &[f64],
    rx: &[f64],
    maximum: f64,
    width: usize,
    height: usize,
) -> WidgetOutput {
    let lines = (0..height)
        .rev()
        .flat_map(|age| {
            let left = sample_at_age(tx, age);
            let right = sample_at_age(rx, age);
            butterfly(left, right, maximum, "", "", width, 1).lines
        })
        .collect();
    WidgetOutput::new(lines)
}

fn sample_at_age(values: &[f64], age: usize) -> f64 {
    values
        .len()
        .checked_sub(age + 1)
        .map_or(0.0, |index| values[index])
}
