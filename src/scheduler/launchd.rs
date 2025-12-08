use crate::cron_entry::CronEntry;
use crate::scheduler::Scheduler;
use anyhow::{Context, Result};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::process::Command;

// Constants
const LABEL_PREFIX: &str = "com.cronmanager";
const STDOUT_PATH_PREFIX: &str = "/tmp";
const STDERR_PATH_PREFIX: &str = "/tmp";

/// Launchd-based scheduler for macOS
pub struct LaunchdScheduler {
    launch_agents_dir: PathBuf,
}

impl LaunchdScheduler {
    pub fn new() -> Self {
        // Use ~/Library/LaunchAgents for user-level tasks
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let launch_agents_dir = home.join("Library/LaunchAgents");

        Self { launch_agents_dir }
    }

    fn ensure_launch_agents_dir(&self) -> Result<()> {
        if !self.launch_agents_dir.exists() {
            fs::create_dir_all(&self.launch_agents_dir)
                .with_context(|| format!("Failed to create LaunchAgents directory: {:?}", self.launch_agents_dir))?;
        }
        Ok(())
    }

    fn entry_to_label(&self, entry: &CronEntry) -> String {
        // Create a unique label for this entry using a hash to avoid collisions
        // Different names like "My Task" and "My/Task" should have different labels
        let mut hasher = DefaultHasher::new();
        entry.name.hash(&mut hasher);
        let hash = hasher.finish();

        // Create a safe name for readability (alphanumeric only)
        let safe_name: String = entry.name
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .take(32) // Limit length
            .collect();

        format!("{}.{}.{:x}", LABEL_PREFIX, safe_name, hash)
    }

    fn plist_path(&self, label: &str) -> PathBuf {
        self.launch_agents_dir.join(format!("{}.plist", label))
    }

    fn escape_xml(&self, text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('\'', "&apos;")
            .replace('"', "&quot;")
    }

    fn unescape_xml(&self, text: &str) -> String {
        text.replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&apos;", "'")
            .replace("&quot;", "\"")
    }

    fn get_uid(&self) -> Result<String> {
        // Get the current user's UID using the id command
        let output = Command::new("id")
            .arg("-u")
            .output()
            .context("Failed to get user ID")?;

        if !output.status.success() {
            anyhow::bail!("Failed to get user ID");
        }

        let uid = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();

        Ok(uid)
    }

    fn validate_command(&self, command: &str) -> Result<()> {
        // Validate command to prevent shell injection vulnerabilities
        // Check for dangerous shell metacharacters that could be exploited
        let dangerous_chars = ['|', '&', ';', '\n', '\r', '`', '$'];

        for ch in dangerous_chars {
            if command.contains(ch) {
                eprintln!(
                    "Warning: Command contains potentially dangerous character '{}'. \
                     Consider reviewing the command for security concerns.",
                    ch
                );
            }
        }

        // Ensure command is not empty
        if command.trim().is_empty() {
            anyhow::bail!("Command cannot be empty");
        }

        Ok(())
    }

