//! Headless tests for nested directory drill-down navigation.
//!
//! Exercises `AppState::navigate_to_dir`, `navigate_up`, and `navigate_to_root`
//! without a display. Covers:
//! - Drill-down into a subdirectory and back up.
//! - Path-traversal attack rejection.
//! - Preservation of safe exclusions and symlink policy across navigation.
//! - No-op guards from non-Loaded states.

mod fs_navigation {
    use std::path::{Path, PathBuf};

    /// Match the canonical paths stored by `AppState` across platforms.
    fn canonical_tmp_path(path: &Path) -> PathBuf {
        std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    }

    /// `on_folder_selected` initialises both `root` and `current_dir` to the same path.
    #[test]
    fn loaded_state_has_root_and_current_dir_equal_on_open() {
        use astynex::app_state::{AppState, FolderState};

        let tmp = tempfile::TempDir::new().unwrap();
        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()));

        let FolderState::Loaded {
            root, current_dir, ..
        } = &state.folder_state
        else {
            panic!("expected Loaded, got {:?}", state.folder_state);
        };

        assert_eq!(
            root, current_dir,
            "root and current_dir must be equal on first open"
        );
    }

    /// Clicking a directory navigates into it: `current_dir` changes.
    #[test]
    fn navigate_to_dir_changes_current_dir() {
        use astynex::app_state::{AppState, FolderState};

        let tmp = tempfile::TempDir::new().unwrap();
        let child = tmp.path().join("src");
        std::fs::create_dir(&child).unwrap();
        std::fs::write(child.join("main.rs"), "fn main() {}").unwrap();

        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()));

        let FolderState::Loaded { current_dir, .. } = &state.folder_state else {
            panic!("expected Loaded");
        };
        assert_eq!(current_dir, &canonical_tmp_path(tmp.path()));

        // Navigate into "src".
        let state = state.navigate_to_dir(&child);

        let FolderState::Loaded {
            current_dir,
            entries,
            ..
        } = &state.folder_state
        else {
            panic!("expected Loaded after navigate_to_dir");
        };
        assert_eq!(current_dir, &canonical_tmp_path(&child));
        // "src" listing must contain main.rs.
        let names: Vec<_> = entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_str().unwrap())
            .collect();
        assert!(names.contains(&"main.rs"), "main.rs must appear: {names:?}");
    }

    /// Navigating into a directory clears the viewer selection.
    #[test]
    fn navigate_to_dir_clears_selected_file() {
        use astynex::app_state::{AppState, FolderState};

        let tmp = tempfile::TempDir::new().unwrap();
        let child = tmp.path().join("src");
        std::fs::create_dir(&child).unwrap();
        std::fs::write(tmp.path().join("root.rs"), "root").unwrap();

        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()))
            .select_file(Some(tmp.path().join("root.rs")));

        let FolderState::Loaded { selected_file, .. } = &state.folder_state else {
            panic!("expected Loaded");
        };
        assert!(selected_file.is_some());

        // Navigate into "src" — selection must be cleared.
        let state = state.navigate_to_dir(&child);
        let FolderState::Loaded { selected_file, .. } = &state.folder_state else {
            panic!("expected Loaded after navigate_to_dir");
        };
        assert!(
            selected_file.is_none(),
            "navigate_to_dir must clear selected_file"
        );
    }

    /// Path traversal attack: navigating to a sibling of `current_dir` is rejected.
    /// A sibling lives at the same level as `current_dir`, not inside it.
    /// We test this by drilling into a child first, then trying to jump to an
    /// adjacent sibling of that child.
    #[test]
    fn navigate_to_dir_rejects_sibling_dir() {
        use astynex::app_state::{AppState, FolderState};

        let tmp = tempfile::TempDir::new().unwrap();

        // Structure:  /project
        //               /src   <- current_dir
        //               /lib   <- sibling of src (must NOT be navigable from /src)
        std::fs::create_dir(tmp.path().join("src")).unwrap();
        let sibling = tmp.path().join("lib");
        std::fs::create_dir(&sibling).unwrap();

        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()));

        // Drill into "src" first.
        let state = state.navigate_to_dir(&tmp.path().join("src"));
        let FolderState::Loaded { current_dir, .. } = &state.folder_state else {
            panic!("expected Loaded after navigating to src");
        };
        assert_eq!(
            current_dir.as_path(),
            canonical_tmp_path(&tmp.path().join("src")).as_path()
        );

        // Try to navigate directly to "lib" (sibling of "src", not a child).
        let state = state.navigate_to_dir(&sibling);

        // Must stay in "src" — sibling navigation is rejected.
        let FolderState::Loaded {
            current_dir: after, ..
        } = &state.folder_state
        else {
            panic!("expected Loaded after attempted sibling navigation");
        };
        assert_eq!(
            after.as_path(),
            canonical_tmp_path(&tmp.path().join("src")).as_path(),
            "sibling navigation must be rejected"
        );
    }

    /// Path traversal attack: navigating to the parent directory is rejected.
    #[test]
    fn navigate_to_dir_rejects_parent_dir() {
        use astynex::app_state::{AppState, FolderState};

        let tmp = tempfile::TempDir::new().unwrap();
        let child = tmp.path().join("src");
        std::fs::create_dir(&child).unwrap();

        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()))
            .navigate_to_dir(&child);

        // Try to navigate up to the parent using navigate_to_dir.
        let state = state.navigate_to_dir(tmp.path());

        let FolderState::Loaded { current_dir, .. } = &state.folder_state else {
            panic!("expected Loaded");
        };
        assert_eq!(
            current_dir,
            &canonical_tmp_path(&child),
            "parent navigation via navigate_to_dir must be rejected"
        );
    }

    /// `navigate_to_dir` from non-Loaded states is a no-op.
    #[test]
    fn navigate_to_dir_noop_from_idle() {
        use astynex::app_state::AppState;

        let state = AppState::default();
        let result = state.navigate_to_dir(std::path::Path::new("/tmp"));
        assert_eq!(state, result, "navigate_to_dir from Idle must be a no-op");
    }

    /// `navigate_to_dir` from Loading is a no-op.
    #[test]
    fn navigate_to_dir_noop_from_loading() {
        use astynex::app_state::AppState;

        let state = AppState::default().open_folder();
        let result = state.navigate_to_dir(std::path::Path::new("/tmp"));
        assert_eq!(
            state, result,
            "navigate_to_dir from Loading must be a no-op"
        );
    }

    /// `navigate_up` moves `current_dir` back to the parent.
    #[test]
    fn navigate_up_moves_to_parent() {
        use astynex::app_state::{AppState, FolderState};

        let tmp = tempfile::TempDir::new().unwrap();
        let child = tmp.path().join("src");
        std::fs::create_dir(&child).unwrap();

        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()))
            .navigate_to_dir(&child);

        assert!(matches!(
            &state.folder_state,
            FolderState::Loaded { current_dir, .. }
                if current_dir == &canonical_tmp_path(&child)
        ));

        let state = state.navigate_up();

        let FolderState::Loaded { current_dir, .. } = &state.folder_state else {
            panic!("expected Loaded after navigate_up");
        };
        assert_eq!(
            current_dir,
            &canonical_tmp_path(tmp.path()),
            "navigate_up must return to root"
        );
    }

    /// `navigate_up` from the root directory is a no-op.
    #[test]
    fn navigate_up_noop_at_root() {
        use astynex::app_state::{AppState, FolderState};

        let tmp = tempfile::TempDir::new().unwrap();
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

        let state = state.navigate_up();

        let FolderState::Loaded {
            current_dir: after, ..
        } = &state.folder_state
        else {
            panic!("expected Loaded after navigate_up");
        };
        assert_eq!(before, after, "navigate_up at root must be a no-op");
    }

    /// `navigate_up` from non-Loaded states is a no-op.
    #[test]
    fn navigate_up_noop_from_idle() {
        use astynex::app_state::AppState;

        let state = AppState::default();
        let result = state.navigate_up();
        assert_eq!(state, result, "navigate_up from Idle must be a no-op");
    }

    /// `navigate_up` from Loading is a no-op.
    #[test]
    fn navigate_up_noop_from_loading() {
        use astynex::app_state::AppState;

        let state = AppState::default().open_folder();
        let result = state.navigate_up();
        assert_eq!(state, result, "navigate_up from Loading must be a no-op");
    }

    /// `navigate_to_root` jumps back to the original root from any depth.
    #[test]
    fn navigate_to_root_returns_to_root() {
        use astynex::app_state::{AppState, FolderState};

        let tmp = tempfile::TempDir::new().unwrap();
        let level1 = tmp.path().join("level1");
        let level2 = level1.join("level2");
        std::fs::create_dir_all(&level2).unwrap();

        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()))
            .navigate_to_dir(&level1)
            .navigate_to_dir(&level2);

        assert!(matches!(
            &state.folder_state,
            FolderState::Loaded { current_dir, .. }
                if current_dir == &canonical_tmp_path(&level2)
        ));

        let state = state.navigate_to_root();

        let FolderState::Loaded {
            root, current_dir, ..
        } = &state.folder_state
        else {
            panic!("expected Loaded after navigate_to_root");
        };
        assert_eq!(current_dir, &canonical_tmp_path(tmp.path()));
        assert_eq!(root, &canonical_tmp_path(tmp.path()));
    }

    /// `navigate_to_root` from the root is a no-op.
    #[test]
    fn navigate_to_root_noop_at_root() {
        use astynex::app_state::{AppState, FolderState};

        let tmp = tempfile::TempDir::new().unwrap();
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

        let state = state.navigate_to_root();

        let FolderState::Loaded {
            current_dir: after, ..
        } = &state.folder_state
        else {
            panic!("expected Loaded after navigate_to_root");
        };
        assert_eq!(before, after, "navigate_to_root at root must be a no-op");
    }

    /// `navigate_to_root` from non-Loaded states is a no-op.
    #[test]
    fn navigate_to_root_noop_from_idle() {
        use astynex::app_state::AppState;

        let state = AppState::default();
        let result = state.navigate_to_root();
        assert_eq!(state, result, "navigate_to_root from Idle must be a no-op");
    }

    /// Selecting a file in a subdirectory works after drilling down.
    #[test]
    fn select_file_in_subdirectory() {
        use astynex::app_state::{AppState, FolderState};

        let tmp = tempfile::TempDir::new().unwrap();
        let child = tmp.path().join("src");
        std::fs::create_dir(&child).unwrap();
        let child_file = child.join("main.rs");
        std::fs::write(&child_file, "fn main() {}").unwrap();

        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()))
            .navigate_to_dir(&child)
            .select_file(Some(child_file.clone()));

        let FolderState::Loaded { selected_file, .. } = &state.folder_state else {
            panic!("expected Loaded");
        };
        assert_eq!(selected_file.as_ref(), Some(&child_file));
    }

    /// Exclusions are applied consistently when scanning after drill-down.
    #[test]
    fn exclusions_preserved_after_drill_down() {
        use astynex::app_state::{AppState, FolderState};

        let tmp = tempfile::TempDir::new().unwrap();
        // Create .git and .astynex inside a subdirectory.
        let child = tmp.path().join("child");
        std::fs::create_dir_all(child.join(".git")).unwrap();
        std::fs::create_dir_all(child.join(".astynex")).unwrap();
        std::fs::create_dir_all(child.join("src")).unwrap();
        std::fs::write(child.join("src").join("lib.rs"), "").unwrap();

        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()))
            .navigate_to_dir(&child);

        let FolderState::Loaded { entries, .. } = &state.folder_state else {
            panic!("expected Loaded");
        };
        let names: Vec<_> = entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_str().unwrap())
            .collect();

        assert!(
            !names.contains(&".git"),
            ".git must be excluded in subdirectory: {names:?}"
        );
        assert!(
            !names.contains(&".astynex"),
            ".astynex must be excluded in subdirectory: {names:?}"
        );
        assert!(names.contains(&"src"), "src must appear: {names:?}");
    }

    /// Exclusions are applied after navigating back up.
    #[test]
    fn exclusions_preserved_after_navigate_up() {
        use astynex::app_state::{AppState, FolderState};

        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
        std::fs::create_dir_all(tmp.path().join("target")).unwrap();
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();

        let child = tmp.path().join("src");
        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()))
            .navigate_to_dir(&child)
            .navigate_up(); // back to root

        let FolderState::Loaded { entries, .. } = &state.folder_state else {
            panic!("expected Loaded");
        };
        let names: Vec<_> = entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_str().unwrap())
            .collect();

        assert!(
            !names.contains(&".git"),
            ".git must still be excluded at root: {names:?}"
        );
        assert!(
            !names.contains(&"target"),
            "target must still be excluded at root: {names:?}"
        );
        assert!(names.contains(&"src"), "src must appear at root: {names:?}");
    }

    /// Symlinks are not followed during drill-down; they appear as Symlink entries.
    #[test]
    fn symlinks_not_followed_during_drill_down() {
        use astynex::app_state::{AppState, FolderState};

        let tmp = tempfile::TempDir::new().unwrap();
        let target = tmp.path().join("target_dir");
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("secret.txt"), "secret").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, tmp.path().join("link_dir")).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&target, tmp.path().join("link_dir")).unwrap();

        // Open folder (root contains link_dir).
        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()));

        let FolderState::Loaded { entries, .. } = &state.folder_state else {
            panic!("expected Loaded");
        };

        // link_dir appears as Symlink kind.
        let symlink_entry = entries
            .iter()
            .find(|e| e.kind == astynex::fs::FileKind::Symlink);
        assert!(
            symlink_entry.is_some(),
            "symlink must appear as kind=Symlink at root"
        );

        // Navigating INTO a symlink is rejected (not a direct child canonical path).
        // The symlink path itself is not a directory under canonical parent control,
        // so navigate_to_dir should treat it as a non-child.
        let symlink_path = tmp.path().join("link_dir");
        let state = state.navigate_to_dir(&symlink_path);

        // Must stay at root (symlink traversal is blocked).
        let FolderState::Loaded { current_dir, .. } = &state.folder_state else {
            panic!("expected Loaded");
        };
        assert_eq!(
            current_dir,
            &canonical_tmp_path(tmp.path()),
            "symlink traversal must be blocked"
        );
    }

    /// Multiple levels of nesting work correctly.
    #[test]
    fn multi_level_navigation() {
        use astynex::app_state::{AppState, FolderState};

        let tmp = tempfile::TempDir::new().unwrap();
        let level1 = tmp.path().join("crates");
        let level2 = level1.join("astynex-core");
        let level3 = level2.join("src");
        std::fs::create_dir_all(&level3).unwrap();
        std::fs::write(level3.join("lib.rs"), "pub fn init() {}").unwrap();

        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(tmp.path().to_path_buf()));

        // Navigate three levels deep.
        let state = state.navigate_to_dir(&level1);
        let FolderState::Loaded {
            current_dir: d1, ..
        } = &state.folder_state
        else {
            panic!("expected Loaded at level 1");
        };
        assert_eq!(d1, &canonical_tmp_path(&level1));

        let state = state.navigate_to_dir(&level2);
        let FolderState::Loaded {
            current_dir: d2, ..
        } = &state.folder_state
        else {
            panic!("expected Loaded at level 2");
        };
        assert_eq!(d2, &canonical_tmp_path(&level2));

        let state = state.navigate_to_dir(&level3);
        let FolderState::Loaded {
            current_dir: d3,
            entries,
            ..
        } = &state.folder_state
        else {
            panic!("expected Loaded at level 3");
        };
        assert_eq!(d3, &canonical_tmp_path(&level3));
        let names: Vec<_> = entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_str().unwrap())
            .collect();
        assert!(names.contains(&"lib.rs"), "lib.rs must appear: {names:?}");

        // Navigate back to root in one step.
        let state = state.navigate_to_root();
        let FolderState::Loaded {
            current_dir: dr, ..
        } = &state.folder_state
        else {
            panic!("expected Loaded at root");
        };
        assert_eq!(dr, &canonical_tmp_path(tmp.path()));
    }
}
