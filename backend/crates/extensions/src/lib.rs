//! ClovaLink Extensions API
//!
//! This crate provides the extensions system for ClovaLink, supporting:
//! - UI Extensions: Inject buttons, components, and sidebar items
//! - File Processing Extensions: Triggered when files are uploaded
//! - Automation Extensions: Run on schedule or webhook trigger

pub mod events;
pub mod manifest;
pub mod models;
pub mod permissions;
pub mod routes;
pub mod scheduler;
pub mod webhook;

pub use events::{dispatch_file_event, FileEvent};
pub use models::*;
pub use permissions::Permission;
pub use routes::*;
pub use webhook::{sign_payload, SignatureAlgorithm};
