//! Application state machine for folder-selection flow.
//!
//! Designed to be unit-testable without a display.

use std::path::{Path, PathBuf};

use crate::fs::tree::TreeCache;
use crate::i18n::Locale;

/// Maximum filesystem entries to cache globally.
/// Matches the NFR-7 discovery ceiling and DISCOVERY_CAP from fs module.
const DEFAULT_TREE_BUDGET: usize = 50_000;

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
    /// Lazy-loaded tree cache with global entry budget.
    /// Manages on-demand directory expansion and collapse/re-expand caching.
    tree_cache: TreeCache,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            folder_state: FolderState::Idle,
            locale: Locale::En,
            locale_dialog_open: false,
            pending_locale: None,
            previous_folder_state: None,
            tree_cache: TreeCache::new(DEFAULT_TREE_BUDGET),
        }
    }
}

impl AppState {
    /// Create AppState with a custom tree cache budget.
    ///
    /// Primarily intended for testing with small budgets, but can be used
    /// in production if a different entry limit is desired.
    pub fn with_tree_budget(budget: usize) -> Self {
        Self {
            folder_state: FolderState::Idle,
            locale: Locale::En,
            locale_dialog_open: false,
            pending_locale: None,
            previous_folder_state: None,
            tree_cache: TreeCache::new(budget),
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

    /// Access the tree cache (read-only).
    pub fn tree_cache(&self) -> &TreeCache {
        &self.tree_cache
    }

    /// Access the tree cache (mutable).
    pub fn tree_cache_mut(&mut self) -> &mut TreeCache {
        &mut self.tree_cache
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
                tree_cache: self.tree_cache.clone(),
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
    ///                 - From Loaded → Loaded (preserves previous folder and tree cache)
    ///
    /// Only valid from `Loading`. From `Idle` or `Loaded`, this is a no-op.
    ///
    /// # Tree Cache Behavior
    /// - On selection confirmation: clears the tree cache (new project)
    /// - On cancellation: preserves the tree cache (restoring previous state)
    pub fn on_folder_selected(&self, path: Option<PathBuf>) -> Self {
        match &self.folder_state {
            FolderState::Loading => {
                let (folder_state, tree_cache) = match path {
                    Some(p) => {
                        // New project selected: clear tree cache but preserve budget
                        let budget = self.tree_cache.budget();
                        let mut cache = TreeCache::new(budget);

                        // Scan root using lazy cache
                        let entries = cache
                            .get_root_entries(&p)
                            .map(|e| e.to_vec())
                            .unwrap_or_default();

                        // Canonicalize entry paths for consistent comparison
                        // This handles cases where TempDir returns non-canonical paths
                        let canonical_entries: Vec<crate::fs::Entry> = entries
                            .into_iter()
                            .map(|mut e| {
                                if let Ok(canonical) = std::fs::canonicalize(&e.path) {
                                    e.path = canonical;
                                }
                                e
                            })
                            .collect();

                        let canonical_root =
                            std::fs::canonicalize(&p).unwrap_or_else(|_| p.clone());

                        let state = FolderState::Loaded {
                            root: canonical_root.clone(),
                            current_dir: canonical_root,
                            entries: canonical_entries,
                            selected_file: None,
                        };
                        (state, cache)
                    }
                    // Cancellation: restore previous state, preserve tree cache
                    None => {
                        let state = self
                            .previous_folder_state
                            .clone()
                            .unwrap_or(FolderState::Idle);
                        (state, self.tree_cache.clone())
                    }
                };
                Self {
                    folder_state,
                    locale: self.locale,
                    locale_dialog_open: self.locale_dialog_open,
                    pending_locale: self.pending_locale,
                    // Clear snapshot after use
                    previous_folder_state: None,
                    tree_cache,
                }
            }
            // Guard: no-op from any other state
            FolderState::Idle | FolderState::Loaded { .. } => self.clone(),
        }
    }

    /// Expand a directory using the lazy tree cache.
    ///
    /// This is a PURE cache operation: it loads/updates the directory's children
    /// in the tree cache WITHOUT changing the navigation state (current_dir,
    /// entries, selected_file).
    ///
    /// Valid from `Loaded` only. No-op from other states.
    ///
    /// Expansion is allowed for:
    /// - A real directory entry visible in the root entries
    /// - A directory that is a descendant of an already-expanded cached parent
    ///
    /// Rejects: files, symlinks, paths outside project root, unknown paths.
    ///
    /// Returns `self` if `dir` cannot be expanded.
    #[must_use]
    pub fn expand_dir(&self, dir: &Path) -> Self {
        let FolderState::Loaded {
            root,
            current_dir,
            entries,
            selected_file,
        } = &self.folder_state
        else {
            return self.clone();
        };

        // Reject symlinks (not followed)
        if dir.is_symlink() {
            return self.clone();
        }

        // Canonicalize for reliable comparison
        let canonical_dir = match std::fs::canonicalize(dir) {
            Ok(p) => p,
            Err(_) => return self.clone(),
        };
        let canonical_root = match std::fs::canonicalize(root) {
            Ok(p) => p,
            Err(_) => return self.clone(),
        };

        // Safety: directory must be under project root
        if !canonical_dir.starts_with(&canonical_root) {
            return self.clone();
        }

        // Safety: must be a real directory (not a file)
        if !canonical_dir.is_dir() {
            return self.clone();
        }

        // Check if this directory is allowed for expansion:
        // 1. It's a direct entry in root OR
        // 2. It's a descendant of an already-expanded cached directory
        let is_allowed = self.is_expansion_allowed(&canonical_dir, &canonical_root, entries);
        if !is_allowed {
            return self.clone();
        }

        // Expand using the lazy cache (this is a pure cache operation)
        let mut cache = self.tree_cache.clone();
        match cache.expand_dir(&canonical_dir) {
            Ok(_) => {
                // Return state with UNCHANGED navigation (only cache updated)
                Self {
                    folder_state: FolderState::Loaded {
                        root: root.clone(),
                        current_dir: current_dir.clone(),
                        entries: entries.clone(),
                        selected_file: selected_file.clone(),
                    },
                    locale: self.locale,
                    locale_dialog_open: self.locale_dialog_open,
                    pending_locale: self.pending_locale,
                    previous_folder_state: self.previous_folder_state.clone(),
                    tree_cache: cache,
                }
            }
            Err(_) => self.clone(),
        }
    }

    /// Check if a directory is allowed for expansion.
    ///
    /// A directory is allowed if:
    /// - It appears as a directory entry in the root's visible entries, OR
    /// - It is a direct child of an already-expanded cached directory
    ///
    /// Note: root itself is NOT an "expanded cached parent" - only explicitly
    /// expanded directories count.
    fn is_expansion_allowed(
        &self,
        target: &PathBuf,
        root: &PathBuf,
        root_entries: &[crate::fs::Entry],
    ) -> bool {
        // Canonicalize target for comparison
        let canonical_target = match std::fs::canonicalize(target) {
            Ok(p) => p,
            Err(_) => return false,
        };

        // Check direct entry in root_entries
        // Handle both canonical and non-canonical path forms
        for entry in root_entries {
            // Direct match
            if entry.path == *target || entry.path == canonical_target {
                return entry.kind == crate::fs::FileKind::Dir;
            }
            // Check if entry path matches canonicalized target
            if let Ok(canonical_entry) = std::fs::canonicalize(&entry.path) {
                if canonical_entry == canonical_target {
                    return entry.kind == crate::fs::FileKind::Dir;
                }
            }
        }

        // Check if target is a direct child of an expanded cached directory
        if let Some(parent) = target.parent() {
            // Skip root itself - it's not an "expanded cached parent"
            if parent != root && self.tree_cache.is_expanded(parent) {
                return true;
            }
        }

        false
    }

    /// Collapse a directory in the tree cache.
    ///
    /// This is a PURE cache operation: it marks the directory as collapsed
    /// WITHOUT changing the navigation state.
    ///
    /// Valid from `Loaded` only. No-op from other states or for non-cached dirs.
    #[must_use]
    pub fn collapse_dir(&self, dir: &Path) -> Self {
        let FolderState::Loaded {
            root,
            current_dir,
            entries,
            selected_file,
        } = &self.folder_state
        else {
            return self.clone();
        };

        let canonical_dir = match std::fs::canonicalize(dir) {
            Ok(p) => p,
            Err(_) => return self.clone(),
        };

        // Only collapse if it's in the cache
        if !self.tree_cache.is_dir_loaded(&canonical_dir) {
            return self.clone();
        }

        let mut cache = self.tree_cache.clone();
        cache.collapse_dir(&canonical_dir);

        // Return state with UNCHANGED navigation (only cache updated)
        Self {
            folder_state: FolderState::Loaded {
                root: root.clone(),
                current_dir: current_dir.clone(),
                entries: entries.clone(),
                selected_file: selected_file.clone(),
            },
            locale: self.locale,
            locale_dialog_open: self.locale_dialog_open,
            pending_locale: self.pending_locale,
            previous_folder_state: self.previous_folder_state.clone(),
            tree_cache: cache,
        }
    }

    /// Navigate into a subdirectory within the currently loaded folder.
    ///
    /// This is the navigation operation that DOES change current_dir and entries.
    /// It clears the viewer selection (legacy behavior for drill-down).
    ///
    /// Valid from `Loaded` only. No-op from other states.
    ///
    /// Allows navigation to:
    /// - A direct child of current_dir
    /// - A descendant of an expanded cached parent (nested navigation)
    pub fn navigate_to_dir(&self, dir: &Path) -> Self {
        let FolderState::Loaded {
            root,
            current_dir,
            selected_file: _,
            ..
        } = &self.folder_state
        else {
            return self.clone();
        };

        // Reject symlinks
        if dir.is_symlink() {
            return self.clone();
        }

        // Canonicalize for reliable comparison
        let canonical_dir = match std::fs::canonicalize(dir) {
            Ok(p) => p,
            Err(_) => return self.clone(),
        };
        let canonical_current = match std::fs::canonicalize(current_dir) {
            Ok(p) => p,
            Err(_) => return self.clone(),
        };

        // Must be under project root
        let canonical_root = match std::fs::canonicalize(root) {
            Ok(p) => p,
            Err(_) => return self.clone(),
        };
        if !canonical_dir.starts_with(&canonical_root) {
            return self.clone();
        }

        // Must be a real directory
        if !canonical_dir.is_dir() {
            return self.clone();
        }

        // Check if navigation is allowed:
        // - Direct child of current_dir, OR
        // - Descendant of an expanded cached parent
        let is_direct_child = canonical_dir.parent() == Some(&canonical_current);
        let is_descendant_of_expanded =
            self.is_descendant_of_expanded(&canonical_dir, &canonical_current);

        if !is_direct_child && !is_descendant_of_expanded {
            return self.clone();
        }

        // Expand the target directory to get its entries
        let mut cache = self.tree_cache.clone();
        let entries = match cache.expand_dir(&canonical_dir) {
            Ok(e) => e.to_vec(),
            Err(_) => return self.clone(),
        };

        // Navigate: update current_dir and entries, clear selection
        Self {
            folder_state: FolderState::Loaded {
                root: canonical_root,
                current_dir: canonical_dir,
                entries,
                selected_file: None,
            },
            locale: self.locale,
            locale_dialog_open: self.locale_dialog_open,
            pending_locale: self.pending_locale,
            previous_folder_state: self.previous_folder_state.clone(),
            tree_cache: cache,
        }
    }

    /// Check if target is a descendant of an expanded cached directory.
    ///
    /// A target is allowed if there's an expanded parent that is:
    /// 1. A descendant of current_dir, AND
    /// 2. An ancestor of target
    ///
    /// This enables nested drill-down:
    /// - current=/root/src, target=/root/src/lib → YES (src is expanded and under current)
    /// - But NOT sibling navigation: current=/root/src, target=/root/lib → NO
    ///   (lib is not under src)
    fn is_descendant_of_expanded(&self, target: &Path, current: &Path) -> bool {
        // Walk up from target, checking each ancestor
        let mut ancestor = target.parent();
        while let Some(parent) = ancestor {
            // If we reach current_dir, stop
            if parent == current {
                break;
            }
            // Check if this ancestor is expanded AND is a descendant of current
            if self.tree_cache.is_expanded(parent) && parent.starts_with(current) {
                return true;
            }
            ancestor = parent.parent();
        }
        false
    }

    /// Navigate up one level toward the root.
    ///
    /// This is a navigation operation that changes current_dir to the parent.
    /// It also marks the current directory as collapsed in the cache.
    ///
    /// Valid from `Loaded` only. If already at the root, returns `self`.
    pub fn navigate_up(&self) -> Self {
        let FolderState::Loaded {
            root,
            current_dir,
            selected_file: _,
            ..
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

        // Get parent directory
        let parent = match canonical_current.parent() {
            Some(p) => p.to_path_buf(),
            None => return self.clone(),
        };

        // Mark current as collapsed in cache (but keep its entries)
        let mut cache = self.tree_cache.clone();
        cache.collapse_dir(&canonical_current);

        // Get parent entries from cache
        let entries = cache
            .get_entries(&parent)
            .map(|e| e.to_vec())
            .unwrap_or_default();

        // Navigate to parent (clear selection - legacy behavior for navigation)
        Self {
            folder_state: FolderState::Loaded {
                root: canonical_root,
                current_dir: parent,
                entries,
                selected_file: None,
            },
            locale: self.locale,
            locale_dialog_open: self.locale_dialog_open,
            pending_locale: self.pending_locale,
            previous_folder_state: self.previous_folder_state.clone(),
            tree_cache: cache,
        }
    }

    /// Navigate back to the root folder.
    ///
    /// Valid from `Loaded` only. No-op from other states.
    pub fn navigate_to_root(&self) -> Self {
        let FolderState::Loaded {
            root,
            current_dir,
            selected_file: _,
            ..
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

        // Get root entries from cache
        let mut cache = self.tree_cache.clone();
        let entries = cache
            .get_root_entries(&canonical_root)
            .map(|e| e.to_vec())
            .unwrap_or_default();

        Self {
            folder_state: FolderState::Loaded {
                root: canonical_root.clone(),
                current_dir: canonical_root,
                entries,
                // Clear selection on jump to root (legacy behavior)
                selected_file: None,
            },
            locale: self.locale,
            locale_dialog_open: self.locale_dialog_open,
            pending_locale: self.pending_locale,
            previous_folder_state: self.previous_folder_state.clone(),
            tree_cache: cache,
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
                tree_cache: self.tree_cache.clone(),
            },
            FolderState::Idle | FolderState::Loading => self.clone(),
        }
    }

    /// Close the currently loaded folder and return to `Idle`.
    ///
    /// Valid from `Loaded` only. No-op from other states.
    ///
    /// Clears the tree cache.
    pub fn reset_folder(&self) -> Self {
        match &self.folder_state {
            FolderState::Loaded { .. } => Self {
                folder_state: FolderState::Idle,
                locale: self.locale,
                locale_dialog_open: self.locale_dialog_open,
                pending_locale: self.pending_locale,
                previous_folder_state: self.previous_folder_state.clone(),
                // Clear tree cache on folder close
                tree_cache: TreeCache::new(DEFAULT_TREE_BUDGET),
            },
            FolderState::Idle | FolderState::Loading => self.clone(),
        }
    }
}
