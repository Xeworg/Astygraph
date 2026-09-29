//! Application state machine for folder-selection flow.
//!
//! Designed to be unit-testable without a display.

use std::path::{Path, PathBuf};

use crate::i18n::Locale;

/// Folder-selection sub-state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolderState {
    /// No folder has been selected.
    Idle,
    /// Native folder picker is open.
    Loading,
    /// A folder is open; holds the root path, current viewing directory,
    /// scanned entries for the current directory, and the selected file.
    Loaded {
        /// The originally opened root path.
        root: PathBuf,
        /// The directory currently displayed in the browser.
        current_dir: PathBuf,
        /// Entries visible in `current_dir`.
        entries: Vec<crate::fs::Entry>,
        /// The file whose contents are shown in the viewer (may be in a subdirectory).
        selected_file: Option<PathBuf>,
    },
}

/// Top-level application state.
///
/// All state transitions are handled through the methods below so they can be
/// exercised in headless tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppState {
    pub folder_state: FolderState,
    /// Active UI locale (committed/active).
    pub locale: Locale,
    /// Whether the locale selection dialog is open.
    locale_dialog_open: bool,
    /// Pending locale draft (None when dialog is closed).
    pending_locale: Option<Locale>,
    /// Previous folder state snapshot — stored before entering `Loading` so that
    /// cancellation can restore the original state (e.g. preserving a loaded
    /// folder when replacing it).
    previous_folder_state: Option<FolderState>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            folder_state: FolderState::Idle,
            locale: Locale::En,
            locale_dialog_open: false,
            pending_locale: None,
            previous_folder_state: None,
        }
    }
}

impl AppState {
    /// Set the UI locale.
    #[must_use]
    pub fn with_locale(self, locale: Locale) -> Self {
        Self { locale, ..self }
    }

    /// Current UI locale.
    pub fn locale(&self) -> Locale {
        self.locale
    }

    /// Whether the locale dialog is currently open.
    #[must_use]
    pub fn is_locale_dialog_open(&self) -> bool {
        self.locale_dialog_open
    }

    /// Current pending locale draft (None if dialog is closed).
    #[must_use]
    pub fn pending_locale(&self) -> Option<Locale> {
        self.pending_locale
    }
}

impl AppState {
    /// Open the locale selection dialog.
    ///
    /// Initializes the pending draft with the current active locale.
    /// If dialog is already open, returns self (no-op).
    #[must_use]
    pub fn open_locale_dialog(&self) -> Self {
        if self.locale_dialog_open {
            return self.clone();
        }
        Self {
            locale_dialog_open: true,
            pending_locale: Some(self.locale),
            ..self.clone()
        }
    }

    /// Draft/preview a locale selection (does not commit).
    ///
    /// Only valid when dialog is open. No-op otherwise.
    #[must_use]
    pub fn draft_locale(&self, locale: Locale) -> Self {
        if !self.locale_dialog_open {
            return self.clone();
        }
        Self {
            pending_locale: Some(locale),
            ..self.clone()
        }
    }

    /// Apply the pending locale draft and close the dialog.
    ///
    /// Commits `pending_locale` to `locale`. If no pending change (same as active),
    /// still closes the dialog. No-op if dialog is closed.
    #[must_use]
    pub fn apply_locale(&self) -> Self {
        if !self.locale_dialog_open {
            return self.clone();
        }
        let new_locale = self.pending_locale.unwrap_or(self.locale);
        Self {
            locale: new_locale,
            locale_dialog_open: false,
            pending_locale: None,
            ..self.clone()
        }
    }

    /// Cancel locale selection and close the dialog.
    ///
    /// Discards the pending draft. Active locale remains unchanged.
    /// No-op if dialog is already closed.
    #[must_use]
    pub fn cancel_locale(&self) -> Self {
        if !self.locale_dialog_open {
            return self.clone();
        }
        Self {
            locale_dialog_open: false,
            pending_locale: None,
            ..self.clone()
        }
    }
}

