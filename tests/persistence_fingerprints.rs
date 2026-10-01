//! Tests for the fingerprint module — deterministic SHA-256 fingerprinting
//! of parse snapshots and semantic analysis.
//
//! # What is being tested
//!
//! - Canonical length-prefixed encoding is deterministic and reproducible.
//! - Parse fingerprint excludes provider/model fields; analysis includes them.
//! - Context membership/reorder/content changes alter the analysis fingerprint.
//! - Path normalization rejects escapes, symlink components, and outside-root paths.
//! - Missing inputs produce a typed error, not a silent fallback.
//!
//! # What is NOT in scope (Task 1 / other tasks)
//!
//! - Database writes or persistence of fingerprints.
//! - Provider calls or retries.
//! - Source file edits.
//! - New dependencies beyond what is already in Cargo.toml.

use std::collections::BTreeMap;

use tempfile::TempDir;

// ─── helpers ────────────────────────────────────────────────────────────────

/// A minimal project root that the tests can use to construct normalized paths.
struct TestProject {
    dir: TempDir,
}

impl TestProject {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        Self { dir }
    }

    /// Returns the absolute path to the project root.
    fn root(&self) -> std::path::PathBuf {
        self.dir.path().to_path_buf()
    }

    /// Create a file at `relative` inside the project root.
    fn write_file(&self, relative: &str, contents: &[u8]) -> std::path::PathBuf {
        let path = self.root().join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, contents).unwrap();
        path
    }

    /// Create a symlink at `link_relative` pointing to `target_relative`.
    /// Returns the symlink path.
    #[cfg(unix)]
    fn create_symlink(&self, link_relative: &str, target_relative: &str) -> std::path::PathBuf {
        use std::os::unix::fs::symlink;
        let link = self.root().join(link_relative);
        let target = self.root().join(target_relative);
        if let Some(parent) = link.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        symlink(&target, &link).unwrap();
        link
    }
}

// ─── parse fingerprint tests ─────────────────────────────────────────────────

mod parse_fingerprint {
    use super::*;

