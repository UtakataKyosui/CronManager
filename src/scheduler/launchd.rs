use crate::cron_entry::CronEntry;
use crate::scheduler::Scheduler;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

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
        // Create a unique label for this entry
        // Replace spaces and special characters with underscores
        let safe_name = entry.name.replace(' ', "_").replace('/', "_");
        format!("com.cronmanager.{}", safe_name)
    }

    fn plist_path(&self, label: &str) -> PathBuf {
        self.launch_agents_dir.join(format!("{}.plist", label))
    }

    fn cron_to_calendar_interval(&self, schedule: &str) -> Result<String> {
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
        let label = self.entry_to_label(entry);
        let calendar = self.cron_to_calendar_interval(&entry.schedule)?;

        // Split command into program and arguments
        let cmd_parts: Vec<&str> = entry.command.split_whitespace().collect();
        let program = cmd_parts.get(0).unwrap_or(&"");
        let args = &cmd_parts[1..];

        let mut plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
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
    <string>/tmp/{}.stdout</string>
    <key>StandardErrorPath</key>
    <string>/tmp/{}.stderr</string>
</dict>
</plist>
"#,
            label,
            entry.command.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;"),
            calendar,
            label,
            label
        );

        Ok(plist)
    }

    fn load_agent(&self, label: &str) -> Result<()> {
        let plist_path = self.plist_path(label);

        let output = Command::new("launchctl")
            .arg("load")
            .arg(&plist_path)
            .output()
            .context("Failed to execute launchctl load")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            eprintln!("Warning: Failed to load launch agent {}: {}", label, error);
        }

        Ok(())
    }

    fn unload_agent(&self, label: &str) -> Result<()> {
        let plist_path = self.plist_path(label);

        if !plist_path.exists() {
            return Ok(());
        }

        let output = Command::new("launchctl")
            .arg("unload")
            .arg(&plist_path)
            .output()
            .context("Failed to execute launchctl unload")?;

        // Ignore errors on unload (agent might not be loaded)
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
                    if stem.starts_with("com.cronmanager.") {
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
        // Extract label
        let label = self.extract_xml_value(&content, "Label")
            .unwrap_or_else(|| "Unknown".to_string());

        // Extract command from ProgramArguments (it's the third string, after /bin/sh and -c)
        let command = self.extract_command(&content)
            .unwrap_or_else(|| "".to_string());

        // Extract calendar interval and convert back to cron
        let schedule = self.extract_calendar_to_cron(&content)
            .unwrap_or_else(|| "0 0 * * *".to_string());

        // Extract name from label
        let name = label.strip_prefix("com.cronmanager.")
            .unwrap_or(&label)
            .replace('_', " ");

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
                        return Some(cmd.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">"));
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
            if let Ok(entry) = self.parse_plist(&plist_path) {
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    fn save(&self, entries: &[CronEntry]) -> Result<()> {
        self.ensure_launch_agents_dir()?;

        // Get list of existing agents managed by us
        let existing_labels = self.list_agents()?;

        // Unload and remove all existing agents
        for label in existing_labels {
            self.unload_agent(&label)?;
            let plist_path = self.plist_path(&label);
            if plist_path.exists() {
                fs::remove_file(&plist_path)?;
            }
        }

        // Create and load new agents for enabled entries
        for entry in entries {
            if entry.enabled {
                let plist_content = self.create_plist(entry)?;
                let label = self.entry_to_label(entry);
                let plist_path = self.plist_path(&label);

                fs::write(&plist_path, plist_content)
                    .with_context(|| format!("Failed to write plist: {:?}", plist_path))?;

                self.load_agent(&label)?;
            }
        }

        Ok(())
    }

    fn backend_name(&self) -> &'static str {
        "Launchd"
    }
}
