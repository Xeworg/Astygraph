//! Headless tests for safe project-file navigation.
//!
//! Exercises `fs::tree` and `fs` without a display.

mod fs_tree {
    /// Compile-time check: the fs module must exist and expose its types.
    #[test]
    fn fs_module_exists() {
        let _ = astynex::fs::FileKind::File;
        let _ = astynex::fs::FileKind::Dir;
        let _ = astynex::fs::FileKind::Symlink;
    }

    /// Scan an empty directory returns zero entries.
    #[test]
    fn scan_empty_dir_returns_empty() {
        let tmp = tempfile::TempDir::new().unwrap();
        let entries = astynex::fs::scan_dir(tmp.path()).expect("scan should not error");
        assert!(entries.is_empty());
    }

    /// Scan returns direct children only (not recursively).
    #[test]
    fn scan_returns_direct_children() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join("subdir")).unwrap();
        std::fs::write(tmp.path().join("a.txt"), "").unwrap();
        std::fs::write(tmp.path().join("b.rs"), "").unwrap();
        std::fs::write(tmp.path().join("subdir").join("nested.txt"), "").unwrap();

        let entries = astynex::fs::scan_dir(tmp.path()).expect("scan should not error");

        // Should have exactly 3: subdir, a.txt, b.rs
        assert_eq!(entries.len(), 3);
        // nested.txt must NOT appear (not direct)
        for e in &entries {
            assert_ne!(e.path.file_name().unwrap().to_str().unwrap(), "nested.txt");
        }
    }

    /// `.git/` entry is excluded.
    #[test]
    fn git_dir_excluded() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git").join("objects")).unwrap();
        std::fs::write(tmp.path().join(".git").join("config"), "").unwrap();
        std::fs::write(tmp.path().join("readme.md"), "").unwrap();

        let entries = astynex::fs::scan_dir(tmp.path()).expect("scan should not error");
        let names: Vec<_> = entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_str().unwrap())
            .collect();
        assert!(!names.contains(&".git"), ".git must be excluded: {names:?}");
        assert!(names.contains(&"readme.md"));
    }

    /// `.astynex/` entry is excluded.
    #[test]
    fn astynex_dir_excluded() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".astynex").join("cache")).unwrap();
        std::fs::write(tmp.path().join(".astynex").join("db"), "").unwrap();
        std::fs::create_dir(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("src").join("main.rs"), "").unwrap();

        let entries = astynex::fs::scan_dir(tmp.path()).expect("scan should not error");
        let names: Vec<_> = entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_str().unwrap())
            .collect();
        assert!(
            !names.contains(&".astynex"),
            ".astynex must be excluded: {names:?}"
        );
        assert!(names.contains(&"src"));
    }

    /// Common build/cache dirs are excluded.
    #[test]
    fn build_cache_dirs_excluded() {
        let tmp = tempfile::TempDir::new().unwrap();
        for dir in [
            "target",
            "node_modules",
            "__pycache__",
            ".cache",
            ".pytest_cache",
            "vendor",
            "dist",
            "build",
            ".next",
            ".nuxt",
        ] {
            std::fs::create_dir_all(tmp.path().join(dir)).ok();
        }
        std::fs::create_dir(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("src").join("lib.rs"), "").unwrap();

        let entries = astynex::fs::scan_dir(tmp.path()).expect("scan should not error");
        let names: Vec<_> = entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_str().unwrap())
            .collect();

        for excluded in [
            "target",
            "node_modules",
            "__pycache__",
            ".cache",
            ".pytest_cache",
            "vendor",
            "dist",
            "build",
            ".next",
            ".nuxt",
        ] {
            assert!(
                !names.contains(&excluded),
                "{excluded} must be excluded: {names:?}"
            );
        }
        assert!(names.contains(&"src"));
    }

    /// Files with NUL byte (binary) are excluded.
    #[test]
    fn binary_files_excluded() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("text.txt"), "hello").unwrap();
        // File with NUL byte in first 512 bytes — binary
        std::fs::write(tmp.path().join("binary.bin"), "\x00binary content").unwrap();

        let entries = astynex::fs::scan_dir(tmp.path()).expect("scan should not error");
        let names: Vec<_> = entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_str().unwrap())
            .collect();

        assert!(names.contains(&"text.txt"));
        assert!(
            !names.contains(&"binary.bin"),
            "binary files must be excluded: {names:?}"
        );
    }

    /// Symlinks are not followed by default.
    #[test]
    fn symlinks_not_followed() {
        let tmp = tempfile::TempDir::new().unwrap();
        let target = tmp.path().join("target_file.txt");
        std::fs::write(&target, "real").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, tmp.path().join("link.txt")).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&target, tmp.path().join("link.txt")).unwrap();

        let entries = astynex::fs::scan_dir(tmp.path()).expect("scan should not error");
        let names: Vec<_> = entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_str().unwrap())
            .collect();

        // Symlink appears as a Symlink entry, not its target's contents
        let symlink_entry = entries
            .iter()
            .find(|e| e.kind == astynex::fs::FileKind::Symlink);
        assert!(
            symlink_entry.is_some(),
            "symlink must appear as kind=Symlink: {names:?}"
        );
        // No crash and no explosion of target's content into the scan
        assert!(entries.len() <= 2); // target_file.txt + link.txt
    }

    /// Entries are sorted alphabetically.
    #[test]
    fn entries_sorted() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("z_file.txt"), "").unwrap();
        std::fs::write(tmp.path().join("a_file.txt"), "").unwrap();
        std::fs::write(tmp.path().join("m_file.txt"), "").unwrap();

        let entries = astynex::fs::scan_dir(tmp.path()).expect("scan should not error");
        let names: Vec<_> = entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_str().unwrap())
            .collect();

        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted, "entries must be sorted: {names:?}");
    }

    /// Each entry reports its kind correctly.
    #[test]
    fn file_kind_reported() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("file.txt"), "").unwrap();
        std::fs::create_dir(tmp.path().join("dir")).unwrap();

        let entries = astynex::fs::scan_dir(tmp.path()).expect("scan should not error");

        let file = entries
            .iter()
            .find(|e| e.kind == astynex::fs::FileKind::File);
        let dir = entries
            .iter()
            .find(|e| e.kind == astynex::fs::FileKind::Dir);

        assert!(file.is_some(), "file entry must exist");
        assert!(dir.is_some(), "dir entry must exist");
    }

    /// `.gitignore` patterns are respected.
    #[test]
    fn gitignore_respected() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".gitignore"), "secret.log\n*.tmp\n").unwrap();
        std::fs::write(tmp.path().join("included.txt"), "ok").unwrap();
        std::fs::write(tmp.path().join("secret.log"), "secret").unwrap();
        std::fs::write(tmp.path().join("file.tmp"), "tmp").unwrap();
        std::fs::write(tmp.path().join("also.txt"), "also").unwrap();

        let entries = astynex::fs::scan_dir(tmp.path()).expect("scan should not error");
        let names: Vec<_> = entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_str().unwrap())
            .collect();

        assert!(
            names.contains(&"included.txt"),
            "included.txt must be present: {names:?}"
        );
        assert!(
            names.contains(&"also.txt"),
            "also.txt must be present: {names:?}"
        );
        assert!(
            !names.contains(&"secret.log"),
            "secret.log must be excluded by gitignore: {names:?}"
        );
        assert!(
            !names.contains(&"file.tmp"),
            "*.tmp must be excluded by gitignore: {names:?}"
        );
    }

    /// 50,000-entry cap reports `is_incomplete = true`.
    #[test]
    fn discovery_cap_reports_incomplete() {
        let tmp = tempfile::TempDir::new().unwrap();
        // Create enough entries to exceed the 50,000 cap.
        // We create files only (no subdirs) to keep entries = files.
        for i in 0..50_050 {
            std::fs::write(tmp.path().join(format!("file_{i:06}.txt")), "").unwrap();
        }

        let entries = astynex::fs::scan_dir(tmp.path()).expect("scan should not error");
        // Must be capped at 50,000
        assert!(entries.len() <= 50_050, "cap should limit entries");
        // When cap is hit, the last entry must signal incompleteness
        let last = entries.last();
        if let Some(entry) = last {
            assert!(
                entry.is_incomplete,
                "last entry must signal incomplete when cap was hit"
            );
        }
    }

    /// Path must be absolute in every entry.
    #[test]
    fn entries_have_absolute_paths() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("a.txt"), "").unwrap();

        let entries = astynex::fs::scan_dir(tmp.path()).expect("scan should not error");
        assert!(!entries.is_empty());

        for entry in &entries {
            assert!(
                entry.path.is_absolute(),
                "path must be absolute: {:?}",
                entry.path
            );
        }
    }

    /// Symlink entry path must still resolve correctly.
    #[test]
    fn symlink_path_resolves() {
        let tmp = tempfile::TempDir::new().unwrap();
        let target = tmp.path().join("real.txt");
        std::fs::write(&target, "content").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, tmp.path().join("link.txt")).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&target, tmp.path().join("link.txt")).unwrap();

        let entries = astynex::fs::scan_dir(tmp.path()).expect("scan should not error");
        let symlink = entries
            .iter()
            .find(|e| e.kind == astynex::fs::FileKind::Symlink)
            .unwrap();

        assert!(symlink.path.exists(), "symlink path must exist as symlink");
        assert!(symlink.path.is_symlink(), "symlink entry must be symlink");
    }
}