    /// Digests of two distinct byte sequences must differ.
    #[test]
    fn parse_fingerprint_differs_for_different_content() {
        let proj = TestProject::new();
        let path_a = proj.write_file("src/main.rs", b"fn main() {}");
        let path_b = proj.write_file("src/other.rs", b"fn other() {}");

        let fp_a = astynex::persistence::fingerprint::ParseFingerprint::compute(
            &proj.root(),
            &path_a,
            b"fn main() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        )
        .unwrap();

        let fp_b = astynex::persistence::fingerprint::ParseFingerprint::compute(
            &proj.root(),
            &path_b,
            b"fn other() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        )
        .unwrap();

        assert_ne!(fp_a.digest(), fp_b.digest());
    }

    /// Identical inputs must reproduce identical fingerprints.
    #[test]
    fn parse_fingerprint_deterministic() {
        let proj = TestProject::new();
        let path = proj.write_file("src/main.rs", b"fn main() {}");

        let fp1 = astynex::persistence::fingerprint::ParseFingerprint::compute(
            &proj.root(),
            &path,
            b"fn main() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        )
        .unwrap();

        let fp2 = astynex::persistence::fingerprint::ParseFingerprint::compute(
            &proj.root(),
            &path,
            b"fn main() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        )
        .unwrap();

        assert_eq!(fp1.digest(), fp2.digest());
    }

    /// Parser version change must alter the fingerprint.
    #[test]
    fn parse_fingerprint_changes_on_parser_version() {
        let proj = TestProject::new();
        let path = proj.write_file("src/main.rs", b"fn main() {}");

        let fp1 = astynex::persistence::fingerprint::ParseFingerprint::compute(
            &proj.root(),
            &path,
            b"fn main() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        )
        .unwrap();

        let fp2 = astynex::persistence::fingerprint::ParseFingerprint::compute(
            &proj.root(),
            &path,
            b"fn main() {}",
            "rust",
            "2.0.0",
            "0.1.0",
            "0.1.0",
        )
        .unwrap();

        assert_ne!(fp1.digest(), fp2.digest());
    }

    /// Grammar version change must alter the fingerprint.
    #[test]
    fn parse_fingerprint_changes_on_grammar_version() {
        let proj = TestProject::new();
        let path = proj.write_file("src/main.rs", b"fn main() {}");

        let fp1 = astynex::persistence::fingerprint::ParseFingerprint::compute(
            &proj.root(),
            &path,
            b"fn main() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        )
        .unwrap();

        let fp2 = astynex::persistence::fingerprint::ParseFingerprint::compute(
            &proj.root(),
            &path,
            b"fn main() {}",
            "rust",
            "1.0.0",
            "0.2.0",
            "0.1.0",
        )
        .unwrap();

        assert_ne!(fp1.digest(), fp2.digest());
    }

    /// Facts schema version change must alter the fingerprint.
    #[test]
    fn parse_fingerprint_changes_on_facts_schema_version() {
        let proj = TestProject::new();
        let path = proj.write_file("src/main.rs", b"fn main() {}");

        let fp1 = astynex::persistence::fingerprint::ParseFingerprint::compute(
            &proj.root(),
            &path,
            b"fn main() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        )
        .unwrap();

        let fp2 = astynex::persistence::fingerprint::ParseFingerprint::compute(
            &proj.root(),
            &path,
            b"fn main() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.2.0",
        )
        .unwrap();

        assert_ne!(fp1.digest(), fp2.digest());
    }

    /// Language change must alter the fingerprint.
    #[test]
    fn parse_fingerprint_changes_on_language() {
        let proj = TestProject::new();
        let path = proj.write_file("src/main.rs", b"fn main() {}");

        let fp1 = astynex::persistence::fingerprint::ParseFingerprint::compute(
            &proj.root(),
            &path,
            b"fn main() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        )
        .unwrap();

        let fp2 = astynex::persistence::fingerprint::ParseFingerprint::compute(
            &proj.root(),
            &path,
            b"fn main() {}",
            "python",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        )
        .unwrap();

        assert_ne!(fp1.digest(), fp2.digest());
    }

    /// Content digest change must alter the fingerprint.
    #[test]
    fn parse_fingerprint_changes_on_content() {
        let proj = TestProject::new();
        let path = proj.write_file("src/main.rs", b"fn main() {}");

        let fp1 = astynex::persistence::fingerprint::ParseFingerprint::compute(
            &proj.root(),
            &path,
            b"fn main() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        )
        .unwrap();

        let fp2 = astynex::persistence::fingerprint::ParseFingerprint::compute(
            &proj.root(),
            &path,
            b"fn changed() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        )
        .unwrap();

        assert_ne!(fp1.digest(), fp2.digest());
    }
}

// ─── analysis fingerprint tests ──────────────────────────────────────────────

mod analysis_fingerprint {
    use super::*;
    use astynex::persistence::fingerprint::{
        AnalysisFingerprint, AnalysisInput, DigestBytes, ParseFingerprint,
    };

    /// Same ordered context produces the same fingerprint.
    #[test]
    fn analysis_fingerprint_deterministic() {
        let proj = TestProject::new();
        let _path = proj.write_file("src/main.rs", b"fn main() {}");

        let root = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let ctx1 = vec![AnalysisInput {
            file_path: "src/dep.rs".into(),
            content_digest: DigestBytes::compute(b"dep content"),
            role: "context".into(),
        }];
        let ctx2 = vec![AnalysisInput {
            file_path: "src/dep.rs".into(),
            content_digest: DigestBytes::compute(b"dep content"),
            role: "context".into(),
        }];

        let fp1 = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "main",
            &ctx1,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();
        let fp2 = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "main",
            &ctx2,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        assert_eq!(fp1.digest(), fp2.digest());
    }

    /// Analysis fingerprint MUST differ when provider changes.
    #[test]
    fn analysis_fingerprint_changes_on_provider() {
        let proj = TestProject::new();
        let _path = proj.write_file("src/main.rs", b"fn main() {}");
        let ctx = vec![];
        let root = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let fp1 = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "main",
            &ctx,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();
        let fp2 = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "main",
            &ctx,
            "1.0.0",
            "openai",
            "gpt-4",
            "v1",
            None,
        )
        .unwrap();

        assert_ne!(fp1.digest(), fp2.digest());
    }

    /// Analysis fingerprint MUST differ when model changes.
    #[test]
    fn analysis_fingerprint_changes_on_model() {
        let proj = TestProject::new();
        let _path = proj.write_file("src/main.rs", b"fn main() {}");
        let ctx = vec![];
        let root = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let fp1 = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "main",
            &ctx,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();
        let fp2 = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "main",
            &ctx,
            "1.0.0",
            "ollama",
            "llama3.1",
            "v1",
            None,
        )
        .unwrap();

        assert_ne!(fp1.digest(), fp2.digest());
    }

    /// Analysis fingerprint MUST differ when prompt template version changes.
    #[test]
    fn analysis_fingerprint_changes_on_prompt_template_version() {
        let proj = TestProject::new();
        let _path = proj.write_file("src/main.rs", b"fn main() {}");
        let ctx = vec![];
        let root = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let fp1 = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "main",
            &ctx,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();
        let fp2 = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "main",
            &ctx,
            "1.0.0",
            "ollama",
            "llama3",
            "v2",
            None,
        )
        .unwrap();

        assert_ne!(fp1.digest(), fp2.digest());
    }

    /// Analysis fingerprint MUST differ when IR schema version changes.
    #[test]
    fn analysis_fingerprint_changes_on_ir_schema_version() {
        let proj = TestProject::new();
        let _path = proj.write_file("src/main.rs", b"fn main() {}");
        let ctx = vec![];
        let root = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let fp1 = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "main",
            &ctx,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();
        let fp2 = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "main",
            &ctx,
            "2.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        assert_ne!(fp1.digest(), fp2.digest());
    }

    /// Reordering context inputs MUST alter the fingerprint.
    #[test]
    fn analysis_fingerprint_changes_on_context_reorder() {
        let proj = TestProject::new();
        let _path = proj.write_file("src/main.rs", b"fn main() {}");
        let root = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        // Two different context files, added in different order.
        let ctx_a = vec![
            AnalysisInput {
                file_path: "src/a.rs".into(),
                content_digest: DigestBytes::compute(b"a"),
                role: "context".into(),
            },
            AnalysisInput {
                file_path: "src/b.rs".into(),
                content_digest: DigestBytes::compute(b"b"),
                role: "context".into(),
            },
        ];

        let ctx_b = vec![
            AnalysisInput {
                file_path: "src/b.rs".into(),
                content_digest: DigestBytes::compute(b"b"),
                role: "context".into(),
            },
            AnalysisInput {
                file_path: "src/a.rs".into(),
                content_digest: DigestBytes::compute(b"a"),
                role: "context".into(),
            },
        ];

        let fp_a = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "main",
            &ctx_a,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();
        let fp_b = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "main",
            &ctx_b,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        assert_ne!(
            fp_a.digest(),
            fp_b.digest(),
            "reordered context must produce a different fingerprint"
        );
    }

    /// Changing the content digest of one context file MUST alter the fingerprint.
    #[test]
    fn analysis_fingerprint_changes_on_context_content_change() {
        let proj = TestProject::new();
        let _path = proj.write_file("src/main.rs", b"fn main() {}");
        let root = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let ctx_a = vec![AnalysisInput {
            file_path: "src/dep.rs".into(),
            content_digest: DigestBytes::compute(b"original"),
            role: "context".into(),
        }];

        let ctx_b = vec![AnalysisInput {
            file_path: "src/dep.rs".into(),
            content_digest: DigestBytes::compute(b"modified"),
            role: "context".into(),
        }];

        let fp_a = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "main",
            &ctx_a,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();
        let fp_b = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "main",
            &ctx_b,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        assert_ne!(
            fp_a.digest(),
            fp_b.digest(),
            "changed context content digest must produce a different fingerprint"
        );
    }

    /// Changing context membership (different files) MUST alter the fingerprint.
    #[test]
    fn analysis_fingerprint_changes_on_context_membership() {
        let proj = TestProject::new();
        let _path = proj.write_file("src/main.rs", b"fn main() {}");
        let root = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let ctx_a = vec![AnalysisInput {
            file_path: "src/a.rs".into(),
            content_digest: DigestBytes::compute(b"a"),
            role: "context".into(),
        }];

        let ctx_b = vec![AnalysisInput {
            file_path: "src/b.rs".into(),
            content_digest: DigestBytes::compute(b"b"),
            role: "context".into(),
        }];

        let fp_a = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "main",
            &ctx_a,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();
        let fp_b = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "main",
            &ctx_b,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        assert_ne!(
            fp_a.digest(),
            fp_b.digest(),
            "different context membership must produce a different fingerprint"
        );
    }

    /// Removing a context file MUST alter the fingerprint.
    #[test]
    fn analysis_fingerprint_changes_on_context_removal() {
        let proj = TestProject::new();
        let _path = proj.write_file("src/main.rs", b"fn main() {}");
        let root = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let ctx_with = vec![AnalysisInput {
            file_path: "src/dep.rs".into(),
            content_digest: DigestBytes::compute(b"dep"),
            role: "context".into(),
        }];
        let ctx_empty: Vec<AnalysisInput> = vec![];

        let fp_with = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "main",
            &ctx_with,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();
        let fp_empty = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "main",
            &ctx_empty,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        assert_ne!(
            fp_with.digest(),
            fp_empty.digest(),
            "removing context must produce a different fingerprint"
        );
    }

    /// Analysis fingerprint includes provider/model — different from parse fingerprint.
    #[test]
    fn analysis_and_parse_fingerprints_are_separate_domains() {
        let proj = TestProject::new();
        let path = proj.write_file("src/main.rs", b"fn main() {}");
        let root = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let pf = ParseFingerprint::compute(
            &proj.root(),
            &path,
            b"fn main() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        )
        .unwrap();

        let af = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "main",
            &[],
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        // Domain separation must produce different digests.
        assert_ne!(pf.digest(), af.digest());
    }

    /// Analysis fingerprint includes output-affecting config.
    #[test]
    fn analysis_fingerprint_changes_on_config() {
        let proj = TestProject::new();
        let _path = proj.write_file("src/main.rs", b"fn main() {}");
        let ctx = vec![];
        let root = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let mut cfg_a = BTreeMap::new();
        cfg_a.insert("temperature".to_string(), "0.0".to_string());

        let mut cfg_b = BTreeMap::new();
        cfg_b.insert("temperature".to_string(), "0.7".to_string());

        let fp_a = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "main",
            &ctx,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            Some(&cfg_a),
        )
        .unwrap();
        let fp_b = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "main",
            &ctx,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            Some(&cfg_b),
        )
        .unwrap();

        assert_ne!(
            fp_a.digest(),
            fp_b.digest(),
            "different output-affecting config must produce a different fingerprint"
        );
    }

    /// Symbol query change must alter the fingerprint.
    #[test]
    fn analysis_fingerprint_changes_on_symbol_query() {
        let proj = TestProject::new();
        let _path = proj.write_file("src/main.rs", b"fn main() {}");
        let ctx = vec![];
        let root = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let fp1 = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "main",
            &ctx,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();
        let fp2 = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "other",
            &ctx,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        assert_ne!(fp1.digest(), fp2.digest());
    }

    /// Root content digest MUST participate in the fingerprint.
    /// Changing only the root file's content digest produces a different fingerprint.
    #[test]
    fn analysis_fingerprint_changes_on_root_content() {
        let proj = TestProject::new();
        let _path = proj.write_file("src/main.rs", b"fn main() {}");
        let ctx = vec![];

        let root_a = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };
        let root_b = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn changed() {}"),
            role: "root".into(),
        };

        let fp_a = AnalysisFingerprint::compute(
            &proj.root(),
            &root_a,
            "main",
            &ctx,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();
        let fp_b = AnalysisFingerprint::compute(
            &proj.root(),
            &root_b,
            "main",
            &ctx,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        assert_ne!(
            fp_a.digest(),
            fp_b.digest(),
            "different root content digest must produce a different fingerprint"
        );
    }

    /// Config absent (None) and config present but empty (Some empty BTreeMap)
    /// MUST produce distinct fingerprints.
    #[test]
    fn analysis_fingerprint_none_vs_empty_config() {
        let proj = TestProject::new();
        let _path = proj.write_file("src/main.rs", b"fn main() {}");
        let ctx = vec![];
        let root = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };
        let empty_config = BTreeMap::<String, String>::new();

        let fp_none = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "main",
            &ctx,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();
        let fp_empty = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "main",
            &ctx,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            Some(&empty_config),
        )
        .unwrap();

        assert_ne!(
            fp_none.digest(),
            fp_empty.digest(),
            "None config and Some(empty BTreeMap) must produce distinct fingerprints"
        );
    }

    /// Root role must be "root"; invalid role is rejected.
    #[test]
    fn analysis_fingerprint_rejects_invalid_root_role() {
        let _proj = TestProject::new();
        let bad_root = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "context".into(), // wrong role
        };

        let result = AnalysisFingerprint::compute(
            std::path::Path::new("/tmp"),
            &bad_root,
            "main",
            &[],
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        );

        assert!(
            result.is_err(),
            "root input with non-'root' role must be rejected"
        );
    }

    /// Duplicate file-path identities across root and context must be rejected.
    #[test]
    fn analysis_fingerprint_rejects_duplicate_identity() {
        let proj = TestProject::new();
        let _path = proj.write_file("src/main.rs", b"fn main() {}");

        let root = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        // Context contains the same path as root.
        let ctx = vec![AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "context".into(),
        }];

        let result = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "main",
            &ctx,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        );

        assert!(
            result.is_err(),
            "duplicate analysis input path must be rejected"
        );
    }
}

// ─── analysis path policy tests ─────────────────────────────────────────────

mod analysis_path_policy {
    use super::*;
    use astynex::persistence::fingerprint::{AnalysisFingerprint, AnalysisInput, DigestBytes};

