//! Optional Mermaid reports and pure terminal viewports.
//!
//! The non-default member requires Rust 1.95; `newtui` stays a dependency-free
//! core at Rust 1.88. Merman owns syntax, semantic models, layout and resource
//! checks. This adapter retains its exact source and complete typed result,
//! then projects that result into the core's bounded widget vocabulary.
//!
//! Rendering occurs once. Resizing and scrolling a [`MermaidViewport`] never
//! parse or lay out the diagram again. No component, fingerprint, terminal,
//! host I/O, cache, or global identity is introduced here.
//!
//! ```
//! use newtui_mermaid::{backend, mermaid, MermaidDocument};
//! let document = MermaidDocument::render(
//!     "flowchart LR\nA[Start] --> B[Done]",
//!     &backend::Renderer::new(),
//!     backend::OperationControl::new(),
//!     backend::ascii::AsciiResourcePolicy::default(),
//! );
//! assert!(document.result().is_ok());
//! let cells = mermaid(&document, 8, 1);
//! cells.validate(8, 1).unwrap();
//! assert!(!cells.notices.is_empty());
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod projection;

/// The exact-pinned backend API for caller-owned policies and operation control.
pub use merman as backend;
pub use projection::MermaidViewport;

use merman::ascii::{AsciiOutput, AsciiRenderOptions, AsciiResourcePolicy, AsciiViewportPolicy};
use merman::{AsciiRequest, OperationControl, RenderError, RenderOutput, RenderRequest, Renderer};
use newtui::{WidgetContentState, WidgetOutput};
use projection::ProjectedText;

/// Original Mermaid source together with the complete result of one operation.
///
/// Even invalid, unsupported, cancelled, or resource-limited input retains its
/// exact bytes. Detailed errors remain typed and are never placed in a
/// `newtui::View`. An empty valid report is distinct from a parse failure.
#[derive(Debug)]
pub struct MermaidDocument {
    source: String,
    result: Result<Option<AsciiOutput>, RenderError>,
    projected: ProjectedText,
}

impl MermaidDocument {
    /// Parse and render once under the caller's policies and operation control.
    ///
    /// The supplied renderer owns input-resource and parse policies. `resources`
    /// is passed unchanged to the ASCII backend, and `control` owns cancellation
    /// or an optional cooperative deadline. This adapter does not start a new
    /// deadline or consult a terminal. Output uses plain ASCII structure and
    /// unrestricted `Allow`; the viewport reports its own cell bounds later.
    #[must_use]
    pub fn render(
        source: impl Into<String>,
        renderer: &Renderer,
        control: OperationControl,
        resources: AsciiResourcePolicy,
    ) -> Self {
        let source = source.into();
        let request = AsciiRequest {
            options: AsciiRenderOptions::ascii(),
            resources,
            viewport: AsciiViewportPolicy::unrestricted(),
        };
        let result = match renderer.render(RenderRequest::ascii(&source, control, request)) {
            Ok(RenderOutput::Ascii(report)) => Ok(report),
            // The request above fixes the output target. Keep this branch
            // fallible if the backend ever widens the dispatch contract.
            Ok(_) => Err(RenderError::UnsupportedTarget("ascii")),
            Err(error) => Err(error),
        };
        let projected = match &result {
            Ok(Some(report)) => ProjectedText::new(&report.text),
            _ => ProjectedText::default(),
        };
        Self {
            source,
            result,
            projected,
        }
    }

    /// Exact original source, unaffected by rendering, errors or projection.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Complete backend text/metrics or the original typed failure.
    #[must_use]
    pub fn result(&self) -> &Result<Option<AsciiOutput>, RenderError> {
        &self.result
    }

    /// Create a pure viewport over the already prepared result.
    #[must_use]
    pub fn viewport(&self) -> MermaidViewport<'_> {
        MermaidViewport::new(self)
    }

    fn content_state(&self) -> Option<WidgetContentState> {
        use WidgetContentState::{
            Cancelled, Empty, Failed, InvalidInput, ResourceLimited, Unsupported,
        };
        match &self.result {
            Ok(Some(report)) if !report.text.is_empty() => None,
            Ok(_) => Some(Empty),
            Err(RenderError::Parse(_) | RenderError::NoDiagram) => Some(InvalidInput),
            Err(RenderError::Cancelled(_)) => Some(Cancelled),
            Err(RenderError::ResourceLimitExceeded(_)) => Some(ResourceLimited),
            Err(RenderError::UnsupportedTarget(_)) => Some(Unsupported),
            Err(RenderError::Ascii(
                merman::ascii::AsciiError::UnsupportedDiagram { .. }
                | merman::ascii::AsciiError::UnsupportedFeature { .. },
            )) => Some(Unsupported),
            Err(_) => Some(Failed),
        }
    }
}

/// Project an existing document at the origin, without repeating backend work.
#[must_use]
pub fn mermaid(document: &MermaidDocument, width: usize, height: usize) -> WidgetOutput {
    document.viewport().render(width, height)
}