impl AppState {
    /// Request opening the native folder picker.
    ///
    /// Valid from `Idle` and `Loaded`. Returns `Loading` and snapshots the
    /// current folder state so cancellation can restore it.
    /// From `Loading`, returns `self` unchanged (no-op guard).
    pub fn open_folder(&self) -> Self {
        match &self.folder_state {
            FolderState::Idle | FolderState::Loaded { .. } => Self {
                folder_state: FolderState::Loading,
                locale: self.locale,
                locale_dialog_open: self.locale_dialog_open,
                pending_locale: self.pending_locale,
                // Snapshot current state for cancellation restore
                previous_folder_state: Some(self.folder_state.clone()),
            },
            // Guard: no-op from Loading
            FolderState::Loading => self.clone(),
        }
    }

    /// Handle the result of the native folder picker.
    ///
    /// `Some(path)` — selection confirmed; transitions to `Loaded` with scanned entries.
    /// `None`       — user cancelled; restores `previous_folder_state`:
    ///                 - From Idle → Idle
    ///                 - From Loaded → Loaded (preserves previous folder)
    ///
    /// Only valid from `Loading`. From `Idle` or `Loaded`, this is a no-op.
    pub fn on_folder_selected(&self, path: Option<PathBuf>) -> Self {
        match &self.folder_state {
            FolderState::Loading => {
                let folder_state = match path {
                    Some(p) => {
                        // Scan the directory; empty vec on error is handled by UI
                        let entries = crate::fs::scan_dir(&p).unwrap_or_default();
                        FolderState::Loaded {
                            root: p.clone(),
                            current_dir: p,
                            entries,
                            selected_file: None,
                        }
                    }
                    // Cancellation: restore previous state if available
                    None => self
                        .previous_folder_state
                        .clone()
                        .unwrap_or(FolderState::Idle),
                };
                Self {
                    folder_state,
                    locale: self.locale,
                    locale_dialog_open: self.locale_dialog_open,
                    pending_locale: self.pending_locale,
                    // Clear snapshot after use
                    previous_folder_state: None,
                }
            }
            // Guard: no-op from any other state
            FolderState::Idle | FolderState::Loaded { .. } => self.clone(),
        }
    }

    /// Navigate into a subdirectory within the currently loaded folder.
    ///
    /// Valid from `Loaded` only. No-op from other states or when `dir` is not
    /// an immediate child of the current directory (prevents path traversal attacks).
    ///
    /// Rescans the target directory and clears the viewer selection.
    pub fn navigate_to_dir(&self, dir: &Path) -> Self {
        let FolderState::Loaded {
            root,
            current_dir,
            entries: _,
            selected_file: _,
        } = &self.folder_state
        else {
            return self.clone();
        };

        // --- Symlink safety ---
        // Reject symlinks: following them would allow escape from the visible tree.
        if dir.is_symlink() {
            return self.clone();
        }

        // --- Path-traversal safety ---
        // Require the input path to be a real direct child of current_dir.
        // We use the non-canonical path comparison: the caller's path (before any
        // symlink resolution) must have current_dir as its parent. This blocks
        // `..` in the path and symlink targets that point outside the visible subtree.
        let Some(parent) = dir.parent() else {
            return self.clone();
        };
        if parent != current_dir.as_path() {
            // Tried to navigate outside the current view — reject
            return self.clone();
        }

        // Final safety: the resolved real path must also have current_dir as its parent.
        // This blocks cases where a legitimate-looking child is actually a symlink
        // pointing outside the tree (e.g. dir -> /tmp/outside).
        let canonical_dir = match std::fs::canonicalize(dir) {
            Ok(p) => p,
            Err(_) => return self.clone(),
        };
        let canonical_current = match std::fs::canonicalize(current_dir) {
            Ok(p) => p,
            Err(_) => return self.clone(),
        };
        let Some(canonical_parent) = canonical_dir.parent() else {
            return self.clone();
        };
        if canonical_parent != canonical_current {
            return self.clone();
        }

        let new_entries = crate::fs::scan_dir(dir).unwrap_or_default();
        Self {
            folder_state: FolderState::Loaded {
                root: root.clone(),
                current_dir: canonical_dir,
                entries: new_entries,
                selected_file: None,
            },
            locale: self.locale,
            locale_dialog_open: self.locale_dialog_open,
            pending_locale: self.pending_locale,
            previous_folder_state: self.previous_folder_state.clone(),
        }
    }

