//! # Snurr
//!
//! `Snurr` can run the process flow from a Business Process Model and Notation (BPMN) 2.0 file created by <https://demo.bpmn.io/new>.
//!
//! - Add your own behavior with Rust code from a small API. The wiring is already setup from the file.
//! - Easy to update the BPMN diagram with new Task and Gateways without the need to refactor your old code.
//! - The BPMN file is the actual design. Forget outdated documentation.
//! - Scaffold the initial BPMN diagram so you don't have to do the boilerplate code.
//! - Contains no database.
//! - Single or multithreaded (opt in)
//!
//! This is not a complete implementation of the BPMN 2.0 specification but intend to be a light weight subset of it.
//!
//! ## Example
//!
//! ### Cargo.toml
//! ```toml
//! [dependencies]
//! snurr = "0.13"
//! log = "0.4"
//! pretty_env_logger = "0.5"
//! ```
//! ### main.rs
//!
//! ```
//! use snurr::{Error, Process};
//!
//! extern crate pretty_env_logger;
//!
//! #[derive(Debug, Default)]
//! struct Counter {
//!     count: u32,
//! }
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     pretty_env_logger::init();
//!
//!     // Create process from BPMN file
//!     let bpmn = Process::<Counter>::new("examples/example.bpmn")?
//!         .task("Count 1", |input| {
//!             let mut data = input.lock().unwrap();
//!             // You can stop process execution with custom errors
//!             if data.count > 100 {
//!                 return Err(Error::ProcessExecution("Count exceeded maximum".into()));
//!             }
//!             data.count += 1;
//!             Ok(None)
//!         })
//!         .exclusive("equal to 3", |input| {
//!             match input.lock().unwrap().count {
//!                 3 => Ok(Some("YES")),
//!                 _ => Ok(Some("NO")),
//!             }
//!         })
//!         .build()?;
//!
//!     // Run the process with input data
//!     let result = bpmn.run(Counter::default())?;
//!
//!     // Print the result.
//!     println!("Count: {}", result.data.count);
//!     println!("Ended at: {}", result.end_node.id);
//!     Ok(())
//! }
//! ```

mod api;
mod bpmn;
mod diagram;
mod error;
mod process;

pub use api::{Boundary, Data, EndNode, IntermediateEvent, ProcessOutput, TaskResult, With};
pub use bpmn::Symbol;
pub use error::{Error, Result};
pub use process::{Build, Process, Run};
