use newtui::{butterfly_history, Tone, WidgetOutput};

fn bars(output: &WidgetOutput) -> Vec<(usize, usize)> {
    output
        .lines
        .iter()
        .map(|line| {
            let text = line.text();
            let (left, right) = text.split_once('|').expect("a stable time axis");
            (
                left.chars().filter(|glyph| *glyph != ' ').count(),
                right.chars().filter(|glyph| *glyph != ' ').count(),
            )
        })
        .collect()
}

// GUARD: butterfly_history_preserves_time_order_and_newest_alignment — this is a guard; tests/mutations.rs must show it red.
#[test]
fn butterfly_history_preserves_time_order_and_newest_alignment() {
    let output = butterfly_history(&[100.0, 0.0, 50.0], &[25.0, 100.0], 100.0, 9, 4);
    output.validate(9, 4).unwrap();
    assert_eq!(bars(&output), [(0, 0), (4, 0), (0, 1), (2, 4)]);

    let cropped = butterfly_history(&[100.0, 0.0, 50.0], &[25.0, 100.0], 100.0, 9, 2);
    assert_eq!(bars(&cropped), [(0, 1), (2, 4)]);

    // Appending a new pair moves each retained old pair up exactly one row.
    let advanced = butterfly_history(&[100.0, 0.0, 50.0, 75.0], &[25.0, 100.0, 0.0], 100.0, 9, 4);
    assert_eq!(bars(&advanced), [(4, 0), (0, 1), (2, 4), (3, 0)]);
    assert_eq!(advanced.lines[..3], output.lines[1..]);
}

// GUARD: butterfly_history_keeps_independent_wings_on_a_shared_scale — this is a guard; tests/mutations.rs must show it red.
#[test]
fn butterfly_history_keeps_independent_wings_on_a_shared_scale() {
    let output = butterfly_history(&[25.0, 100.0, 0.0], &[75.0, 0.0, 100.0], 100.0, 9, 3);
    assert_eq!(bars(&output), [(1, 3), (4, 0), (0, 4)]);
    let doubled = butterfly_history(&[25.0, 100.0, 0.0], &[75.0, 0.0, 100.0], 200.0, 9, 3);
    assert_eq!(bars(&doubled), [(1, 2), (2, 0), (0, 2)]);

    for width in 1..=32 {
        let full = butterfly_history(&[100.0], &[100.0], 100.0, width, 1);
        full.validate(width, 1).unwrap();
        let text: Vec<char> = full.lines[0].text().chars().collect();
        let center = (width - 1).div_ceil(2);
        assert_eq!(text[center], '|');
        assert_eq!(bars(&full), [(center, width - center - 1)]);
        assert!(full.lines[0]
            .runs
            .iter()
            .any(|run| run.text == "|" && run.tone == Tone::Accent));
    }
    // The bars touch the axis and leave unused capacity at the outer edges.
    assert_eq!(output.lines[0].text(), "   █|██▒ ");
}

#[test]
fn butterfly_history_handles_empty_nonfinite_and_tiny_inputs() {
    let domain = [
        0.0,
        -10.0,
        25.0,
        100.0,
        200.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ];
    for maximum in [100.0, 0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for (width, height) in [(0, 0), (0, 5), (1, 0), (1, 3), (2, 1), (9, 8), (12, 10)] {
            for (left, right) in [
                (&domain[..], &[][..]),
                (&[][..], &domain[..]),
                (&domain[..], &domain[..]),
            ] {
                butterfly_history(left, right, maximum, width, height)
                    .validate(width, height)
                    .unwrap();
            }
        }
        if maximum != 100.0 {
            assert_eq!(
                bars(&butterfly_history(&domain, &domain, maximum, 9, 8)),
                [(0, 0); 8]
            );
        }
    }
    assert_eq!(
        bars(&butterfly_history(&domain, &domain, 100.0, 9, 8)),
        [
            (0, 0),
            (0, 0),
            (1, 1),
            (4, 4),
            (4, 4),
            (0, 0),
            (0, 0),
            (0, 0)
        ]
    );
    assert_eq!(bars(&butterfly_history(&[], &[], 100.0, 9, 3)), [(0, 0); 3]);
}
