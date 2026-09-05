#![cfg(feature = "ratatui")]

// The test imports the executable host because the release claim belongs to
// that host, not to a second renderer made only for tests.
#[allow(dead_code)]
#[path = "../examples/demo.rs"]
mod demo;

// GUARD: every_recorded_widget_has_visible_content — this is a guard; tests/mutations.rs must show it red.
#[test]
fn every_recorded_widget_has_visible_content() {
    demo::assert_recorded_widgets_render_content();
}
