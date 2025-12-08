use crate::cron_entry::CronEntry;
use crate::cron_parser::CronParser;
use crate::scheduler::Scheduler;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// File-based scheduler for local testing/development
pub struct FileScheduler {
    file_path: PathBuf,
}

impl FileScheduler {
    pub fn new(custom_path: Option<PathBuf>) -> Self {
        let file_path = custom_path.unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".cron-manager-crontab")
        });

        Self { file_path }
    }
}

impl Scheduler for FileScheduler {
    fn load(&self) -> Result<Vec<CronEntry>> {
        let content = if !self.file_path.exists() {
            // Try to import from system on first run
            #[cfg(not(target_os = "macos"))]
            {
                match crate::scheduler::cron::CronScheduler::new().load() {
                    Ok(entries) if !entries.is_empty() => {
                        // Save imported entries
                        let content = CronParser::serialize(&entries);
                        fs::write(&self.file_path, &content)
                            .with_context(|| format!("Failed to create initial file: {:?}", self.file_path))?;
                        content
                    }
                    _ => String::new(),
                }
            }
            #[cfg(target_os = "macos")]
            {
                // On macOS, try to import from launchd
                match crate::scheduler::launchd::LaunchdScheduler::new().load() {
                    Ok(entries) if !entries.is_empty() => {
                        let content = CronParser::serialize(&entries);
                        fs::write(&self.file_path, &content)
                            .with_context(|| format!("Failed to create initial file: {:?}", self.file_path))?;
                        content
                    }
                    _ => String::new(),
                }
            }
        } else {
            fs::read_to_string(&self.file_path)
                .with_context(|| format!("Failed to read file: {:?}", self.file_path))?
        };

        CronParser::parse(&content)
    }

    fn save(&self, entries: &[CronEntry]) -> Result<()> {
        let content = CronParser::serialize(entries);
        fs::write(&self.file_path, content)
            .with_context(|| format!("Failed to write file: {:?}", self.file_path))?;
        Ok(())
    }

    fn backend_name(&self) -> &'static str {
        "File"
    }
}
