//! Headless app-shell state tests.
//!
//! Exercises `AppState` and its folder-selection state machine without a display.

mod app_shell {
    /// Ensure the app-state module is reachable from integration tests.
    #[test]
    fn app_state_module_exists() {
        // Compile-time check: the types used below must exist and be public enough
        // for integration tests to construct and query.
        use astynex::app_state::AppState;
        use astynex::app_state::FolderState;

        let state = AppState::default();
        assert!(matches!(state.folder_state, FolderState::Idle));

        // Idle -> Loading is always valid
        let state = state.open_folder();
        assert!(matches!(state.folder_state, FolderState::Loading));

        // Cancellation returns to Idle
        let state = state.on_folder_selected(None);
        assert!(matches!(state.folder_state, FolderState::Idle));
    }

    #[test]
    fn open_folder_then_select_folder_transitions_to_loaded() {
        use astynex::app_state::AppState;
        use astynex::app_state::FolderState;

        let state = AppState::default();
        let state = state.open_folder();
        assert!(matches!(state.folder_state, FolderState::Loading));

        // Simulate a successful folder selection
        let selected_path = std::path::PathBuf::from("/tmp");
        let state = state.on_folder_selected(Some(selected_path.clone()));

        let FolderState::Loaded {
            root, current_dir, ..
        } = state.folder_state
        else {
            panic!("expected Loaded state, got {:?}", state.folder_state);
        };
        assert_eq!(root, selected_path);
        assert_eq!(current_dir, selected_path);
    }

    #[test]
    fn double_open_folder_from_loading_is_noop() {
        use astynex::app_state::AppState;
        use astynex::app_state::FolderState;

        let state = AppState::default();
        let state = state.open_folder();
        let state = state.open_folder(); // Second call from Loading

        // Must stay Loading (not panic, not corrupt)
        assert!(matches!(state.folder_state, FolderState::Loading));
    }

    #[test]
    fn on_folder_selected_from_loaded_is_noop() {
        use astynex::app_state::AppState;
        use astynex::app_state::FolderState;

        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(std::path::PathBuf::from("/tmp")));

        assert!(matches!(state.folder_state, FolderState::Loaded { .. }));

        // Calling on_folder_selected from Loaded must not panic or corrupt state
        let state = state.on_folder_selected(None);
        assert!(matches!(state.folder_state, FolderState::Loaded { .. }));

        let state = state.on_folder_selected(Some(std::path::PathBuf::from("/home")));
        assert!(matches!(state.folder_state, FolderState::Loaded { .. }));
    }

    #[test]
    fn on_folder_selected_from_idle_is_noop() {
        use astynex::app_state::AppState;
        use astynex::app_state::FolderState;

        let state = AppState::default();
        assert!(matches!(state.folder_state, FolderState::Idle));

        // Calling on_folder_selected from Idle must not panic or corrupt state
        let state = state.on_folder_selected(Some(std::path::PathBuf::from("/tmp")));
        assert!(matches!(state.folder_state, FolderState::Idle));

        let state = state.on_folder_selected(None);
        assert!(matches!(state.folder_state, FolderState::Idle));
    }

    #[test]
    fn reset_folder_returns_to_idle() {
        use astynex::app_state::AppState;
        use astynex::app_state::FolderState;

        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(std::path::PathBuf::from("/tmp")));

        assert!(matches!(state.folder_state, FolderState::Loaded { .. }));

        let state = state.reset_folder();
        assert!(matches!(state.folder_state, FolderState::Idle));
    }

    // --- ODD Task 1: Headless folder replacement and locale draft/apply/cancel ---

    #[test]
    fn picker_cancellation_from_idle_returns_idle() {
        use astynex::app_state::AppState;
        use astynex::app_state::FolderState;

        // Start from Idle, open picker, cancel
        let state = AppState::default();
        let state = state.open_folder();
        assert!(matches!(state.folder_state, FolderState::Loading));

        let state = state.on_folder_selected(None);
        assert!(matches!(state.folder_state, FolderState::Idle));
    }

    #[test]
    fn picker_cancellation_while_replacing_loaded_folder_preserves_previous() {
        use astynex::app_state::AppState;
        use astynex::app_state::FolderState;

        // Load initial folder /tmp
        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(std::path::PathBuf::from("/tmp")));

        let FolderState::Loaded {
            root: original_root,
            ..
        } = &state.folder_state
        else {
            panic!("expected Loaded state");
        };
        assert_eq!(*original_root, std::path::PathBuf::from("/tmp"));

        // Open picker to replace folder, then cancel
        let state = state.open_folder();
        assert!(matches!(state.folder_state, FolderState::Loading));

        let state = state.on_folder_selected(None);

        // Must preserve the original folder, not return to Idle
        let FolderState::Loaded { root, .. } = &state.folder_state else {
            panic!(
                "expected Loaded state after cancellation, got {:?}",
                state.folder_state
            );
        };
        assert_eq!(root, original_root);
    }

    #[test]
    fn confirmed_replacement_from_loaded_folder_switches_to_new() {
        use astynex::app_state::AppState;
        use astynex::app_state::FolderState;

        // Load initial folder /tmp
        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(std::path::PathBuf::from("/tmp")));

        assert!(matches!(state.folder_state, FolderState::Loaded { .. }));

        // Open picker to replace with /home
        let state = state.open_folder();
        assert!(matches!(state.folder_state, FolderState::Loading));

        let new_path = std::path::PathBuf::from("/home");
        let state = state.on_folder_selected(Some(new_path.clone()));

        let FolderState::Loaded {
            root, current_dir, ..
        } = &state.folder_state
        else {
            panic!("expected Loaded state after selection");
        };
        assert_eq!(root, &new_path);
        assert_eq!(current_dir, &new_path);
    }

    #[test]
    fn open_folder_available_from_loaded_state() {
        use astynex::app_state::AppState;
        use astynex::app_state::FolderState;

        // Load a folder first
        let state = AppState::default()
            .open_folder()
            .on_folder_selected(Some(std::path::PathBuf::from("/tmp")));

        // open_folder from Loaded must transition to Loading (not stay Loaded)
        let state = state.open_folder();
        assert!(matches!(state.folder_state, FolderState::Loading));
    }
}

