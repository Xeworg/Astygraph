//! Deterministic SHA-256 fingerprinting for parse snapshots and semantic analysis.
//!
//! # Public API
//!
//! - [`ParseFingerprint::compute`] — compose a fingerprint for a parse snapshot.
//!   Includes: normalized project-relative file identity, content digest,
//!   language/parser/grammar versions, and parser-facts schema version.
//!   Excludes: provider/model.
//!
//! - [`AnalysisFingerprint::compute`] — compose a fingerprint for a semantic
//!   analysis.  Includes: stable root file/symbol query identity, ordered
//!   context input membership + per-file content digests, IR schema version,
//!   provider/model, prompt-template version, and output-affecting config.
//!
//! - [`DigestBytes`] — raw SHA-256 digest of arbitrary bytes, used as the
//!   canonical content-digest type within fingerprints.
//!
//! - [`ParseFingerprint::digest`], [`AnalysisFingerprint::digest`] — opaque
//!   32-byte SHA-256 digest accessors for cache-key equality and comparison.
//!
//! # Canonical encoding
//!
//! Every fingerprint is produced by a deterministic binary encoding pass:
//!
//! ```text
//! CanonicalUint64(N)  = 8-byte big-endian length of N bytes that follow
//! CanonicalStr(S)     = CanonicalUint64(len(S)) || S as UTF-8 bytes
//! CanonicalBytes(B)   = CanonicalUint64(len(B)) || B
//!
//! ParseFingerprint:
//!   domain byte 0x01
//!   CanonicalStr(normalized_rel_path)
//!   CanonicalBytes(content_digest)          // 32 bytes
//!   CanonicalStr(language)
//!   CanonicalStr(parser_version)
//!   CanonicalStr(grammar_version)
//!   CanonicalStr(facts_schema_version)
//!
//! AnalysisFingerprint:
//!   domain byte 0x02
//!   CanonicalStr(root_file_path)
//!   CanonicalBytes(root_content_digest)     // 32 bytes
//!   CanonicalStr(root_role)
//!   CanonicalStr(symbol_query)
//!   CanonicalUint64(len(context))
//!   for each input in context (ordered):
//!     CanonicalStr(file_path)
//!     CanonicalBytes(content_digest)         // 32 bytes
//!     CanonicalStr(role)
//!   CanonicalStr(ir_schema_version)
//!   CanonicalStr(provider)
//!   CanonicalStr(model)
//!   CanonicalStr(prompt_template_version)
//!   config_encoding                         // 0xFF if None; count+pairs if Some
//! ```
//!
//! The domain byte is the first byte fed to the hasher and guarantees that
//! parse and analysis fingerprints never collide even when all their other
//! fields are identical.  Canonical binary encoding avoids JSON map-key
//! ordering assumptions.  Content digests are fed as 32-byte sequences,
//! not re-hashed through the outer fingerprint hasher.
//!
//! # Path policy
//!
//! - Paths are resolved relative to the canonical project root.
//! - `..` components that resolve outside the project root are rejected.
//! - Symlink components in the input path are rejected (symlinks are not
//!   followed).  This is point-in-time validation: filesystem mutation can
//!   race component checks or canonicalize calls; the check is a correctness
//!   guard, not a security boundary.
//! - The project root itself is not a valid source file.
//!
//! # Constraints
//!
//! - No database writes; fingerprints are computed in-memory only.
//! - No provider calls or retries.
//! - No new crate dependencies beyond what is already in Cargo.toml.

use std::collections::BTreeMap;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::persistence::Error;

// ─── domain constants ──────────────────────────────────────────────────────────

/// Domain byte for parse fingerprints.  Prepended to the canonical encoding
/// so parse and analysis fingerprints never collide.
const DOMAIN_PARSE: u8 = 0x01;

/// Domain byte for analysis fingerprints.
const DOMAIN_ANALYSIS: u8 = 0x02;

// ─── digest bytes ─────────────────────────────────────────────────────────────

/// Raw SHA-256 digest of arbitrary bytes.
///
/// This is used as the canonical content-digest type inside parse and analysis
/// fingerprints.  It is distinct from `ParseFingerprint` and `AnalysisFingerprint`
/// which carry additional semantic identity.
#[derive(Clone, Copy)]
pub struct DigestBytes([u8; 32]);

