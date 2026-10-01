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
//! # Public API
//!
//! - [`cache_db_path`] — resolve `<project>/.astynex/cache.db`, creating the
//!   directory if absent.
//! - [`open_cache_db`] — open (or create) the SQLite cache with integrity
//!   pragmas and the ordered migration framework.
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

// ─── public API surface ───────────────────────────────────────────────────────

/// SQLite open lifecycle and per-connection pragmas.
pub mod open;
/// Cache path resolution.
pub mod paths;
/// Ordered migration framework via `PRAGMA user_version`.
mod schema;

// Re-export the two primary public entry points at crate level so callers
// can use `persistence::cache_db_path` and `persistence::open_cache_db`.
pub use open::open_cache_db;
pub use paths::cache_db_path;
