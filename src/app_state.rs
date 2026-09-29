//! Application state machine for folder-selection flow.
//!
//! Designed to be unit-testable without a display.

use std::path::{Path, PathBuf};

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
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            folder_state: FolderState::Idle,
        }
    }
}

impl AppState {
    /// Request opening the native folder picker.
    ///
    /// Valid from `Idle` only. Returns `Loading`.
    /// From any other state, returns `self` unchanged (no-op guard).
    pub fn open_folder(&self) -> Self {
        match &self.folder_state {
            FolderState::Idle => Self {
                folder_state: FolderState::Loading,
            },
            // Guard: no-op from any other state
            FolderState::Loading | FolderState::Loaded { .. } => self.clone(),
        }
    }

    /// Handle the result of the native folder picker.
    ///
    /// `Some(path)` — selection confirmed; transitions to `Loaded` with scanned entries.
    /// `None`       — user cancelled or picker error; returns to `Idle`.
    ///
    /// Only valid from `Loading`. From `Idle` or `Loaded`, this is a no-op.
    pub fn on_folder_selected(&self, path: Option<PathBuf>) -> Self {
        match &self.folder_state {
            FolderState::Loading => Self {
                folder_state: match path {
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
                    None => FolderState::Idle,
                },
            },
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
            },
            FolderState::Idle | FolderState::Loading => self.clone(),
        }
    }
}
