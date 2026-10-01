//! Persistence layer — SQLite cache with typed error boundary.
//!
//! # Scope
//!
//! This module provides the headless persistence foundation for Astynex:
//! - Path resolution and cache-directory creation (`.astynex/cache.db`).
//! - SQLite open/configuration with per-connection integrity settings.
//! - Atomic ordered migration framework via `PRAGMA user_version`.
//!
//! No source bytes, prompts, raw provider responses, or credentials are ever
//! written to the database.  Source hashing, parser/analysis schema, and
//! graph integration are handled by higher-level modules.
//!
//! # Error model
//!
//! Every fallible operation returns a typed [`Error`] that never panics and
//! preserves the underlying cause.  Callers convert to user-facing diagnostics
//! at the application boundary.

// ─── types ────────────────────────────────────────────────────────────────────

mod error;

/// Typed persistence errors.  See [`error`] for variants and conversions.
pub use error::Error;
