pub(crate) fn intensity_glyph(intensity: f64) -> char {
    if intensity > 0.66 {
        '\u{2588}'
    } else if intensity > 0.33 {
        '\u{2592}'
    } else if intensity > 0.0 {
        '\u{2591}'
    } else {
        ' '
    }
}
