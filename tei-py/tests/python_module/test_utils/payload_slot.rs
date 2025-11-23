//! Shared payload storage for behaviour-driven tests.

use anyhow::{Context, Result};
use std::cell::RefCell;

pub struct PayloadSlot<T> {
    slot: RefCell<Option<T>>,
    description: &'static str,
}

impl<T> PayloadSlot<T> {
    pub const fn new(description: &'static str) -> Self {
        Self {
            slot: RefCell::new(None),
            description,
        }
    }

    pub fn store(&self, value: T) {
        *self.slot.borrow_mut() = Some(value);
    }

    pub fn clear(&self) {
        self.slot.borrow_mut().take();
    }
}

impl<T> PayloadSlot<T>
where
    T: Clone,
{
    pub fn load(&self) -> Result<T> {
        self.slot
            .borrow()
            .as_ref()
            .cloned()
            .with_context(|| format!("{} must be prepared before use", self.description))
    }
}