    fn cron_to_calendar_interval(&self, schedule: &str) -> Result<String> {
        // Check for special syntax first (like @daily, @hourly, etc.)
        if schedule.starts_with('@') {
            anyhow::bail!(
                "Cron expression contains unsupported special syntax '{}'. \
                 Please use explicit minute/hour/day values.",
                schedule
            );
        }

        // Parse cron expression: minute hour day month weekday
        let parts: Vec<&str> = schedule.split_whitespace().collect();
        if parts.len() != 5 {
            anyhow::bail!("Invalid cron expression: {}", schedule);
        }

        let minute = parts[0];
        let hour = parts[1];
        let day = parts[2];
        let month = parts[3];
        let weekday = parts[4];

        // Validate that cron expressions are supported (simple values only)
        // launchd doesn't support ranges (1-5), lists (1,3,5), or step values (*/15)
        for (i, part) in parts.iter().enumerate() {
            let field_name = match i {
                0 => "minute",
                1 => "hour",
                2 => "day",
                3 => "month",
                4 => "weekday",
                _ => unreachable!(),
            };

            if part.contains('-') {
                anyhow::bail!(
                    "Cron expression contains unsupported range '{}' in {} field. \
                     launchd only supports simple values or * wildcard.",
                    part, field_name
                );
            }
            if part.contains(',') {
                anyhow::bail!(
                    "Cron expression contains unsupported list '{}' in {} field. \
                     launchd only supports simple values or * wildcard.",
                    part, field_name
                );
            }
            if part.contains('/') {
                anyhow::bail!(
                    "Cron expression contains unsupported step value '{}' in {} field. \
                     launchd only supports simple values or * wildcard.",
                    part, field_name
                );
            }

            // Validate it's either * or a number
            if *part != "*" && part.parse::<u32>().is_err() {
                anyhow::bail!(
                    "Invalid value '{}' in {} field. Must be a number or *.",
                    part, field_name
                );
            }
        }

        // Convert to launchd calendar format
        let mut calendar_dict = String::new();

        // Month (1-12)
        if month != "*" {
            calendar_dict.push_str(&format!("        <key>Month</key>\n        <integer>{}</integer>\n", month));
        }

        // Day (1-31)
        if day != "*" {
            calendar_dict.push_str(&format!("        <key>Day</key>\n        <integer>{}</integer>\n", day));
        }

        // Weekday (0-7, where 0 and 7 are Sunday)
        if weekday != "*" {
            let wd = if weekday == "7" { "0" } else { weekday };
            calendar_dict.push_str(&format!("        <key>Weekday</key>\n        <integer>{}</integer>\n", wd));
        }

        // Hour (0-23)
        if hour != "*" {
            calendar_dict.push_str(&format!("        <key>Hour</key>\n        <integer>{}</integer>\n", hour));
        }

        // Minute (0-59)
        if minute != "*" {
            calendar_dict.push_str(&format!("        <key>Minute</key>\n        <integer>{}</integer>\n", minute));
        }

        Ok(calendar_dict)
    }

    fn create_plist(&self, entry: &CronEntry) -> Result<String> {
        // Validate command before creating plist
        self.validate_command(&entry.command)?;

        let label = self.entry_to_label(entry);
        let calendar = self.cron_to_calendar_interval(&entry.schedule)?;

        let plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{}</string>
    <key>CronManagerTaskName</key>
    <string>{}</string>
    <key>ProgramArguments</key>
    <array>
        <string>/bin/sh</string>
        <string>-c</string>
        <string>{}</string>
    </array>
    <key>StartCalendarInterval</key>
    <dict>
{}    </dict>
    <key>StandardOutPath</key>
    <string>{}/{}.stdout</string>
    <key>StandardErrorPath</key>
    <string>{}/{}.stderr</string>
</dict>
</plist>
"#,
            label,
            self.escape_xml(&entry.name),
            self.escape_xml(&entry.command),
            calendar,
            STDOUT_PATH_PREFIX,
            label,
            STDERR_PATH_PREFIX,
            label
        );

