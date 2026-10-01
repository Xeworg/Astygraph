//! Freshness revalidation API — point-in-time check that a previously recorded
//! fingerprint still matches the current file contents.
//!
//! This module provides two revalidation entry points:
//! - [`revalidate_parse`] — re-read a single parse snapshot and compare.
//! - [`revalidate_analysis`] — re-read the root and ordered context inputs,
//!   rebuild the current fingerprint, and compare.
//!
//! # Outcome model
//!
//! - `Fresh` — recorded fingerprint matches the recomputed fingerprint.
//! - `Stale` — recorded fingerprint differs from recomputed fingerprint.
//!   A mismatch covers: changed bytes, changed identity (path/language/
//!   version/provider/model/config), or changed membership/order.
//! - `MissingInput` — one or more explicit input files could not be read.
//!   This is a distinct outcome so callers can distinguish "input gone"
//!   from "input changed".
//!
//! Typed errors cover:
//! - Invalid paths (symlink components, outside project root, escape via `..`).
//! - I/O errors other than NotFound (permission denied, etc.).
//!
//! # Path validation order
//!
//! Path containment is checked BEFORE attempting to read the file.  This
//! ensures that an out-of-root path that does not exist returns a typed error
//! rather than `MissingInput`.  Only after containment is confirmed does
//! `SourceSnapshot::from_path` run; that call then determines `Fresh` vs
//! `Stale` vs `MissingInput` based solely on whether the file is readable.
//!
//! # What is NOT in this module
//!
//! - Database reads/writes or cache payload storage.
//! - Provider calls, retries, watcher, or application integration.
//! - Project-wide scan; only caller-supplied paths are read.

use std::collections::BTreeMap;
use std::path::Path;

use crate::persistence::fingerprint::{
    AnalysisFingerprint, AnalysisInput, DigestBytes, ParseFingerprint,
};
use crate::persistence::snapshot::SourceSnapshot;
use crate::persistence::Error;

// ─── outcome types ───────────────────────────────────────────────────────────

/// Outcome of a parse fingerprint revalidation check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseFreshnessResult {
    /// The recorded fingerprint matches the current file fingerprint.
    Fresh,
    /// The recorded fingerprint differs from the current fingerprint.
    Stale,
    /// The input file is missing (NotFound I/O).
    MissingInput,
}

/// Outcome of an analysis fingerprint revalidation check.
///
/// Uses the same three variants as parse revalidation; the distinction
/// between "stale" and "missing" is caller-visible so that orchestration
/// can decide how to handle each case.
pub type FreshnessResult = ParseFreshnessResult;

// ─── request types ───────────────────────────────────────────────────────────

/// Compact request struct for parse fingerprint revalidation.
///
/// All fields are caller-supplied; no scanning or defaulting occurs.
pub struct ParseFreshnessRequest<'a> {
    /// The fingerprint that was recorded when the parse snapshot was created.
    pub recorded_fingerprint: ParseFingerprint,
    /// Absolute path to the source file (may be relative to `project_root`).
    pub file_path: &'a Path,
    /// Canonical absolute project root used for path/symlink validation.
    pub project_root: &'a Path,
    /// Language identifier used during the original parse (e.g. `"rust"`).
    pub language: &'a str,
    /// Parser adapter version.
    pub parser_version: &'a str,
    /// Grammar/parser version.
    pub grammar_version: &'a str,
    /// Parser-facts output schema version.
    pub facts_schema_version: &'a str,
}

/// Compact request struct for analysis fingerprint revalidation.
///
/// All fields are caller-supplied; no scanning or defaulting occurs.
/// The `root` and `context` paths are validated against `project_root`
/// before any file is read.
pub struct AnalysisFreshnessRequest<'a> {
    /// The fingerprint that was recorded when the analysis was performed.
    pub recorded_fingerprint: AnalysisFingerprint,
    /// Root input used for the analysis.  Its `content_digest` is the
    /// digest that was recorded; the revalidation reads the file and
    /// recomputes the digest.
    pub root: AnalysisInput,
    /// Stable parser-derived symbol/range identity for the query.
    pub symbol_query: &'a str,
    /// Ordered list of context inputs.  Order is significant and is
    /// incorporated into the fingerprint.
    pub context: Vec<AnalysisInput>,
    /// IR output schema version.
    pub ir_schema_version: &'a str,
    /// Provider identifier (e.g. `"ollama"`, `"openai"`).
    pub provider: &'a str,
    /// Model name within the provider.
    pub model: &'a str,
    /// Prompt template version.
    pub prompt_template_version: &'a str,
    /// Optional output-affecting analysis configuration.
    pub config: Option<&'a BTreeMap<String, String>>,
}

