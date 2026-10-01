//! Cache path resolution.
//!
//! Resolves `<project_dir>/.astynex/cache.db`, creating the `.astynex/`
//! directory if absent.  Returns a typed [`Error::CacheDir`] if the project
//! directory does not exist.

use crate::persistence::Error;
use std::path::{Path, PathBuf};

/// The name of the Astynex cache directory inside a project folder.
pub const CACHE_DIR_NAME: &str = ".astynex";

/// The filename of the SQLite cache database.
pub const CACHE_DB_NAME: &str = "cache.db";

/// Returns the absolute path to `<project_dir>/.astynex/cache.db`.
///
/// The `.astynex/` directory is created lazily if it does not already exist.
/// Returns a `CacheDir` error if `project_dir` does not exist or is not a
/// directory.
///
/// # Privacy contract
///
/// This function creates only the `.astynex/` directory and returns a path.
/// It does not open or write the database, does not read project source, and
/// does not transmit any content.
pub fn cache_db_path(project_dir: &Path) -> Result<PathBuf, Error> {
    // Reject non-existent or non-directory paths early.
    if !project_dir.is_dir() {
        return Err(Error::cache_dir(format!(
            "project directory does not exist or is not a directory: {}",
            project_dir.display()
        )));
    }

    let cache_dir = project_dir.join(CACHE_DIR_NAME);

    // Create the cache directory if absent.  fs::create_dir is idempotent
    // when the directory already exists (returns Ok on existing dir).
    if !cache_dir.is_dir() {
        std::fs::create_dir(&cache_dir)?;
    }

    Ok(cache_dir.join(CACHE_DB_NAME))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn cache_db_path_returns_correct_subpath() {
        let tmp = TempDir::new().unwrap();
        let project = tmp.path();
        let result = cache_db_path(project).expect("must succeed for existing dir");
        assert_eq!(result, project.join(".astynex").join("cache.db"));
    }

    #[test]
    fn cache_db_path_creates_dot_astynex() {
        let tmp = TempDir::new().unwrap();
        let project = tmp.path();
        let dot_astynex = project.join(".astynex");
        assert!(
            !dot_astynex.exists(),
            "precondition: .astynex must not exist"
        );
        let _ = cache_db_path(project).expect("must succeed");
        assert!(dot_astynex.is_dir(), ".astynex must be created");
    }

    #[test]
    fn cache_db_path_idempotent_on_existing_dir() {
        let tmp = TempDir::new().unwrap();
        let project = tmp.path();
        let _ = cache_db_path(project).expect("first call must succeed");
        let _ = cache_db_path(project).expect("second call must succeed");
        assert!(project.join(".astynex").is_dir());
    }

    #[test]
    fn cache_db_path_rejects_non_existent_parent() {
        let fake = PathBuf::from("/this/path/does/not/exist/xyz789");
        let result = cache_db_path(&fake);
        assert!(matches!(result, Err(Error::CacheDir(_))));
    }

    #[test]
    fn cache_db_path_rejects_file_as_parent() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("project_file");
        fs::write(&file, b"not a directory").unwrap();
        let result = cache_db_path(&file);
        assert!(matches!(result, Err(Error::CacheDir(_))));
    }
}
