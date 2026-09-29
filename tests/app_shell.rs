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
}
