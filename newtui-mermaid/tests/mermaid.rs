use merman::ascii::{AsciiResourceLimitId, AsciiResourcePolicy};
use merman::{OperationControl, RenderError, Renderer};
use newtui::{NoticeVisibility, WidgetContentState, WidgetNoticeKind};
use newtui_mermaid::{MermaidDocument, mermaid};

const FAMILIES: [(&str, &str); 5] = [
    ("flowchart", "flowchart LR\nA[Start] --> B[Finish]"),
    (
        "sequence",
        "sequenceDiagram\nAlice->>Bob: Hello\nBob-->>Alice: Hi",
    ),
    (
        "state",
        "stateDiagram-v2\n[*] --> Idle\nIdle --> Running: begin\nRunning --> [*]",
    ),
    (
        "class",
        "classDiagram\nclass Animal {\n+String name\n+eat()\n}\nAnimal <|-- Duck",
    ),
    (
        "er",
        "erDiagram\nCUSTOMER ||--o{ ORDER : places\nCUSTOMER {\nstring name\n}",
    ),
];

fn render(source: &str) -> MermaidDocument {
    MermaidDocument::render(
        source,
        &Renderer::new(),
        OperationControl::new(),
        AsciiResourcePolicy::default(),
    )
}

// GUARD: five_families_keep_the_complete_report_and_exact_viewport — executed by the member's mutation runner.
#[test]
fn five_families_keep_the_complete_report_and_exact_viewport() {
    for (family, source) in FAMILIES {
        let document = render(source);
        let report = document.result().as_ref().unwrap().as_ref().unwrap();
        assert!(!report.text.is_empty(), "{family} must render real content");
        assert!(report.primary_extent.width > 0 && report.primary_extent.height > 0);
        let original_report = report.text.clone();
        for (width, height) in [(0, 0), (0, 1), (1, 0), (1, 1), (8, 1), (80, 24), (120, 40)] {
            let output = mermaid(&document, width, height);
            output.validate(width, height).unwrap();
            // A one-row viewport reserves that row for the notice caption;
            // omitted rows then describe the whole diagram. Column windows
            // describe rows that actually have a body slot.
            if width > 0 && width < report.emitted_extent.width && height > 1 {
                assert!(
                    output
                        .notices
                        .iter()
                        .any(|n| matches!(n.kind, WidgetNoticeKind::ClippedColumns { .. })),
                    "{family} width {width} lost overflow"
                );
            }
            if height < report.emitted_extent.height {
                assert!(
                    output
                        .notices
                        .iter()
                        .any(|n| matches!(n.kind, WidgetNoticeKind::OmittedRows { .. })),
                    "{family} height {height} lost overflow"
                );
            }
            if width == 0 || height == 0 {
                assert!(
                    output
                        .notices
                        .iter()
                        .all(|n| n.visibility == NoticeVisibility::Hidden)
                );
            }
            if width == 120 && height == 40 {
                assert!(
                    output
                        .lines
                        .iter()
                        .any(|line| line.text().chars().any(|c| c.is_ascii_alphanumeric())),
                    "{family} preview is blank"
                );
            }
        }
        assert_eq!(document.source().as_bytes(), source.as_bytes());
        assert_eq!(
            document.result().as_ref().unwrap().as_ref().unwrap().text,
            original_report
        );
    }
}

// GUARD: invalid_input_is_a_typed_failure_never_empty_success — executed by the member's mutation runner.
#[test]
fn invalid_input_is_a_typed_failure_never_empty_success() {
    let source = "flowchart TD\nA[unterminated --> B";
    let invalid = render(source);
    let Err(RenderError::Parse(error)) = invalid.result() else {
        panic!("parse failure was suppressed")
    };
    assert!(error.to_string().contains("Unterminated"));
    let details = error.terminal_diagnostic_details();
    assert_eq!(details.code, "merman.parse.diagram_parse");
    let span = details.span.expect("the backend's exact span survives");
    assert_eq!((span.start, span.end), (14, 33));
    assert_eq!(invalid.source(), source);
    for (width, height) in [(0, 0), (1, 1), (8, 1), (80, 3)] {
        let output = mermaid(&invalid, width, height);
        output.validate(width, height).unwrap();
        assert!(output.notices.iter().any(|n| n.kind
            == WidgetNoticeKind::ContentState {
                state: WidgetContentState::InvalidInput
            }));
        assert!(!output.notices.iter().any(|n| n.kind
            == WidgetNoticeKind::ContentState {
                state: WidgetContentState::Empty
            }));
        if width > 0 && height > 0 {
            assert!(
                output
                    .lines
                    .iter()
                    .any(|line| !line.text().trim().is_empty())
            );
        }
    }
    let empty = render("flowchart TD\n");
    assert!(
        empty
            .result()
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .text
            .is_empty()
    );
    assert!(mermaid(&empty, 0, 0).notices.iter().any(|n| n.kind
        == WidgetNoticeKind::ContentState {
            state: WidgetContentState::Empty
        }));
    assert!(
        render("").result().is_err(),
        "absent language must not become a valid empty graph"
    );
}

