//! Astynex core library.
//!
//! Smoke placeholder — real structural-analysis features are post-bootstrap.

/// Canonical application name, used in identity and telemetry surfaces.
pub const APPLICATION_NAME: &str = "Astynex";

/// Filesystem access layer — safe project-file discovery.
pub mod fs;
/// Text/code viewer layer — line-numbered viewing with UTF-8 handling.
pub mod viewer;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod app;
pub mod app_state;
