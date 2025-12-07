use crate::cron_entry::CronEntry;
use crate::cron_parser::CronParser;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Storage {
    file_path: PathBuf,
    use_system_crontab: bool,
}

impl Storage {
    pub fn new(custom_path: Option<PathBuf>) -> Self {
        match custom_path {
            Some(path) => Self {
                file_path: path,
                use_system_crontab: false,
            },
            None => {
                // Use a local file for testing/development
                let default_path = dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".cron-manager-crontab");

                Self {
                    file_path: default_path,
                    use_system_crontab: false,
                }
            }
        }
    }

    pub fn with_system_crontab() -> Self {
        Self {
            file_path: PathBuf::from("/tmp/crontab-temp"),
            use_system_crontab: true,
        }
    }

    pub fn load(&self) -> Result<Vec<CronEntry>> {
        let content = if self.use_system_crontab {
            self.load_from_system()?
        } else {
            if !self.file_path.exists() {
                // Try to import from system crontab on first run
                match self.load_from_system() {
                    Ok(system_content) if !system_content.is_empty() => {
                        // Save the imported content to local file
                        fs::write(&self.file_path, &system_content)
                            .with_context(|| format!("Failed to create initial file: {:?}", self.file_path))?;
                        system_content
                    }
                    _ => {
                        // No system crontab or error reading it, start with empty file
                        String::new()
                    }
                }
            } else {
                fs::read_to_string(&self.file_path)
                    .with_context(|| format!("Failed to read file: {:?}", self.file_path))?
            }
        };

        CronParser::parse(&content)
    }

    pub fn save(&self, entries: &[CronEntry]) -> Result<()> {
        let content = CronParser::serialize(entries);

        if self.use_system_crontab {
            self.save_to_system(&content)?;
        } else {
            fs::write(&self.file_path, content)
                .with_context(|| format!("Failed to write file: {:?}", self.file_path))?;
        }

        Ok(())
    }

    fn load_from_system(&self) -> Result<String> {
        let output = Command::new("crontab")
            .arg("-l")
            .output()
            .context("Failed to execute crontab -l")?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            // No crontab exists yet
            Ok(String::new())
        }
    }

    fn save_to_system(&self, content: &str) -> Result<()> {
        // Write to temporary file first
        fs::write(&self.file_path, content)
            .with_context(|| format!("Failed to write temp file: {:?}", self.file_path))?;

        // Load the temporary file into crontab
        let output = Command::new("crontab")
            .arg(&self.file_path)
            .output()
            .context("Failed to execute crontab command")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to install crontab: {}", error);
        }

        Ok(())
    }

    pub fn get_file_path(&self) -> &Path {
        &self.file_path
    }
}

// Add the dirs dependency
