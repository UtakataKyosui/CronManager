use crate::cron_entry::CronEntry;
use crate::storage::Storage;
use anyhow::Result;

#[derive(Debug, PartialEq)]
pub enum InputMode {
    Normal,
    AddingName,
    AddingSchedule,
    AddingCommand,
    EditingName,
    EditingSchedule,
    EditingCommand,
}

pub struct App {
    pub entries: Vec<CronEntry>,
    pub selected_index: usize,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub storage: Storage,
    pub message: Option<String>,
    pub should_quit: bool,
    // Temporary state for adding new entries
    temp_name: String,
    temp_schedule: String,
}

impl App {
    pub fn new(storage: Storage) -> Result<Self> {
        let entries = storage.load()?;
        Ok(Self {
            entries,
            selected_index: 0,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            storage,
            message: None,
            should_quit: false,
            temp_name: String::new(),
            temp_schedule: String::new(),
        })
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_selection_down(&mut self) {
        if !self.entries.is_empty() && self.selected_index < self.entries.len() - 1 {
            self.selected_index += 1;
        }
    }

    pub fn start_add_entry(&mut self) {
        self.input_mode = InputMode::AddingName;
        self.input_buffer.clear();
        self.message = Some("Enter name for new cron entry:".to_string());
    }

    pub fn start_edit_name(&mut self) {
        if let Some(entry) = self.entries.get(self.selected_index) {
            self.input_mode = InputMode::EditingName;
            self.input_buffer = entry.name.clone();
            self.message = Some("Edit name:".to_string());
        }
    }

    pub fn start_edit_schedule(&mut self) {
        if let Some(entry) = self.entries.get(self.selected_index) {
            self.input_mode = InputMode::EditingSchedule;
            self.input_buffer = entry.schedule.clone();
            self.message = Some("Edit schedule (cron format):".to_string());
        }
    }

    pub fn start_edit_command(&mut self) {
        if let Some(entry) = self.entries.get(self.selected_index) {
            self.input_mode = InputMode::EditingCommand;
            self.input_buffer = entry.command.clone();
            self.message = Some("Edit command:".to_string());
        }
    }

    pub fn delete_entry(&mut self) -> Result<()> {
        if !self.entries.is_empty() && self.selected_index < self.entries.len() {
            self.entries.remove(self.selected_index);
            if self.selected_index > 0 && self.selected_index >= self.entries.len() {
                self.selected_index -= 1;
            }
            self.save()?;
            self.message = Some("Entry deleted".to_string());
        }
        Ok(())
    }

    pub fn toggle_enabled(&mut self) -> Result<()> {
        if let Some(entry) = self.entries.get_mut(self.selected_index) {
            entry.enabled = !entry.enabled;
        }
        self.save()?;
        if let Some(entry) = self.entries.get(self.selected_index) {
            self.message = Some(format!(
                "Entry {} {}",
                entry.name,
                if entry.enabled { "enabled" } else { "disabled" }
            ));
        }
        Ok(())
    }

    pub fn handle_input_char(&mut self, c: char) {
        self.input_buffer.push(c);
    }

    pub fn handle_input_backspace(&mut self) {
        self.input_buffer.pop();
    }

    pub fn confirm_input(&mut self) -> Result<()> {
        match self.input_mode {
            InputMode::AddingName => {
                if !self.input_buffer.is_empty() {
                    self.temp_name = self.input_buffer.clone();
                    self.input_buffer.clear();
                    self.input_mode = InputMode::AddingSchedule;
                    self.message = Some(format!("Name: {} | Enter schedule (cron format):", self.temp_name));
                }
            }
            InputMode::AddingSchedule => {
                if !self.input_buffer.is_empty() {
                    self.temp_schedule = self.input_buffer.clone();
                    self.input_buffer.clear();
                    self.input_mode = InputMode::AddingCommand;
                    self.message = Some(format!("Name: {} | Schedule: {} | Enter command:", self.temp_name, self.temp_schedule));
                }
            }
            InputMode::AddingCommand => {
                if !self.input_buffer.is_empty() {
                    self.finish_add_entry()?;
                }
            }
            InputMode::EditingName => {
                if let Some(entry) = self.entries.get_mut(self.selected_index) {
                    entry.name = self.input_buffer.clone();
                    self.save()?;
                    self.input_mode = InputMode::Normal;
                    self.input_buffer.clear();
                    self.message = Some("Name updated".to_string());
                }
            }
            InputMode::EditingSchedule => {
                if let Some(entry) = self.entries.get_mut(self.selected_index) {
                    entry.schedule = self.input_buffer.clone();
                    if !entry.validate_schedule() {
                        self.message = Some("Warning: Invalid cron schedule format".to_string());
                    }
                    self.save()?;
                    self.input_mode = InputMode::Normal;
                    self.input_buffer.clear();
                    self.message = Some("Schedule updated".to_string());
                }
            }
            InputMode::EditingCommand => {
                if let Some(entry) = self.entries.get_mut(self.selected_index) {
                    entry.command = self.input_buffer.clone();
                    self.save()?;
                    self.input_mode = InputMode::Normal;
                    self.input_buffer.clear();
                    self.message = Some("Command updated".to_string());
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn finish_add_entry(&mut self) -> Result<()> {
        let command = self.input_buffer.clone();
        let entry = CronEntry::new(
            self.temp_name.clone(),
            self.temp_schedule.clone(),
            command,
        );

        if !entry.validate_schedule() {
            self.message = Some("Warning: Invalid cron schedule format. Entry still added.".to_string());
        } else {
            self.message = Some("Entry added successfully".to_string());
        }

        self.entries.push(entry);
        self.save()?;
        self.input_mode = InputMode::Normal;
        self.input_buffer.clear();
        self.temp_name.clear();
        self.temp_schedule.clear();
        Ok(())
    }

    pub fn cancel_input(&mut self) {
        self.input_mode = InputMode::Normal;
        self.input_buffer.clear();
        self.temp_name.clear();
        self.temp_schedule.clear();
        self.message = Some("Cancelled".to_string());
    }

    pub fn save(&mut self) -> Result<()> {
        self.storage.save(&self.entries)?;
        Ok(())
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}
