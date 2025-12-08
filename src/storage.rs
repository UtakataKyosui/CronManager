use crate::cron_entry::CronEntry;
use crate::scheduler::{create_scheduler, Scheduler};
use anyhow::Result;

pub struct Storage {
    scheduler: Box<dyn Scheduler>,
}

impl Storage {
    /// Create a new Storage instance with a local file backend
    pub fn new(custom_path: Option<std::path::PathBuf>) -> Self {
        let scheduler = Box::new(crate::scheduler::file::FileScheduler::new(custom_path));
        Self { scheduler }
    }

    /// Create a Storage instance with the system scheduler backend
    /// (cron on Linux, launchd on macOS)
    pub fn with_system_scheduler() -> Self {
        let scheduler = create_scheduler(true);
        Self { scheduler }
    }

    /// Load all cron entries from the scheduler
    pub fn load(&self) -> Result<Vec<CronEntry>> {
        self.scheduler.load()
    }

    /// Save all cron entries to the scheduler
    pub fn save(&self, entries: &[CronEntry]) -> Result<()> {
        self.scheduler.save(entries)
    }

    /// Get the backend name for display purposes
    pub fn get_backend_name(&self) -> &'static str {
        self.scheduler.backend_name()
    }
}
