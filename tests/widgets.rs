use newtui::{
    bar, butterfly, core_grid, gauge, heat_meter, sparkline, CoreSeries, Run, SparkDirection, Tone,
    WidgetLine, WidgetOutput,
};

#[test]
fn widget_output_is_styled_text_without_component_semantics() {
    let output = WidgetOutput::new(vec![WidgetLine::new(vec![
        Run::new("cpu ", Tone::Label),
        Run::new("\u{2588}\u{2592}\u{2591}", Tone::Healthy),
    ])]);

    assert_eq!(output.lines[0].runs[0].text, "cpu ");
    assert_eq!(output.lines[0].runs[1].tone, Tone::Healthy);

    let meter = butterfly(25.0, 75.0, 100.0, "TX", "RX", 16, 1);
    assert!(meter.lines[0]
        .runs
        .iter()
        .any(|run| run.text == "|" && run.tone == Tone::Accent));
}

fn domains() -> Vec<Vec<f64>> {
    vec![
        vec![],
        vec![4.0],
        vec![4.0, 4.0, 4.0],
        vec![0.0, 0.0, 0.0],
        vec![0.0, 100.0, 0.0],
        vec![200.0],
        vec![-10.0],
        vec![f64::NAN],
        vec![f64::INFINITY],
        vec![f64::NEG_INFINITY],
    ]
}

fn rectangles() -> [(usize, usize); 5] {
    [(0, 0), (0, 1), (1, 0), (1, 1), (8, 3)]
}

fn assert_shape(output: &WidgetOutput, width: usize, height: usize) {
    output.validate(width, height).unwrap_or_else(|error| {
        panic!("{width}x{height} output violated its rectangle: {error}: {output:?}")
    });
}

// GUARD: every_widget_survives_the_full_numeric_domain_and_degenerate_rectangles — this is a guard; tests/mutations.rs must show it red.
#[test]
fn every_widget_survives_the_full_numeric_domain_and_degenerate_rectangles() {
    for values in domains() {
        for (width, height) in rectangles() {
            assert_shape(
                &sparkline(&values, 100.0, width, height, SparkDirection::Up),
                width,
                height,
            );
            assert_shape(
                &sparkline(&values, 100.0, width, height, SparkDirection::Down),
                width,
                height,
            );

            let value = values.first().copied().unwrap_or(0.0);
            assert_shape(
                &butterfly(value, value, 100.0, "TX", "RX", width, height),
                width,
                height,
            );
            assert_shape(
                &heat_meter("label", value, "100%", width, height),
                width,
                height,
            );
            assert_shape(&gauge("daily", value, 100.0, width, height), width, height);
            assert_shape(
                &bar("cpu", value, 100.0, "100%", width, height),
                width,
                height,
            );

            let core = CoreSeries {
                label: "c0",
                current: value,
                history: &values,
                maximum: 100.0,
            };
            assert_shape(&core_grid(&[core], width, height), width, height);
        }
    }
}

#[test]
fn invalid_maxima_are_empty_signal_not_panics() {
    for maximum in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_shape(&sparkline(&[50.0], maximum, 8, 2, SparkDirection::Up), 8, 2);
        assert_shape(&butterfly(50.0, 50.0, maximum, "", "", 8, 1), 8, 1);
        assert_shape(&gauge("g", 50.0, maximum, 8, 1), 8, 1);
        assert_shape(&bar("b", 50.0, maximum, "x", 8, 1), 8, 1);
        assert_shape(
            &core_grid(
                &[CoreSeries {
                    label: "0",
                    current: 50.0,
                    history: &[50.0],
                    maximum,
                }],
                8,
                1,
            ),
            8,
            1,
        );
    }
}

#[test]
fn narrow_widgets_clip_labels_instead_of_panicking() {
    for output in [
        heat_meter("a label wider than its home", 50.0, "also wide", 3, 1),
        gauge("a label wider than its home", 50.0, 100.0, 3, 1),
        bar(
            "a label wider than its home",
            50.0,
            100.0,
            "also wide",
            3,
            1,
        ),
        butterfly(50.0, 50.0, 100.0, "left is wide", "right is wide", 3, 1),
    ] {
        assert_shape(&output, 3, 1);
    }
}

#[cfg(feature = "ratatui")]
#[test]
fn ratatui_conversion_asks_the_host_for_every_style() {
    use std::cell::Cell;

    let output = WidgetOutput::new(vec![WidgetLine::new(vec![
        Run::new("ok", Tone::Healthy),
        Run::new("!", Tone::Critical),
    ])]);
    let calls = Cell::new(0);
    let lines = newtui::ratatui_lines(&output, |_| {
        calls.set(calls.get() + 1);
        ratatui::style::Style::default()
    });
    assert_eq!(calls.get(), 2);
    assert_eq!(lines[0].spans.len(), 2);
}