// ─── path containment validation ─────────────────────────────────────────────

/// Check whether `file_path` resolves to a location inside `project_root`
/// using canonical absolute paths.
///
/// This is the outside-root guard that runs BEFORE `SourceSnapshot::from_path`.
///
/// Returns `Ok(MissingMarker)` if the file does not exist (caller should
/// continue to `SourceSnapshot::from_path` which will confirm `MissingInput`),
/// `Ok(())` if the path is contained and exists (safe to read),
/// or `Err(CacheDir)` if the resolved path escapes the project root.
fn check_path_contained_in_root(
    file_path: &Path,
    project_root: &Path,
) -> Result<MissingMarker, Error> {
    // Canonicalize the project root to get a stable reference point.
    let canonical_root = project_root
        .canonicalize()
        .map_err(|e| Error::cache_dir(format!("project root is not accessible: {e}")))?;

    if file_path.is_absolute() {
        // Absolute path: check for symlink components first, then canonicalize.
        check_no_symlink_components(file_path)?;
        match file_path.canonicalize() {
            Ok(canonical_file) => match canonical_file.strip_prefix(&canonical_root) {
                Ok(remaining) => {
                    if remaining.as_os_str().is_empty() {
                        Err(Error::cache_dir(format!(
                            "path '{}' is the project root itself",
                            canonical_file.display()
                        )))
                    } else {
                        Ok(MissingMarker::Contained)
                    }
                }
                Err(_) => Err(Error::cache_dir(format!(
                    "path '{}' is not inside project root '{}'",
                    canonical_file.display(),
                    canonical_root.display()
                ))),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // File does not exist.  Use the parent directory to check
                // containment: if the parent canonicalizes inside the project
                // root, the file (when created) would be inside; if the parent
                // canonicalizes outside, the file is outside the root.
                // Also verify no symlink components are in the path itself.
                check_no_symlink_components(file_path)?;
                if let Some(parent) = file_path.parent() {
                    match parent.canonicalize() {
                        Ok(canonical_parent) => {
                            match canonical_parent.strip_prefix(&canonical_root) {
                                Ok(_) => Ok(MissingMarker::Missing),
                                Err(_) => Err(Error::cache_dir(format!(
                                    "path '{}' is not inside project root '{}'",
                                    file_path.display(),
                                    canonical_root.display()
                                ))),
                            }
                        }
                        Err(_) => {
                            // Parent also doesn't exist — use strip_prefix on
                            // the non-canonical path as a fallback containment
                            // check (this correctly rejects paths with `..`
                            // that escape even if the parent doesn't exist).
                            match file_path.strip_prefix(&canonical_root) {
                                Ok(_) => Ok(MissingMarker::Missing),
                                Err(_) => Err(Error::cache_dir(format!(
                                    "path '{}' is not inside project root '{}'",
                                    file_path.display(),
                                    canonical_root.display()
                                ))),
                            }
                        }
                    }
                } else {
                    // No parent — reject as outside root.
                    Err(Error::cache_dir(format!(
                        "path '{}' is not inside project root '{}'",
                        file_path.display(),
                        canonical_root.display()
                    )))
                }
            }
            Err(e) => Err(Error::cache_dir(format!(
                "file path is not accessible: {}: {e}",
                file_path.display()
            ))),
        }
    } else {
        // Relative path: check for symlink components, then join and canonicalize.
        check_symlink_components_from_base(file_path, &canonical_root)?;
        let joined = canonical_root.join(file_path);
        match joined.canonicalize() {
            Ok(canonical_file) => match canonical_file.strip_prefix(&canonical_root) {
                Ok(remaining) => {
                    if remaining.as_os_str().is_empty() {
                        Err(Error::cache_dir(format!(
                            "path '{}' is the project root itself",
                            canonical_file.display()
                        )))
                    } else {
                        Ok(MissingMarker::Contained)
                    }
                }
                Err(_) => Err(Error::cache_dir(format!(
                    "path '{}' is not inside project root '{}'",
                    canonical_file.display(),
                    canonical_root.display()
                ))),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // File does not exist.  Check containment via the parent
                // directory: if the parent canonicalizes inside the project
                // root, the file (when created) would be inside.
                if let Some(parent) = joined.parent() {
                    match parent.canonicalize() {
                        Ok(canonical_parent) => {
                            match canonical_parent.strip_prefix(&canonical_root) {
                                Ok(_) => Ok(MissingMarker::Missing),
                                Err(_) => Err(Error::cache_dir(format!(
                                    "path '{}' is not inside project root '{}'",
                                    joined.display(),
                                    canonical_root.display()
                                ))),
                            }
                        }
                        Err(_) => {
                            // Parent also doesn't exist — fall back to
                            // checking `..` escape via Path::components.
                            let escapes = joined
                                .components()
                                .any(|c| c == std::path::Component::ParentDir);
                            if escapes {
                                return Err(Error::cache_dir(format!(
                                    "path '{}' escapes the project root via '..' components",
                                    joined.display()
                                )));
                            }
                            Ok(MissingMarker::Missing)
                        }
                    }
                } else {
                    // No parent — reject as outside root.
                    Err(Error::cache_dir(format!(
                        "path '{}' is not inside project root '{}'",
                        joined.display(),
                        canonical_root.display()
                    )))
                }
            }
            Err(e) => Err(Error::cache_dir(format!(
                "file path is not accessible: {}: {e}",
                joined.display()
            ))),
        }
    }
}

