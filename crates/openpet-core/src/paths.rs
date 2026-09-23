use std::fs;
use std::io;
use std::path::PathBuf;

/// Strongly typed application filesystem hierarchy for OpenPet.
/// Avoids magic strings and ensures all components use consistent, safe paths.
#[derive(Debug, Clone)]
pub struct AppPaths {
    pub root: PathBuf,
    pub data: PathBuf,
    pub packs: PathBuf,
    pub plugins: PathBuf,
    pub cache: PathBuf,
    pub logs: PathBuf,
    pub staging: PathBuf,
    pub updates: PathBuf,
}

impl AppPaths {
    /// Constructs default paths located in `%LOCALAPPDATA%\OpenPet` or user-specified root.
    pub fn new(root: PathBuf) -> Self {
        Self {
            data: root.join("data"),
            packs: root.join("packs"),
            plugins: root.join("plugins"),
            cache: root.join("cache"),
            logs: root.join("logs"),
            staging: root.join("staging"),
            updates: root.join("updates"),
            root,
        }
    }

    /// Resolves canonical application directory for current user environment.
    pub fn default_windows_paths() -> Self {
        let base = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        Self::new(base.join("OpenPet"))
    }

    /// Creates all defined application directories if they do not already exist.
    pub fn ensure_all_dirs(&self) -> io::Result<()> {
        fs::create_dir_all(&self.root)?;
        fs::create_dir_all(&self.data)?;
        fs::create_dir_all(&self.packs)?;
        fs::create_dir_all(&self.plugins)?;
        fs::create_dir_all(&self.cache)?;
        fs::create_dir_all(&self.logs)?;
        fs::create_dir_all(&self.staging)?;
        fs::create_dir_all(&self.updates)?;
        Ok(())
    }

    /// Primary database file path (`openpet.db`).
    pub fn database_file(&self) -> PathBuf {
        self.data.join("openpet.db")
    }

    /// Cleanup dangling temporary extraction artifacts in `staging/` after startup.
    pub fn cleanup_orphaned_staging(&self) -> io::Result<()> {
        if self.staging.exists() {
            for entry in fs::read_dir(&self.staging)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    let _ = fs::remove_dir_all(&path);
                } else {
                    let _ = fs::remove_file(&path);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn test_app_paths_creation() {
        let temp_root = temp_dir().join(format!("openpet_test_{}", uuid::Uuid::new_v4()));
        let paths = AppPaths::new(temp_root.clone());

        assert!(paths.ensure_all_dirs().is_ok());
        assert!(paths.data.exists());
        assert!(paths.staging.exists());

        // Test staging cleanup
        let dummy_file = paths.staging.join("test.tmp");
        fs::write(&dummy_file, b"test").unwrap();
        assert!(dummy_file.exists());

        paths.cleanup_orphaned_staging().unwrap();
        assert!(!dummy_file.exists());

        let _ = fs::remove_dir_all(&temp_root);
    }
}
