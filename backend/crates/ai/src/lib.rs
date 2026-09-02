//! ClovaLink AI Module
//!
//! Provides AI capabilities (summarization, Q&A, embeddings) with:
//! - Provider abstraction for multiple AI backends
//! - PII redaction before sending to external APIs
//! - Usage tracking and rate limiting
//! - Compliance enforcement (HIPAA/SOX)

pub mod error;
pub mod models;
pub mod openai;
pub mod provider;
pub mod redact;
pub mod service;

pub use error::AiError;
pub use models::*;
pub use openai::OpenAiProvider;
pub use provider::{AiProvider, AiResponse};
pub use redact::RedactionService;
pub use service::AiService;