/// Sentinel marker used internally to communicate whether a non-existent
/// path was inside the project root (should return MissingInput) or outside
/// (should return an error).
enum MissingMarker {
    /// Path is inside the project root but the file does not exist.
    Missing,
    /// Path is inside the project root and the file exists (readable).
    Contained,
}

/// Detect symlink components in an absolute `path` using `symlink_metadata`
/// without following the symlink.  This mirrors the policy in
/// `ParseFingerprint::compute` → `normalize_path`.
///
/// Walks from an empty base path (for absolute inputs) so that each component
/// is checked in the filesystem namespace where it resolves.
fn check_no_symlink_components(path: &Path) -> Result<(), Error> {
    let mut check_path = std::path::PathBuf::new();
    for component in path.components() {
        let as_osstr: &std::ffi::OsStr = component.as_os_str();
        if as_osstr.is_empty() {
            continue;
        }
        check_path.push(std::path::Path::new(as_osstr));
        // symlink_metadata checks without following symlinks and works
        // even for dangling symlinks.
        if let Ok(meta) = std::fs::symlink_metadata(&check_path) {
            if meta.file_type().is_symlink() {
                return Err(Error::cache_dir(format!(
                    "path '{}' contains a symlink component and is not allowed",
                    path.display()
                )));
            }
        }
    }
    Ok(())
}

/// Detect symlink components in a relative `rel_path` by walking from the
/// canonical `base` directory, checking each component via `symlink_metadata`
/// without following the symlink.  This catches symlinks in parent directories
/// (e.g. `link_dir/file.rs` where `link_dir` itself is a symlink).
fn check_symlink_components_from_base(rel_path: &Path, base: &Path) -> Result<(), Error> {
    let mut check_path = base.to_path_buf();
    for component in rel_path.components() {
        let as_osstr: &std::ffi::OsStr = component.as_os_str();
        if as_osstr.is_empty() {
            continue;
        }
        check_path.push(std::path::Path::new(as_osstr));
        if let Ok(meta) = std::fs::symlink_metadata(&check_path) {
            if meta.file_type().is_symlink() {
                return Err(Error::cache_dir(format!(
                    "path '{}' contains a symlink component and is not allowed",
                    rel_path.display()
                )));
            }
        }
    }
    Ok(())
}