    /// Context paths that are POSIX absolute must be rejected.
    #[test]
    fn analysis_rejects_absolute_context_path() {
        let _proj = TestProject::new();
        let root = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let ctx = vec![AnalysisInput {
            file_path: "/etc/passwd".into(),
            content_digest: DigestBytes::compute(b"evil"),
            role: "context".into(),
        }];

        let result = AnalysisFingerprint::compute(
            std::path::Path::new("/tmp"),
            &root,
            "main",
            &ctx,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        );

        assert!(
            result.is_err(),
            "POSIX absolute context path must be rejected"
        );
    }

    /// Context paths that escape via `..` must be rejected.
    #[test]
    fn analysis_rejects_traversal_context_path() {
        let _proj = TestProject::new();
        let root = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let ctx = vec![AnalysisInput {
            file_path: "../secret.txt".into(),
            content_digest: DigestBytes::compute(b"data"),
            role: "context".into(),
        }];

        let result = AnalysisFingerprint::compute(
            std::path::Path::new("/tmp"),
            &root,
            "main",
            &ctx,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        );

        assert!(result.is_err(), "traversal context path must be rejected");
    }

    /// Context paths that normalize to empty must be rejected.
    #[test]
    fn analysis_rejects_empty_context_path() {
        let _proj = TestProject::new();
        let root = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let ctx = vec![AnalysisInput {
            file_path: "".into(),
            content_digest: DigestBytes::compute(b"data"),
            role: "context".into(),
        }];

        let result = AnalysisFingerprint::compute(
            std::path::Path::new("/tmp"),
            &root,
            "main",
            &ctx,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        );

        assert!(result.is_err(), "empty context path must be rejected");
    }

