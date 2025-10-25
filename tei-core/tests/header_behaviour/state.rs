use anyhow::{Context, Result, ensure};
use std::cell::{RefCell, RefMut};
use tei_core::{
    EncodingDesc, HeaderValidationError, ProfileDesc, RevisionChange, RevisionDesc, TeiDocument,
};

#[derive(Default)]
pub(crate) struct HeaderState {
    title: RefCell<Option<String>>,
    profile: RefCell<ProfileDesc>,
    encoding: RefCell<EncodingDesc>,
    revision: RefCell<RevisionDesc>,
    document: RefCell<Option<TeiDocument>>,
    revision_attempt: RefCell<Option<Result<RevisionChange, HeaderValidationError>>>,
    pending_revision_description: RefCell<Option<String>>,
}

impl HeaderState {
    pub(crate) fn set_title(&self, title: String) {
        *self.title.borrow_mut() = Some(title);
    }

    pub(crate) fn title(&self) -> Result<String> {
        self.title
            .borrow()
            .as_ref()
            .cloned()
            .context("scenario must declare a document title")
    }

    pub(crate) fn profile(&self) -> ProfileDesc {
        self.profile.borrow().clone()
    }

    pub(crate) fn profile_mut(&self) -> RefMut<'_, ProfileDesc> {
        self.profile.borrow_mut()
    }

    pub(crate) fn encoding(&self) -> EncodingDesc {
        self.encoding.borrow().clone()
    }

    pub(crate) fn encoding_mut(&self) -> RefMut<'_, EncodingDesc> {
        self.encoding.borrow_mut()
    }

    pub(crate) fn revision(&self) -> RevisionDesc {
        self.revision.borrow().clone()
    }

    pub(crate) fn revision_mut(&self) -> RefMut<'_, RevisionDesc> {
        self.revision.borrow_mut()
    }

    pub(crate) fn set_document(&self, document: TeiDocument) {
        *self.document.borrow_mut() = Some(document);
    }

    pub(crate) fn document(&self) -> Result<TeiDocument> {
        self.document
            .borrow()
            .as_ref()
            .cloned()
            .context("document construction must run before assertions")
    }

    pub(crate) fn set_revision_attempt(
        &self,
        attempt: Result<RevisionChange, HeaderValidationError>,
    ) {
        *self.revision_attempt.borrow_mut() = Some(attempt);
    }

    pub(crate) fn revision_attempt(&self) -> Result<Result<RevisionChange, HeaderValidationError>> {
        self.revision_attempt
            .borrow()
            .as_ref()
            .cloned()
            .context("revision attempt must run before assertions")
    }

    pub(crate) fn set_pending_revision_description(&self, description: String) {
        *self.pending_revision_description.borrow_mut() = Some(description);
    }

    pub(crate) fn pending_revision_description(&self) -> Option<String> {
        self.pending_revision_description.borrow().clone()
    }
}

pub(crate) fn build_state() -> Result<HeaderState> {
    let state = HeaderState::default();
    ensure!(
        state.title.borrow().is_none(),
        "fresh state should not carry a title"
    );
    ensure!(
        state.document.borrow().is_none(),
        "fresh state should not carry a document"
    );
    ensure!(
        state.revision_attempt.borrow().is_none(),
        "fresh state should not carry revision attempts"
    );
    Ok(state)
}