impl DigestBytes {
    /// Compute SHA-256 over `bytes`.
    #[inline]
    pub fn compute(bytes: &[u8]) -> Self {
        let digest: [u8; 32] = Sha256::digest(bytes).into();
        Self(digest)
    }

    /// Return the raw 32-byte digest.
    #[inline]
    pub fn as_bytes(&self) -> [u8; 32] {
        self.0
    }

    /// Return the lowercase hex representation (64 ASCII characters).
    #[inline]
    pub fn as_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl PartialEq for DigestBytes {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for DigestBytes {}

impl std::hash::Hash for DigestBytes {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl std::fmt::Debug for DigestBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DigestBytes({:.7}..)", &self.as_hex()[..7])
    }
}

// ─── canonical binary encoding ────────────────────────────────────────────────

/// Encode a string as an 8-byte big-endian length followed by UTF-8 bytes.
/// This is deterministic and avoids JSON map-key ordering assumptions.
fn encode_str(bytes: &mut Vec<u8>, s: &str) {
    encode_bytes(bytes, s.as_bytes());
}

/// Encode a 32-byte digest as an 8-byte big-endian length followed by the digest.
fn encode_digest(bytes: &mut Vec<u8>, digest: &DigestBytes) {
    encode_bytes(bytes, &digest.0);
}

/// Encode an unsigned 64-bit integer as an 8-byte big-endian prefix followed
/// by the bytes that follow.  All length prefixes use 8 bytes (not 4) so that
/// context vectors with up to 2^56-1 members are representable without
/// ambiguity.
fn encode_usize(bytes: &mut Vec<u8>, n: usize) {
    let mut buf = [0u8; 8];
    big_endian_u64(&mut buf, n as u64);
    bytes.extend_from_slice(&buf);
}

fn encode_bytes(bytes: &mut Vec<u8>, data: &[u8]) {
    let mut len_buf = [0u8; 8];
    big_endian_u64(&mut len_buf, data.len() as u64);
    bytes.extend_from_slice(&len_buf);
    bytes.extend_from_slice(data);
}

#[inline]
fn big_endian_u64(buf: &mut [u8; 8], n: u64) {
    buf[0] = (n >> 56) as u8;
    buf[1] = (n >> 48) as u8;
    buf[2] = (n >> 40) as u8;
    buf[3] = (n >> 32) as u8;
    buf[4] = (n >> 24) as u8;
    buf[5] = (n >> 16) as u8;
    buf[6] = (n >> 8) as u8;
    buf[7] = n as u8;
}

/// Encode an optional `BTreeMap<String, String>` config deterministically.
///
/// The map is encoded as an ordered sequence of key-value pairs using its
/// natural BTreeMap iteration order (which is sorted by key), prefixed with
/// an 8-byte count.
///
/// **Absent vs. empty distinction:** `None` (config not present) is encoded
/// as a single `0xFF` sentinel byte. `Some(empty BTreeMap)` is encoded as an
/// 8-byte zero count prefix (no key-value pairs follow). These produce
/// distinct byte sequences, so fingerprints differ between the two cases.
fn encode_config(bytes: &mut Vec<u8>, config: Option<&BTreeMap<String, String>>) {
    match config {
        Some(cfg) => {
            // 8-byte count of key-value pairs follows.
            encode_usize(bytes, cfg.len());
            for (k, v) in cfg {
                encode_str(bytes, k);
                encode_str(bytes, v);
            }
        }
        None => {
            // Single 0xFF sentinel byte: config is absent.
            // This is distinct from encode_usize(0) which is 8 zero bytes.
            bytes.push(0xFF);
        }
    }
}

// ─── parse fingerprint ─────────────────────────────────────────────────────────

/// Opaque SHA-256 fingerprint for a parse snapshot.
///
/// Produced by feeding the canonical binary encoding (documented at the module
/// level) through SHA-256.  The fingerprint encodes all dimensions that affect
/// parser output but excludes AI provider/model settings.
pub struct ParseFingerprint {
    digest: [u8; 32],
}

impl ParseFingerprint {
    /// Compute the parse fingerprint for a source file.
    ///
    /// # Parameters
    ///
    /// - `project_root` — canonical absolute project root; used for path/symlink
    ///   validation: relative `file_path` values are resolved relative to this
    ///   directory; absolute `file_path` values are canonicalized independently.
    ///   The resolved file path must be a regular file inside `project_root`.
    /// - `file_path` — path to the source file.  May be absolute or relative to
    ///   `project_root`.  It must resolve to a real regular file inside
    ///   `project_root` through a path that contains no symlink components.
    /// - `content` — the exact source bytes that were read and will be parsed.
    /// - `language` — detected language identifier (e.g. `"rust"`, `"python"`).
    /// - `parser_version` — version of the parser adapter.
    /// - `grammar_version` — version of the grammar/parser used.
    /// - `facts_schema_version` — version of the parser-facts output schema.
    ///
    /// # Path policy
    ///
    /// - The path is normalized relative to `project_root`.
    /// - `..` components that escape the project root are rejected.
    /// - Symlink components in the path are rejected.  Point-in-time validation:
    ///   filesystem mutation can race component checks or canonicalize calls;
    ///   the check is a correctness guard, not a security boundary.
    /// - The project root itself is not a valid source file.
    ///
    /// # Errors
    ///
    /// Returns an error if the path escapes the project root, contains
    /// symlink components, or is the project root itself.
    pub fn compute(
        project_root: &Path,
        file_path: &Path,
        content: &[u8],
        language: &str,
        parser_version: &str,
        grammar_version: &str,
        facts_schema_version: &str,
    ) -> Result<Self, Error> {
        let rel_path = normalize_path(project_root, file_path)?;

        let mut encoding = Vec::with_capacity(256);
        encoding.push(DOMAIN_PARSE);
        encode_str(&mut encoding, &rel_path);
        encode_digest(&mut encoding, &DigestBytes::compute(content));
        encode_str(&mut encoding, language);
        encode_str(&mut encoding, parser_version);
        encode_str(&mut encoding, grammar_version);
        encode_str(&mut encoding, facts_schema_version);

        let digest: [u8; 32] = Sha256::digest(&encoding).into();
        Ok(Self { digest })
    }