    /// Context paths that are root-only (".") must be rejected.
    #[test]
    fn analysis_rejects_dot_only_context_path() {
        let _proj = TestProject::new();
        let root = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let ctx = vec![AnalysisInput {
            file_path: ".".into(),
            content_digest: DigestBytes::compute(b"data"),
            role: "context".into(),
        }];

        let result = AnalysisFingerprint::compute(
            std::path::Path::new("/tmp"),
            &root,
            "main",
            &ctx,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        );

        assert!(result.is_err(), "dot-only context path must be rejected");
    }

    /// Windows drive-letter absolute paths in context must be rejected.
    #[test]
    fn analysis_rejects_windows_drive_context_path() {
        let _proj = TestProject::new();
        let root = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let ctx = vec![AnalysisInput {
            file_path: "C:\\Windows\\System32".into(),
            content_digest: DigestBytes::compute(b"data"),
            role: "context".into(),
        }];

        let result = AnalysisFingerprint::compute(
            std::path::Path::new("/tmp"),
            &root,
            "main",
            &ctx,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        );

        assert!(
            result.is_err(),
            "Windows drive-letter context path must be rejected"
        );
    }

    /// UNC paths in context must be rejected.
    #[test]
    fn analysis_rejects_unc_context_path() {
        let _proj = TestProject::new();
        let root = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let ctx = vec![AnalysisInput {
            file_path: "\\\\server\\share\\file".into(),
            content_digest: DigestBytes::compute(b"data"),
            role: "context".into(),
        }];

        let result = AnalysisFingerprint::compute(
            std::path::Path::new("/tmp"),
            &root,
            "main",
            &ctx,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        );

        assert!(result.is_err(), "UNC context path must be rejected");
    }

