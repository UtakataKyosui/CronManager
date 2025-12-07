use crate::cron_entry::CronEntry;
use anyhow::Result;

pub struct CronParser;

impl CronParser {
    pub fn parse(content: &str) -> Result<Vec<CronEntry>> {
        let mut entries = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();

            // Skip empty lines
            if line.is_empty() {
                i += 1;
                continue;
            }

            // Check if this is a NAME comment
            if line.starts_with("# NAME:") {
                let name = line.strip_prefix("# NAME:").unwrap().trim().to_string();
                i += 1;

                if i < lines.len() {
                    let next_line = lines[i].trim();

                    // Check if the entry is commented out (disabled)
                    let (enabled, cron_line) = if next_line.starts_with("# ") && !next_line.starts_with("# NAME:") {
                        (false, next_line.strip_prefix("# ").unwrap())
                    } else {
                        (true, next_line)
                    };

                    // Parse the cron line
                    if let Some((schedule, command)) = Self::parse_cron_line(cron_line) {
                        let mut entry = CronEntry::new(name, schedule, command);
                        entry.enabled = enabled;
                        entries.push(entry);
                    }
                }
            } else if !line.starts_with("#") {
                // Regular cron line without a name
                if let Some((schedule, command)) = Self::parse_cron_line(line) {
                    let name = format!("Unnamed ({})", entries.len() + 1);
                    entries.push(CronEntry::new(name, schedule, command));
                }
            }

            i += 1;
        }

        Ok(entries)
    }

    fn parse_cron_line(line: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = line.splitn(6, ' ').collect();

        if parts.len() >= 6 {
            // Standard cron format: minute hour day month weekday command
            let schedule = parts[0..5].join(" ");
            let command = parts[5..].join(" ");
            Some((schedule, command))
        } else {
            None
        }
    }

    pub fn serialize(entries: &[CronEntry]) -> String {
        let mut output = String::new();

        for entry in entries {
            output.push_str(&entry.to_crontab_string());
            output.push('\n');
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_crontab() {
        let content = r#"# NAME: Daily Backup
0 2 * * * /bin/backup.sh

# NAME: Hourly Check
0 * * * * /bin/check.sh
"#;

        let entries = CronParser::parse(content).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "Daily Backup");
        assert_eq!(entries[0].schedule, "0 2 * * *");
        assert_eq!(entries[0].command, "/bin/backup.sh");
        assert!(entries[0].enabled);
    }

    #[test]
    fn test_parse_disabled_entry() {
        let content = r#"# NAME: Disabled Job
# 0 2 * * * /bin/disabled.sh
"#;

        let entries = CronParser::parse(content).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(!entries[0].enabled);
    }

    #[test]
    fn test_serialize() {
        let entries = vec![
            CronEntry::new(
                "Test".to_string(),
                "0 2 * * *".to_string(),
                "/bin/test".to_string(),
            ),
        ];

        let output = CronParser::serialize(&entries);
        assert!(output.contains("# NAME: Test"));
        assert!(output.contains("0 2 * * * /bin/test"));
    }
}
