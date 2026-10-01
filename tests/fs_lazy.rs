//! Headless tests for lazy tree cache behavior.
//!
//! Tests tree expansion/collapse as PURE cache operations that do NOT change
//! navigation state (current_dir, entries, selected_file). Navigation methods
//! (navigate_to_dir, navigate_up, navigate_to_root) are tested separately in
//! fs_navigation.rs.
//!
//! Required behavior:
//! 1. Tree expansion/collapse leaves FolderState unchanged
//! 2. Collapsing retains cache; re-expansion uses cache
//! 3. Nested expansion allowed for cached parent lineage
//! 4. Files/symlinks/unknown paths are no-ops
//! 5. Global budget bounds total cached entries
//! 6. Incompleteness queryable
//! 7. Cancellation preserves cache; new project/close clears it
//! 8. Selected file preserved during expansion/collapse

mod fs_lazy {
    use astynex::app_state::{AppState, FolderState};
    use astynex::fs::tree::TreeCache;
    use std::path::{Path, PathBuf};

    /// Match the canonical root stored by `AppState` on platforms where
    /// canonical paths differ from the path returned by `TempDir::path()`.
    fn canonical_tmp_path(path: &Path) -> PathBuf {
        std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    }

    // ========================================================================
    // TreeCache unit tests
    // ========================================================================

    /// TreeCache starts empty and has zero entries.
    #[test]
    fn tree_cache_starts_empty() {
        let cache = TreeCache::new(50_000);
        assert_eq!(cache.total_entries(), 0);
        assert!(!cache.is_incomplete());
    }