    /// Real `./` normalization: identical files produce the same fingerprint.
    #[test]
    fn analysis_normalizes_dot_prefix() {
        let proj = TestProject::new();
        let _path = proj.write_file("src/main.rs", b"fn main() {}");

        let root_a = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };
        let root_b = AnalysisInput {
            file_path: "./src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let fp_a = AnalysisFingerprint::compute(
            &proj.root(),
            &root_a,
            "main",
            &[],
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();
        let fp_b = AnalysisFingerprint::compute(
            &proj.root(),
            &root_b,
            "main",
            &[],
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        // "./src/main.rs" and "src/main.rs" must normalize to the same path.
        assert_eq!(
            fp_a.digest(),
            fp_b.digest(),
            "`./src/main.rs` and `src/main.rs` must produce identical fingerprints"
        );
    }

    /// Internal `..` components are normalized away, producing alias equality.
    /// For example, `a/../src/main.rs` and `src/main.rs` produce the same fingerprint.
    #[test]
    fn analysis_normalizes_internal_dotdot() {
        let proj = TestProject::new();
        let _path = proj.write_file("src/main.rs", b"fn main() {}");

        let root_a = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };
        let root_b = AnalysisInput {
            file_path: "a/../src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let fp_a = AnalysisFingerprint::compute(
            &proj.root(),
            &root_a,
            "main",
            &[],
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();
        let fp_b = AnalysisFingerprint::compute(
            &proj.root(),
            &root_b,
            "main",
            &[],
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        // `a/../src/main.rs` and `src/main.rs` must normalize to the same path.
        assert_eq!(
            fp_a.digest(),
            fp_b.digest(),
            "`a/../src/main.rs` and `src/main.rs` must produce identical fingerprints"
        );
    }
}

// ─── domain separation tests ─────────────────────────────────────────────────

mod domain_separation {
    use super::*;
    use astynex::persistence::fingerprint::{AnalysisFingerprint, DigestBytes, ParseFingerprint};

    /// Identical inputs must NOT produce identical digests across domains.
    /// Domain bytes are prepended to each fingerprint to prevent collisions.
    #[test]
    fn parse_and_analysis_domains_differ() {
        let proj = TestProject::new();
        let path = proj.write_file("src/main.rs", b"fn main() {}");
        let root = astynex::persistence::fingerprint::AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        // Using the same content digest for both fingerprints.
        let pf = ParseFingerprint::compute(
            &proj.root(),
            &path,
            b"fn main() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        )
        .unwrap();

        let af = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "main",
            &[],
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        // Even though the underlying content is the same, domain separation
        // must produce different digests.
        assert_ne!(pf.digest(), af.digest());
    }
}

// ─── path normalization tests ─────────────────────────────────────────────────

mod path_normalization {
    use super::*;
    use astynex::persistence::fingerprint::ParseFingerprint;

    /// Paths with a leading `./` are normalized to the same fingerprint as
    /// without the prefix.  Both inputs are expressed as relative `Path` values
    /// with the project root passed separately, so the dot-component
    /// normalization happens inside `normalize_path` (via `canonicalize`).
    #[test]
    fn parse_fingerprint_normalizes_dot_prefix() {
        let proj = TestProject::new();
        let _path = proj.write_file("src/main.rs", b"fn main() {}");

        // Path without "./" prefix, expressed as a relative PathBuf.
        let fp1 = ParseFingerprint::compute(
            &proj.root(),
            std::path::Path::new("src/main.rs"),
            b"fn main() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        )
        .unwrap();

        // Path with "./" prefix, expressed as a relative PathBuf (the root is
        // passed as the separate `project_root` argument).
        let fp2 = ParseFingerprint::compute(
            &proj.root(),
            std::path::Path::new("./src/main.rs"),
            b"fn main() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        )
        .unwrap();

        // Both must produce the same fingerprint for the same file.
        assert_eq!(
            fp1.digest(),
            fp2.digest(),
            "`./src/main.rs` and `src/main.rs` must produce identical fingerprints"
        );
    }

    /// Verify that `./` normalization works with AnalysisFingerprint too,
    /// and that root-relative paths containing `./` produce equal fingerprints.
    #[test]
    fn analysis_fingerprint_normalizes_dot_prefix_relative() {
        use astynex::persistence::fingerprint::{AnalysisFingerprint, AnalysisInput, DigestBytes};

        let proj = TestProject::new();
        let _path = proj.write_file("src/main.rs", b"fn main() {}");

        let root_a = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };
        let root_b = AnalysisInput {
            file_path: "./src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };

        let fp_a = AnalysisFingerprint::compute(
            &proj.root(),
            &root_a,
            "main",
            &[],
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();
        let fp_b = AnalysisFingerprint::compute(
            &proj.root(),
            &root_b,
            "main",
            &[],
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        )
        .unwrap();

        assert_eq!(
            fp_a.digest(),
            fp_b.digest(),
            "`./src/main.rs` and `src/main.rs` must normalize to same identity"
        );
    }

    /// Relative symlink component (e.g. `symlink_to_dir`) in a context path
    /// is rejected because symlinks are not followed.
    #[test]
    #[cfg(unix)]
    fn analysis_fingerprint_rejects_relative_symlink_component() {
        use astynex::persistence::fingerprint::{AnalysisFingerprint, AnalysisInput, DigestBytes};

        let proj = TestProject::new();
        let _path = proj.write_file("src/main.rs", b"fn main() {}");
        let _dep = proj.write_file("deps/lib.rs", b"pub fn dep() {}");

        // Create a symlink inside the project.
        let symlink_path = proj.root().join("linked_dep");
        std::os::unix::fs::symlink("deps", &symlink_path).unwrap();

        // Context path through the symlink must be rejected.
        let root = AnalysisInput {
            file_path: "src/main.rs".into(),
            content_digest: DigestBytes::compute(b"fn main() {}"),
            role: "root".into(),
        };
        let ctx_through_symlink = vec![AnalysisInput {
            file_path: "linked_dep/lib.rs".into(),
            content_digest: DigestBytes::compute(b"pub fn dep() {}"),
            role: "context".into(),
        }];

        let result = AnalysisFingerprint::compute(
            &proj.root(),
            &root,
            "main",
            &ctx_through_symlink,
            "1.0.0",
            "ollama",
            "llama3",
            "v1",
            None,
        );

        // Symlink component in context path must be rejected.
        assert!(
            result.is_err(),
            "symlink component in context path must be rejected"
        );
    }
}

// ─── path escape / symlink tests ────────────────────────────────────────────

mod path_policy {
    use super::*;
    use astynex::persistence::fingerprint::ParseFingerprint;

    /// A path inside a directory whose name shares a prefix with the project root
    /// (e.g. `<root>` vs `<root>_sibling`) must NOT be accepted as inside the
    /// project root.  This is a regression test for lossy string-prefix
    /// containment (sibling-prefix paths like `project_outside/file.rs` share
    /// the prefix `project` with `project/`).
    #[test]
    fn parse_fingerprint_rejects_sibling_prefix_path() {
        let proj = TestProject::new();
        let _path = proj.write_file("src/main.rs", b"fn main() {}");

        // Create a sibling directory whose name starts with the same prefix
        // as the project root (e.g. "project" vs "project_outside").
        let sibling_root = proj.root().parent().unwrap().join(format!(
            "{}_sibling",
            proj.root().file_name().unwrap().to_string_lossy()
        ));
        std::fs::create_dir_all(&sibling_root).unwrap();
        let sibling_file = sibling_root.join("file.rs");
        std::fs::write(&sibling_file, b"fn sibling() {}").unwrap();

        let result = ParseFingerprint::compute(
            &proj.root(),
            &sibling_file,
            b"fn sibling() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        );

        assert!(
            result.is_err(),
            "sibling-prefix path must NOT be accepted as inside project root"
        );
    }

    /// A path with `..` that escapes the project root is rejected.
    #[test]
    fn parse_fingerprint_rejects_parent_escape() {
        let proj = TestProject::new();
        let _path = proj.write_file("src/main.rs", b"fn main() {}");

        // Attempting to normalize to a path outside the project root.
        let malicious_path = proj.root().join("../etc/passwd");
        let result = ParseFingerprint::compute(
            &proj.root(),
            &malicious_path,
            b"fn main() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        );

        assert!(result.is_err(), "parent-escape path must be rejected");
    }

    /// A path that normalizes to the project root itself is rejected
    /// (only files inside the project are valid).
    #[test]
    fn parse_fingerprint_rejects_project_root() {
        let proj = TestProject::new();

        let result = ParseFingerprint::compute(
            &proj.root(),
            &proj.root(), // the project root itself
            b"not a source file",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        );

        assert!(result.is_err(), "project root itself must be rejected");
    }

    /// A path containing `..` components that resolve inside the project
    /// root is accepted and produces the same fingerprint as the resolved path.
    #[test]
    fn parse_fingerprint_accepts_dotdot_resolving_inside_root() {
        let proj = TestProject::new();
        let path = proj.write_file("src/lib.rs", b"fn lib() {}");

        // Path with .. that resolves to src/lib.rs.
        let resolved = path.canonicalize().unwrap();
        let parent = resolved.parent().unwrap().to_path_buf();
        let dotdot_path = parent.join("..").join("src").join("lib.rs");

        let fp_direct = ParseFingerprint::compute(
            &proj.root(),
            &path,
            b"fn lib() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        )
        .unwrap();

        let fp_dotdot = ParseFingerprint::compute(
            &proj.root(),
            &dotdot_path,
            b"fn lib() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        );

        // The path must be accepted (not rejected for containing ..) because
        // it resolves inside the project root.
        assert!(
            fp_dotdot.is_ok(),
            "dotdot path resolving inside root must be accepted, not rejected"
        );
        assert_eq!(
            fp_direct.digest(),
            fp_dotdot.unwrap().digest(),
            "dotdot path resolving to same file must match direct path fingerprint"
        );
    }

    /// Missing file paths produce an error, not a silent fallback.
    #[test]
    fn parse_fingerprint_rejects_missing_file() {
        let proj = TestProject::new();
        let missing = proj.root().join("src").join("does_not_exist.rs");

        let result = ParseFingerprint::compute(
            &proj.root(),
            &missing,
            b"fn main() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        );

        assert!(result.is_err(), "missing file must produce an error");
    }

    /// Symlink components in the path (but resolving inside the project root)
    /// are rejected — we do not follow symlinks.
    #[test]
    fn parse_fingerprint_rejects_symlink_in_path() {
        let proj = TestProject::new();
        // Create: project/real_dir/file.rs and project/link -> real_dir
        let _real_file = proj.write_file("real_dir/file.rs", b"fn real() {}");
        // Create a symlink inside the project pointing into the project.
        #[cfg(unix)]
        let _symlink = proj.create_symlink("link_dir", "real_dir");

        #[cfg(unix)]
        {
            let symlink_path = proj.root().join("link_dir").join("file.rs");
            // The symlink path must be rejected because symlinks are not followed.
            let result = ParseFingerprint::compute(
                &proj.root(),
                &symlink_path,
                b"fn real() {}",
                "rust",
                "1.0.0",
                "0.1.0",
                "0.1.0",
            );

            assert!(
                result.is_err(),
                "symlink path must be rejected even if it resolves inside project"
            );
        }

        #[cfg(not(unix))]
        {
            let _ = _real_file; // silence unused warning
        }
    }

    /// A path where a symlink component resolves outside the project root
    /// is rejected (escape detection after normalization).
    #[test]
    fn parse_fingerprint_rejects_symlink_escape() {
        let proj = TestProject::new();
        let _real_file = proj.write_file("real_dir/file.rs", b"fn real() {}");

        #[cfg(unix)]
        {
            // Create a symlink that points to the project root (outside the project
            // when followed from within).
            let escape_link = proj.root().join("escape").join("link_to_project");
            std::fs::create_dir_all(escape_link.parent().unwrap()).unwrap();
            std::os::unix::fs::symlink(proj.root().as_path(), &escape_link).unwrap();

            // Try to access a file through the escape symlink.
            let through_escape = escape_link.join("real_dir").join("file.rs");
            let result = ParseFingerprint::compute(
                &proj.root(),
                &through_escape,
                b"fn real() {}",
                "rust",
                "1.0.0",
                "0.1.0",
                "0.1.0",
            );

            assert!(
                result.is_err(),
                "symlink path that escapes the project root must be rejected"
            );
        }

        #[cfg(not(unix))]
        {
            // On non-Unix, this test is a no-op but must compile.
        }
    }

    /// A directory path must be rejected — only regular files are valid source files.
    #[test]
    fn parse_fingerprint_rejects_directory() {
        let proj = TestProject::new();
        let _path = proj.write_file("src/main.rs", b"fn main() {}");

        // Attempting to compute fingerprint for a directory.
        let dir_path = proj.root().join("src");
        let result = ParseFingerprint::compute(
            &proj.root(),
            &dir_path,
            b"directory content (irrelevant)",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        );

        assert!(
            result.is_err(),
            "directory path must be rejected; only regular files are valid"
        );
    }
}

// ─── non-UTF-8 path rejection tests ────────────────────────────────────────

#[cfg(unix)]
mod non_utf8_path {
    use super::*;
    use astynex::persistence::fingerprint::ParseFingerprint;

    /// A regular file with an invalid UTF-8 filename must cause
    /// `ParseFingerprint::compute` to fail with a UTF-8 cache-key error.
    /// This verifies fail-closed UTF-8 conversion: after canonical resolution
    /// the stripped-relative path is checked with `to_str()`; lossy conversion
    /// would silently produce a wrong cache key, so we must reject instead.
    /// The symlink check is exercised first (it passes for a regular file),
    /// then canonicalize succeeds (file exists), and then `to_str()` fails
    /// on the non-UTF-8 resolved path — producing the UTF-8 cache-key error.
    #[test]
    #[cfg(unix)]
    fn parse_fingerprint_rejects_non_utf8_filename() {
        use std::os::unix::ffi::OsStrExt;

        let proj = TestProject::new();

        // Create a directory.
        let dir = proj.root().join("src");
        std::fs::create_dir_all(&dir).unwrap();

        // Create a regular file whose filename contains invalid UTF-8 bytes.
        // On Unix, OsStrExt::from_bytes bypasses Rust's UTF-8 OsStr validation
        // and lets us pass raw bytes to the filesystem, which accepts them.
        let invalid_utf8_name = std::ffi::OsStr::from_bytes(b"main\xff.rs");
        let file_path = dir.join(invalid_utf8_name);

        // Create the regular file with those bytes as its filename.
        std::fs::File::create(&file_path).unwrap();

        // Attempting to compute a fingerprint for the non-UTF-8 path
        // must fail with a UTF-8-specific cache-key error, not merely any
        // CacheDir variant (e.g. not "file path is not accessible").
        let result = ParseFingerprint::compute(
            &proj.root(),
            &file_path,
            b"fn main() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        );

        assert!(
            result.is_err(),
            "non-UTF-8 filename must be rejected, not silently converted"
        );
        let err = result.unwrap_err();
        let msg = err.to_string();
        // Assert the UTF-8 cache-key error specifically, not merely CacheDir.
        assert!(
            msg.contains("not valid UTF-8") && msg.contains("cache key"),
            "expected UTF-8 cache-key error, got: {err}"
        );
    }
}

// ─── symlink parent component tests ────────────────────────────────────────

#[cfg(unix)]
mod symlink_parent_component {
    use super::*;
    use astynex::persistence::fingerprint::ParseFingerprint;

    /// `ParseFingerprint` must reject a path whose parent component is a
    /// symlink, even when the path is expressed as a relative `PathBuf`
    /// (not an absolute path).  This confirms the component-by-component
    /// symlink check in `normalize_path` catches symlink parents before
    /// any canonicalization.
    #[test]
    #[cfg(unix)]
    fn parse_fingerprint_rejects_symlink_parent_relative_path() {
        use std::os::unix::fs::symlink;

        let proj = TestProject::new();
        // Create the real directory and file.
        let _real_file = proj.write_file("real_dir/file.rs", b"fn real() {}");

        // Create a symlink inside the project root pointing to the real dir.
        let symlink_dir = proj.root().join("link_dir");
        symlink("real_dir", &symlink_dir).unwrap();

        // Construct a RELATIVE PathBuf that traverses through the symlink.
        // This is "link_dir" + "file.rs" expressed as a relative path.
        // Using std::path::PathBuf (not joined to project_root) to keep it relative.
        let relative_through_symlink = std::path::PathBuf::from("link_dir").join("file.rs");

        // The path must be rejected because "link_dir" is a symlink component.
        let result = ParseFingerprint::compute(
            &proj.root(),
            &relative_through_symlink,
            b"fn real() {}",
            "rust",
            "1.0.0",
            "0.1.0",
            "0.1.0",
        );

        assert!(
            result.is_err(),
            "relative path through symlink parent must be rejected"
        );
    }
}

// ─── digest bytes (raw SHA-256 without domain) tests ────────────────────────

mod digest_bytes {
    use astynex::persistence::fingerprint::DigestBytes;

    #[test]
    fn digest_bytes_deterministic() {
        let d1 = DigestBytes::compute(b"abc");
        let d2 = DigestBytes::compute(b"abc");
        assert_eq!(d1, d2);
    }

    #[test]
    fn digest_bytes_differs_for_different_content() {
        let d1 = DigestBytes::compute(b"abc");
        let d2 = DigestBytes::compute(b"def");
        assert_ne!(d1, d2);
    }

    #[test]
    fn digest_bytes_empty_input() {
        let d = DigestBytes::compute(b"");
        // SHA-256 of empty input.
        assert_eq!(
            d.as_hex(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn digest_bytes_is_hashable() {
        use std::collections::HashSet;
        let d1 = DigestBytes::compute(b"a");
        let d2 = DigestBytes::compute(b"a");
        let mut set: HashSet<DigestBytes> = HashSet::new();
        assert!(set.insert(d1));
        assert!(!set.insert(d2));
    }

    #[test]
    fn digest_bytes_known_vector() {
        // SHA-256("abc").
        let d = DigestBytes::compute(b"abc");
        assert_eq!(
            d.as_hex(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
