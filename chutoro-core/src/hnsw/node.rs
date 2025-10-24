//! Node and trimming helpers for the HNSW graph.

use std::collections::VecDeque;

use super::types::EdgeContext;

/// Stored representation of a graph node.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Node {
    neighbours: Vec<Vec<usize>>,
}

impl Node {
    /// Creates a new node with capacity for `level + 1` layers.
    #[must_use]
    pub(crate) fn new(level: usize) -> Self {
        let mut neighbours = Vec::with_capacity(level + 1);
        neighbours.resize_with(level + 1, Vec::new);
        Self { neighbours }
    }

    /// Returns the neighbours for a level if it exists.
    pub(crate) fn neighbours(&self, level: usize) -> &[usize] {
        self.neighbours.get(level).map_or(&[], Vec::as_slice)
    }

    /// Returns mutable neighbours for a level, resizing if necessary.
    pub(crate) fn neighbours_mut(&mut self, level: usize) -> &mut Vec<usize> {
        if level >= self.neighbours.len() {
            self.neighbours.resize_with(level + 1, Vec::new);
        }
        let Some(slot) = self.neighbours.get_mut(level) else {
            unreachable!("vector resized above");
        };
        slot
    }
}

/// Captures the neighbour candidates for a node that may require trimming.
#[derive(Clone, Debug)]
pub(super) struct TrimJob {
    pub(crate) node: usize,
    pub(crate) ctx: EdgeContext,
    pub(crate) candidates: Vec<usize>,
}

impl TrimJob {
    /// Prioritises the newly inserted node for validation by moving it to the front.
    pub(crate) fn prioritise(&mut self, new_node: usize) {
        if let Some(index) = self
            .candidates
            .iter()
            .position(|&candidate| candidate == new_node)
        {
            self.candidates.swap(0, index);
        }
    }
}

/// Outcome of trimming a neighbour list.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct TrimResultInternal {
    pub(crate) node: usize,
    pub(crate) ctx: EdgeContext,
    pub(crate) neighbours: Vec<usize>,
}

impl TrimResultInternal {
    /// Converts the internal representation into the public-facing result.
    pub(crate) fn into_public(self) -> super::types::TrimResult {
        super::types::TrimResult {
            node: self.node,
            level: self.ctx.level,
            neighbours: self.neighbours,
        }
    }
}

/// Maintains insertion candidates grouped by `(node, level)`.
#[derive(Default)]
pub(super) struct CandidateMap {
    entries: VecDeque<((usize, usize), Vec<usize>)>,
}

impl CandidateMap {
    pub(super) fn entry_mut(&mut self, key: (usize, usize)) -> &mut Vec<usize> {
        if let Some(position) = self
            .entries
            .iter()
            .position(|(existing, _)| *existing == key)
        {
            let Some((_, values)) = self.entries.get_mut(position) else {
                unreachable!("position derived from iterator");
            };
            return values;
        }
        self.entries.push_back((key, Vec::new()));
        let Some(entry) = self.entries.back_mut() else {
            unreachable!("entry inserted above");
        };
        &mut entry.1
    }

    pub(super) fn into_vec(self) -> Vec<(usize, usize, Vec<usize>)> {
        self.entries
            .into_iter()
            .map(|(key, values)| (key.0, key.1, values))
            .collect()
    }
}
