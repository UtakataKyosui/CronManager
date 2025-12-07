use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CronEntry {
    pub name: String,
    pub schedule: String,  // Cron expression (e.g., "0 2 * * *")
    pub command: String,   // Command to execute
    pub enabled: bool,     // Whether this entry is active
}

impl CronEntry {
    pub fn new(name: String, schedule: String, command: String) -> Self {
        Self {
            name,
            schedule,
            command,
            enabled: true,
        }
    }

    pub fn validate_schedule(&self) -> bool {
        cron::Schedule::from_str(&self.schedule).is_ok()
    }

    pub fn to_crontab_string(&self) -> String {
        if self.enabled {
            format!("# NAME: {}\n{} {}", self.name, self.schedule, self.command)
        } else {
            format!("# NAME: {}\n# {} {}", self.name, self.schedule, self.command)
        }
    }
}

use std::str::FromStr;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cron_entry_creation() {
        let entry = CronEntry::new(
            "Test Job".to_string(),
            "0 2 * * *".to_string(),
            "/bin/backup.sh".to_string(),
        );
        assert_eq!(entry.name, "Test Job");
        assert_eq!(entry.schedule, "0 2 * * *");
        assert!(entry.enabled);
    }

    #[test]
    fn test_validate_schedule() {
        let entry = CronEntry::new(
            "Valid".to_string(),
            "0 2 * * *".to_string(),
            "/bin/test".to_string(),
        );
        assert!(entry.validate_schedule());

        let invalid = CronEntry::new(
            "Invalid".to_string(),
            "invalid cron".to_string(),
            "/bin/test".to_string(),
        );
        assert!(!invalid.validate_schedule());
    }
}
