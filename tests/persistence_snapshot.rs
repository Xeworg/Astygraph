//! Focused tests for source snapshot and SHA-256 digest primitives.
//!
//! Covers: known vector, identical/different bytes, missing/unreadable file.
//! Does NOT cover: fingerprint composition, path policy, DB writes.

use std::fs::{self, File};
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
    use astynex::persistence::snapshot::SourceDigest;

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
    use astynex::persistence::snapshot::SourceDigest;

    let bytes: Vec<u8> = (0..1_000_000).map(|_| b'a').collect();
    let digest = SourceDigest::compute_from_bytes(&bytes);
    let expected = "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0";
    assert_eq!(digest.as_hex(), expected);
}

/// Round-trip: file → snapshot → digest → hex → identical across reads
#[test]
fn known_vector_file_roundtrip() {
    use astynex::persistence::snapshot::SourceSnapshot;

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
    use astynex::persistence::snapshot::SourceSnapshot;

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
    use astynex::persistence::snapshot::SourceSnapshot;

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

    let temp = TempDir::new().unwrap();
    let missing = temp.path().join("not-created.txt");
    let result = SourceSnapshot::from_path(&missing);
    assert!(matches!(
        result,
        Err(astynex::persistence::Error::SnapshotRead { path, .. }) if path.contains("not-created.txt")
    ));
}

#[cfg(unix)]
#[test]
fn unreadable_file_returns_permission_error() {
    use astynex::persistence::snapshot::SourceSnapshot;

    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("unreadable.txt");
    fs::write(&file_path, b"sensitive").unwrap();

    // Unix mode bits provide a reliable denied-read fixture on this platform.
    let original_permissions = fs::metadata(&file_path).unwrap().permissions();
    let mut denied_permissions = original_permissions.clone();
    denied_permissions.set_mode(0o000);
    fs::set_permissions(&file_path, denied_permissions).unwrap();

    let result = SourceSnapshot::from_path(&file_path);
    // Restore before asserting so cleanup occurs even when the expectation fails.
    let _ = fs::set_permissions(&file_path, original_permissions);
    assert!(
        matches!(
            result,
            Err(astynex::persistence::Error::SnapshotRead { .. })
        ),
        "unreadable file should return SnapshotRead"
    );
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
    assert!(set.insert(d1));
    assert!(!set.insert(d1)); // duplicate rejected
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

// ─── end-to-end smoke test ───────────────────────────────────────────────────

/// Smoke test: SourceSnapshot + DB open integration.
///
/// Verifies:
/// - Snapshot digest is deterministic for the known sample payload.
/// - Source file bytes remain unchanged and readable.
/// - Fresh DB schema version is 0.
/// - In this fresh, empty-schema DB open operation, raw sample bytes were not copied into the DB file.
#[test]
fn smoke_snapshot_and_db_open() {
    use astynex::persistence::open_cache_db;
    use astynex::persistence::snapshot::SourceSnapshot;

    // Unique payload unlikely to appear elsewhere in DB.
    const PAYLOAD: &[u8] = b"SMOKE_TEST_PAYLOAD_7f3a9c42_XYZW_EOF";

    // Pre-computed SHA-256 of PAYLOAD for deterministic assertion.
    // Reproduce with: printf %s 'SMOKE_TEST_PAYLOAD_7f3a9c42_XYZW_EOF' | sha256sum
    const EXPECTED_DIGEST: &str =
        "21805adbe9f5d0fd4831beab19e072abd701220211cbd5a4e4594e367fad5bdf";

    let project = TempDir::new().unwrap();

    // Write sample source file.
    let source_path = project.path().join("sample.asty");
    fs::write(&source_path, PAYLOAD).expect("sample file must be writable");

    // Build snapshot from source.
    let snap =
        SourceSnapshot::from_path(&source_path).expect("snapshot must be created from sample file");

    // Assert known digest.
    assert_eq!(
        snap.digest().as_hex(),
        EXPECTED_DIGEST,
        "snapshot digest must match pre-computed SHA-256 of payload"
    );

    // Assert source bytes unchanged and readable.
    let read_back = fs::read(&source_path).expect("sample file must be re-readable");
    assert_eq!(&read_back, PAYLOAD, "source bytes must be unchanged");
    assert_eq!(snap.bytes(), PAYLOAD, "snapshot must hold identical bytes");

    // Open cache DB in project directory.
    let conn = open_cache_db(project.path()).expect("open_cache_db must succeed on fresh project");

    // Assert DB user_version is 0.
    let user_version: i32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("user_version pragma must be queryable");
    assert_eq!(user_version, 0, "fresh DB schema version must be 0");

    // Assert DB file does not contain the unique payload bytes.
    let db_path = project.path().join(".astynex").join("cache.db");
    let db_bytes = fs::read(&db_path).expect("DB file must exist after open");
    assert!(
        !db_bytes.windows(PAYLOAD.len()).any(|w| w == PAYLOAD),
        "fresh DB open must not copy the sample source bytes into the database"
    );
}
