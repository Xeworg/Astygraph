//! Application state machine for folder-selection flow.
//!
//! Designed to be unit-testable without a display.

use std::path::PathBuf;

/// Folder-selection sub-state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolderState {
    /// No folder has been selected.
    Idle,
    /// Native folder picker is open.
    Loading,
    /// A folder is open; holds the selected path.
    Loaded { path: PathBuf },
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
    /// `Some(path)` — selection confirmed; transitions to `Loaded`.
    /// `None`       — user cancelled or picker error; returns to `Idle`.
    ///
    /// Only valid from `Loading`. From `Idle` or `Loaded`, this is a no-op.
    pub fn on_folder_selected(&self, path: Option<PathBuf>) -> Self {
        match &self.folder_state {
            FolderState::Loading => Self {
                folder_state: match path {
                    Some(p) => FolderState::Loaded { path: p },
                    None => FolderState::Idle,
                },
            },
            // Guard: no-op from any other state
            FolderState::Idle | FolderState::Loaded { .. } => self.clone(),
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
