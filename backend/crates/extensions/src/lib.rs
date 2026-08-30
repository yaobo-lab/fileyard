//! ClovaLink Extensions API
//!
//! This crate provides the extensions system for ClovaLink, supporting:
//! - UI Extensions: Inject buttons, components, and sidebar items
//! - File Processing Extensions: Triggered when files are uploaded
//! - Automation Extensions: Run on schedule or webhook trigger

pub mod models;
pub mod routes;
pub mod manifest;
pub mod webhook;
pub mod permissions;
pub mod scheduler;
pub mod events;

pub use models::*;
pub use routes::*;
pub use permissions::Permission;
pub use webhook::{sign_payload, SignatureAlgorithm};
pub use events::{dispatch_file_event, FileEvent};

