//! Tests for the freshness revalidation API — parse and analysis fingerprint
//! revalidation against current file contents.
//!
//! # What is being tested
//!
//! - Parse fingerprint revalidation: unchanged → Fresh, bytes changed → Stale,
//!   path missing → MissingInput.
//! - Analysis fingerprint revalidation: unchanged → Fresh; root/context bytes
//!   changed → Stale; root changed → Stale; context changed → Stale; missing
//!   root/context → MissingInput; context reordered → Stale; membership
//!   added/removed → Stale; provider/model/config changed → Stale.
//! - Symlink/outside-root paths are rejected before reading.
//! - Typed I/O errors (permission denied) remain errors, not Fresh/Stale.
//!
//! # What is NOT in scope
//!
//! - Database writes or persistence payloads.
//! - Provider calls, retries, watcher, or application integration.
//! - SQLite privacy tests (no DB layer in this API).

use std::collections::BTreeMap;
use tempfile::TempDir;

// ─── test project helper ─────────────────────────────────────────────────────

/// Minimal project root that tests can use to create files and resolve paths.
struct TestProject {
    dir: TempDir,
}

impl TestProject {
    fn new() -> Self {
        Self {
            dir: TempDir::new().unwrap(),
        }
    }

    fn root(&self) -> std::path::PathBuf {
        self.dir.path().to_path_buf()
    }

    /// Write a file at a project-relative path. Returns the absolute path.
    fn write_file(&self, relative: &str, contents: &[u8]) -> std::path::PathBuf {
        let path = self.root().join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, contents).unwrap();
        path
    }

    /// Remove a file at a project-relative path.
    fn remove_file(&self, relative: &str) {
        let path = self.root().join(relative);
        std::fs::remove_file(&path).unwrap();
    }
}

// ─── parse freshness tests ───────────────────────────────────────────────────

mod parse_freshness {
    use super::*;
    use astynex::persistence::fingerprint::ParseFingerprint;
    use astynex::persistence::freshness::{ParseFreshnessRequest, ParseFreshnessResult};

    /// Parse unchanged → Fresh.
    #[test]
    fn parse_unchanged_is_fresh() {
        let proj = TestProject::new();
        let abs_path = proj.write_file("src/main.rs", b"fn main() {}");

        // Record the fingerprint with the current content.
        let recorded_fp = ParseFingerprint::compute(
            &proj.root(),
            &abs_path,
            b"fn main() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        )
        .unwrap();

        let request = ParseFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            file_path: &abs_path,
            project_root: &proj.root(),
            language: "rust",
            parser_version: "1.0.0",
            grammar_version: "0.1.0",
            facts_schema_version: "0.1.0",
        };

        let result = astynex::persistence::freshness::revalidate_parse(&request);
        assert!(
            matches!(result, Ok(ParseFreshnessResult::Fresh)),
            "unchanged parse must be Fresh, got {result:?}"
        );
    }

    /// Parse bytes changed at same path → Stale.
    #[test]
    fn parse_bytes_changed_is_stale() {
        let proj = TestProject::new();
        let abs_path = proj.write_file("src/main.rs", b"fn main() {}");

        // Record with original content.
        let recorded_fp = ParseFingerprint::compute(
            &proj.root(),
            &abs_path,
            b"fn main() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        )
        .unwrap();

        // Modify the file.
        std::fs::write(&abs_path, b"fn changed() {}").unwrap();

        let request = ParseFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            file_path: &abs_path,
            project_root: &proj.root(),
            language: "rust",
            parser_version: "1.0.0",
            grammar_version: "0.1.0",
            facts_schema_version: "0.1.0",
        };

        let result = astynex::persistence::freshness::revalidate_parse(&request);
        assert!(
            matches!(result, Ok(ParseFreshnessResult::Stale)),
            "changed parse bytes must be Stale, got {result:?}"
        );
    }

    /// Parse path missing → MissingInput.
    #[test]
    fn parse_path_missing_is_missing_input() {
        let proj = TestProject::new();
        let abs_path = proj.write_file("src/main.rs", b"fn main() {}");

        let recorded_fp = ParseFingerprint::compute(
            &proj.root(),
            &abs_path,
            b"fn main() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        )
        .unwrap();

        // Remove the file.
        std::fs::remove_file(&abs_path).unwrap();

        let request = ParseFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            file_path: &abs_path,
            project_root: &proj.root(),
            language: "rust",
            parser_version: "1.0.0",
            grammar_version: "0.1.0",
            facts_schema_version: "0.1.0",
        };

        let result = astynex::persistence::freshness::revalidate_parse(&request);
        assert!(
            matches!(result, Ok(ParseFreshnessResult::MissingInput)),
            "missing path must be MissingInput, got {result:?}"
        );
    }

    /// Parse version change → Stale.
    #[test]
    fn parse_version_changed_is_stale() {
        let proj = TestProject::new();
        let abs_path = proj.write_file("src/main.rs", b"fn main() {}");

        let recorded_fp = ParseFingerprint::compute(
            &proj.root(),
            &abs_path,
            b"fn main() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        )
        .unwrap();

        // Request with different parser version.
        let request = ParseFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            file_path: &abs_path,
            project_root: &proj.root(),
            language: "rust",
            parser_version: "2.0.0", // changed
            grammar_version: "0.1.0",
            facts_schema_version: "0.1.0",
        };

        let result = astynex::persistence::freshness::revalidate_parse(&request);
        assert!(
            matches!(result, Ok(ParseFreshnessResult::Stale)),
            "version change must be Stale, got {result:?}"
        );
    }