    /// Return the 32-byte SHA-256 digest of this fingerprint.
    #[inline]
    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }
}

impl PartialEq for ParseFingerprint {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.digest == other.digest
    }
}

impl Eq for ParseFingerprint {}

impl std::fmt::Debug for ParseFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ParseFingerprint({:.7}..)",
            &hex::encode(self.digest)[..7]
        )
    }
}

// ─── analysis fingerprint ─────────────────────────────────────────────────────

/// Opaque SHA-256 fingerprint for a semantic analysis.
///
/// Produced by feeding the canonical binary encoding (documented at the module
/// level) through SHA-256.  The fingerprint encodes all dimensions that affect
/// analysis output, including provider/model, prompt template, and ordered
/// context membership.
pub struct AnalysisFingerprint {
    digest: [u8; 32],
}

/// One context input in the ordered context set for an analysis.
///
/// Each input carries the project-relative file path, the SHA-256 digest of
/// its content, and a role label (e.g. `"root"`, `"context"`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisInput {
    /// Project-relative file path.
    pub file_path: String,
    /// SHA-256 of the file's content at analysis time.
    pub content_digest: DigestBytes,
    /// Role label for this input (e.g. `"root"` or `"context"`).
    pub role: String,
}

impl AnalysisFingerprint {
    /// Compute the analysis fingerprint for a semantic analysis request.
    ///
    /// # Parameters
    ///
    /// - `project_root` — canonical absolute project root; used for path/symlink
    ///   validation: relative context paths are resolved relative to this directory,
    ///   and symlink components in context paths are checked against it.
    /// - `root` — the root file being analyzed, expressed as an `AnalysisInput`
    ///   with `role: "root"`.  Its content digest is explicitly included in
    ///   the fingerprint, so changing the file content changes the fingerprint.
    /// - `symbol_query` — stable parser-derived symbol/range identity for the
    ///   query (e.g. `"fn main"`, `"class Foo"`).  An empty string indicates
    ///   a file-level analysis.
    /// - `context` — ordered list of context inputs; order is significant
    ///   and is incorporated into the fingerprint.  Each entry's content
    ///   digest participates in the fingerprint.
    /// - `ir_schema_version` — version of the Astynex IR output schema.
    /// - `provider` — provider identifier (e.g. `"ollama"`, `"openai"`).
    /// - `model` — model name within the provider (e.g. `"llama3"`).
    /// - `prompt_template_version` — version of the prompt template used.
    /// - `config` — optional output-affecting analysis configuration (e.g.
    ///   temperature, max_tokens).  `None` (config absent) and
    ///   `Some(empty BTreeMap)` produce distinct fingerprints.
    ///
    /// # Encoding order
    ///
    /// The canonical binary encoding follows this order:
    ///   1. Domain byte (0x02)
    ///   2. Root file path (normalized, via `normalize_relative_path`)
    ///   3. Root content digest
    ///   4. Root role (e.g. `"root"`)
    ///   5. Symbol query string
    ///   6. Context count
    ///   7. Each context entry: normalized path, content digest, role
    ///   8. IR schema version, provider, model, prompt template version
    ///   9. Config encoding (`None` → 0xFF sentinel; `Some(map)` → count + pairs)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The root's role is not `"root"`.
    /// - Any path is absolute, empty, or escapes via `..` components.
    /// - Any duplicate file-path identities appear across root + context.
    /// - Any context path contains a symlink component.  Point-in-time
    ///   validation: filesystem mutation can race checks; the check is a
    ///   correctness guard, not a security boundary.
    #[allow(clippy::too_many_arguments)]
    pub fn compute(
        project_root: &Path,
        root: &AnalysisInput,
        symbol_query: &str,
        context: &[AnalysisInput],
        ir_schema_version: &str,
        provider: &str,
        model: &str,
        prompt_template_version: &str,
        config: Option<&BTreeMap<String, String>>,
    ) -> Result<Self, Error> {
        // Validate root role.
        if root.role != "root" {
            return Err(Error::cache_dir(format!(
                "analysis root input must have role 'root', got '{}'",
                root.role
            )));
        }

        // Validate and normalize root path (checks for symlinks via project_root).
        let normalized_root = normalize_relative_path(&root.file_path, project_root)?;

        // Validate and normalize context paths; also collect identities for
        // duplicate-detection.
        let mut normalized_context = Vec::with_capacity(context.len());
        let mut seen_identities = std::collections::HashSet::new();
        seen_identities.insert(normalized_root.clone());

        for input in context {
            let normalized = normalize_relative_path(&input.file_path, project_root)?;
            if !seen_identities.insert(normalized.clone()) {
                return Err(Error::cache_dir(format!(
                    "duplicate analysis input path '{}' is not allowed",
                    input.file_path
                )));
            }
            normalized_context.push((normalized, input));
        }

        // Encode in canonical order.
        let mut encoding = Vec::with_capacity(512);
        encoding.push(DOMAIN_ANALYSIS);

        // Root entry: path, content digest, role.
        encode_str(&mut encoding, &normalized_root);
        encode_digest(&mut encoding, &root.content_digest);
        encode_str(&mut encoding, &root.role);

        // Symbol query.
        encode_str(&mut encoding, symbol_query);

        // Context entries.
        encode_usize(&mut encoding, normalized_context.len());
        for (normalized_path, input) in &normalized_context {
            encode_str(&mut encoding, normalized_path);
            encode_digest(&mut encoding, &input.content_digest);
            encode_str(&mut encoding, &input.role);
        }

        encode_str(&mut encoding, ir_schema_version);
        encode_str(&mut encoding, provider);
        encode_str(&mut encoding, model);
        encode_str(&mut encoding, prompt_template_version);
        encode_config(&mut encoding, config);

        let digest: [u8; 32] = Sha256::digest(&encoding).into();
        Ok(Self { digest })
    }

