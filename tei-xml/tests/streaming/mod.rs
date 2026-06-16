//! Shared harness and feature modules for the streaming pull-parser BDD
//! scenarios. The harness lives in [`support`], the Gherkin step definitions in
//! [`steps`], and each feature owns its scenario bindings (see [`namespace`]).

pub(crate) mod support;

mod namespace;
mod steps;