    /// Parse language change → Stale.
    #[test]
    fn parse_language_changed_is_stale() {
        let proj = TestProject::new();
        let abs_path = proj.write_file("src/main.rs", b"fn main() {}");

        let recorded_fp = ParseFingerprint::compute(
            &proj.root(),
            &abs_path,
            b"fn main() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        )
        .unwrap();

        let request = ParseFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            file_path: &abs_path,
            project_root: &proj.root(),
            language: "python", // changed
            parser_version: "1.0.0",
            grammar_version: "0.1.0",
            facts_schema_version: "0.1.0",
        };

        let result = astynex::persistence::freshness::revalidate_parse(&request);
        assert!(
            matches!(result, Ok(ParseFreshnessResult::Stale)),
            "language change must be Stale, got {result:?}"
        );
    }

    /// Parse symlink path → error (rejected before reading).
    #[test]
    #[cfg(unix)]
    fn parse_symlink_path_rejected_before_read() {
        use std::os::unix::fs::symlink;

        let proj = TestProject::new();
        let real_path = proj.write_file("real_dir/main.rs", b"fn main() {}");

        // Create a symlink to the file.
        let link_path = proj.root().join("link_dir");
        std::fs::create_dir_all(&link_path).unwrap();
        let symlink_path = link_path.join("main.rs");
        symlink(&real_path, &symlink_path).unwrap();

        let recorded_fp = ParseFingerprint::compute(
            &proj.root(),
            &real_path,
            b"fn main() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        )
        .unwrap();

        let request = ParseFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            file_path: &symlink_path,
            project_root: &proj.root(),
            language: "rust",
            parser_version: "1.0.0",
            grammar_version: "0.1.0",
            facts_schema_version: "0.1.0",
        };

        let result = astynex::persistence::freshness::revalidate_parse(&request);
        assert!(
            result.is_err(),
            "symlink path must be rejected before reading, got {result:?}"
        );
        // Verify it did NOT become Fresh (safe-path enforcement).
        assert!(
            !matches!(result, Ok(ParseFreshnessResult::Fresh)),
            "symlink must never return Fresh"
        );
    }

    /// Parse path outside root → error (rejected before reading).
    #[test]
    fn parse_outside_root_rejected_before_read() {
        let proj = TestProject::new();
        let abs_path = proj.write_file("src/main.rs", b"fn main() {}");

        let recorded_fp = ParseFingerprint::compute(
            &proj.root(),
            &abs_path,
            b"fn main() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        )
        .unwrap();

        // Path that escapes the project root.
        let escape_path = proj.root().join("..").join("outside.rs");

        let request = ParseFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            file_path: &escape_path,
            project_root: &proj.root(),
            language: "rust",
            parser_version: "1.0.0",
            grammar_version: "0.1.0",
            facts_schema_version: "0.1.0",
        };

        let result = astynex::persistence::freshness::revalidate_parse(&request);
        assert!(
            result.is_err(),
            "outside-root path must be rejected before reading, got {result:?}"
        );
        assert!(
            !matches!(result, Ok(ParseFreshnessResult::Fresh)),
            "outside-root must never return Fresh"
        );
    }

    /// Parse permission-denied file → error (not Fresh).
    #[test]
    #[cfg(unix)]
    fn parse_permission_denied_is_error() {
        use std::os::unix::fs::PermissionsExt;

        let proj = TestProject::new();
        let abs_path = proj.write_file("src/main.rs", b"fn main() {}");

        let recorded_fp = ParseFingerprint::compute(
            &proj.root(),
            &abs_path,
            b"fn main() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        )
        .unwrap();

        // Deny read permission.
        let original = std::fs::metadata(&abs_path).unwrap().permissions();
        let mut denied = original.clone();
        denied.set_mode(0o000);
        std::fs::set_permissions(&abs_path, denied).unwrap();

        let request = ParseFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            file_path: &abs_path,
            project_root: &proj.root(),
            language: "rust",
            parser_version: "1.0.0",
            grammar_version: "0.1.0",
            facts_schema_version: "0.1.0",
        };

        let result = astynex::persistence::freshness::revalidate_parse(&request);
        // Restore before asserting.
        let _ = std::fs::set_permissions(&abs_path, original);
        assert!(
            result.is_err(),
            "permission-denied read must be an error, got {result:?}"
        );
        assert!(
            !matches!(result, Ok(ParseFreshnessResult::Fresh)),
            "permission-denied must never return Fresh"
        );
    }

    /// Parse: dangling symlink (target does not exist) → error at fingerprint
    /// recording time (symlink component detected).
    #[test]
    #[cfg(unix)]
    fn parse_dangling_symlink_is_error() {
        use std::os::unix::fs::symlink;

        let proj = TestProject::new();
        // Create a dangling symlink: link points to a non-existent file.
        let link_dir = proj.root().join("link_dir");
        std::fs::create_dir_all(&link_dir).unwrap();
        let dangling_link = link_dir.join("ghost.rs");
        symlink("/nonexistent/target.rs", &dangling_link).unwrap();

        let recorded_fp = ParseFingerprint::compute(
            &proj.root(),
            &dangling_link,
            b"// content doesn't matter",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        );
        // Recording with a dangling symlink should fail (symlink detected).
        assert!(
            recorded_fp.is_err(),
            "ParseFingerprint::compute must reject dangling symlink path"
        );
    }

