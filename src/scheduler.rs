use crate::cron_entry::CronEntry;
use anyhow::Result;

/// Trait for different scheduler backends (cron, launchd, etc.)
pub trait Scheduler: Send + Sync {
    /// Load all scheduled entries from the scheduler
    fn load(&self) -> Result<Vec<CronEntry>>;

    /// Save all scheduled entries to the scheduler
    fn save(&self, entries: &[CronEntry]) -> Result<()>;

    /// Get a human-readable name for this scheduler backend
    fn backend_name(&self) -> &'static str;
}

/// Auto-detect and create the appropriate scheduler for the current OS
pub fn create_scheduler(use_system: bool) -> Box<dyn Scheduler> {
    #[cfg(target_os = "macos")]
    {
        if use_system {
            Box::new(crate::scheduler::launchd::LaunchdScheduler::new())
        } else {
            Box::new(crate::scheduler::file::FileScheduler::new(None))
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        if use_system {
            Box::new(crate::scheduler::cron::CronScheduler::new())
        } else {
            Box::new(crate::scheduler::file::FileScheduler::new(None))
        }
    }
}

pub mod file;
pub mod cron;

#[cfg(target_os = "macos")]
pub mod launchd;
