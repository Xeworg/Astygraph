//! Astynex core library.
//!
//! Smoke placeholder — real structural-analysis features are post-bootstrap.

/// Canonical application name, used in identity and telemetry surfaces.
pub const APPLICATION_NAME: &str = "Astynex";

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod app;
pub mod app_state;