    /// Return the 32-byte SHA-256 digest of this fingerprint.
    #[inline]
    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }
}

impl PartialEq for AnalysisFingerprint {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.digest == other.digest
    }
}

impl Eq for AnalysisFingerprint {}

impl std::fmt::Debug for AnalysisFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AnalysisFingerprint({:.7}..)",
            &hex::encode(self.digest)[..7]
        )
    }
}

// ─── path normalization ──────────────────────────────────────────────────────

/// Validate and normalize an absolute `file_path` relative to `project_root`.
///
/// Returns a `/`-separated project-relative path string on success.
///
/// # Path policy
///
/// - Canonicalizes both paths to resolve `.` and `..` components.
/// - Checks each path component with `symlink_metadata` before canonicalization;
///   if any component is a symlink, the path is rejected (symlinks are not
///   followed).
/// - Verifies the resolved path starts with the canonical project root.
/// - Rejects the project root itself (only sub-files are valid).
/// - Rejects paths that escape the project root after canonicalization.
fn normalize_path(project_root: &Path, file_path: &Path) -> Result<String, Error> {
    // Canonicalize the project root first to get a stable reference point.
    let canonical_root = project_root
        .canonicalize()
        .map_err(|e| Error::cache_dir(format!("project root is not accessible: {e}")))?;

    // Point-in-time symlink check: filesystem mutation can race this check or
    // the later canonicalize call.  This is a correctness guard, not a security
    // boundary.
    //
    // For relative paths: walk from canonical_root, pushing each component.
    // For absolute paths: start from an empty PathBuf and push all components
    // so that drive-prefix / RootDir components are handled as an absolute
    // candidate path, not appended to the project root.  This is platform-
    // robust for Windows absolute paths like `C:\Users\file.rs`.
    //
    // The incremental walk (component-by-component from the base) is essential:
    // it catches symlinks in parent directories (e.g. `link_dir/file.rs` where
    // `link_dir` is a symlink), not just the final file.
    //
    // NOTE: `symlink_metadata()` checks metadata without following symlinks
    // and works even for dangling symlinks.
    let mut check_path: std::path::PathBuf = if file_path.is_absolute() {
        std::path::PathBuf::new()
    } else {
        canonical_root.to_path_buf()
    };
    for component in file_path.components() {
        // Only named components can be symlinks; RootDir/ParentDir/CurDir are
        // path syntax only and don't exist as filesystem entries.
        let as_osstr: &std::ffi::OsStr = component.as_os_str();
        if as_osstr.is_empty() {
            continue;
        }
        check_path.push(std::path::Path::new(as_osstr));
        // Use symlink_metadata to detect symlinks without following them.
        if let Ok(meta) = std::fs::symlink_metadata(&check_path) {
            if meta.file_type().is_symlink() {
                return Err(Error::cache_dir(format!(
                    "path '{}' contains a symlink component and is not allowed",
                    file_path.display()
                )));
            }
        }
    }

    // Resolve file_path.  If it is already absolute, canonicalize it directly;
    // if relative, join it with the canonical root first.
    //
    // Canonicalizing the joined path is essential when project_root is a
    // symlink: joining first and then canonicalizing keeps both paths in the
    // same symlink-free coordinate system.
    let canonical_file = if file_path.is_absolute() {
        file_path.canonicalize().map_err(|e| {
            Error::cache_dir(format!(
                "file path is not accessible: {}: {e}",
                file_path.display()
            ))
        })?
    } else {
        canonical_root.join(file_path).canonicalize().map_err(|e| {
            Error::cache_dir(format!(
                "file path is not accessible: {} (original: {}): {e}",
                canonical_root.join(file_path).display(),
                file_path.display()
            ))
        })?
    };

    // Reject directories and non-regular files; only regular files are valid source files.
    if !canonical_file.is_file() {
        return Err(Error::cache_dir(format!(
            "path '{}' is not a regular file (directories are not valid source files)",
            canonical_file.display()
        )));
    };

    // The canonical file path must be inside the canonical project root.
    // Use Path::strip_prefix for proper path-component-aware containment,
    // avoiding the lossy string-prefix bug where sibling-prefix paths like
    // "<root>_outside/file.rs" could pass as inside "<root>/".
    let rel_after_root = match canonical_file.strip_prefix(&canonical_root) {
        Ok(rest) => rest,
        Err(_) => {
            return Err(Error::cache_dir(format!(
                "path '{}' is not inside project root '{}'",
                canonical_file.display(),
                canonical_root.display()
            )));
        }
    };

    // Reject the project root itself (only sub-files are valid).
    if rel_after_root.as_os_str().is_empty() {
        return Err(Error::cache_dir(format!(
            "path '{}' is the project root itself and is not a valid source file",
            canonical_file.display()
        )));
    }

    // Strip leading separator and convert to "/" separated relative path.
    // Fail-closed UTF-8 conversion: non-UTF-8 resolved paths cannot be used as
    // cache key identities because the canonical binary encoding requires UTF-8
    // strings.  Rejecting is correct; lossy conversion would produce wrong keys.
    let rel = rel_after_root
        .strip_prefix(std::path::Component::RootDir)
        .unwrap_or(rel_after_root);
    let rel_str = rel.to_str().ok_or_else(|| {
        Error::cache_dir(format!(
            "resolved file path '{}' is not valid UTF-8 and cannot be used as a cache key",
            rel.display()
        ))
    })?;
    let rel = rel_str.replace(std::path::MAIN_SEPARATOR, "/");

    Ok(rel)
}