    /// Parse: parent directory is a symlink → error (rejected before reading).
    /// The parent dir `link_dir` is itself a symlink to `real_dir`.
    #[test]
    #[cfg(unix)]
    fn parse_symlink_parent_dir_rejected_before_read() {
        use std::os::unix::fs::symlink;

        let proj = TestProject::new();
        // Create real_dir with a file, then create link_dir as a symlink to real_dir.
        let real_dir = proj.root().join("real_dir");
        std::fs::create_dir_all(&real_dir).unwrap();
        std::fs::write(real_dir.join("main.rs"), b"fn main() {}").unwrap();

        // link_dir is a symlink to real_dir.
        let link_dir = proj.root().join("link_dir");
        symlink(&real_dir, &link_dir).unwrap();

        // Record fingerprint using the real path (valid).
        let real_path = real_dir.join("main.rs");
        let recorded_fp = ParseFingerprint::compute(
            &proj.root(),
            &real_path,
            b"fn main() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        )
        .unwrap();

        // Request using path through the symlink parent directory.
        let symlink_through_parent = link_dir.join("main.rs");
        let request = ParseFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            file_path: &symlink_through_parent,
            project_root: &proj.root(),
            language: "rust",
            parser_version: "1.0.0",
            grammar_version: "0.1.0",
            facts_schema_version: "0.1.0",
        };

        let result = astynex::persistence::freshness::revalidate_parse(&request);
        assert!(
            result.is_err(),
            "path with symlink parent dir must be rejected before reading, got {result:?}"
        );
        assert!(
            !matches!(result, Ok(ParseFreshnessResult::Fresh)),
            "symlink parent dir must never return Fresh"
        );
    }

    /// Parse: identical bytes, same path, different grammar version → Stale.
    /// Content is byte-for-byte identical but an identity field changed.
    #[test]
    fn parse_identical_bytes_different_identity_is_stale() {
        let proj = TestProject::new();
        let abs_path = proj.write_file("src/main.rs", b"fn main() {}");

        let recorded_fp = ParseFingerprint::compute(
            &proj.root(),
            &abs_path,
            b"fn main() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        )
        .unwrap();

        // Grammar version differs — content is identical but identity changed.
        let request = ParseFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            file_path: &abs_path,
            project_root: &proj.root(),
            language: "rust",
            parser_version: "1.0.0",
            grammar_version: "0.2.0", // changed
            facts_schema_version: "0.1.0",
        };

        let result = astynex::persistence::freshness::revalidate_parse(&request);
        assert!(
            matches!(result, Ok(ParseFreshnessResult::Stale)),
            "identical bytes with changed identity must be Stale, got {result:?}"
        );
    }
}

// ─── analysis freshness tests ─────────────────────────────────────────────────

mod analysis_freshness {
    use super::*;
    use astynex::persistence::fingerprint::{AnalysisFingerprint, AnalysisInput, DigestBytes};
    use astynex::persistence::freshness::{AnalysisFreshnessRequest, FreshnessResult};

    /// Analysis unchanged root+context → Fresh.
    #[test]
    fn analysis_unchanged_is_fresh() {
        let proj = TestProject::new();
        let _root_path = proj.write_file("src/main.rs", b"fn main() {}");
        let _ctx_path = proj.write_file("src/dep.rs", b"pub fn dep() {}");

        let root_input = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };
        let ctx_input = AnalysisInput {
            file_path: "src/dep.rs".into(),
            content_digest: DigestBytes::compute(b"pub fn dep() {}"),
            role: "context".into(),
        };

        let recorded_fp = AnalysisFingerprint::compute(
            &proj.root(),
            &root_input,
            "main",
            std::slice::from_ref(&ctx_input),
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        let request = AnalysisFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            root: AnalysisInput {
                file_path: "src/main.rs".into(),
                content_digest: DigestBytes::compute(b"fn main() {}"),
                role: "root".into(),
            },
            symbol_query: "main",
            context: vec![AnalysisInput {
                file_path: "src/dep.rs".into(),
                content_digest: DigestBytes::compute(b"pub fn dep() {}"),
                role: "context".into(),
            }],
            ir_schema_version: "1.0.0",
            provider: "ollama",
            model: "llama3",
            prompt_template_version: "v1",
            config: None,
        };