mod locale_draft_tests {
    //! Tests for locale draft/apply/cancel state machine.

    use astynex::app_state::AppState;
    use astynex::i18n::Locale;

    #[test]
    fn locale_dialog_closed_by_default() {
        let state = AppState::default();
        assert!(
            !state.is_locale_dialog_open(),
            "dialog should be closed by default"
        );
    }

    #[test]
    fn open_locale_dialog_sets_pending_draft() {
        let state = AppState::default();
        // Initially En
        assert_eq!(state.locale(), Locale::En);

        // Open dialog — draft starts as current locale
        let state = state.open_locale_dialog();
        assert!(state.is_locale_dialog_open());
        assert_eq!(state.pending_locale(), Some(Locale::En));
        // Active locale unchanged
        assert_eq!(state.locale(), Locale::En);
    }

    #[test]
    fn draft_locale_preview_changes_pending_only() {
        let state = AppState::default();
        let state = state.open_locale_dialog();
        assert_eq!(state.pending_locale(), Some(Locale::En));

        // Preview Spanish
        let state = state.draft_locale(Locale::Es);
        assert_eq!(state.pending_locale(), Some(Locale::Es));
        // Active locale still English
        assert_eq!(state.locale(), Locale::En);
    }

    #[test]
    fn apply_locale_commits_pending_and_closes_dialog() {
        let state = AppState::default(); // En
        let state = state.open_locale_dialog();
        let state = state.draft_locale(Locale::Es);

        // Apply the pending locale
        let state = state.apply_locale();
        assert!(
            !state.is_locale_dialog_open(),
            "dialog should close after apply"
        );
        assert_eq!(state.pending_locale(), None, "pending should be cleared");
        assert_eq!(state.locale(), Locale::Es, "active locale should change");
    }

    #[test]
    fn apply_locale_with_no_draft_is_noop() {
        let state = AppState::default();
        // Open dialog and immediately apply without drafting
        let state = state.open_locale_dialog();
        let state = state.apply_locale();

        // Dialog should close (no-op on apply with no pending change)
        assert!(!state.is_locale_dialog_open());
        assert_eq!(state.locale(), Locale::En, "locale unchanged");
    }

    #[test]
    fn cancel_locale_preserves_active_and_closes_dialog() {
        let state = AppState::default().with_locale(Locale::Es);
        assert_eq!(state.locale(), Locale::Es);

        // Open dialog, draft English, then cancel
        let state = state.open_locale_dialog();
        let state = state.draft_locale(Locale::En);
        let state = state.cancel_locale();

        assert!(!state.is_locale_dialog_open());
        assert_eq!(state.pending_locale(), None);
        assert_eq!(
            state.locale(),
            Locale::Es,
            "active locale preserved on cancel"
        );
    }

    #[test]
    fn cancel_without_open_dialog_is_noop() {
        let state = AppState::default().with_locale(Locale::En);
        let state = state.cancel_locale();
        assert!(!state.is_locale_dialog_open());
        assert_eq!(state.locale(), Locale::En);
    }

    #[test]
    fn locale_preserved_after_folder_operations() {
        use astynex::app_state::FolderState;

        let state = AppState::default().with_locale(Locale::Es);
        assert_eq!(state.locale(), Locale::Es);

        // Open folder flow
        let state = state.open_folder();
        assert_eq!(state.locale(), Locale::Es);

        let state = state.on_folder_selected(None); // Cancel
        assert_eq!(state.locale(), Locale::Es);

        // Folder replacement flow
        let state = state
            .open_folder()
            .on_folder_selected(Some(std::path::PathBuf::from("/tmp")));
        assert_eq!(state.locale(), Locale::Es);

        // Close folder
        let FolderState::Loaded { .. } = &state.folder_state else {
            panic!("expected Loaded");
        };
        let state = state.reset_folder();
        assert_eq!(state.locale(), Locale::Es);
    }

    #[test]
    fn locale_dialog_close_folder_preserves_dialog_and_draft() {
        let state = AppState::default().with_locale(Locale::En);

        // Load folder, open locale dialog, draft change
        let state = state
            .open_folder()
            .on_folder_selected(Some(std::path::PathBuf::from("/tmp")));
        let state = state.open_locale_dialog();
        let state = state.draft_locale(Locale::Es);

        // Close folder — dialog and draft should remain open
        let state = state.reset_folder();
        assert!(state.is_locale_dialog_open());
        assert_eq!(state.pending_locale(), Some(Locale::Es));
        assert_eq!(state.locale(), Locale::En, "active unchanged until apply");
    }
}