#[test]
fn supplied_controls_and_limits_keep_their_typed_failures() {
    use merman::resources::{InputResourceLimitId, InputResourcePolicy};
    let control = OperationControl::new();
    control.cancel();
    let cancelled = MermaidDocument::render(
        FAMILIES[0].1,
        &Renderer::new(),
        control,
        AsciiResourcePolicy::default(),
    );
    assert!(matches!(cancelled.result(), Err(RenderError::Cancelled(_))));
    assert!(mermaid(&cancelled, 0, 0).notices.iter().any(|n| n.kind
        == WidgetNoticeKind::ContentState {
            state: WidgetContentState::Cancelled
        }));
    let resources = AsciiResourcePolicy::default()
        .with_limit(AsciiResourceLimitId::MaxOutputBytes, 32)
        .unwrap();
    let limited = MermaidDocument::render(
        FAMILIES[0].1,
        &Renderer::new(),
        OperationControl::new(),
        resources,
    );
    assert!(matches!(
        limited.result(),
        Err(RenderError::ResourceLimitExceeded(_))
    ));
    assert!(mermaid(&limited, 0, 0).notices.iter().any(|n| n.kind
        == WidgetNoticeKind::ContentState {
            state: WidgetContentState::ResourceLimited
        }));
    let unsupported = render("pie\n\"A\" : 5\n\"B\" : 2");
    assert!(mermaid(&unsupported, 0, 0).notices.iter().any(|n| n.kind
        == WidgetNoticeKind::ContentState {
            state: WidgetContentState::Unsupported
        }));
}

#[test]
fn scrolling_reprojects_without_changing_source_or_report() {
    let document = render(FAMILIES[2].1);
    let text = document
        .result()
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .text
        .clone();
    let output = document
        .viewport()
        .row_offset(4)
        .column_offset(2)
        .render(8, 3);
    output.validate(8, 3).unwrap();
    assert!(
        output
            .notices
            .iter()
            .any(|n| matches!(n.kind, WidgetNoticeKind::OmittedRows { before: 4, .. }))
    );
    assert!(
        output
            .notices
            .iter()
            .any(|n| matches!(n.kind, WidgetNoticeKind::ClippedColumns { before: 2, .. }))
    );
    let beyond = document
        .viewport()
        .row_offset(usize::MAX)
        .column_offset(usize::MAX)
        .render(1, 1);
    beyond.validate(1, 1).unwrap();
    assert_eq!(document.source(), FAMILIES[2].1);
    assert_eq!(
        document.result().as_ref().unwrap().as_ref().unwrap().text,
        text
    );
    let renderer = Renderer::new().with_resource_policy(
        InputResourcePolicy::default()
            .with_limit(InputResourceLimitId::MaxSourceBytes, 16)
            .unwrap(),
    );
    let source_limited = MermaidDocument::render(
        FAMILIES[0].1,
        &renderer,
        OperationControl::new(),
        AsciiResourcePolicy::default(),
    );
    assert!(matches!(
        source_limited.result(),
        Err(RenderError::ResourceLimitExceeded(_))
    ));
    assert_eq!(source_limited.source(), FAMILIES[0].1);
}

#[test]
fn a_large_diagram_keeps_all_rows_when_the_viewport_is_one_cell() {
    let mut source = String::from("flowchart TD\n");
    for node in 0..20 {
        source.push_str(&format!("N{node} --> N{}\n", node + 1));
    }
    let control = OperationControl::new();
    let document = MermaidDocument::render(
        &source,
        &Renderer::new(),
        control.clone(),
        AsciiResourcePolicy::default(),
    );
    let report = document.result().as_ref().unwrap().as_ref().unwrap();
    assert!(report.emitted_extent.height > 100);
    // Rendering has finished. Later viewport operations use the retained
    // report, independently of the operation's cancellation state.
    control.cancel();
    let output = mermaid(&document, 1, 1);
    output.validate(1, 1).unwrap();
    assert!(output.notices.iter().any(|n| n.kind
        == WidgetNoticeKind::OmittedRows {
            before: 0,
            after: report.emitted_extent.height
        }));
    assert!(
        output
            .notices
            .iter()
            .all(|n| n.visibility == NoticeVisibility::Indicator)
    );
    assert_eq!(document.source(), source);
}

// GUARD: authored_unicode_survives_with_measured_glyph_notices — executed by the member's mutation runner.
#[test]
fn authored_unicode_survives_with_measured_glyph_notices() {
    let source = "flowchart LR\nA[日本語 café] --> B[Done]\n";
    let document = render(source);
    let report = document.result().as_ref().unwrap().as_ref().unwrap();
    assert!(report.text.contains("日本語"));
    let output = mermaid(&document, 80, 15);
    output.validate(80, 15).unwrap();
    assert!(
        output
            .notices
            .iter()
            .any(|n| matches!(n.kind, WidgetNoticeKind::GlyphReplacements { count: 4 }))
    );
    assert_eq!(document.source().as_bytes(), source.as_bytes());
    assert!(
        mermaid(&document, 0, 0)
            .notices
            .iter()
            .any(|n| matches!(n.kind, WidgetNoticeKind::GlyphReplacements { count: 4 }))
    );
}
