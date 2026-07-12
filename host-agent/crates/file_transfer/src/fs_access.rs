use anyhow::Result;
use std::path::{Path, PathBuf};

/// Secure file system access with path traversal prevention
pub struct FsAccess {
    allowed_roots: Vec<PathBuf>,
}

impl FsAccess {
    pub fn new(allowed_roots: Vec<PathBuf>) -> Self {
        // Canonicalize the roots so they match canonicalized target paths. On
        // Windows canonicalize returns a `\\?\`-prefixed path, so an un-prefixed
        // root would never `starts_with` a canonicalized target.
        let allowed_roots = allowed_roots
            .into_iter()
            .map(|r| std::fs::canonicalize(&r).unwrap_or(r))
            .collect();
        Self { allowed_roots }
    }

    /// Validate that the path is within allowed roots (no traversal)
    pub fn validate_path(&self, path: &Path) -> Result<PathBuf> {
        // Canonicalize to resolve symlinks and ..
        let canonical = std::fs::canonicalize(path)
            .or_else(|_| {
                // File may not exist yet (for uploads); validate parent
                path.parent()
                    .map(|p| std::fs::canonicalize(p).map(|c| c.join(path.file_name().unwrap_or_default())))
                    .unwrap_or_else(|| Ok(path.to_path_buf()))
            })?;

        for root in &self.allowed_roots {
            if canonical.starts_with(root) {
                return Ok(canonical);
            }
        }

        anyhow::bail!(
            "Path '{}' is not in any allowed directory",
            path.display()
        )
    }

    pub fn is_allowed(&self, path: &Path) -> bool {
        self.validate_path(path).is_ok()
    }
}
