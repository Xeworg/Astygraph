//! Filesystem access layer.
//!
//! Provides safe project-file discovery with ignore rules, exclusion defaults,
//! and bounded discovery. Keeps all logic headless-testable.

pub mod tree;

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};

/// Maximum filesystem entries to return before signaling incompleteness.
/// Matches the NFR-7 discovery ceiling from the PRD.
pub const DISCOVERY_CAP: usize = 50_000;

/// Entries excluded by default regardless of `.gitignore`.
pub const DEFAULT_EXCLUSIONS: &[&str] = &[
    ".git",
    ".astynex",
    "target",
    "node_modules",
    "__pycache__",
    ".cache",
    ".pytest_cache",
    "vendor",
    "dist",
    "build",
    ".next",
    ".nuxt",
];

/// Kind of filesystem entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    File,
    Dir,
    Symlink,
}

/// A single filesystem entry returned by [`scan_dir`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Absolute path to the entry.
    pub path: PathBuf,
    /// What kind of filesystem object this is.
    pub kind: FileKind,
    /// True when the discovery cap was hit and this entry is the last visible one.
    pub is_incomplete: bool,
}

/// Parses `.gitignore` at root into a [`GlobSet`].
///
/// Supports basic patterns: `foo`, `*.tmp`, `dir/`, `**/foo`.
/// Negation patterns (`!pattern`) are ignored (negation not in PRD scope).
fn parse_gitignore(root: &Path) -> GlobSet {
    let gitignore_path = root.join(".gitignore");
    let Ok(content) = fs::read_to_string(&gitignore_path) else {
        return GlobSet::empty();
    };

    let patterns: Vec<String> = content
        .lines()
        .filter(|line| {
            let line = line.trim();
            // Skip empty lines, comments, and negation patterns
            !line.is_empty() && !line.starts_with('#') && !line.starts_with('!')
        })
        .map(|line| {
            // Normalize: make patterns that don't start with / into **/foo
            let line = line.trim_end_matches('/');
            if line.starts_with('/') {
                line.to_string()
            } else {
                format!("**/{line}")
            }
        })
        .collect();

    let mut builder = GlobSetBuilder::new();
    for pattern in &patterns {
        if let Ok(glob) = Glob::new(pattern) {
            builder.add(glob);
        }
    }
    builder.build().unwrap_or_else(|_| GlobSet::empty())
}

/// Scans a directory and returns its direct entries (non-recursive), sorted
/// alphabetically.
///
/// Exclusion policy:
/// 1. Default exclusions (`.git/`, `.astynex/`, build/cache dirs).
/// 2. `.gitignore` patterns (via globset, for non-git directories).
/// 3. Binary files (NUL byte in first 512 bytes).
/// 4. Symlinks are **not** followed — they appear as `Symlink` kind.
///
/// Returns at most [`DISCOVERY_CAP`] entries. When the cap is reached the
/// last entry has `is_incomplete = true`.
///
/// Uses `std::fs::read_dir` for reliable cross-platform behavior without
/// the `ignore` crate's traversal complexity. The `ignore` crate is retained
/// in dependencies for future glob-based rule expansion.
pub fn scan_dir(root: &Path) -> io::Result<Vec<Entry>> {
    let mut entries = Vec::with_capacity(256);
    let mut hit_cap = false;

    // Parse .gitignore at root (handles non-git directories).
    let gitignore = parse_gitignore(root);

    // Use std::fs::read_dir for reliable direct-children listing.
    // WalkBuilder adds complexity and hidden-directory access that causes
    // I/O errors in some test environments.
    let dir = fs::read_dir(root)?;

    for entry_result in dir {
        if hit_cap {
            break;
        }

        let entry = match entry_result {
            Ok(e) => e,
            Err(_) => continue, // skip entries we can't read
        };

        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // --- Default exclusions ---
        if DEFAULT_EXCLUSIONS.contains(&name) {
            continue;
        }

        // --- .gitignore patterns ---
        // Check both relative to root and absolute path.
        let relative = path.strip_prefix(root).unwrap_or(&path);
        if gitignore.is_match(relative) || gitignore.is_match(&path) {
            continue;
        }

        // --- Symlink handling ---
        // Check symlink first since is_symlink is fast.
        let kind = if path.is_symlink() {
            FileKind::Symlink
        } else if path.is_dir() {
            FileKind::Dir
        } else if path.is_file() {
            // Binary detection: check first 512 bytes for NUL byte.
            if is_binary(&path)? {
                continue;
            }
            FileKind::File
        } else {
            continue; // skip unusual entries (devices, sockets, etc.)
        };

        if entries.len() >= DISCOVERY_CAP {
            hit_cap = true;
        }

        entries.push(Entry {
            path: path.to_path_buf(),
            kind,
            is_incomplete: false,
        });
    }

    // Sort alphabetically by file name.
    entries.sort_by(|a, b| {
        let name_a = a.path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let name_b = b.path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        name_a.cmp(name_b)
    });

    // Signal incompleteness on the last entry if cap was hit.
    if hit_cap {
        if let Some(last) = entries.last_mut() {
            last.is_incomplete = true;
        }
    }

    Ok(entries)
}

/// Returns true if the file is binary (contains a NUL byte in its first 512 bytes).
fn is_binary(path: &Path) -> io::Result<bool> {
    let mut file = fs::File::open(path)?;
    let mut buf = [0u8; 512];
    let n = file.read(&mut buf)?;
    Ok(buf[..n].contains(&0x00))
}
