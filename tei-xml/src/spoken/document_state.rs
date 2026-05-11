//! TEI document-shell state machine for spoken-text extraction.

use tei_core::TeiError;

use super::observability;

/// Tracks the expected TEI document-shell parsing phase.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct DocumentState {
    phase: DocumentPhase,
}

/// Ordered parser phases for the required TEI shell.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum DocumentPhase {
    /// No document root has been accepted yet.
    #[default]
    Start,
    /// The document root is `<TEI>`.
    SawTei,
    /// The root-level `<teiHeader>` has been accepted.
    SawHeader,
    /// The root-level `<text>` has been accepted after `<teiHeader>`.
    SawText,
    /// The `<body>` child of root-level `<text>` has been accepted.
    SawBody,
}

impl DocumentState {
    /// Returns the current phase for diagnostics.
    pub(super) const fn phase(self) -> DocumentPhase {
        self.phase
    }

    /// Records that the document root is the expected `<TEI>` element.
    pub(super) fn record_tei_root(&mut self, is_document_root: bool) -> Result<(), TeiError> {
        let ok = is_document_root && self.phase == DocumentPhase::Start;
        self.advance_phase_if(
            ok,
            DocumentPhase::SawTei,
            "TEI root element must be the document root",
        )
    }

    /// Records that a root-level `<teiHeader>` has been encountered.
    pub(super) fn record_tei_header(&mut self, is_inside_tei_root: bool) -> Result<(), TeiError> {
        let ok = is_inside_tei_root && self.phase == DocumentPhase::SawTei;
        self.advance_phase_if(
            ok,
            DocumentPhase::SawHeader,
            "teiHeader element must be inside TEI root",
        )
    }

    /// Validates and records the root-level `<text>` path.
    pub(super) fn validate_text_path(&mut self, is_inside_tei_root: bool) -> Result<(), TeiError> {
        if !is_inside_tei_root {
            return Err(TeiError::xml("text element must be inside TEI root"));
        }
        if self.phase == DocumentPhase::SawTei {
            return Err(TeiError::xml("missing teiHeader element"));
        }
        if self.phase == DocumentPhase::SawHeader {
            let previous_phase = self.phase;
            self.phase = DocumentPhase::SawText;
            observability::phase_transition(previous_phase, DocumentPhase::SawText);
            Ok(())
        } else {
            Err(TeiError::xml("duplicate or misplaced text element"))
        }
    }

    /// Records that the accepted root-level `<text>` contains a `<body>`.
    pub(super) fn record_body(
        &mut self,
        is_direct_child_of_text_in_tei: bool,
    ) -> Result<(), TeiError> {
        let ok = is_direct_child_of_text_in_tei && self.phase == DocumentPhase::SawText;
        self.advance_phase_if(
            ok,
            DocumentPhase::SawBody,
            "body element must be inside TEI text",
        )
    }

    /// Returns whether the parser accepted a TEI root.
    pub(super) const fn saw_tei(self) -> bool {
        !matches!(self.phase, DocumentPhase::Start)
    }

    /// Returns whether the parser accepted a TEI header.
    pub(super) const fn saw_header(self) -> bool {
        matches!(
            self.phase,
            DocumentPhase::SawHeader | DocumentPhase::SawText | DocumentPhase::SawBody
        )
    }

    /// Returns whether the parser accepted a TEI body.
    pub(super) const fn saw_body(self) -> bool {
        matches!(self.phase, DocumentPhase::SawBody)
    }

    fn advance_phase_if(
        &mut self,
        condition: bool,
        next: DocumentPhase,
        error: &str,
    ) -> Result<(), TeiError> {
        if condition {
            let previous_phase = self.phase;
            self.phase = next;
            observability::phase_transition(previous_phase, next);
            Ok(())
        } else {
            observability::phase_rejected(self.phase, next, error);
            Err(TeiError::xml(error))
        }
    }
}
