//! Astynex core library.
//!
//! Smoke placeholder — real structural-analysis features are post-bootstrap.

/// Canonical application name, used in identity and telemetry surfaces.
pub const APPLICATION_NAME: &str = "Astynex";

/// Filesystem access layer — safe project-file discovery.
pub mod fs;
/// UI-independent graph model boundary for future rendering layers.
pub mod graph;
/// Internationalization — Spanish/English locale-aware UI strings.
pub mod i18n;
/// Heroicons outline SVG assets for essential desktop UI actions.
pub mod icons;
/// Text/code viewer layer — line-numbered viewing with UTF-8 handling.
pub mod viewer;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod app;
pub mod app_state;
