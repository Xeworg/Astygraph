//! Headless tests for i18n and icons modules.
//!
//! Verifies locale switching, fallback behavior, key coverage, and icon
//! asset availability — all without a display.

mod i18n_tests {
    use astynex::app_state::AppState;
    use astynex::i18n::{I18n, Locale};

    // --- Locale switching ---

    #[test]
    fn app_state_default_locale_is_english() {
        let state = AppState::default();
        assert_eq!(state.locale(), Locale::En);
    }

    #[test]
    fn app_state_locale_transitions() {
        let state = AppState::default();
        assert_eq!(state.locale(), Locale::En);

        let state = state.with_locale(Locale::Es);
        assert_eq!(state.locale(), Locale::Es);

        let state = state.with_locale(Locale::En);
        assert_eq!(state.locale(), Locale::En);
    }

    #[test]
    fn locale_toggle() {
        assert_eq!(Locale::En.toggle(), Locale::Es);
        assert_eq!(Locale::Es.toggle(), Locale::En);
    }

    #[test]
    fn i18n_switching_affects_translations() {
        let en = I18n::new(Locale::En);
        let es = I18n::new(Locale::Es);

        let en_open = en.t("folder.open");
        let es_open = es.t("folder.open");

        assert_eq!(en_open, "Open Folder");
        assert_eq!(es_open, "Abrir carpeta");
        assert_ne!(en_open.as_ref(), es_open.as_ref());
    }

    // --- Fallback ---

    #[test]
    fn missing_key_returns_key_itself() {
        let i18n = I18n::new(Locale::En);
        assert_eq!(i18n.t("no.such.key"), "no.such.key");
        assert_eq!(i18n.t(""), "");
    }

    // --- Key coverage ---

    #[test]
    fn all_required_keys_present_in_english() {
        let i18n = I18n::new(Locale::En);
        let keys = [
            "app.title",
            "folder.open",
            "folder.close",
            "folder.opening",
            "explorer.title",
            "explorer.up",
            "explorer.discovery_incomplete",
            "explorer.parent_dir",
            "source.title",
            "source.select_file",
            "source.empty_file",
            "source.large_file",
            "source.error",
            "source.io_error",
            "source.utf8_error",
            "locale.toggle",
        ];
        for key in keys {
            assert_ne!(
                i18n.t(key).as_ref(),
                key,
                "key '{key}' is missing from English locale"
            );
        }
    }

    #[test]
    fn all_required_keys_present_in_spanish() {
        let i18n = I18n::new(Locale::Es);
        let keys = [
            "app.title",
            "folder.open",
            "folder.close",
            "folder.opening",
            "explorer.title",
            "explorer.up",
            "explorer.discovery_incomplete",
            "explorer.parent_dir",
            "source.title",
            "source.select_file",
            "source.empty_file",
            "source.large_file",
            "source.error",
            "source.io_error",
            "source.utf8_error",
            "locale.toggle",
        ];
        for key in keys {
            assert_ne!(
                i18n.t(key).as_ref(),
                key,
                "key '{key}' is missing from Spanish locale"
            );
        }
    }

    #[test]
    fn args_substitution() {
        let i18n = I18n::new(Locale::En);
        let result = i18n.t_args("source.error", &["file not found"]);
        assert_eq!(result, "Error: file not found");
    }

    #[test]
    fn app_title_is_brand_name_in_both_locales() {
        let en = I18n::new(Locale::En);
        let es = I18n::new(Locale::Es);
        assert_eq!(en.t("app.title"), "Astynex");
        assert_eq!(es.t("app.title"), "Astynex");
    }
}

mod icons_tests {
    use astynex::icons::{self, Icon, ICON_SIZE, STROKE_WIDTH};

    #[test]
    fn all_icon_variants_return_valid_svg_path() {
        for icon in Icon::all() {
            let data = icon.path_data();
            assert!(!data.is_empty(), "icon {:?} has empty path data", icon);
            // Heroicons use M/m moveto as the starting command.
            assert!(
                data.contains('M'),
                "icon {:?} has no M moveto command: {data}",
                icon
            );
        }
    }

    #[test]
    fn parse_path_returns_non_empty_points_for_all_icons() {
        for icon in Icon::all() {
            let pts = icons::parse_path(icon.path_data(), ICON_SIZE);
            assert!(!pts.is_empty(), "icon {:?} parsed to zero points", icon);
        }
    }

    #[test]
    fn parse_path_empty_string_returns_empty() {
        let pts = icons::parse_path("", 16.0);
        assert!(pts.is_empty());
    }

    #[test]
    fn parse_path_absolute_moveto_preserves_coordinates() {
        // "M4 20" should produce first point (4, 20) in the raw 24x24 space.
        let pts = icons::parse_path("M4 20h16V10H4z", 24.0);
        assert!(!pts.is_empty());
        assert!(
            (pts[0].x - 4.0).abs() < 0.01,
            "expected x=4, got {:?}",
            pts[0].x
        );
        assert!(
            (pts[0].y - 20.0).abs() < 0.01,
            "expected y=20, got {:?}",
            pts[0].y
        );
    }

    #[test]
    fn icon_size_constant_is_reasonable() {
        assert!(ICON_SIZE > 0.0);
        assert!(ICON_SIZE <= 64.0);
    }

    #[test]
    fn stroke_width_is_positive() {
        assert!(STROKE_WIDTH > 0.0);
    }

    #[test]
    fn icon_names_are_descriptive() {
        assert_eq!(Icon::Folder.to_string(), "folder");
        assert_eq!(Icon::FolderOpen.to_string(), "folder-open");
        assert_eq!(Icon::Document.to_string(), "document");
        assert_eq!(Icon::Link.to_string(), "link");
        assert_eq!(Icon::ArrowUp.to_string(), "arrow-up");
        assert_eq!(Icon::XMark.to_string(), "x-mark");
    }

    #[test]
    fn icon_display_shows_name() {
        use std::fmt::Write;
        for icon in Icon::all() {
            let mut s = String::new();
            let _ = write!(&mut s, "{}", icon);
            assert!(!s.is_empty());
        }
    }

    #[test]
    fn locale_preserved_after_folder_operations() {
        use astynex::app_state::AppState;

        let state = AppState::default().with_locale(astynex::i18n::Locale::Es);

        // Open folder flow should not reset locale.
        let state = state.open_folder();
        assert_eq!(state.locale(), astynex::i18n::Locale::Es);

        let state = state.on_folder_selected(None);
        assert_eq!(state.locale(), astynex::i18n::Locale::Es);
    }

    #[test]
    fn icon_paths_are_valid_utf8() {
        for icon in Icon::all() {
            let data = icon.path_data();
            assert!(
                std::str::from_utf8(data.as_bytes()).is_ok(),
                "icon {:?} path data is not valid UTF-8",
                icon
            );
        }
    }
}
