//! Proptest strategies for inline content elements.
//!
//! Generates `Inline`, `Hi`, and `Pause` values respecting validation
//! constraints. Hi elements are depth-limited to prevent stack overflow.

use proptest::prelude::*;
use tei_core::{Hi, Inline, Pause};

use super::primitives::{
    duration_strategy, pause_type_strategy, rend_strategy, text_segment_strategy,
    trimmed_text_segment_strategy,
};

/// Maximum nesting depth for Hi elements to prevent stack overflow.
const MAX_HI_DEPTH: usize = 2;

/// Generates a `Pause` with optional attributes.
pub fn pause_strategy() -> impl Strategy<Value = Pause> {
    (
        proptest::option::of(duration_strategy()),
        proptest::option::of(pause_type_strategy()),
    )
        .prop_map(|(duration, kind)| {
            let mut pause = Pause::new();
            if let Some(d) = duration {
                pause.set_duration(d);
            }
            if let Some(k) = kind {
                pause.set_kind(k);
            }
            pause
        })
}

/// Generates `Hi` elements with content, respecting the depth limit.
pub fn hi_strategy(depth: usize) -> BoxedStrategy<Hi> {
    if depth >= MAX_HI_DEPTH {
        // At max depth, only generate text content
        (
            proptest::option::of(rend_strategy()),
            prop::collection::vec(text_segment_strategy().prop_map(Inline::text), 1..=3),
        )
            .prop_map(|(rend, content)| build_hi(rend, content))
            .boxed()
    } else {
        (
            proptest::option::of(rend_strategy()),
            prop::collection::vec(inline_strategy_at_depth(depth + 1), 1..=3)
                .prop_filter("must have visible content", |v| has_visible_content(v)),
        )
            .prop_map(|(rend, content)| build_hi(rend, content))
            .boxed()
    }
}

fn build_hi(rend: Option<String>, content: Vec<Inline>) -> Hi {
    match rend {
        Some(r) => Hi::with_rend(r, content),
        None => Hi::new(content),
    }
}

fn inline_strategy_at_depth(depth: usize) -> BoxedStrategy<Inline> {
    if depth >= MAX_HI_DEPTH {
        prop_oneof![
            8 => text_segment_strategy().prop_map(Inline::text),
            2 => pause_strategy().prop_map(Inline::Pause),
        ]
        .boxed()
    } else {
        prop_oneof![
            6 => text_segment_strategy().prop_map(Inline::text),
            2 => hi_strategy(depth).prop_map(Inline::Hi),
            2 => pause_strategy().prop_map(Inline::Pause),
        ]
        .boxed()
    }
}

/// Generates `Inline` content (Text, Hi, or Pause).
pub fn inline_strategy() -> BoxedStrategy<Inline> {
    inline_strategy_at_depth(0)
}

/// Generates text-only `Inline` content (no Hi or Pause elements).
///
/// This is needed for XML round-trip testing because `quick-xml`'s serde
/// integration does not support serializing complex inline structures.
/// Uses trimmed text to avoid whitespace normalization differences.
pub fn text_only_inline_strategy() -> impl Strategy<Value = Inline> {
    trimmed_text_segment_strategy().prop_map(Inline::text)
}

/// Checks if inline content has at least one visible text segment.
pub fn has_visible_content(inlines: &[Inline]) -> bool {
    inlines.iter().any(|inline| match inline {
        Inline::Text(s) => !s.trim().is_empty(),
        Inline::Hi(hi) => has_visible_content_slice(hi.content()),
        Inline::Pause(_) => true,
    })
}

/// Slice-based version for recursive calls and test assertions.
pub fn has_visible_content_slice(inlines: &[Inline]) -> bool {
    inlines.iter().any(|inline| match inline {
        Inline::Text(s) => !s.trim().is_empty(),
        Inline::Hi(hi) => has_visible_content_slice(hi.content()),
        Inline::Pause(_) => true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;

    #[test]
    fn pause_strategy_produces_valid_pauses() {
        let mut runner = TestRunner::default();
        for _ in 0..20 {
            let pause = pause_strategy()
                .new_tree(&mut runner)
                .unwrap_or_else(|error| panic!("strategy should generate values: {error}"))
                .current();
            // Pause is always valid
            let _ = pause;
        }
    }

    #[test]
    fn inline_strategy_produces_valid_content() {
        let mut runner = TestRunner::default();
        for _ in 0..20 {
            let inline = inline_strategy()
                .new_tree(&mut runner)
                .unwrap_or_else(|error| panic!("strategy should generate values: {error}"))
                .current();

            // Verify generated inline is non-empty
            match &inline {
                Inline::Text(s) => assert!(!s.trim().is_empty(), "text must not be empty"),
                Inline::Hi(hi) => assert!(
                    has_visible_content_slice(hi.content()),
                    "hi must have visible content"
                ),
                Inline::Pause(_) => {}
            }
        }
    }

    fn max_depth(inlines: &[Inline]) -> usize {
        inlines
            .iter()
            .map(|inline| match inline {
                Inline::Hi(hi) => 1 + max_depth(hi.content()),
                _ => 0,
            })
            .max()
            .unwrap_or(0)
    }

    #[test]
    fn hi_strategy_respects_depth_limit() {
        let mut runner = TestRunner::default();
        for _ in 0..20 {
            let hi = hi_strategy(0)
                .new_tree(&mut runner)
                .unwrap_or_else(|error| panic!("strategy should generate values: {error}"))
                .current();

            let depth = max_depth(hi.content());
            assert!(
                depth <= MAX_HI_DEPTH,
                "hi nesting depth {depth} exceeds limit {MAX_HI_DEPTH}"
            );
        }
    }
}