        let result = astynex::persistence::freshness::revalidate_analysis(&proj.root(), &request);
        assert!(
            matches!(result, Ok(FreshnessResult::Fresh)),
            "unchanged analysis must be Fresh, got {result:?}"
        );
    }

    /// Root bytes changed at same path → Stale.
    #[test]
    fn analysis_root_bytes_changed_is_stale() {
        let proj = TestProject::new();
        let _root_path = proj.write_file("src/main.rs", b"fn main() {}");

        let root_input = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let recorded_fp = AnalysisFingerprint::compute(
            &proj.root(),
            &root_input,
            "main",
            &[],
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        // Change the root file content.
        proj.write_file("src/main.rs", b"fn changed() {}");

        let request = AnalysisFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            root: AnalysisInput {
                file_path: "src/main.rs".into(),
                content_digest: DigestBytes::compute(b"fn main() {}"), // stale recorded digest
                role: "root".into(),
            },
            symbol_query: "main",
            context: vec![],
            ir_schema_version: "1.0.0",
            provider: "ollama",
            model: "llama3",
            prompt_template_version: "v1",
            config: None,
        };

        let result = astynex::persistence::freshness::revalidate_analysis(&proj.root(), &request);
        assert!(
            matches!(result, Ok(FreshnessResult::Stale)),
            "changed root bytes must be Stale, got {result:?}"
        );
    }

    /// Root path changed (different file) → Stale.
    #[test]
    fn analysis_root_path_changed_is_stale() {
        let proj = TestProject::new();
        let _path1 = proj.write_file("src/main.rs", b"fn main1() {}");
        let _path2 = proj.write_file("src/other.rs", b"fn other() {}");

        let root_input = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main1() {}"),
            role: "root".into(),
        };

        let recorded_fp = AnalysisFingerprint::compute(
            &proj.root(),
            &root_input,
            "main1",
            &[],
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        // Request with a different root file path.
        let request = AnalysisFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            root: AnalysisInput {
                file_path: "src/other.rs".into(), // different path
                content_digest: DigestBytes::compute(b"fn other() {}"),
                role: "root".into(),
            },
            symbol_query: "other",
            context: vec![],
            ir_schema_version: "1.0.0",
            provider: "ollama",
            model: "llama3",
            prompt_template_version: "v1",
            config: None,
        };

        let result = astynex::persistence::freshness::revalidate_analysis(&proj.root(), &request);
        assert!(
            matches!(result, Ok(FreshnessResult::Stale)),
            "different root path must be Stale, got {result:?}"
        );
    }

    /// Context bytes changed → Stale.
    #[test]
    fn analysis_context_bytes_changed_is_stale() {
        let proj = TestProject::new();
        let _root_path = proj.write_file("src/main.rs", b"fn main() {}");
        let _ctx_path = proj.write_file("src/dep.rs", b"pub fn dep() {}");

        let root_input = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };
        let ctx_input = AnalysisInput {
            file_path: "src/dep.rs".into(),
            content_digest: DigestBytes::compute(b"pub fn dep() {}"),
            role: "context".into(),
        };

        let recorded_fp = AnalysisFingerprint::compute(
            &proj.root(),
            &root_input,
            "main",
            std::slice::from_ref(&ctx_input),
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        // Modify the context file content.
        proj.write_file("src/dep.rs", b"pub fn modified() {}");

        let request = AnalysisFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            root: AnalysisInput {
                file_path: "src/main.rs".into(),
                content_digest: DigestBytes::compute(b"fn main() {}"),
                role: "root".into(),
            },
            symbol_query: "main",
            context: vec![AnalysisInput {
                file_path: "src/dep.rs".into(),
                content_digest: DigestBytes::compute(b"pub fn dep() {}"), // stale
                role: "context".into(),
            }],
            ir_schema_version: "1.0.0",
            provider: "ollama",
            model: "llama3",
            prompt_template_version: "v1",
            config: None,
        };

        let result = astynex::persistence::freshness::revalidate_analysis(&proj.root(), &request);
        assert!(
            matches!(result, Ok(FreshnessResult::Stale)),
            "changed context bytes must be Stale, got {result:?}"
        );
    }

    /// Missing root file → MissingInput.
    #[test]
    fn analysis_missing_root_is_missing_input() {
        let proj = TestProject::new();
        let _root_path = proj.write_file("src/main.rs", b"fn main() {}");

        let root_input = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let recorded_fp = AnalysisFingerprint::compute(
            &proj.root(),
            &root_input,
            "main",
            &[],
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        // Remove the root file.
        proj.remove_file("src/main.rs");

        let request = AnalysisFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            root: AnalysisInput {
                file_path: "src/main.rs".into(),
                content_digest: DigestBytes::compute(b"fn main() {}"),
                role: "root".into(),
            },
            symbol_query: "main",
            context: vec![],
            ir_schema_version: "1.0.0",
            provider: "ollama",
            model: "llama3",
            prompt_template_version: "v1",
            config: None,
        };

        let result = astynex::persistence::freshness::revalidate_analysis(&proj.root(), &request);
        assert!(
            matches!(result, Ok(FreshnessResult::MissingInput)),
            "missing root must be MissingInput, got {result:?}"
        );
    }

    /// Missing context file → MissingInput.
    #[test]
    fn analysis_missing_context_is_missing_input() {
        let proj = TestProject::new();
        let _root_path = proj.write_file("src/main.rs", b"fn main() {}");
        let _ctx_path = proj.write_file("src/dep.rs", b"pub fn dep() {}");

        let root_input = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };
        let ctx_input = AnalysisInput {
            file_path: "src/dep.rs".into(),
            content_digest: DigestBytes::compute(b"pub fn dep() {}"),
            role: "context".into(),
        };

        let recorded_fp = AnalysisFingerprint::compute(
            &proj.root(),
            &root_input,
            "main",
            std::slice::from_ref(&ctx_input),
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        // Remove the context file.
        proj.remove_file("src/dep.rs");

        let request = AnalysisFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            root: AnalysisInput {
                file_path: "src/main.rs".into(),
                content_digest: DigestBytes::compute(b"fn main() {}"),
                role: "root".into(),
            },
            symbol_query: "main",
            context: vec![AnalysisInput {
                file_path: "src/dep.rs".into(),
                content_digest: DigestBytes::compute(b"pub fn dep() {}"),
                role: "context".into(),
            }],
            ir_schema_version: "1.0.0",
            provider: "ollama",
            model: "llama3",
            prompt_template_version: "v1",
            config: None,
        };

        let result = astynex::persistence::freshness::revalidate_analysis(&proj.root(), &request);
        assert!(
            matches!(result, Ok(FreshnessResult::MissingInput)),
            "missing context must be MissingInput, got {result:?}"
        );
    }

    /// Context reordered → Stale.
    #[test]
    fn analysis_context_reordered_is_stale() {
        let proj = TestProject::new();
        let _main_path = proj.write_file("src/main.rs", b"fn main() {}");
        let _path_a = proj.write_file("src/a.rs", b"// file a");
        let _path_b = proj.write_file("src/b.rs", b"// file b");

        let root_input = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        // Recorded with order: a, b
        let ctx_recorded = vec![
            AnalysisInput {
                file_path: "src/a.rs".into(),
                content_digest: DigestBytes::compute(b"// file a"),
                role: "context".into(),
            },
            AnalysisInput {
                file_path: "src/b.rs".into(),
                content_digest: DigestBytes::compute(b"// file b"),
                role: "context".into(),
            },
        ];

        let recorded_fp = AnalysisFingerprint::compute(
            &proj.root(),
            &root_input,
            "main",
            &ctx_recorded,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        // Request with order: b, a (reordered)
        let request = AnalysisFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            root: AnalysisInput {
                file_path: "src/main.rs".into(),
                content_digest: DigestBytes::compute(b"fn main() {}"),
                role: "root".into(),
            },
            symbol_query: "main",
            context: vec![
                AnalysisInput {
                    file_path: "src/b.rs".into(),
                    content_digest: DigestBytes::compute(b"// file b"),
                    role: "context".into(),
                },
                AnalysisInput {
                    file_path: "src/a.rs".into(),
                    content_digest: DigestBytes::compute(b"// file a"),
                    role: "context".into(),
                },
            ],
            ir_schema_version: "1.0.0",
            provider: "ollama",
            model: "llama3",
            prompt_template_version: "v1",
            config: None,
        };

        let result = astynex::persistence::freshness::revalidate_analysis(&proj.root(), &request);
        assert!(
            matches!(result, Ok(FreshnessResult::Stale)),
            "reordered context must be Stale, got {result:?}"
        );
    }

    /// Context membership added → Stale.
    #[test]
    fn analysis_context_added_is_stale() {
        let proj = TestProject::new();
        let _root_path = proj.write_file("src/main.rs", b"fn main() {}");
        let _path_a = proj.write_file("src/a.rs", b"// file a");
        let _path_b = proj.write_file("src/b.rs", b"// file b");

        let root_input = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        // Recorded with only a.rs.
        let ctx_recorded = vec![AnalysisInput {
            file_path: "src/a.rs".into(),
            content_digest: DigestBytes::compute(b"// file a"),
            role: "context".into(),
        }];

        let recorded_fp = AnalysisFingerprint::compute(
            &proj.root(),
            &root_input,
            "main",
            &ctx_recorded,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        // Request with both a.rs and b.rs.
        let request = AnalysisFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            root: AnalysisInput {
                file_path: "src/main.rs".into(),
                content_digest: DigestBytes::compute(b"fn main() {}"),
                role: "root".into(),
            },
            symbol_query: "main",
            context: vec![
                AnalysisInput {
                    file_path: "src/a.rs".into(),
                    content_digest: DigestBytes::compute(b"// file a"),
                    role: "context".into(),
                },
                AnalysisInput {
                    file_path: "src/b.rs".into(),
                    content_digest: DigestBytes::compute(b"// file b"),
                    role: "context".into(),
                },
            ],
            ir_schema_version: "1.0.0",
            provider: "ollama",
            model: "llama3",
            prompt_template_version: "v1",
            config: None,
        };

        let result = astynex::persistence::freshness::revalidate_analysis(&proj.root(), &request);
        assert!(
            matches!(result, Ok(FreshnessResult::Stale)),
            "added context membership must be Stale, got {result:?}"
        );
    }

    /// Context membership removed → Stale.
    #[test]
    fn analysis_context_removed_is_stale() {
        let proj = TestProject::new();
        let _root_path = proj.write_file("src/main.rs", b"fn main() {}");
        let _path_a = proj.write_file("src/a.rs", b"// file a");
        let _path_b = proj.write_file("src/b.rs", b"// file b");

        let root_input = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        // Recorded with both a.rs and b.rs.
        let ctx_recorded = vec![
            AnalysisInput {
                file_path: "src/a.rs".into(),
                content_digest: DigestBytes::compute(b"// file a"),
                role: "context".into(),
            },
            AnalysisInput {
                file_path: "src/b.rs".into(),
                content_digest: DigestBytes::compute(b"// file b"),
                role: "context".into(),
            },
        ];

        let recorded_fp = AnalysisFingerprint::compute(
            &proj.root(),
            &root_input,
            "main",
            &ctx_recorded,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        // Request with only a.rs (b.rs removed).
        let request = AnalysisFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            root: AnalysisInput {
                file_path: "src/main.rs".into(),
                content_digest: DigestBytes::compute(b"fn main() {}"),
                role: "root".into(),
            },
            symbol_query: "main",
            context: vec![AnalysisInput {
                file_path: "src/a.rs".into(),
                content_digest: DigestBytes::compute(b"// file a"),
                role: "context".into(),
            }],
            ir_schema_version: "1.0.0",
            provider: "ollama",
            model: "llama3",
            prompt_template_version: "v1",
            config: None,
        };

        let result = astynex::persistence::freshness::revalidate_analysis(&proj.root(), &request);
        assert!(
            matches!(result, Ok(FreshnessResult::Stale)),
            "removed context membership must be Stale, got {result:?}"
        );
    }

    /// Provider changed → Stale.
    #[test]
    fn analysis_provider_changed_is_stale() {
        let proj = TestProject::new();
        let _root_path = proj.write_file("src/main.rs", b"fn main() {}");

        let root_input = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let recorded_fp = AnalysisFingerprint::compute(
            &proj.root(),
            &root_input,
            "main",
            &[],
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        let request = AnalysisFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            root: AnalysisInput {
                file_path: "src/main.rs".into(),
                content_digest: DigestBytes::compute(b"fn main() {}"),
                role: "root".into(),
            },
            symbol_query: "main",
            context: vec![],
            ir_schema_version: "1.0.0",
            provider: "openai", // changed
            model: "gpt-4",
            prompt_template_version: "v1",
            config: None,
        };

        let result = astynex::persistence::freshness::revalidate_analysis(&proj.root(), &request);
        assert!(
            matches!(result, Ok(FreshnessResult::Stale)),
            "provider change must be Stale, got {result:?}"
        );
    }

    /// Model changed → Stale.
    #[test]
    fn analysis_model_changed_is_stale() {
        let proj = TestProject::new();
        let _root_path = proj.write_file("src/main.rs", b"fn main() {}");

        let root_input = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let recorded_fp = AnalysisFingerprint::compute(
            &proj.root(),
            &root_input,
            "main",
            &[],
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        let request = AnalysisFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            root: AnalysisInput {
                file_path: "src/main.rs".into(),
                content_digest: DigestBytes::compute(b"fn main() {}"),
                role: "root".into(),
            },
            symbol_query: "main",
            context: vec![],
            ir_schema_version: "1.0.0",
            provider: "ollama",
            model: "llama3.1", // changed
            prompt_template_version: "v1",
            config: None,
        };

        let result = astynex::persistence::freshness::revalidate_analysis(&proj.root(), &request);
        assert!(
            matches!(result, Ok(FreshnessResult::Stale)),
            "model change must be Stale, got {result:?}"
        );
    }

    /// Config changed → Stale.
    #[test]
    fn analysis_config_changed_is_stale() {
        let proj = TestProject::new();
        let _root_path = proj.write_file("src/main.rs", b"fn main() {}");

        let root_input = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let mut cfg_recorded = BTreeMap::new();
        cfg_recorded.insert("temperature".to_string(), "0.0".to_string());

        let recorded_fp = AnalysisFingerprint::compute(
            &proj.root(),
            &root_input,
            "main",
            &[],
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            Some(&cfg_recorded),
        )
        .unwrap();

        let mut cfg_current = BTreeMap::new();
        cfg_current.insert("temperature".to_string(), "0.7".to_string()); // changed

        let request = AnalysisFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            root: AnalysisInput {
                file_path: "src/main.rs".into(),
                content_digest: DigestBytes::compute(b"fn main() {}"),
                role: "root".into(),
            },
            symbol_query: "main",
            context: vec![],
            ir_schema_version: "1.0.0",
            provider: "ollama",
            model: "llama3",
            prompt_template_version: "v1",
            config: Some(&cfg_current),
        };

        let result = astynex::persistence::freshness::revalidate_analysis(&proj.root(), &request);
        assert!(
            matches!(result, Ok(FreshnessResult::Stale)),
            "config change must be Stale, got {result:?}"
        );
    }

    /// Symbol query changed → Stale.
    #[test]
    fn analysis_symbol_query_changed_is_stale() {
        let proj = TestProject::new();
        let _root_path = proj.write_file("src/main.rs", b"fn main() {}");

        let root_input = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let recorded_fp = AnalysisFingerprint::compute(
            &proj.root(),
            &root_input,
            "main",
            &[],
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        let request = AnalysisFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            root: AnalysisInput {
                file_path: "src/main.rs".into(),
                content_digest: DigestBytes::compute(b"fn main() {}"),
                role: "root".into(),
            },
            symbol_query: "other", // changed
            context: vec![],
            ir_schema_version: "1.0.0",
            provider: "ollama",
            model: "llama3",
            prompt_template_version: "v1",
            config: None,
        };

        let result = astynex::persistence::freshness::revalidate_analysis(&proj.root(), &request);
        assert!(
            matches!(result, Ok(FreshnessResult::Stale)),
            "symbol query change must be Stale, got {result:?}"
        );
    }

    /// Root symlink → error (rejected before reading).
    #[test]
    #[cfg(unix)]
    fn analysis_root_symlink_rejected_before_read() {
        use std::os::unix::fs::symlink;

        let proj = TestProject::new();
        // Create the real file (used for recorded fingerprint) and a symlink.
        let _real_path = proj.write_file("real_dir/main.rs", b"fn main() {}");

        // Create symlink to the file.
        let link_dir = proj.root().join("link_dir");
        std::fs::create_dir_all(&link_dir).unwrap();
        let symlink_path = link_dir.join("main.rs");
        symlink(&_real_path, &symlink_path).unwrap();

        // Record the fingerprint using the REAL (non-symlink) path.
        // Using the symlink path here would panic in AnalysisFingerprint::compute
        // (symlink component check in normalize_relative_path).
        let valid_root_input = AnalysisInput {
            file_path: "real_dir/main.rs".into(), // valid path
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let recorded_fp = AnalysisFingerprint::compute(
            &proj.root(),
            &valid_root_input,
            "main",
            &[],
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        // Request with the SYMLINK path — revalidate_analysis must reject it.
        let request = AnalysisFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            root: AnalysisInput {
                file_path: "link_dir/main.rs".into(), // symlink path
                content_digest: DigestBytes::compute(b"fn main() {}"),
                role: "root".into(),
            },
            symbol_query: "main",
            context: vec![],
            ir_schema_version: "1.0.0",
            provider: "ollama",
            model: "llama3",
            prompt_template_version: "v1",
            config: None,
        };

        let result = astynex::persistence::freshness::revalidate_analysis(&proj.root(), &request);
        assert!(
            result.is_err(),
            "symlink root path must be rejected before reading, got {result:?}"
        );
        assert!(
            !matches!(result, Ok(FreshnessResult::Fresh)),
            "symlink root must never return Fresh"
        );
    }

    /// Context symlink → error (rejected before reading).
    #[test]
    #[cfg(unix)]
    fn analysis_context_symlink_rejected_before_read() {
        use std::os::unix::fs::symlink;

        let proj = TestProject::new();
        let _root_path = proj.write_file("src/main.rs", b"fn main() {}");
        let _real_ctx = proj.write_file("real_dep/dep.rs", b"pub fn dep() {}");

        // Create symlink at linked_dep/dep.rs → real_dep/dep.rs.
        let link_dep = proj.root().join("linked_dep");
        std::fs::create_dir_all(&link_dep).unwrap();
        let symlink_ctx = link_dep.join("dep.rs");
        symlink(&_real_ctx, &symlink_ctx).unwrap();

        // Record the fingerprint using the REAL (non-symlink) context path.
        // Using "linked_dep/dep.rs" here would panic in AnalysisFingerprint::compute
        // because normalize_relative_path detects the symlink component.
        let root_input = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };
        let valid_ctx_input = AnalysisInput {
            file_path: "real_dep/dep.rs".into(), // valid path, not a symlink
            content_digest: DigestBytes::compute(b"pub fn dep() {}"),
            role: "context".into(),
        };

        let recorded_fp = AnalysisFingerprint::compute(
            &proj.root(),
            &root_input,
            "main",
            std::slice::from_ref(&valid_ctx_input),
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        // Request with the SYMLINK context path — revalidate_analysis must reject it.
        let request = AnalysisFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            root: AnalysisInput {
                file_path: "src/main.rs".into(),
                content_digest: DigestBytes::compute(b"fn main() {}"),
                role: "root".into(),
            },
            symbol_query: "main",
            context: vec![AnalysisInput {
                file_path: "linked_dep/dep.rs".into(), // symlink path
                content_digest: DigestBytes::compute(b"pub fn dep() {}"),
                role: "context".into(),
            }],
            ir_schema_version: "1.0.0",
            provider: "ollama",
            model: "llama3",
            prompt_template_version: "v1",
            config: None,
        };

        let result = astynex::persistence::freshness::revalidate_analysis(&proj.root(), &request);
        assert!(
            result.is_err(),
            "symlink context path must be rejected before reading, got {result:?}"
        );
        assert!(
            !matches!(result, Ok(FreshnessResult::Fresh)),
            "symlink context must never return Fresh"
        );
    }

    /// Context outside root → error (rejected before reading).
    #[test]
    fn analysis_context_outside_root_rejected_before_read() {
        let proj = TestProject::new();
        let _root_path = proj.write_file("src/main.rs", b"fn main() {}");
        // Create a valid context file for the recorded fingerprint.
        let _ctx_path = proj.write_file("src/dep.rs", b"pub fn dep() {}");

        // Record the fingerprint with a VALID context path.
        // Using "../outside.rs" here would panic in AnalysisFingerprint::compute
        // (normalize_relative_path rejects escape via "..").
        let root_input = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };
        let valid_ctx_input = AnalysisInput {
            file_path: "src/dep.rs".into(), // valid path
            content_digest: DigestBytes::compute(b"pub fn dep() {}"),
            role: "context".into(),
        };

        let recorded_fp = AnalysisFingerprint::compute(
            &proj.root(),
            &root_input,
            "main",
            std::slice::from_ref(&valid_ctx_input),
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        // Request with the OUTSIDE-ROOT context path — revalidate_analysis must reject it.
        let request = AnalysisFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            root: AnalysisInput {
                file_path: "src/main.rs".into(),
                content_digest: DigestBytes::compute(b"fn main() {}"),
                role: "root".into(),
            },
            symbol_query: "main",
            context: vec![AnalysisInput {
                file_path: "../outside.rs".into(), // outside-root path
                content_digest: DigestBytes::compute(b"// evil"),
                role: "context".into(),
            }],
            ir_schema_version: "1.0.0",
            provider: "ollama",
            model: "llama3",
            prompt_template_version: "v1",
            config: None,
        };

        let result = astynex::persistence::freshness::revalidate_analysis(&proj.root(), &request);
        assert!(
            result.is_err(),
            "outside-root context path must be rejected before reading, got {result:?}"
        );
        assert!(
            !matches!(result, Ok(FreshnessResult::Fresh)),
            "outside-root context must never return Fresh"
        );
    }

    /// None vs Some(empty) config → Stale.
    #[test]
    fn analysis_config_none_vs_empty_is_stale() {
        let proj = TestProject::new();
        let _root_path = proj.write_file("src/main.rs", b"fn main() {}");

        let root_input = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let recorded_fp = AnalysisFingerprint::compute(
            &proj.root(),
            &root_input,
            "main",
            &[],
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        let empty_config = BTreeMap::<String, String>::new();

        let request = AnalysisFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            root: AnalysisInput {
                file_path: "src/main.rs".into(),
                content_digest: DigestBytes::compute(b"fn main() {}"),
                role: "root".into(),
            },
            symbol_query: "main",
            context: vec![],
            ir_schema_version: "1.0.0",
            provider: "ollama",
            model: "llama3",
            prompt_template_version: "v1",
            config: Some(&empty_config), // Some(empty) vs None recorded
        };

        let result = astynex::persistence::freshness::revalidate_analysis(&proj.root(), &request);
        assert!(
            matches!(result, Ok(FreshnessResult::Stale)),
            "None vs Some(empty) config must be Stale, got {result:?}"
        );
    }

    /// Analysis: dangling symlink context path → error (not MissingInput).
    /// The fingerprint computation rejects the dangling path before reading.
    #[test]
    #[cfg(unix)]
    fn analysis_dangling_context_symlink_is_error() {
        use std::os::unix::fs::symlink;

        let proj = TestProject::new();
        let _root_path = proj.write_file("src/main.rs", b"fn main() {}");

        // Create a dangling symlink in the context position.
        let ctx_dir = proj.root().join("ctx_dir");
        std::fs::create_dir_all(&ctx_dir).unwrap();
        let dangling_ctx = ctx_dir.join("ghost.rs");
        symlink("/nonexistent/ghost.rs", &dangling_ctx).unwrap();

        // Record fingerprint with valid context first (needed to have a valid fingerprint).
        // But computing fingerprint with the dangling context path should fail.
        let root_input = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };
        let ctx_input = AnalysisInput {
            file_path: "ctx_dir/ghost.rs".into(),
            content_digest: DigestBytes::compute(b"// placeholder"),
            role: "context".into(),
        };

        // Computing fingerprint with dangling context path should fail.
        let fp_result = AnalysisFingerprint::compute(
            &proj.root(),
            &root_input,
            "main",
            std::slice::from_ref(&ctx_input),
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        );
        assert!(
            fp_result.is_err(),
            "AnalysisFingerprint::compute must reject dangling symlink context"
        );
    }

    /// Analysis: context path whose parent directory is a symlink → error.
    /// The `linked_dep` directory is itself a symlink to `real_dep`.
    #[test]
    #[cfg(unix)]
    fn analysis_context_symlink_parent_dir_rejected_before_read() {
        use std::os::unix::fs::symlink;

        let proj = TestProject::new();
        let _root_path = proj.write_file("src/main.rs", b"fn main() {}");

        // Create real_dep with a file.
        let real_dep = proj.root().join("real_dep");
        std::fs::create_dir_all(&real_dep).unwrap();
        std::fs::write(real_dep.join("dep.rs"), b"pub fn dep() {}").unwrap();

        // linked_dep is a symlink to real_dep.
        let linked_dep = proj.root().join("linked_dep");
        symlink(&real_dep, &linked_dep).unwrap();

        // Record fingerprint using the real context path (valid).
        let root_input = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };
        let valid_ctx = AnalysisInput {
            file_path: "real_dep/dep.rs".into(),
            content_digest: DigestBytes::compute(b"pub fn dep() {}"),
            role: "context".into(),
        };

        let recorded_fp = AnalysisFingerprint::compute(
            &proj.root(),
            &root_input,
            "main",
            std::slice::from_ref(&valid_ctx),
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        // Request with context path through the symlink parent directory.
        let request = AnalysisFreshnessRequest {
            recorded_fingerprint: recorded_fp,
            root: AnalysisInput {
                file_path: "src/main.rs".into(),
                content_digest: DigestBytes::compute(b"fn main() {}"),
                role: "root".into(),
            },
            symbol_query: "main",
            context: vec![AnalysisInput {
                file_path: "linked_dep/dep.rs".into(), // symlink parent dir
                content_digest: DigestBytes::compute(b"pub fn dep() {}"),
                role: "context".into(),
            }],
            ir_schema_version: "1.0.0",
            provider: "ollama",
            model: "llama3",
            prompt_template_version: "v1",
            config: None,
        };

        let result = astynex::persistence::freshness::revalidate_analysis(&proj.root(), &request);
        assert!(
            result.is_err(),
            "context path with symlink parent dir must be rejected, got {result:?}"
        );
        assert!(
            !matches!(result, Ok(FreshnessResult::Fresh)),
            "symlink parent dir context must never return Fresh"
        );
    }
}