        Ok(plist)
    }

    fn load_agent(&self, label: &str) -> Result<()> {
        let plist_path = self.plist_path(label);

        // Use modern bootstrap command (macOS 10.11+)
        // Format: launchctl bootstrap gui/<uid> <plist_path>
        let uid = self.get_uid()?;
        let domain = format!("gui/{}", uid);

        let output = Command::new("launchctl")
            .arg("bootstrap")
            .arg(&domain)
            .arg(&plist_path)
            .output()
            .context("Failed to execute launchctl bootstrap")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            // Only fail if it's not already loaded
            if !error.contains("Already loaded") && !error.contains("service already loaded") {
                anyhow::bail!("Failed to load launch agent {}: {}", label, error);
            }
        }

        Ok(())
    }

    fn unload_agent(&self, label: &str) -> Result<()> {
        let plist_path = self.plist_path(label);

        if !plist_path.exists() {
            return Ok(());
        }

        // Use modern bootout command (macOS 10.11+)
        // Format: launchctl bootout gui/<uid>/<label>
        let uid = self.get_uid()?;
        let service_target = format!("gui/{}/{}", uid, label);

        let _output = Command::new("launchctl")
            .arg("bootout")
            .arg(&service_target)
            .output()
            .context("Failed to execute launchctl bootout")?;

        // Ignore errors on bootout (agent might not be loaded)
        // This is expected behavior when unloading agents that aren't running
        Ok(())
    }

    fn list_agents(&self) -> Result<Vec<String>> {
        let mut labels = Vec::new();

        if !self.launch_agents_dir.exists() {
            return Ok(labels);
        }

        for entry in fs::read_dir(&self.launch_agents_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("plist") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if stem.starts_with(LABEL_PREFIX) {
                        labels.push(stem.to_string());
                    }
                }
            }
        }

        Ok(labels)
    }

    fn parse_plist(&self, path: &PathBuf) -> Result<CronEntry> {
        let content = fs::read_to_string(path)?;

        // Simple XML parsing (we know our own format)
        // Extract name from CronManagerTaskName if available, otherwise from Label
        let name = if let Some(task_name) = self.extract_xml_value(&content, "CronManagerTaskName") {
            self.unescape_xml(&task_name)
        } else {
            // Fallback for old format: extract from label
            let label = self.extract_xml_value(&content, "Label")
                .unwrap_or_else(|| "Unknown".to_string());
            label.strip_prefix(&format!("{}.", LABEL_PREFIX))
                .unwrap_or(&label)
                .split('.')
                .next()
                .unwrap_or(&label)
                .replace('_', " ")
        };

        // Extract command from ProgramArguments (it's the third string, after /bin/sh and -c)
        let command = self.extract_command(&content)
            .unwrap_or_else(|| "".to_string());

        // Extract calendar interval and convert back to cron
        let schedule = self.extract_calendar_to_cron(&content)
            .unwrap_or_else(|| "0 0 * * *".to_string());

        Ok(CronEntry::new(name, schedule, command))
    }

    fn extract_xml_value(&self, content: &str, key: &str) -> Option<String> {
        let key_pattern = format!("<key>{}</key>", key);
        if let Some(pos) = content.find(&key_pattern) {
            let after_key = &content[pos + key_pattern.len()..];
            if let Some(string_start) = after_key.find("<string>") {
                let after_string = &after_key[string_start + 8..];
                if let Some(string_end) = after_string.find("</string>") {
                    return Some(after_string[..string_end].to_string());
                }
            }
        }
        None
    }

    fn extract_command(&self, content: &str) -> Option<String> {
        // Find ProgramArguments array, extract the third string
        if let Some(array_start) = content.find("<key>ProgramArguments</key>") {
            let after_array = &content[array_start..];

            // Count <string> tags and get the third one
            let mut count = 0;
            let mut pos = 0;

            while let Some(string_start) = after_array[pos..].find("<string>") {
                count += 1;
                pos += string_start + 8;

                if count == 3 {
                    if let Some(string_end) = after_array[pos..].find("</string>") {
                        let cmd = &after_array[pos..pos + string_end];
                        // Decode XML entities
                        return Some(self.unescape_xml(cmd));
                    }
                }
            }
        }
        None
    }

    fn extract_calendar_to_cron(&self, content: &str) -> Option<String> {
        // Extract calendar values
        let minute = self.extract_calendar_value(content, "Minute").unwrap_or("*".to_string());
        let hour = self.extract_calendar_value(content, "Hour").unwrap_or("*".to_string());
        let day = self.extract_calendar_value(content, "Day").unwrap_or("*".to_string());
        let month = self.extract_calendar_value(content, "Month").unwrap_or("*".to_string());
        let weekday = self.extract_calendar_value(content, "Weekday").unwrap_or("*".to_string());

        Some(format!("{} {} {} {} {}", minute, hour, day, month, weekday))
    }

    fn extract_calendar_value(&self, content: &str, key: &str) -> Option<String> {
        let key_pattern = format!("<key>{}</key>", key);
        if let Some(pos) = content.find(&key_pattern) {
            let after_key = &content[pos + key_pattern.len()..];
            if let Some(int_start) = after_key.find("<integer>") {
                let after_int = &after_key[int_start + 9..];
                if let Some(int_end) = after_int.find("</integer>") {
                    return Some(after_int[..int_end].to_string());
                }
            }
        }
        None
    }
}

