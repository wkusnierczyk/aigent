//! Symlink-safe filesystem helpers.
//!
//! These helpers use `symlink_metadata()` instead of `metadata()` to avoid
//! following symlinks. This prevents symlink-based directory escape attacks
//! in security-sensitive paths like skill directory traversal.

use std::path::Path;

/// Returns `true` if the path is a regular file (not a symlink).
///
/// Uses `symlink_metadata()` to avoid following symlinks.
#[must_use]
pub(crate) fn is_regular_file(path: &Path) -> bool {
    path.symlink_metadata()
        .map(|m| m.file_type().is_file())
        .unwrap_or(false)
}

/// Returns `true` if the path is a regular directory (not a symlink).
///
/// Uses `symlink_metadata()` to avoid following symlinks.
#[must_use]
pub(crate) fn is_regular_dir(path: &Path) -> bool {
    path.symlink_metadata()
        .map(|m| m.file_type().is_dir())
        .unwrap_or(false)
}

/// Returns `true` if the path is a symlink.
#[must_use]
pub(crate) fn is_symlink(path: &Path) -> bool {
    path.symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn is_regular_file_true_for_regular_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, "hello").unwrap();
        assert!(is_regular_file(&file));
    }

    #[test]
    fn is_regular_file_false_for_directory() {
        let dir = tempdir().unwrap();
        assert!(!is_regular_file(dir.path()));
    }

    #[test]
    fn is_regular_file_false_for_nonexistent() {
        let path = Path::new("/nonexistent/path/file.txt");
        assert!(!is_regular_file(path));
    }

    #[cfg(unix)]
    #[test]
    fn is_regular_file_false_for_symlink_to_file() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("target.txt");
        fs::write(&target, "hello").unwrap();
        let link = dir.path().join("link.txt");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(!is_regular_file(&link));
    }

    #[test]
    fn is_regular_dir_true_for_regular_dir() {
        let dir = tempdir().unwrap();
        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        assert!(is_regular_dir(&subdir));
    }

    #[test]
    fn is_regular_dir_false_for_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, "hello").unwrap();
        assert!(!is_regular_dir(&file));
    }

    #[test]
    fn is_regular_dir_false_for_nonexistent() {
        let path = Path::new("/nonexistent/path/dir");
        assert!(!is_regular_dir(path));
    }

    #[cfg(unix)]
    #[test]
    fn is_regular_dir_false_for_symlink_to_dir() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("target_dir");
        fs::create_dir(&target).unwrap();
        let link = dir.path().join("link_dir");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(!is_regular_dir(&link));
    }

    #[cfg(unix)]
    #[test]
    fn is_symlink_true_for_symlink() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("target.txt");
        fs::write(&target, "hello").unwrap();
        let link = dir.path().join("link.txt");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(is_symlink(&link));
    }

    #[test]
    fn is_symlink_false_for_regular_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, "hello").unwrap();
        assert!(!is_symlink(&file));
    }

    #[test]
    fn is_symlink_false_for_regular_dir() {
        let dir = tempdir().unwrap();
        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        assert!(!is_symlink(&subdir));
    }

    #[test]
    fn is_symlink_false_for_nonexistent() {
        let path = Path::new("/nonexistent/path/file.txt");
        assert!(!is_symlink(path));
    }
}
