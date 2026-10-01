//! Tree data structures for lazy-loaded nested navigation.
//!
//! Provides a global budget-bounded tree cache that:
//! - Loads directory children only on demand (lazy expansion)
//! - Retains cached children across collapse/re-expand cycles
//! - Bounds total cached entries with a configurable global budget
//! - Reports incomplete/limit state when budget is exhausted

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::fs::{scan_dir, Entry};

/// A lazily-loaded tree cache with a global entry budget.
///
/// # Budget Model
/// - Root entries count toward the budget
/// - All cached directory descendants count toward the budget
/// - The global cap (default 50,000) prevents unbounded memory growth
///
/// # Lifecycle
/// 1. Root scanned on project open → root entries cached
/// 2. Directory expanded → children loaded and cached
/// 3. Directory collapsed → children retained (cache not cleared)
/// 4. Directory re-expanded → children served from cache (no re-scan)
/// 5. Project closed/replaced → cache cleared
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeCache {
    /// Global entry budget (e.g., 50,000).
    budget: usize,
    /// Total entries currently cached (root + all descendants).
    total_entries: usize,
    /// Cached entries per directory path.
    /// Key: absolute canonical path of the directory.
    /// Value: entries visible in that directory.
    entries: HashMap<PathBuf, Vec<Entry>>,
    /// Directories that have been expanded at least once.
    /// Used to distinguish "never expanded" from "expanded and collapsed".
    expanded_dirs: HashMap<PathBuf, bool>,
    /// Whether the budget was exhausted at some point during this session.
    incomplete: bool,
}

impl TreeCache {
    /// Create a new empty tree cache with the given global budget.
    pub fn new(budget: usize) -> Self {
        Self {
            budget,
            total_entries: 0,
            entries: HashMap::new(),
            expanded_dirs: HashMap::new(),
            incomplete: false,
        }
    }

    /// Get or scan root entries, respecting the budget.
    ///
    /// Returns entries visible at the root level.
    /// Marks the root as expanded if not already.
    pub fn get_root_entries(&mut self, root: &Path) -> std::io::Result<&[Entry]> {
        let canonical_root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());

        if !self.entries.contains_key(&canonical_root) {
            self.cache_entries(&canonical_root)?;
        }

        // Mark root as expanded
        self.expanded_dirs.insert(canonical_root.clone(), true);

        Ok(self
            .entries
            .get(&canonical_root)
            .map(|v| v.as_slice())
            .unwrap_or(&[]))
    }

    /// Get entries for a directory, using canonical path as key.
    ///
    /// This ensures consistent path handling regardless of how the path
    /// was originally specified (canonical or non-canonical form).
    pub fn get_entries(&mut self, dir: &Path) -> std::io::Result<&[Entry]> {
        let canonical = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());

        if !self.entries.contains_key(&canonical) {
            self.cache_entries(&canonical)?;
        }

        Ok(self
            .entries
            .get(&canonical)
            .map(|v| v.as_slice())
            .unwrap_or(&[]))
    }

    /// Expand a directory, loading its children if not already cached.
    ///
    /// Returns a reference to the directory's entries.
    /// Marks the directory as expanded (even if already cached).
    ///
    /// Rejects symlinks: directories that are symlinks cannot be expanded.
    pub fn expand_dir(&mut self, dir: &Path) -> std::io::Result<&[Entry]> {
        // Safety: reject symlinks (not followed)
        // Check if the path itself is a symlink
        if dir.is_symlink() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "cannot expand symlink",
            ));
        }

        let canonical = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());

        // Verify the canonical path is not a symlink (symlink to directory)
        if std::fs::symlink_metadata(&canonical)
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false)
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "cannot expand symlink",
            ));
        }

        // Check if it's already cached
        if !self.entries.contains_key(&canonical) {
            self.cache_entries(&canonical)?;
        }

        // Mark as expanded
        self.expanded_dirs.insert(canonical.clone(), true);

        Ok(self
            .entries
            .get(&canonical)
            .map(|v| v.as_slice())
            .unwrap_or(&[]))
    }

    /// Mark a directory as collapsed without clearing its cached children.
    ///
    /// The directory's children remain in the cache for re-use on re-expansion.
    /// The directory is marked as NOT expanded so re-expand will show it as collapsed.
    pub fn collapse_dir(&mut self, dir: &Path) {
        let canonical = match std::fs::canonicalize(dir) {
            Ok(p) => p,
            Err(_) => return,
        };
        // Mark as NOT expanded (but keep the cached entries)
        self.expanded_dirs.insert(canonical, false);
    }

    /// Check if a directory has been loaded (expanded at least once).
    pub fn is_dir_loaded(&self, dir: &Path) -> bool {
        let canonical = match std::fs::canonicalize(dir) {
            Ok(p) => p,
            Err(_) => return false,
        };
        self.entries.contains_key(&canonical)
    }

    /// Check if a directory is currently marked as expanded (not collapsed).
    ///
    /// Returns true if the directory has been expanded and not subsequently
    /// collapsed. A directory that was never expanded returns false.
    pub fn is_expanded(&self, dir: &Path) -> bool {
        let canonical = match std::fs::canonicalize(dir) {
            Ok(p) => p,
            Err(_) => return false,
        };
        self.expanded_dirs.get(&canonical).copied().unwrap_or(false)
    }

    /// Get the total number of cached entries.
    pub fn total_entries(&self) -> usize {
        self.total_entries
    }

    /// Whether the cache has encountered the budget limit.
    pub fn is_incomplete(&self) -> bool {
        self.incomplete
    }

    /// Get the configured global budget.
    pub fn budget(&self) -> usize {
        self.budget
    }

    /// Clear all cached entries and reset state.
    pub fn clear(&mut self) {
        self.total_entries = 0;
        self.entries.clear();
        self.expanded_dirs.clear();
        self.incomplete = false;
    }

    /// Internal: scan and cache entries for a directory.
    fn cache_entries(&mut self, dir: &Path) -> std::io::Result<()> {
        let entries = scan_dir(dir)?;

        // Count how many new entries we're adding
        let total_entries_count = entries.len();
        let remaining = self.budget.saturating_sub(self.total_entries);

        // Check if adding these entries would exceed budget
        if total_entries_count > remaining {
            self.incomplete = true;
        }

        // Only cache what fits in the budget
        let mut entries_to_cache: Vec<Entry> = entries.into_iter().take(remaining).collect();

        // If we truncated entries due to budget, mark the last one as incomplete
        let was_truncated = total_entries_count > remaining;
        if was_truncated {
            if let Some(last) = entries_to_cache.last_mut() {
                last.is_incomplete = true;
            }
        }

        let cached_count = entries_to_cache.len();
        self.total_entries = self.total_entries.saturating_add(cached_count);

        self.entries.insert(dir.to_path_buf(), entries_to_cache);

        Ok(())
    }
}

impl Default for TreeCache {
    fn default() -> Self {
        Self::new(50_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_cache_default_budget() {
        let cache = TreeCache::default();
        assert_eq!(cache.total_entries(), 0);
        assert!(!cache.is_incomplete());
    }

    #[test]
    fn tree_cache_custom_budget() {
        let cache = TreeCache::new(100);
        assert_eq!(cache.total_entries(), 0);
        assert!(!cache.is_incomplete());
    }
}
