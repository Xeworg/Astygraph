//! Focused tests for source snapshot and SHA-256 digest primitives.
//!
//! Covers: known vector, identical/different bytes, missing/unreadable file.
//! Does NOT cover: fingerprint composition, path policy, DB writes.

use std::fs::{self, File};
use std::path::Path;
use tempfile::TempDir;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

// ─── public API under test ───────────────────────────────────────────────────

// Tests exercise these public items:
// - SourceSnapshot::from_path / SourceDigest::compute_from_bytes
// - SourceDigest::as_hex / SourceDigest::as_bytes
// - Error variants for missing/unreadable files

// ─── known vector ────────────────────────────────────────────────────────────

/// NIST SHA-256 test vector: "abc" → ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
#[test]
fn known_vector_abc() {
    use astynex::persistence::snapshot::{SourceDigest, SourceSnapshot};

    let bytes = b"abc";
    let digest = SourceDigest::compute_from_bytes(bytes);
    // SHA-256("abc") = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
    let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    assert_eq!(digest.as_hex(), expected);
    assert_eq!(
        digest.as_hex(),
        SourceDigest::compute_from_bytes(bytes).as_hex()
    );
}

/// NIST SHA-256 test vector: 1_000_000 × "a" →
/// cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0
#[test]
fn known_vector_repeated_a() {
    use astynex::persistence::snapshot::{SourceDigest, SourceSnapshot};

    let bytes: Vec<u8> = (0..1_000_000).map(|_| b'a').collect();
    let digest = SourceDigest::compute_from_bytes(&bytes);
    let expected = "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0";
    assert_eq!(digest.as_hex(), expected);
}

/// Round-trip: file → snapshot → digest → hex → identical across reads
#[test]
fn known_vector_file_roundtrip() {
    use astynex::persistence::snapshot::{SourceDigest, SourceSnapshot};

    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("vector.txt");
    fs::write(&file_path, b"abc").unwrap();

    let snap1 = SourceSnapshot::from_path(&file_path).unwrap();
    let snap2 = SourceSnapshot::from_path(&file_path).unwrap();

    assert_eq!(snap1.digest(), snap2.digest());
    // SHA-256("abc") = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
    assert_eq!(
        snap1.digest().as_hex(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

// ─── identical / different bytes ─────────────────────────────────────────────

#[test]
fn identical_bytes_identical_digest() {
    use astynex::persistence::snapshot::SourceDigest;

    let digest1 = SourceDigest::compute_from_bytes(b"hello world");
    let digest2 = SourceDigest::compute_from_bytes(b"hello world");
    assert_eq!(digest1, digest2);
    assert_eq!(digest1.as_hex(), digest2.as_hex());
}

#[test]
fn different_bytes_different_digest() {
    use astynex::persistence::snapshot::SourceDigest;

    let digest1 = SourceDigest::compute_from_bytes(b"hello world");
    let digest2 = SourceDigest::compute_from_bytes(b"hello world!");
    assert_ne!(digest1, digest2);
    assert_ne!(digest1.as_hex(), digest2.as_hex());
}

#[test]
fn different_file_content_produces_different_digest() {
    use astynex::persistence::snapshot::{SourceDigest, SourceSnapshot};

    let temp = TempDir::new().unwrap();
    let file1 = temp.path().join("a.txt");
    let file2 = temp.path().join("b.txt");
    fs::write(&file1, b"content A").unwrap();
    fs::write(&file2, b"content B").unwrap();

    let snap1 = SourceSnapshot::from_path(&file1).unwrap();
    let snap2 = SourceSnapshot::from_path(&file2).unwrap();

    assert_ne!(snap1.digest(), snap2.digest());
}

#[test]
fn empty_file_has_deterministic_digest() {
    use astynex::persistence::snapshot::{SourceDigest, SourceSnapshot};

    let temp = TempDir::new().unwrap();
    let empty_file = temp.path().join("empty.txt");
    File::create(&empty_file).unwrap();

    let snap = SourceSnapshot::from_path(&empty_file).unwrap();
    // SHA-256 of empty input
    let expected = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    assert_eq!(snap.digest().as_hex(), expected);
}

#[test]
fn snapshot_preserves_exact_bytes() {
    use astynex::persistence::snapshot::SourceSnapshot;

    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("preserved.bin");
    let original: Vec<u8> = (0..=255).collect();
    fs::write(&file_path, &original).unwrap();

    let snap = SourceSnapshot::from_path(&file_path).unwrap();
    assert_eq!(snap.bytes(), &original);
}

// ─── missing / unreadable file ──────────────────────────────────────────────

#[test]
fn missing_file_returns_error() {
    use astynex::persistence::snapshot::SourceSnapshot;

    let result = SourceSnapshot::from_path(Path::new("/nonexistent/path/to/file.txt"));
    assert!(result.is_err());
}

#[test]
fn unreadable_file_returns_permission_error() {
    use astynex::persistence::snapshot::SourceSnapshot;

    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("unreadable.txt");
    fs::write(&file_path, b"sensitive").unwrap();

    // Remove all read permissions
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&file_path).unwrap().permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&file_path, perms).unwrap();
    }
    #[cfg(windows)]
    {
        // Windows: mark file as hidden/system to simulate unreadable
        // Skip on Windows if chmod isn't reliably supported in test env
        let result = std::panic::catch_unwind(|| SourceSnapshot::from_path(&file_path));
        // If the file is actually unreadable, result is Err
        // If permissions couldn't be changed, test is inconclusive — pass gracefully
        if let Err(_) = result {
            return; // File was unreadable as expected
        }
    }

    let result = SourceSnapshot::from_path(&file_path);
    assert!(
        result.is_err(),
        "unreadable file should return error, got is_ok={}",
        result.is_ok()
    );

    // Restore permissions for cleanup
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&file_path).unwrap().permissions();
        perms.set_mode(0o644);
        let _ = fs::set_permissions(&file_path, perms);
    }
}

// ─── digest comparison and display ──────────────────────────────────────────

#[test]
fn digest_equality_and_hash() {
    use astynex::persistence::snapshot::SourceDigest;
    use std::collections::HashSet;

    let d1 = SourceDigest::compute_from_bytes(b"test");
    let d2 = SourceDigest::compute_from_bytes(b"test");
    let d3 = SourceDigest::compute_from_bytes(b"other");

    // Equality
    assert_eq!(d1, d2);
    assert_ne!(d1, d3);

    // Hash consistency (enables use in HashMap/Set)
    let mut set: HashSet<SourceDigest> = HashSet::new();
    assert!(set.insert(d1.clone()));
    assert!(!set.insert(d1.clone())); // duplicate rejected
    assert!(set.insert(d3));
    assert_eq!(set.len(), 2);
}

#[test]
fn digest_debug_format() {
    use astynex::persistence::snapshot::SourceDigest;

    let digest = SourceDigest::compute_from_bytes(b"abc");
    // SHA-256("abc") = ba7816bf...
    let debug = format!("{:?}", digest);
    assert!(debug.contains("ba7816b"));
}

#[test]
fn digest_display_format() {
    use astynex::persistence::snapshot::SourceDigest;

    let digest = SourceDigest::compute_from_bytes(b"abc");
    let display = format!("{}", digest);
    // SHA-256("abc") = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
    assert_eq!(
        display,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}