    /// Navigate up one level toward the root.
    ///
    /// Valid from `Loaded` only. If already at the root, returns `self`.
    pub fn navigate_up(&self) -> Self {
        let FolderState::Loaded {
            root,
            current_dir,
            entries: _,
            selected_file: _,
        } = &self.folder_state
        else {
            return self.clone();
        };

        let canonical_root = match std::fs::canonicalize(root) {
            Ok(p) => p,
            Err(_) => return self.clone(),
        };
        let canonical_current = match std::fs::canonicalize(current_dir) {
            Ok(p) => p,
            Err(_) => return self.clone(),
        };

        // Already at root — nothing to do
        if canonical_current == canonical_root {
            return self.clone();
        }

        let parent = match canonical_current.parent() {
            Some(p) => p.to_path_buf(),
            None => return self.clone(),
        };

        let new_entries = crate::fs::scan_dir(&parent).unwrap_or_default();
        Self {
            folder_state: FolderState::Loaded {
                root: canonical_root,
                current_dir: parent,
                entries: new_entries,
                selected_file: None,
            },
            locale: self.locale,
            locale_dialog_open: self.locale_dialog_open,
            pending_locale: self.pending_locale,
            previous_folder_state: self.previous_folder_state.clone(),
        }
    }

    /// Navigate back to the root folder.
    ///
    /// Valid from `Loaded` only. No-op from other states.
    pub fn navigate_to_root(&self) -> Self {
        let FolderState::Loaded {
            root,
            current_dir,
            entries: _,
            selected_file: _,
        } = &self.folder_state
        else {
            return self.clone();
        };

        let canonical_root = match std::fs::canonicalize(root) {
            Ok(p) => p,
            Err(_) => return self.clone(),
        };
        let canonical_current = match std::fs::canonicalize(current_dir) {
            Ok(p) => p,
            Err(_) => return self.clone(),
        };

        if canonical_current == canonical_root {
            return self.clone();
        }

        let new_entries = crate::fs::scan_dir(&canonical_root).unwrap_or_default();
        Self {
            folder_state: FolderState::Loaded {
                root: canonical_root.clone(),
                current_dir: canonical_root,
                entries: new_entries,
                selected_file: None,
            },
            locale: self.locale,
            locale_dialog_open: self.locale_dialog_open,
            pending_locale: self.pending_locale,
            previous_folder_state: self.previous_folder_state.clone(),
        }
    }

    /// Select a file within the currently loaded folder.
    ///
    /// Valid from `Loaded` only. No-op from other states.
    pub fn select_file(&self, selected: Option<PathBuf>) -> Self {
        match &self.folder_state {
            FolderState::Loaded {
                root,
                current_dir,
                entries,
                selected_file: _,
            } => Self {
                folder_state: FolderState::Loaded {
                    root: root.clone(),
                    current_dir: current_dir.clone(),
                    entries: entries.clone(),
                    selected_file: selected,
                },
                locale: self.locale,
                locale_dialog_open: self.locale_dialog_open,
                pending_locale: self.pending_locale,
                previous_folder_state: self.previous_folder_state.clone(),
            },
            FolderState::Idle | FolderState::Loading => self.clone(),
        }
    }

    /// Close the currently loaded folder and return to `Idle`.
    ///
    /// Valid from `Loaded` only. No-op from other states.
    pub fn reset_folder(&self) -> Self {
        match &self.folder_state {
            FolderState::Loaded { .. } => Self {
                folder_state: FolderState::Idle,
                locale: self.locale,
                locale_dialog_open: self.locale_dialog_open,
                pending_locale: self.pending_locale,
                previous_folder_state: self.previous_folder_state.clone(),
            },
            FolderState::Idle | FolderState::Loading => self.clone(),
        }
    }
}
