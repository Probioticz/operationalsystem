//! RetroOS File System Module
//! 
//! Lightweight file system abstraction for file operations and virtual filesystem.

use anyhow::{Result, Context};
use log::{info, debug, error};
use std::path::{Path, PathBuf};
use std::fs;
use std::io::{Read, Write};

/// File system abstraction
pub struct FileSystem {
    root_path: PathBuf,
}

impl FileSystem {
    /// Create a new file system with specified root
    pub fn new(root: &str) -> Self {
        FileSystem {
            root_path: PathBuf::from(root),
        }
    }

    /// Get the root path
    pub fn root(&self) -> &Path {
        &self.root_path
    }

    /// Read a file's contents
    pub fn read_file(&self, path: &str) -> Result<String> {
        let full_path = self.root_path.join(path);
        debug!("Reading file: {:?}", full_path);
        fs::read_to_string(&full_path)
            .with_context(|| format!("Failed to read file: {:?}", full_path))
    }

    /// Write contents to a file
    pub fn write_file(&self, path: &str, content: &str) -> Result<()> {
        let full_path = self.root_path.join(path);
        debug!("Writing file: {:?}", full_path);
        
        // Create parent directories if they don't exist
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::write(&full_path, content)
            .with_context(|| format!("Failed to write file: {:?}", full_path))
    }

    /// Check if a path exists
    pub fn exists(&self, path: &str) -> bool {
        self.root_path.join(path).exists()
    }

    /// Check if a path is a directory
    pub fn is_dir(&self, path: &str) -> bool {
        self.root_path.join(path).is_dir()
    }

    /// Check if a path is a file
    pub fn is_file(&self, path: &str) -> bool {
        self.root_path.join(path).is_file()
    }

    /// List directory contents
    pub fn list_dir(&self, path: &str) -> Result<Vec<String>> {
        let full_path = self.root_path.join(path);
        debug!("Listing directory: {:?}", full_path);
        
        let entries = fs::read_dir(&full_path)
            .with_context(|| format!("Failed to read directory: {:?}", full_path))?;
        
        let mut names = Vec::new();
        for entry in entries {
            if let Ok(entry) = entry {
                if let Some(name) = entry.file_name().to_str() {
                    names.push(name.to_string());
                }
            }
        }
        
        Ok(names)
    }

    /// Create a directory
    pub fn create_dir(&self, path: &str) -> Result<()> {
        let full_path = self.root_path.join(path);
        debug!("Creating directory: {:?}", full_path);
        fs::create_dir_all(&full_path)
            .with_context(|| format!("Failed to create directory: {:?}", full_path))
    }

    /// Remove a file or directory
    pub fn remove(&self, path: &str) -> Result<()> {
        let full_path = self.root_path.join(path);
        debug!("Removing: {:?}", full_path);
        
        if full_path.is_dir() {
            fs::remove_dir_all(&full_path)
        } else {
            fs::remove_file(&full_path)
        }.with_context(|| format!("Failed to remove: {:?}", full_path))
    }

    /// Get file metadata
    pub fn metadata(&self, path: &str) -> Result<fs::Metadata> {
        let full_path = self.root_path.join(path);
        fs::metadata(&full_path)
            .with_context(|| format!("Failed to get metadata: {:?}", full_path))
    }

    /// Get file size in bytes
    pub fn file_size(&self, path: &str) -> Result<u64> {
        self.metadata(path).map(|m| m.len())
    }
}

impl Default for FileSystem {
    fn default() -> Self {
        Self::new("/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_filesystem_creation() {
        let fs = FileSystem::new("/tmp");
        assert_eq!(fs.root(), Path::new("/tmp"));
    }

    #[test]
    fn test_read_write_file() {
        let temp_dir = env::temp_dir().join("retro_os_test");
        let fs = FileSystem::new(temp_dir.to_str().unwrap());
        
        // Create test directory
        fs.create_dir("test").unwrap();
        
        // Write file
        fs.write_file("test/hello.txt", "Hello, RetroOS!").unwrap();
        
        // Read file
        let content = fs.read_file("test/hello.txt").unwrap();
        assert_eq!(content, "Hello, RetroOS!");
        
        // Clean up
        fs.remove("test").unwrap();
    }
}