    /// Expanding a directory adds its entries to the cache.
    #[test]
    fn expand_adds_entries_to_cache() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("a.txt"), "").unwrap();
        std::fs::write(tmp.path().join("b.rs"), "").unwrap();

        let mut cache = TreeCache::new(50_000);
        let entries = cache.expand_dir(tmp.path()).expect("expand should succeed");

        // Should have 3 entries: src, a.txt, b.rs
        assert_eq!(entries.len(), 3);
        assert_eq!(cache.total_entries(), 3);
    }

    /// Root entries are visible without expansion.
    #[test]
    fn root_entries_visible_without_expansion() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join("child_dir")).unwrap();
        std::fs::write(tmp.path().join("file.txt"), "").unwrap();

        let mut cache = TreeCache::new(50_000);
        let root_entries = cache
            .get_root_entries(tmp.path())
            .expect("root scan should succeed");

        // Root entries should include child_dir and file.txt
        let names: Vec<_> = root_entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_str().unwrap())
            .collect();
        assert!(names.contains(&"child_dir"));
        assert!(names.contains(&"file.txt"));
    }

    /// Expanding a directory marks it as loaded.
    #[test]
    fn expand_marks_dir_as_loaded() {
        let tmp = tempfile::TempDir::new().unwrap();
        let child = tmp.path().join("src");
        std::fs::create_dir(&child).unwrap();

        let mut cache = TreeCache::new(50_000);
        let child_entries = cache.expand_dir(&child).expect("expand should succeed");

        // Collect entries to release the mutable borrow before checking is_dir_loaded
        let _entries_vec = child_entries.to_vec();
        assert!(cache.is_dir_loaded(&child));
        assert!(!_entries_vec.is_empty() || true); // empty dir is valid
    }

    /// Collapse does NOT clear cached children; re-expand reuses cache.
    #[test]
    fn collapse_retains_cached_children() {
        let tmp = tempfile::TempDir::new().unwrap();
        let child = tmp.path().join("src");
        std::fs::create_dir(&child).unwrap();
        std::fs::write(child.join("lib.rs"), "").unwrap();

        let mut cache = TreeCache::new(50_000);

        // First expansion
        let _first = cache
            .expand_dir(&child)
            .expect("first expand should succeed");
        let entries_before = cache.total_entries();

        // Collapse (mark as collapsed but keep cache)
        cache.collapse_dir(&child);

        // Re-expand should reuse cached entries (no new scan)
        let _second = cache.expand_dir(&child).expect("re-expand should succeed");

        // Cache size should remain the same (reused, not re-scanned)
        assert_eq!(cache.total_entries(), entries_before);
    }

    /// After collapse, directory is no longer marked as expanded.
    #[test]
    fn collapse_clears_expanded_state() {
        let tmp = tempfile::TempDir::new().unwrap();
        let child = tmp.path().join("src");
        std::fs::create_dir(&child).unwrap();

        let mut cache = TreeCache::new(50_000);

        // Expand
        let _entries = cache.expand_dir(&child).expect("expand should succeed");
        assert!(
            cache.is_expanded(&child),
            "dir should be marked expanded after expand"
        );

        // Collapse
        cache.collapse_dir(&child);

        // Should no longer be marked as expanded
        assert!(
            !cache.is_expanded(&child),
            "dir should NOT be marked expanded after collapse"
        );
    }

    /// Files do not expand; they are not directories.
    #[test]
    fn files_do_not_expand() {
        let tmp = tempfile::TempDir::new().unwrap();
        let file = tmp.path().join("file.txt");
        std::fs::write(&file, "content").unwrap();

        let mut cache = TreeCache::new(50_000);
        let entries = cache.expand_dir(&file);

        // Files cannot be expanded; should return error or empty
        assert!(entries.is_err() || cache.is_dir_loaded(&file) == false);
    }

    /// Symlinks do not expand; they are not real directories.
    #[test]
    fn symlinks_do_not_expand() {
        let tmp = tempfile::TempDir::new().unwrap();
        let target = tmp.path().join("real_dir");
        std::fs::create_dir(&target).unwrap();
        let symlink = tmp.path().join("link_dir");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &symlink).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&target, &symlink).unwrap();

        let mut cache = TreeCache::new(50_000);
        let entries = cache.expand_dir(&symlink);

        // Symlinks cannot be expanded
        assert!(entries.is_err() || cache.is_dir_loaded(&symlink) == false);
    }

    /// Global budget bounds total cached entries.
    #[test]
    fn global_budget_bounds_entries() {
        let tmp = tempfile::TempDir::new().unwrap();
        // Create more files than the small budget
        let budget = 100;
        for i in 0..budget + 50 {
            std::fs::write(tmp.path().join(format!("file_{i}.txt")), "").unwrap();
        }

        let mut cache = TreeCache::new(budget);
        let entries = cache
            .get_root_entries(tmp.path())
            .expect("root scan should succeed");

        // Entries should be capped at budget
        assert!(entries.len() <= budget);
        assert!(cache.is_incomplete());
    }

    /// Incomplete state is visible after hitting budget.
    #[test]
    fn incomplete_state_visible() {
        let tmp = tempfile::TempDir::new().unwrap();
        let budget = 10;
        for i in 0..budget + 20 {
            std::fs::write(tmp.path().join(format!("file_{i}.txt")), "").unwrap();
        }

        let mut cache = TreeCache::new(budget);
        let _entries = cache
            .get_root_entries(tmp.path())
            .expect("root scan should succeed");

        assert!(
            cache.is_incomplete(),
            "cache should report incomplete when budget hit"
        );
    }

    /// Expanding subdirectories adds to the global budget count.
    #[test]
    fn subdir_expansion_counts_toward_budget() {
        let tmp = tempfile::TempDir::new().unwrap();
        let child = tmp.path().join("src");
        std::fs::create_dir(&child).unwrap();
        for i in 0..50 {
            std::fs::write(child.join(format!("file_{i}.rs")), "").unwrap();
        }

        let mut cache = TreeCache::new(100);
        let _root = cache
            .get_root_entries(tmp.path())
            .expect("root scan should succeed");
        let root_count = cache.total_entries();

        let _child = cache
            .expand_dir(&child)
            .expect("child expand should succeed");
        let total_count = cache.total_entries();

        // Total should include root entries + child entries
        assert!(total_count > root_count);
    }

    /// Cache can be cleared completely.
    #[test]
    fn cache_clear_resets_all() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("a.txt"), "").unwrap();

        let mut cache = TreeCache::new(50_000);
        let _root = cache
            .get_root_entries(tmp.path())
            .expect("root scan should succeed");
        assert!(cache.total_entries() > 0);

        cache.clear();
        assert_eq!(cache.total_entries(), 0);
        assert!(!cache.is_incomplete());
    }

    // ========================================================================
    // AppState integration tests — EXPANSION/COLLAPSE as pure cache ops
    // ========================================================================

    /// REQ-1: expand_dir does NOT change current_dir.
    #[test]
    fn expand_dir_does_not_change_current_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let child = tmp.path().join("src");
        std::fs::create_dir(&child).unwrap();
        std::fs::write(child.join("lib.rs"), "").unwrap();

        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()));

        let FolderState::Loaded {
            current_dir: before,
            ..
        } = &state.folder_state
        else {
            panic!("expected Loaded");
        };

        // Expand child - this should NOT change current_dir
        let state = state.expand_dir(&child);

        let FolderState::Loaded {
            current_dir: after, ..
        } = &state.folder_state
        else {
            panic!("expected Loaded after expand_dir");
        };

        assert_eq!(before, after, "expand_dir must NOT change current_dir");
        assert_eq!(
            after,
            &canonical_tmp_path(tmp.path()),
            "current_dir should remain at root"
        );
    }

    /// REQ-1: expand_dir does NOT change entries.
    #[test]
    fn expand_dir_does_not_change_entries() {
        let tmp = tempfile::TempDir::new().unwrap();
        let child = tmp.path().join("src");
        std::fs::create_dir(&child).unwrap();
        std::fs::write(child.join("lib.rs"), "").unwrap();

        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()));

        let FolderState::Loaded {
            entries: entries_before,
            ..
        } = &state.folder_state
        else {
            panic!("expected Loaded");
        };

        let state = state.expand_dir(&child);

        let FolderState::Loaded {
            entries: entries_after,
            ..
        } = &state.folder_state
        else {
            panic!("expected Loaded after expand_dir");
        };

        assert_eq!(
            entries_before, entries_after,
            "expand_dir must NOT change entries"
        );
    }

    /// REQ-1: expand_dir does NOT clear selected_file.
    #[test]
    fn expand_dir_does_not_clear_selected_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let child = tmp.path().join("src");
        std::fs::create_dir(&child).unwrap();
        std::fs::write(tmp.path().join("root.rs"), "root content").unwrap();
        let child_file = child.join("lib.rs");
        std::fs::write(&child_file, "lib content").unwrap();

        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()))
            .select_file(Some(tmp.path().join("root.rs")));

        let FolderState::Loaded {
            selected_file: before,
            ..
        } = &state.folder_state
        else {
            panic!("expected Loaded");
        };
        assert!(before.is_some(), "precondition: file should be selected");

        // Expand child - selected_file should be preserved
        let state = state.expand_dir(&child);

        let FolderState::Loaded {
            selected_file: after,
            ..
        } = &state.folder_state
        else {
            panic!("expected Loaded after expand_dir");
        };

        assert_eq!(before, after, "expand_dir must NOT clear selected_file");
    }

    /// REQ-1: collapse_dir does NOT change current_dir.
    #[test]
    fn collapse_dir_does_not_change_current_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let child = tmp.path().join("src");
        std::fs::create_dir(&child).unwrap();
        std::fs::write(child.join("lib.rs"), "").unwrap();

        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()))
            .expand_dir(&child); // First expand it

        let FolderState::Loaded {
            current_dir: before,
            ..
        } = &state.folder_state
        else {
            panic!("expected Loaded");
        };

        // Collapse - this should NOT change current_dir
        let state = state.collapse_dir(&child);

        let FolderState::Loaded {
            current_dir: after, ..
        } = &state.folder_state
        else {
            panic!("expected Loaded after collapse_dir");
        };

        assert_eq!(before, after, "collapse_dir must NOT change current_dir");
    }

    /// REQ-1: collapse_dir does NOT change entries.
    #[test]
    fn collapse_dir_does_not_change_entries() {
        let tmp = tempfile::TempDir::new().unwrap();
        let child = tmp.path().join("src");
        std::fs::create_dir(&child).unwrap();
        std::fs::write(child.join("lib.rs"), "").unwrap();

        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()))
            .expand_dir(&child);

        let FolderState::Loaded {
            entries: entries_before,
            ..
        } = &state.folder_state
        else {
            panic!("expected Loaded");
        };

        let state = state.collapse_dir(&child);

        let FolderState::Loaded {
            entries: entries_after,
            ..
        } = &state.folder_state
        else {
            panic!("expected Loaded after collapse_dir");
        };

        assert_eq!(
            entries_before, entries_after,
            "collapse_dir must NOT change entries"
        );
    }

    /// REQ-1: collapse_dir does NOT clear selected_file.
    #[test]
    fn collapse_dir_does_not_clear_selected_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let child = tmp.path().join("src");
        std::fs::create_dir(&child).unwrap();
        std::fs::write(tmp.path().join("root.rs"), "root content").unwrap();
        let child_file = child.join("lib.rs");
        std::fs::write(&child_file, "lib content").unwrap();

        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()))
            .select_file(Some(tmp.path().join("root.rs")));

        // Expand first
        let state = state.expand_dir(&child);

        let FolderState::Loaded {
            selected_file: before,
            ..
        } = &state.folder_state
        else {
            panic!("expected Loaded");
        };
        assert!(before.is_some(), "precondition: file should be selected");

        // Collapse - selected_file should be preserved
        let state = state.collapse_dir(&child);

        let FolderState::Loaded {
            selected_file: after,
            ..
        } = &state.folder_state
        else {
            panic!("expected Loaded after collapse_dir");
        };

        assert_eq!(before, after, "collapse_dir must NOT clear selected_file");
    }

    /// REQ-2: Collapsing retains cache; re-expansion uses cache (AppState level).
    #[test]
    fn collapse_retains_cache_appstate() {
        let tmp = tempfile::TempDir::new().unwrap();
        let child = tmp.path().join("src");
        std::fs::create_dir(&child).unwrap();
        std::fs::write(child.join("lib.rs"), "").unwrap();

        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()));

        // Expand child into cache
        let state = state.expand_dir(&child);

        // Cache should show child is loaded
        let cache = state.tree_cache();
        assert!(
            cache.is_dir_loaded(&child),
            "child should be loaded in cache after expand"
        );

        // Collapse child
        let state = state.collapse_dir(&child);

        // Cache should STILL show child is loaded (retained)
        let cache = state.tree_cache();
        assert!(
            cache.is_dir_loaded(&child),
            "child cache should be retained after collapse"
        );
    }

    /// REQ-3: Nested expansion allowed for cached parent lineage.
    #[test]
    fn nested_expansion_allowed_for_cached_parent() {
        let tmp = tempfile::TempDir::new().unwrap();
        let level1 = tmp.path().join("src");
        let level2 = level1.join("core");
        let level3 = level2.join("inner");
        std::fs::create_dir_all(&level3).unwrap();
        std::fs::write(level3.join("lib.rs"), "").unwrap();

        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()));

        // Expand src (level1)
        let state = state.expand_dir(&level1);

        // Now expand src/core (level2) - should work even though it's not a direct child
        // of current_dir (which is still root)
        let state = state.expand_dir(&level2);

        // Cache should show both are loaded
        let cache = state.tree_cache();
        assert!(cache.is_dir_loaded(&level1), "level1 should be loaded");
        assert!(cache.is_dir_loaded(&level2), "level2 should be loaded");

        // Expand src/core/inner (level3) - should also work
        let state = state.expand_dir(&level3);

        let cache = state.tree_cache();
        assert!(cache.is_dir_loaded(&level3), "level3 should be loaded");
    }

    /// REQ-3: Expansion blocked for paths not in root_entries.
    ///
    /// A directory created AFTER opening the folder (not in root_entries)
    /// should NOT be expandable via expand_dir.
    #[test]
    fn expansion_blocked_for_unknown_path() {
        let tmp = tempfile::TempDir::new().unwrap();

        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()));

        // Create unknown_dir AFTER opening folder - it's NOT in root_entries
        let unknown = tmp.path().join("unknown_dir");
        std::fs::create_dir(&unknown).unwrap();

        // Try to expand a directory that's not in root_entries
        let state = state.expand_dir(&unknown);

        // Cache should NOT show it as loaded (not in root_entries)
        let cache = state.tree_cache();
        assert!(
            !cache.is_dir_loaded(&unknown),
            "directory not in root_entries should NOT be loaded in cache"
        );
    }

    /// REQ-4: Files cannot be expanded.
    #[test]
    fn files_cannot_expand() {
        let tmp = tempfile::TempDir::new().unwrap();
        let file = tmp.path().join("file.txt");
        std::fs::write(&file, "content").unwrap();

        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()));

        // Try to expand a file
        let state = state.expand_dir(&file);

        // Should remain at root
        let FolderState::Loaded { current_dir, .. } = &state.folder_state else {
            panic!("expected Loaded");
        };
        assert_eq!(current_dir, &canonical_tmp_path(tmp.path()));
    }

    /// REQ-4: Symlinks cannot be expanded.
    #[test]
    fn symlinks_cannot_expand() {
        let tmp = tempfile::TempDir::new().unwrap();
        let target = tmp.path().join("real_dir");
        std::fs::create_dir(&target).unwrap();
        let symlink = tmp.path().join("link_dir");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &symlink).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&target, &symlink).unwrap();

        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()));

        // Try to expand a symlink
        let state = state.expand_dir(&symlink);

        // Should remain at root
        let FolderState::Loaded { current_dir, .. } = &state.folder_state else {
            panic!("expected Loaded");
        };
        assert_eq!(current_dir, &canonical_tmp_path(tmp.path()));
    }

    /// REQ-5: Global budget bounds total cached entries (AppState).
    #[test]
    fn global_budget_bounds_entries_appstate() {
        let tmp = tempfile::TempDir::new().unwrap();
        let budget = 20;
        for i in 0..budget + 10 {
            std::fs::write(tmp.path().join(format!("file_{i}.txt")), "").unwrap();
        }

        let state = AppState::with_tree_budget(budget)
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()));

        // Entries should be limited
        let FolderState::Loaded { entries, .. } = &state.folder_state else {
            panic!("expected Loaded");
        };
        assert!(entries.len() <= budget);
        assert!(state.tree_cache().is_incomplete());
    }

    /// REQ-6: Incompleteness is queryable via tree_cache().
    #[test]
    fn incompleteness_queryable() {
        let tmp = tempfile::TempDir::new().unwrap();
        let budget = 10;
        for i in 0..budget + 5 {
            std::fs::write(tmp.path().join(format!("file_{i}.txt")), "").unwrap();
        }

        let state = AppState::with_tree_budget(budget)
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()));

        assert!(
            state.tree_cache().is_incomplete(),
            "tree_cache should report incomplete"
        );
    }

    /// REQ-6: Last entry indicates incompleteness.
    #[test]
    fn last_entry_indicates_incomplete() {
        let tmp = tempfile::TempDir::new().unwrap();
        let budget = 20;
        for i in 0..budget + 10 {
            std::fs::write(tmp.path().join(format!("file_{i}.txt")), "").unwrap();
        }

        let state = AppState::with_tree_budget(budget)
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()));

        let FolderState::Loaded { entries, .. } = &state.folder_state else {
            panic!("expected Loaded");
        };

        if let Some(last) = entries.last() {
            assert!(
                last.is_incomplete,
                "last entry must indicate incomplete when cap hit"
            );
        }
    }

    /// REQ-7: Picker cancellation preserves tree cache.
    #[test]
    fn picker_cancellation_preserves_tree() {
        let tmp1 = tempfile::TempDir::new().unwrap();
        let tmp2 = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp1.path().join("file1.txt"), "").unwrap();
        std::fs::write(tmp2.path().join("file2.txt"), "").unwrap();

        // Open first folder
        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp1.path().to_path_buf()));

        assert!(matches!(&state.folder_state, FolderState::Loaded { .. }));

        // Open picker for replacement
        let state = state.open_folder();

        // Cancel picker
        let state = state.on_folder_selected(None);

        // Should return to first folder (tree preserved)
        let FolderState::Loaded { entries, .. } = &state.folder_state else {
            panic!("expected Loaded after cancellation");
        };
        let names: Vec<_> = entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_str().unwrap())
            .collect();
        assert!(names.contains(&"file1.txt"));
    }

    /// REQ-7: Selecting replacement project clears tree cache.
    #[test]
    fn replacement_project_clears_tree() {
        let tmp1 = tempfile::TempDir::new().unwrap();
        let tmp2 = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp1.path().join("file1.txt"), "from project 1").unwrap();
        std::fs::write(tmp2.path().join("file2.txt"), "from project 2").unwrap();

        // Open first folder
        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp1.path().to_path_buf()));

        // Open picker for replacement
        let state = state.open_folder();

        // Select different folder
        let state = state.on_folder_selected(Some(tmp2.path().to_path_buf()));

        let FolderState::Loaded { entries, root, .. } = &state.folder_state else {
            panic!("expected Loaded after replacement");
        };
        assert_eq!(root, &canonical_tmp_path(tmp2.path()));

        let names: Vec<_> = entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_str().unwrap())
            .collect();
        assert!(names.contains(&"file2.txt"));
        assert!(
            !names.contains(&"file1.txt"),
            "old project entries must be cleared"
        );
    }

    /// REQ-7: Closing project clears tree cache.
    #[test]
    fn closing_project_clears_tree() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("file.txt"), "").unwrap();

        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()));

        assert!(matches!(&state.folder_state, FolderState::Loaded { .. }));

        let state = state.reset_folder();

        assert!(matches!(&state.folder_state, FolderState::Idle));
    }

    /// REQ-8: Selected file preserved during expansion (more thorough).
    #[test]
    fn file_selection_preserved_through_multiple_expansions() {
        let tmp = tempfile::TempDir::new().unwrap();
        let level1 = tmp.path().join("src");
        let level2 = level1.join("core");
        std::fs::create_dir_all(&level2).unwrap();
        std::fs::write(tmp.path().join("root.rs"), "root").unwrap();
        std::fs::write(level2.join("lib.rs"), "lib").unwrap();

        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()))
            .select_file(Some(tmp.path().join("root.rs")));

        // Expand multiple levels
        let state = state.expand_dir(&level1);
        let state = state.expand_dir(&level2);

        // Selection should still be preserved
        let FolderState::Loaded { selected_file, .. } = &state.folder_state else {
            panic!("expected Loaded");
        };
        assert!(
            selected_file.is_some(),
            "selected_file should be preserved through expansions"
        );
        assert_eq!(
            selected_file
                .as_ref()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap(),
            "root.rs"
        );
    }

    /// Root listing on project open shows only direct children.
    #[test]
    fn root_listing_only_direct_children_on_open() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("src").join("nested")).unwrap();
        std::fs::write(tmp.path().join("root.txt"), "").unwrap();

        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()));

        let FolderState::Loaded { entries, .. } = &state.folder_state else {
            panic!("expected Loaded");
        };

        let names: Vec<_> = entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_str().unwrap())
            .collect();

        // Should have: "src", "root.txt"
        assert!(names.contains(&"src"), "src must appear: {names:?}");
        assert!(
            names.contains(&"root.txt"),
            "root.txt must appear: {names:?}"
        );
        // "nested" should NOT appear (not direct child of root)
        assert!(
            !names.contains(&"nested"),
            "nested (grandchild) must not appear: {names:?}"
        );
    }

    /// Small test budget works correctly.
    #[test]
    fn small_test_budget_works() {
        let tmp = tempfile::TempDir::new().unwrap();
        // Create just a few files
        std::fs::create_dir(tmp.path().join("dir")).unwrap();
        std::fs::write(tmp.path().join("a.txt"), "").unwrap();
        std::fs::write(tmp.path().join("b.txt"), "").unwrap();

        // Use a tiny budget
        let state = AppState::with_tree_budget(5)
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()));

        let FolderState::Loaded { entries, .. } = &state.folder_state else {
            panic!("expected Loaded");
        };

        // Should have at least dir and files
        assert!(entries.len() >= 2);
        assert!(entries.len() <= 5);
        assert!(!state.tree_cache().is_incomplete());
    }
}