/// Validate and normalize a project-relative path string (from context inputs).
///
/// This variant is used by `AnalysisFingerprint` where the context input paths
/// are relative identifiers.  It checks for symlink components by resolving
/// each component against the canonical project root.
///
/// # Path policy
///
/// - Rejects POSIX absolute paths (starting with `/`).
/// - Rejects Windows drive-letter absolute paths (e.g. `C:\`).
/// - Rejects UNC paths (starting with `\\`).
/// - Rejects single-backslash paths (ambiguous absolute on Windows).
/// - Rejects paths containing symlink components (symlinks are not followed).
/// - Normalizes safe internal `..` components using a component stack.
/// - Rejects paths that escape above the project root via `..`.
/// - The resulting relative path uses `/` as separator.
fn normalize_relative_path(rel_path: &str, project_root: &Path) -> Result<String, Error> {
    // Canonicalize the project root first to get a stable reference point for
    // symlink detection.
    let canonical_root = match project_root.canonicalize() {
        Ok(root) => root,
        Err(e) => {
            return Err(Error::cache_dir(format!(
                "project root is not accessible: {e}"
            )));
        }
    };

    // Reject various absolute-path forms before any normalization.
    // Check on the original string to catch Windows drive letters in their
    // original backslash form.

    // POSIX absolute: starts with /
    if rel_path.starts_with('/') {
        return Err(Error::cache_dir(format!(
            "context path '{}' is absolute (POSIX) and not allowed; use project-relative paths",
            rel_path
        )));
    }

    // Windows drive-letter absolute: X:\  or  X:/
    if rel_path.len() >= 2 && rel_path.chars().nth(1) == Some(':') {
        let first_char = rel_path.chars().next().unwrap();
        if first_char.is_alphabetic() {
            // e.g. "C:\src\main.rs" or "c:src/main.rs"
            return Err(Error::cache_dir(format!(
                "context path '{}' is a Windows drive-letter absolute path and not allowed",
                rel_path
            )));
        }
    }

    // UNC path: \\ (double backslash at start)
    if rel_path.starts_with("\\\\") {
        return Err(Error::cache_dir(format!(
            "context path '{}' is a UNC path and not allowed",
            rel_path
        )));
    }

    // Also reject a single backslash (ambiguous absolute on Windows).
    if rel_path.starts_with('\\') {
        return Err(Error::cache_dir(format!(
            "context path '{}' starts with a backslash and is not a valid relative path",
            rel_path
        )));
    }

    // Walk from canonical root and check each component for symlinks.
    // This works for any path that could exist on the filesystem.
    let mut check_path = canonical_root.clone();
    for component in Path::new(rel_path).components() {
        let as_osstr: &std::ffi::OsStr = component.as_os_str();
        if as_osstr.is_empty() {
            continue;
        }
        check_path.push(Path::new(as_osstr));
        // Use symlink_metadata to detect symlinks without following them.
        if let Ok(meta) = std::fs::symlink_metadata(&check_path) {
            if meta.file_type().is_symlink() {
                return Err(Error::cache_dir(format!(
                    "context path '{}' contains a symlink component and is not allowed",
                    rel_path
                )));
            }
        }
    }

    // Normalize separators for consistent processing.
    let normalized = rel_path.replace('\\', "/");

    // Use a component stack to normalize `.` and safe internal `..` components.
    // This properly handles paths like "a/../src/main.rs" → "src/main.rs"
    // while rejecting paths where `..` would escape above the root.
    let mut components: Vec<&str> = Vec::new();
    for component in normalized.split('/') {
        match component {
            "" | "." => {
                // Skip empty and current-dir components.
            }
            ".." => {
                if components.is_empty() {
                    // `..` at the start would escape above the project root.
                    return Err(Error::cache_dir(format!(
                        "context path '{}' escapes the project root via '..'",
                        rel_path
                    )));
                }
                // Pop the last component (the parent we're traversing into).
                components.pop();
            }
            other => {
                components.push(other);
            }
        }
    }

    // Verify the normalized path doesn't collapse to empty.
    if components.is_empty() {
        return Err(Error::cache_dir(format!(
            "context path '{}' normalizes to an invalid project-relative identity",
            rel_path
        )));
    }

    let cleaned = components.join("/");
    Ok(cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: the comprehensive integration tests live in
    // `tests/persistence_fingerprints.rs`.  The unit tests here cover only
    // the canonical encoding helpers.

    #[test]
    fn encode_str_deterministic() {
        let mut a = Vec::new();
        encode_str(&mut a, "hello");
        let mut b = Vec::new();
        encode_str(&mut b, "hello");
        assert_eq!(a, b);
    }

    #[test]
    fn encode_str_different_input() {
        let mut a = Vec::new();
        encode_str(&mut a, "hello");
        let mut b = Vec::new();
        encode_str(&mut b, "world");
        assert_ne!(a, b);
    }

    #[test]
    fn encode_usize_deterministic() {
        let mut a = Vec::new();
        encode_usize(&mut a, 42);
        let mut b = Vec::new();
        encode_usize(&mut b, 42);
        assert_eq!(a, b);
    }

    #[test]
    fn encode_usize_different_values() {
        let mut a = Vec::new();
        encode_usize(&mut a, 1);
        let mut b = Vec::new();
        encode_usize(&mut b, 2);
        assert_ne!(a, b);
    }

    #[test]
    fn encode_config_some() {
        let mut cfg = BTreeMap::new();
        cfg.insert("temperature".to_string(), "0.7".to_string());
        cfg.insert("max_tokens".to_string(), "1024".to_string());

        let mut enc = Vec::new();
        encode_config(&mut enc, Some(&cfg));

        // The BTreeMap produces sorted order, so the encoding is deterministic.
        let mut expected = Vec::new();
        encode_usize(&mut expected, 2);
        encode_str(&mut expected, "max_tokens");
        encode_str(&mut expected, "1024");
        encode_str(&mut expected, "temperature");
        encode_str(&mut expected, "0.7");

        assert_eq!(enc, expected);
    }

    #[test]
    fn encode_config_none() {
        // Absent config (None) is encoded as a single 0xFF sentinel byte.
        // This is distinct from encode_usize(0) which is 8 zero bytes.
        let mut enc = Vec::new();
        encode_config(&mut enc, None);
        assert_eq!(enc.as_slice(), &[0xFF]);
    }

    #[test]
    fn encode_config_none_vs_some_empty() {
        // None (config absent) and Some(empty BTreeMap) must produce distinct
        // byte sequences so that fingerprints differ.
        let empty_map = BTreeMap::<String, String>::new();

        let mut enc_none = Vec::new();
        encode_config(&mut enc_none, None);

        let mut enc_empty = Vec::new();
        encode_config(&mut enc_empty, Some(&empty_map));

        // None → [0xFF]; Some(empty) → 8 zero bytes (usize 0 encoding).
        assert_ne!(enc_none, enc_empty);
        assert_eq!(enc_none.as_slice(), &[0xFF]);

        let mut expected_empty = Vec::new();
        encode_usize(&mut expected_empty, 0);
        assert_eq!(enc_empty, expected_empty);
    }

    #[test]
    fn encode_config_deterministic_order() {
        // Two BTreeMaps with the same key-value pairs produce identical encoding.
        let mut cfg1 = BTreeMap::new();
        cfg1.insert("b".to_string(), "2".to_string());
        cfg1.insert("a".to_string(), "1".to_string());

        let mut cfg2 = BTreeMap::new();
        cfg2.insert("a".to_string(), "1".to_string());
        cfg2.insert("b".to_string(), "2".to_string());

        let mut enc1 = Vec::new();
        encode_config(&mut enc1, Some(&cfg1));
        let mut enc2 = Vec::new();
        encode_config(&mut enc2, Some(&cfg2));

        assert_eq!(enc1, enc2);
    }

    #[test]
    fn digest_bytes_known_vector() {
        let d = DigestBytes::compute(b"abc");
        assert_eq!(
            d.as_hex(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn digest_bytes_empty_input() {
        let d = DigestBytes::compute(b"");
        assert_eq!(
            d.as_hex(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn digest_bytes_deterministic() {
        let d1 = DigestBytes::compute(b"test");
        let d2 = DigestBytes::compute(b"test");
        assert_eq!(d1, d2);
    }

    #[test]
    fn digest_bytes_hashable() {
        use std::collections::HashSet;
        let d1 = DigestBytes::compute(b"a");
        let d2 = DigestBytes::compute(b"a");
        let mut set: HashSet<DigestBytes> = HashSet::new();
        assert!(set.insert(d1));
        assert!(!set.insert(d2));
    }

    // ─── normalize_relative_path tests ──────────────────────────────────────

    // Helper to create a temp project root for tests.
    fn temp_project_root() -> std::path::PathBuf {
        let temp_dir = tempfile::TempDir::new().unwrap();
        temp_dir.keep()
    }

    #[test]
    fn normalize_relative_path_accepts_simple_relative() {
        let root = temp_project_root();
        assert_eq!(
            normalize_relative_path("src/main.rs", &root).unwrap(),
            "src/main.rs"
        );
    }

    #[test]
    fn normalize_relative_path_normalizes_dot_prefix() {
        let root = temp_project_root();
        // "./src/main.rs" normalizes to "src/main.rs" (dot-only components stripped).
        assert_eq!(
            normalize_relative_path("./src/main.rs", &root).unwrap(),
            "src/main.rs"
        );
        // "src/./main.rs" normalizes to "src/main.rs".
        assert_eq!(
            normalize_relative_path("src/./main.rs", &root).unwrap(),
            "src/main.rs"
        );
        // "src/../lib.rs": internal ".." is normalized away using a component
        // stack, producing "lib.rs". This represents a relative traversal that
        // resolves to "lib.rs" at runtime. The function normalizes both "."
        // components and safe internal ".." components.
        assert_eq!(
            normalize_relative_path("src/../lib.rs", &root).unwrap(),
            "lib.rs"
        );
    }

    #[test]
    fn normalize_relative_path_rejects_posix_absolute() {
        let root = temp_project_root();
        let result = normalize_relative_path("/etc/passwd", &root);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("absolute (POSIX)"));
    }

    #[test]
    fn normalize_relative_path_rejects_windows_drive_letter() {
        let root = temp_project_root();
        // Uppercase drive letter.
        let result = normalize_relative_path("C:\\src\\main.rs", &root);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("drive-letter"));

        // Lowercase drive letter.
        let result = normalize_relative_path("c:/src/main.rs", &root);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("drive-letter"));
    }

    #[test]
    fn normalize_relative_path_rejects_unc_path() {
        let root = temp_project_root();
        let result = normalize_relative_path("\\\\server\\share\\file", &root);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("UNC"));
    }

    #[test]
    fn normalize_relative_path_rejects_single_backslash() {
        let root = temp_project_root();
        // Single backslash at start is ambiguous on Windows.
        let result = normalize_relative_path("\\src\\main.rs", &root);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("backslash"));
    }

    #[test]
    fn normalize_relative_path_rejects_traversal() {
        let root = temp_project_root();
        // "../" alone escapes.
        let result = normalize_relative_path("..", &root);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("escapes"));

        // "../file" from root escapes.
        let result = normalize_relative_path("../file", &root);
        assert!(result.is_err());

        // Deep traversal.
        let result = normalize_relative_path("a/b/../../../../etc", &root);
        assert!(result.is_err());
    }

    #[test]
    fn normalize_relative_path_rejects_empty() {
        let root = temp_project_root();
        let result = normalize_relative_path("", &root);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid project-relative identity"));
    }

    #[test]
    fn normalize_relative_path_rejects_dot_only() {
        let root = temp_project_root();
        let result = normalize_relative_path(".", &root);
        assert!(result.is_err());
    }

    #[test]
    fn normalize_relative_path_rejects_windows_backslash_normalized() {
        let root = temp_project_root();
        // Backslashes are converted to forward slashes during normalization.
        // "src\\..\\lib.rs" normalizes to "src/../lib.rs" then to "lib.rs"
        // via the component stack normalization.
        let result = normalize_relative_path("src\\..\\lib.rs", &root);
        assert_eq!(result.unwrap(), "lib.rs");

        // "..\\file" normalizes to "../file" which still escapes.
        let result = normalize_relative_path("..\\file", &root);
        assert!(result.is_err());
    }

    /// Symlink component in a relative context path must be rejected.
    #[test]
    #[cfg(unix)]
    fn normalize_relative_path_rejects_symlink_component() {
        use std::os::unix::fs::symlink;

        let root = temp_project_root();
        // Create a real directory and a symlink to it.
        let real_dir = root.join("real_dir");
        std::fs::create_dir_all(&real_dir).unwrap();
        let symlink_dir = root.join("link_dir");
        symlink(&real_dir, &symlink_dir).unwrap();

        // Path through the symlink must be rejected.
        let result = normalize_relative_path("link_dir/file.rs", &root);
        assert!(
            result.is_err(),
            "symlink component in context path must be rejected"
        );
    }
}
