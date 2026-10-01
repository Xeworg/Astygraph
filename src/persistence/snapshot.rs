//! Source snapshot primitives — in-memory SHA-256 digest of exact file bytes.
//!
//! This module provides deterministic file snapshots for the freshness protocol.
//! Source bytes are read once into memory and hashed; they are never written
//! to SQLite or any persistence layer.
//!
//! # Public API
//!
//! - [`SourceSnapshot::from_path`] — read a file's exact bytes into an
//!   in-memory snapshot and compute its SHA-256 digest.
//! - [`SourceDigest`] — opaque SHA-256 digest with hex/bytes access.
//! - [`SourceSnapshot::digest`] / [`SourceSnapshot::bytes`] — accessors.
//!
//! # What is NOT in this module (Task 2 scope)
//!
//! - Fingerprint composition (parse fingerprint, analysis fingerprint).
//! - Path policy / project-root normalization.
//! - Ordered context sets and membership digests.
//!
//! # Constraints
//!
//! - Bytes are read exactly once; no streaming or partial reads.
//! - No source bytes are ever written to SQLite or any persistence store.
//! - Digests are opaque; equality and hashing are the primary operations.

use std::fs;
use std::path::Path;
use std::{fmt, hash};

use sha2::{Digest, Sha256};

/// SHA-256 digest of a source snapshot.
///
/// Opaque wrapper around a 32-byte SHA-256 digest. Provides `Eq`, `Hash`,
/// `Display` (hex), and `Debug` for use as cache keys and equality witnesses.
#[derive(Clone, Copy)]
pub struct SourceDigest([u8; 32]);

impl SourceDigest {
    /// Compute SHA-256 digest over the given byte slice.
    ///
    /// The slice represents the exact bytes read from a source file;
    /// identical bytes always produce identical digests.
    #[inline]
    pub fn compute_from_bytes(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let result: [u8; 32] = hasher.finalize().into();
        Self(result)
    }

    /// Return the raw 32-byte digest.
    #[inline]
    pub fn as_bytes(&self) -> [u8; 32] {
        self.0
    }

    /// Return the lowercase hex representation (64 ASCII characters).
    #[inline]
    pub fn as_hex(&self) -> String {
        // Use the `hex` crate for portable, correct hex encoding.
        hex::encode(self.0)
    }
}

// ─── fmt / display ──────────────────────────────────────────────────────────

impl fmt::Display for SourceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_hex())
    }
}

impl fmt::Debug for SourceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Show first 7 hex chars as a recognizable prefix (like git commits).
        write!(f, "SourceDigest({:.7}..)", &self.as_hex()[..7])
    }
}

// ─── equality and hashing ────────────────────────────────────────────────────

impl PartialEq for SourceDigest {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for SourceDigest {}

impl hash::Hash for SourceDigest {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        // Hash the full 32-byte array directly — no pre-mixing needed;
        // Hash trait impl over [u8; 32] is correct and efficient.
        self.0.hash(state);
    }
}

// ─── source snapshot ─────────────────────────────────────────────────────────

/// One-time in-memory snapshot of a source file's exact bytes.
///
/// The file is read entirely into memory on construction and its SHA-256
/// digest is computed immediately. The snapshot holds both the bytes and
/// the digest for the caller's use. Bytes are never written to any
/// persistence layer.
///
/// # Example
///
/// ```ignore
/// let snap = SourceSnapshot::from_path(Path::new("src/main.rs"))?;
/// println!("digest: {}", snap.digest());
/// // Use snap.bytes() for the exact source bytes when needed.
/// ```
pub struct SourceSnapshot {
    /// Owned bytes — exact content read from the source file.
    bytes: Vec<u8>,
    /// SHA-256 of `bytes`.
    digest: SourceDigest,
}

impl SourceSnapshot {
    /// Read the exact file bytes from `path` and compute their SHA-256 digest.
    ///
    /// # Errors
    ///
    /// Returns an error if the file does not exist, is not readable, or
    /// cannot be read due to I/O failure.
    ///
    /// # Guarantees
    ///
    /// - File is read exactly once.
    /// - Bytes are stored only in memory (not persisted).
    /// - Digest is computed immediately after reading.
    pub fn from_path(path: &Path) -> Result<Self, crate::persistence::Error> {
        let bytes = fs::read(path)
            .map_err(|source| crate::persistence::Error::snapshot_read_failure(path, source))?;
        let digest = SourceDigest::compute_from_bytes(&bytes);
        Ok(Self { bytes, digest })
    }

    /// Return a reference to this snapshot's SHA-256 digest.
    #[inline]
    pub fn digest(&self) -> &SourceDigest {
        &self.digest
    }

    /// Return a reference to the exact source bytes held in this snapshot.
    ///
    /// The returned slice is identical to what was read from the file at
    /// snapshot creation time. Callers use this for operations that need
    /// the raw content (e.g., parsing).
    #[inline]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

// ─── tests inline ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_compute_from_bytes() {
        let digest = SourceDigest::compute_from_bytes(b"abc");
        // SHA-256("abc") = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
        assert_eq!(
            digest.as_hex(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn digest_equality() {
        let d1 = SourceDigest::compute_from_bytes(b"test");
        let d2 = SourceDigest::compute_from_bytes(b"test");
        let d3 = SourceDigest::compute_from_bytes(b"other");
        assert_eq!(d1, d2);
        assert_ne!(d1, d3);
    }

    #[test]
    fn digest_is_hashable() {
        use std::collections::HashSet;
        let d1 = SourceDigest::compute_from_bytes(b"a");
        let d2 = SourceDigest::compute_from_bytes(b"a");
        let mut set: HashSet<SourceDigest> = HashSet::new();
        assert!(set.insert(d1));
        assert!(!set.insert(d2)); // duplicate rejected
    }

    #[test]
    fn snapshot_from_path_roundtrip() {
        use std::fs;
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("roundtrip.txt");
        fs::write(&path, b"abc").unwrap();

        let snap = SourceSnapshot::from_path(&path).unwrap();
        // SHA-256("abc") = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
        assert_eq!(
            snap.digest().as_hex(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(snap.bytes(), b"abc");
    }
}
