use std::path::{Path, PathBuf};
use tokio::fs;

use crate::error::{Error, Result};

#[derive(Clone)]
pub struct PathValidator {
    allowed_dirs: Vec<PathBuf>,
}

impl PathValidator {
    pub fn new(allowed_dirs: Vec<PathBuf>) -> Self {
        Self { 
            allowed_dirs: allowed_dirs.into_iter()
                .map(|p| p.canonicalize().unwrap_or(p))
                .collect() 
        }
    }

    pub fn allowed_directories(&self) -> &[PathBuf] {
        &self.allowed_dirs
    }

    pub async fn validate_path<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf> {
        let path = path.as_ref();
        
        // Convert to absolute path
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(path)
        };

        // Canonicalize path if it exists
        let real_path = match fs::metadata(&abs_path).await {
            Ok(_) => tokio::fs::canonicalize(&abs_path).await?,
            Err(_) => {
                // For non-existent paths, validate parent directory
                let parent = abs_path.parent().ok_or_else(|| {
                    Error::InvalidPath("Path has no parent directory".into())
                })?;
                let parent = tokio::fs::canonicalize(parent).await?;
                parent.join(abs_path.file_name().ok_or_else(|| {
                    Error::InvalidPath("Path has no file name".into())
                })?)
            }
        };

        // Check if path is within allowed directories
        if !self.is_path_allowed(&real_path) {
            return Err(Error::PathNotAllowed(format!(
                "Path {} is outside of allowed directories",
                real_path.display()
            )));
        }

        Ok(real_path)
    }

    fn is_path_allowed(&self, path: &Path) -> bool {
        self.allowed_dirs.iter().any(|allowed| {
            path.starts_with(allowed)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_path_validation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let validator = PathValidator::new(vec![temp_dir.path().to_path_buf()]);

        // Test valid path
        let valid_path = temp_dir.path().join("test.txt");
        assert!(validator.validate_path(&valid_path).await.is_ok());

        // Test path outside allowed directories
        let invalid_path = PathBuf::from("/somewhere/else/test.txt");
        assert!(validator.validate_path(&invalid_path).await.is_err());
    }
}