impl Scheduler for LaunchdScheduler {
    fn load(&self) -> Result<Vec<CronEntry>> {
        let mut entries = Vec::new();

        let labels = self.list_agents()?;

        for label in labels {
            let plist_path = self.plist_path(&label);
            match self.parse_plist(&plist_path) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to parse plist for label '{}': {}",
                        label, e
                    );
                }
            }
        }

        Ok(entries)
    }

    fn save(&self, entries: &[CronEntry]) -> Result<()> {
        self.ensure_launch_agents_dir()?;

        // Phase 1: Create all new plist files with .new suffix
        let mut new_plists = Vec::new();
        for entry in entries {
            if entry.enabled {
                let plist_content = self.create_plist(entry)?;
                let label = self.entry_to_label(entry);
                let plist_path = self.plist_path(&label);
                let temp_plist_path = plist_path.with_extension("plist.new");

                fs::write(&temp_plist_path, plist_content)
                    .with_context(|| format!("Failed to write temp plist: {:?}", temp_plist_path))?;

                new_plists.push((label, plist_path, temp_plist_path));
            }
        }

        // Phase 2: Get list of existing agents and unload them
        let existing_labels = self.list_agents()?;
        for label in &existing_labels {
            self.unload_agent(label)?;
        }

        // Phase 3: Remove old plist files
        for label in &existing_labels {
            let plist_path = self.plist_path(label);
            if plist_path.exists() {
                fs::remove_file(&plist_path)
                    .with_context(|| format!("Failed to remove old plist: {:?}", plist_path))?;
            }
        }

        // Phase 4: Atomically rename new plist files
        for (_label, plist_path, temp_plist_path) in &new_plists {
            fs::rename(temp_plist_path, plist_path)
                .with_context(|| format!("Failed to rename plist: {:?} -> {:?}", temp_plist_path, plist_path))?;
        }

        // Phase 5: Load all new agents
        for (label, _plist_path, _temp_plist_path) in &new_plists {
            if let Err(e) = self.load_agent(label) {
                eprintln!("Warning: Failed to load agent '{}': {}", label, e);
            }
        }

        Ok(())
    }

    fn backend_name(&self) -> &'static str {
        "Launchd"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_launchd_scheduler_backend_name() {
        let scheduler = LaunchdScheduler::new();
        assert_eq!(scheduler.backend_name(), "Launchd");
    }

    #[test]
    fn test_xml_escape_and_unescape_roundtrip() {
        let scheduler = LaunchdScheduler::new();

        let test_strings = vec![
            "simple text",
            "text with & ampersand",
            "text with < less than",
            "text with > greater than",
            "text with 'apostrophe'",
            "text with \"quotes\"",
            "&<>\"'",
            "complex & text < with > all 'special' \"characters\"",
        ];

        for original in test_strings {
            let escaped = scheduler.escape_xml(original);
            let unescaped = scheduler.unescape_xml(&escaped);
            assert_eq!(
                original, unescaped,
                "Roundtrip failed for: {}",
                original
            );
        }
    }

    #[test]
    fn test_entry_to_label_uniqueness() {
        let scheduler = LaunchdScheduler::new();

        // Different names should generate different labels
        let entry1 = CronEntry::new("My Task".to_string(), "0 0 * * *".to_string(), "echo".to_string());
        let entry2 = CronEntry::new("My/Task".to_string(), "0 0 * * *".to_string(), "echo".to_string());
        let entry3 = CronEntry::new("My_Task".to_string(), "0 0 * * *".to_string(), "echo".to_string());

        let label1 = scheduler.entry_to_label(&entry1);
        let label2 = scheduler.entry_to_label(&entry2);
        let label3 = scheduler.entry_to_label(&entry3);

        // All labels should be different
        assert_ne!(label1, label2, "Labels should be unique for different names");
        assert_ne!(label1, label3, "Labels should be unique for different names");
        assert_ne!(label2, label3, "Labels should be unique for different names");

        // All labels should start with the prefix
        assert!(label1.starts_with(LABEL_PREFIX));
        assert!(label2.starts_with(LABEL_PREFIX));
        assert!(label3.starts_with(LABEL_PREFIX));
    }

    #[test]
    fn test_entry_to_label_consistency() {
        let scheduler = LaunchdScheduler::new();

        // Same entry should always generate the same label
        let entry = CronEntry::new("Test Task".to_string(), "0 0 * * *".to_string(), "echo".to_string());

        let label1 = scheduler.entry_to_label(&entry);
        let label2 = scheduler.entry_to_label(&entry);

        assert_eq!(label1, label2, "Same entry should generate consistent labels");
    }

    #[test]
    fn test_validate_command_empty() {
        let scheduler = LaunchdScheduler::new();

        let result = scheduler.validate_command("");
        assert!(result.is_err(), "Empty command should fail validation");

        let result = scheduler.validate_command("   ");
        assert!(result.is_err(), "Whitespace-only command should fail validation");
    }

    #[test]
    fn test_validate_command_simple() {
        let scheduler = LaunchdScheduler::new();

        let result = scheduler.validate_command("echo hello");
        assert!(result.is_ok(), "Simple command should pass validation");
    }

    #[test]
    fn test_cron_to_calendar_valid_expressions() {
        let scheduler = LaunchdScheduler::new();

        // Test simple valid expressions
        let result = scheduler.cron_to_calendar_interval("0 0 * * *");
        assert!(result.is_ok(), "Simple cron expression should be valid");

        let result = scheduler.cron_to_calendar_interval("30 14 * * *");
        assert!(result.is_ok(), "Cron with specific hour and minute should be valid");

        let result = scheduler.cron_to_calendar_interval("0 0 1 1 *");
        assert!(result.is_ok(), "Cron with specific day and month should be valid");
    }

    #[test]
    fn test_cron_to_calendar_invalid_expressions() {
        let scheduler = LaunchdScheduler::new();

        // Test unsupported expressions
        let result = scheduler.cron_to_calendar_interval("0-30 * * * *");
        assert!(result.is_err(), "Range expression should be rejected");
        assert!(result.unwrap_err().to_string().contains("range"));

        let result = scheduler.cron_to_calendar_interval("0,30 * * * *");
        assert!(result.is_err(), "List expression should be rejected");
        assert!(result.unwrap_err().to_string().contains("list"));

        let result = scheduler.cron_to_calendar_interval("*/15 * * * *");
        assert!(result.is_err(), "Step value expression should be rejected");
        assert!(result.unwrap_err().to_string().contains("step"));

        let result = scheduler.cron_to_calendar_interval("@daily");
        assert!(result.is_err(), "Special syntax should be rejected");
        assert!(result.unwrap_err().to_string().contains("special"));

        let result = scheduler.cron_to_calendar_interval("invalid * * * *");
        assert!(result.is_err(), "Invalid value should be rejected");

        let result = scheduler.cron_to_calendar_interval("0 0 * *");
        assert!(result.is_err(), "Incomplete expression should be rejected");
    }

    #[test]
    fn test_cron_to_calendar_minute_hour() {
        let scheduler = LaunchdScheduler::new();

        // Test specific minute and hour
        let result = scheduler.cron_to_calendar_interval("30 14 * * *");
        assert!(result.is_ok());
        let calendar = result.unwrap();

        assert!(calendar.contains("<key>Minute</key>"));
        assert!(calendar.contains("<integer>30</integer>"));
        assert!(calendar.contains("<key>Hour</key>"));
        assert!(calendar.contains("<integer>14</integer>"));
    }

    #[test]
    fn test_create_plist_validates_command() {
        let scheduler = LaunchdScheduler::new();

        // Empty command should fail
        let entry = CronEntry::new("test".to_string(), "0 0 * * *".to_string(), "".to_string());
        let result = scheduler.create_plist(&entry);
        assert!(result.is_err(), "Empty command should fail");

        // Invalid cron should fail
        let entry = CronEntry::new("test".to_string(), "invalid".to_string(), "echo test".to_string());
        let result = scheduler.create_plist(&entry);
        assert!(result.is_err(), "Invalid cron expression should fail");
    }
}