/// Like `check_path_contained_in_root` but takes a pre-built absolute path
/// (already joined with the project root).  Applies the same symlink guard.
fn check_absolute_path_contained_in_root(
    abs_path: &Path,
    project_root: &Path,
) -> Result<MissingMarker, Error> {
    let canonical_root = project_root
        .canonicalize()
        .map_err(|e| Error::cache_dir(format!("project root is not accessible: {e}")))?;

    // Check for symlink components before canonicalizing.
    check_no_symlink_components(abs_path)?;

    match abs_path.canonicalize() {
        Ok(canonical_file) => match canonical_file.strip_prefix(&canonical_root) {
            Ok(remaining) => {
                if remaining.as_os_str().is_empty() {
                    Err(Error::cache_dir(format!(
                        "path '{}' is the project root itself",
                        canonical_file.display()
                    )))
                } else {
                    Ok(MissingMarker::Contained)
                }
            }
            Err(_) => Err(Error::cache_dir(format!(
                "path '{}' is not inside project root '{}'",
                canonical_file.display(),
                canonical_root.display()
            ))),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // File does not exist.  Check containment via the parent directory.
            if let Some(parent) = abs_path.parent() {
                match parent.canonicalize() {
                    Ok(canonical_parent) => match canonical_parent.strip_prefix(&canonical_root) {
                        Ok(_) => Ok(MissingMarker::Missing),
                        Err(_) => Err(Error::cache_dir(format!(
                            "path '{}' is not inside project root '{}'",
                            abs_path.display(),
                            canonical_root.display()
                        ))),
                    },
                    Err(_) => {
                        // Parent also doesn't exist — fall back to
                        // strip_prefix on the non-canonical path.
                        match abs_path.strip_prefix(&canonical_root) {
                            Ok(_) => Ok(MissingMarker::Missing),
                            Err(_) => Err(Error::cache_dir(format!(
                                "path '{}' is not inside project root '{}'",
                                abs_path.display(),
                                canonical_root.display()
                            ))),
                        }
                    }
                }
            } else {
                Err(Error::cache_dir(format!(
                    "path '{}' is not inside project root '{}'",
                    abs_path.display(),
                    canonical_root.display()
                )))
            }
        }
        Err(e) => Err(Error::cache_dir(format!(
            "file path is not accessible: {}: {e}",
            abs_path.display()
        ))),
    }
}

// ─── parse revalidation ──────────────────────────────────────────────────────

/// Revalidate a previously recorded parse fingerprint against the current file
/// contents at the same path.
///
/// # Flow
///
/// 1. **Validate containment**: check that `file_path` resolves inside
///    `project_root` using canonical absolute paths.  If the resolved path
///    escapes (e.g. via `..` components or is outside the root), return a
///    typed `CacheDir` error immediately.  This guard runs BEFORE reading
///    so that an out-of-root non-existent path returns an error, not
///    `MissingInput`.
/// 2. Attempt to read the file with `SourceSnapshot::from_path`.
///    - NotFound → return `MissingInput`.
///    - Other I/O error → return a typed error.
/// 3. Recompute the parse fingerprint using the current bytes and the
///    caller-supplied identity fields (language, versions, etc.).
/// 4. Compare the recomputed fingerprint digest with `recorded_fingerprint`.
///    - Equal → return `Fresh`.
///    - Different → return `Stale`.
pub fn revalidate_parse(
    request: &ParseFreshnessRequest<'_>,
) -> Result<ParseFreshnessResult, Error> {
    // Step 1: containment validation BEFORE reading.
    match check_path_contained_in_root(request.file_path, request.project_root)? {
        MissingMarker::Missing => {
            // Path is inside the root but the file does not exist.
            return Ok(ParseFreshnessResult::MissingInput);
        }
        MissingMarker::Contained => {
            // Path is inside the root and exists — proceed to read.
        }
    }

    // Step 2: attempt to read the file.
    let snap = match SourceSnapshot::from_path(request.file_path) {
        Ok(s) => s,
        Err(Error::SnapshotRead { path: _, source })
            if source.kind() == std::io::ErrorKind::NotFound =>
        {
            return Ok(ParseFreshnessResult::MissingInput);
        }
        Err(e) => return Err(e),
    };

    // Step 3: recompute fingerprint from current bytes.
    let recomputed = ParseFingerprint::compute(
        request.project_root,
        request.file_path,
        snap.bytes(),
        request.language,
        request.parser_version,
        request.grammar_version,
        request.facts_schema_version,
    )?;

    // Step 4: compare.
    if recomputed.digest() == request.recorded_fingerprint.digest() {
        Ok(ParseFreshnessResult::Fresh)
    } else {
        Ok(ParseFreshnessResult::Stale)
    }
}

// ─── analysis revalidation ───────────────────────────────────────────────────

