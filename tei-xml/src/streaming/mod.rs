//! Streaming pull parser for incremental TEI document processing.
//!
//! This module provides [`TeiPullParser`], an iterator-based parser that yields
//! high-level domain events as it processes a TEI document. This allows handling
//! of very large documents without loading the entire content into memory.
//!
//! # Feature Flag
//!
//! This module is only available when the `streaming` feature is enabled:
//!
//! ```toml
//! [dependencies]
//! tei-xml = { version = "0.1", features = ["streaming"] }
//! ```
//!
//! # Examples
//!
//! ```no_run
//! use std::io::BufReader;
//! use std::fs::File;
//! use tei_xml::streaming::{TeiPullParser, TeiEvent};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let file = File::open("large-episode.tei.xml")?;
//!     let reader = BufReader::new(file);
//!
//!     for event in TeiPullParser::new(reader) {
//!         match event? {
//!             TeiEvent::DocumentStart => println!("Parsing started"),
//!             TeiEvent::Header(header) => {
//!                 println!("Title: {}", header.file_desc().title().as_str());
//!             }
//!             TeiEvent::BodyBlock(block) => println!("Received block: {block:?}"),
//!             TeiEvent::DocumentEnd => println!("Parsing complete"),
//!         }
//!     }
//!     Ok(())
//! }
//! ```

mod event;
mod helpers;
mod parser;
mod state;

pub use event::TeiEvent;
pub use parser::TeiPullParser;
