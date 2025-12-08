use crate::cron_entry::CronEntry;
use crate::cron_parser::CronParser;
use crate::scheduler::Scheduler;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Cron-based scheduler for Linux and other Unix systems
pub struct CronScheduler {
    temp_file: PathBuf,
}

impl CronScheduler {
    pub fn new() -> Self {
        Self {
            temp_file: PathBuf::from("/tmp/crontab-temp"),
        }
    }

    fn load_from_crontab(&self) -> Result<String> {
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

    fn save_to_crontab(&self, content: &str) -> Result<()> {
        // Write to temporary file first
        fs::write(&self.temp_file, content)
            .with_context(|| format!("Failed to write temp file: {:?}", self.temp_file))?;

        // Load the temporary file into crontab
        let output = Command::new("crontab")
            .arg(&self.temp_file)
            .output()
            .context("Failed to execute crontab command")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to install crontab: {}", error);
        }

        Ok(())
    }
}

impl Scheduler for CronScheduler {
    fn load(&self) -> Result<Vec<CronEntry>> {
        let content = self.load_from_crontab()?;
        CronParser::parse(&content)
    }

    fn save(&self, entries: &[CronEntry]) -> Result<()> {
        let content = CronParser::serialize(entries);
        self.save_to_crontab(&content)
    }

    fn backend_name(&self) -> &'static str {
        "Cron"
    }
}