/// Revalidate a previously recorded analysis fingerprint against the current
/// root and context file contents.
///
/// # Flow
///
/// 1. **Validate containment** for the root path BEFORE reading.
/// 2. **Validate containment** for each context path BEFORE reading.
///    If any containment check fails, return a typed `CacheDir` error
///    immediately (not `MissingInput`).
/// 3. Read the root file with `SourceSnapshot::from_path`.
///    - NotFound → return `MissingInput`.
///    - Other I/O error → return a typed error.
/// 4. Read each context file in order with `SourceSnapshot::from_path`.
///    - Any NotFound → return `MissingInput`.
///    - Any other I/O error → return a typed error.
/// 5. Build fresh `AnalysisInput` values with the current digests.
/// 6. Recompute the analysis fingerprint using the current root/context
///    inputs and the caller-supplied identity fields.
/// 7. Compare the recomputed fingerprint digest with `recorded_fingerprint`.
///    - Equal → return `Fresh`.
///    - Different → return `Stale`.
pub fn revalidate_analysis(
    project_root: &Path,
    request: &AnalysisFreshnessRequest<'_>,
) -> Result<FreshnessResult, Error> {
    // Step 1: containment validation for root BEFORE reading.
    let root_path = project_root.join(&request.root.file_path);
    match check_absolute_path_contained_in_root(&root_path, project_root)? {
        MissingMarker::Missing => {
            return Ok(FreshnessResult::MissingInput);
        }
        MissingMarker::Contained => {}
    }

    // Step 3: read the root file.
    let root_snap = match SourceSnapshot::from_path(&root_path) {
        Ok(s) => s,
        Err(Error::SnapshotRead { path: _, source })
            if source.kind() == std::io::ErrorKind::NotFound =>
        {
            return Ok(FreshnessResult::MissingInput);
        }
        Err(e) => return Err(e),
    };

    // Step 2 continued: containment validation for each context path BEFORE reading.
    let mut fresh_context = Vec::with_capacity(request.context.len());
    for ctx_input in &request.context {
        let ctx_path = project_root.join(&ctx_input.file_path);
        match check_absolute_path_contained_in_root(&ctx_path, project_root)? {
            MissingMarker::Missing => {
                return Ok(FreshnessResult::MissingInput);
            }
            MissingMarker::Contained => {}
        }

        // Step 4: read the context file.
        let ctx_snap = match SourceSnapshot::from_path(&ctx_path) {
            Ok(s) => s,
            Err(Error::SnapshotRead { path: _, source })
                if source.kind() == std::io::ErrorKind::NotFound =>
            {
                return Ok(FreshnessResult::MissingInput);
            }
            Err(e) => return Err(e),
        };

        fresh_context.push(AnalysisInput {
            file_path: ctx_input.file_path.clone(),
            content_digest: DigestBytes::from_bytes(ctx_snap.digest().as_bytes()),
            role: ctx_input.role.clone(),
        });
    }

    // Step 5: build fresh root input with current digest.
    let fresh_root = AnalysisInput {
        file_path: request.root.file_path.clone(),
        content_digest: DigestBytes::from_bytes(root_snap.digest().as_bytes()),
        role: request.root.role.clone(),
    };

    // Step 6: recompute the analysis fingerprint.
    let recomputed = AnalysisFingerprint::compute(
        project_root,
        &fresh_root,
        request.symbol_query,
        &fresh_context,
        request.ir_schema_version,
        request.provider,
        request.model,
        request.prompt_template_version,
        request.config,
    )?;

    // Step 7: compare.
    if recomputed.digest() == request.recorded_fingerprint.digest() {
        Ok(FreshnessResult::Fresh)
    } else {
        Ok(FreshnessResult::Stale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_freshness_result_variants() {
        assert_eq!(format!("{:?}", ParseFreshnessResult::Fresh), "Fresh");
        assert_eq!(format!("{:?}", ParseFreshnessResult::Stale), "Stale");
        assert_eq!(
            format!("{:?}", ParseFreshnessResult::MissingInput),
            "MissingInput"
        );
    }

    #[test]
    fn freshness_result_is_same_type() {
        assert_eq!(
            std::any::type_name_of_val(&FreshnessResult::Fresh),
            std::any::type_name_of_val(&ParseFreshnessResult::Fresh)
        );
    }
}
