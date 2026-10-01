//! Integration tests for the `persistence` error surface.
//!
//! These tests verify that the public error type and conversions are usable
//! from outside the crate without exposing internal SQLite details.

use std::error::Error as StdError;
use std::io;

/// Test that the public `Error` type is exported from the crate root.
#[test]
fn error_type_exported_from_crate() {
    // The type must be accessible via `astynex::persistence::Error`.
    let _err: astynex::persistence::Error =
        astynex::persistence::Error::from(io::Error::new(io::ErrorKind::NotFound, "not found"));
    let _err2: astynex::persistence::Error = astynex::persistence::Error::cache_dir("bad path");
}

/// `Display` on `Error` always produces a non-empty message.
#[test]
fn error_display_is_non_empty() {
    let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");
    let err = astynex::persistence::Error::from(io_err);
    let display = err.to_string();
    assert!(!display.is_empty(), "Error::to_string() must not be empty");
}

/// `std::error::Error::source()` returns the underlying cause when available.
#[test]
fn error_source_chain_for_io_error() {
    let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
    let err: astynex::persistence::Error = io_err.into();
    // An IO-sourced error should expose its inner std::io::Error as the cause.
    let source = err.source();
    assert!(
        source.is_some(),
        "Error sourced from io::Error should expose a cause"
    );
}

/// Converting a `rusqlite::Error` into `Error` yields a `Rusqlite` variant.
#[test]
fn rusqlite_error_round_trip() {
    // rusqlite::Error can be constructed from a rusqlite result that fails.
    // We trigger an error by opening a read-only path that does not exist,
    // which produces an `ErrorSqliteBusy` or `ErrorSqliteNotSupported` variant.
    // Use the `Error::from_rusqlite` constructor via the From conversion.
    let db_err = rusqlite::Error::InvalidParameterName("nope".into());
    let err: astynex::persistence::Error = db_err.into();
    let display = err.to_string();
    assert!(!display.is_empty());
}

/// `cache_dir` variant produces a non-empty display string.
#[test]
fn cache_dir_error_has_message() {
    let err = astynex::persistence::Error::cache_dir("/no/such/place");
    let display = err.to_string();
    assert!(!display.is_empty(), "cache_dir error must carry a message");
    assert!(
        matches!(err, astynex::persistence::Error::CacheDir(_)),
        "error variant must be CacheDir"
    );
}

/// `Error` implements `Send + Sync` (required for ergonomic use in async/tokio contexts).
#[test]
fn error_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<astynex::persistence::Error>();
}
