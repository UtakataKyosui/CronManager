use cron_manager::storage::Storage;

fn main() {
    println!("Testing crontab import...\n");

    // Remove existing file to test import
    let home = dirs::home_dir().unwrap();
    let cron_file = home.join(".cron-manager-crontab");
    if cron_file.exists() {
        std::fs::remove_file(&cron_file).ok();
        println!("Removed existing {}", cron_file.display());
    }

    // Create storage and load (should import from system crontab)
    let storage = Storage::new(None);
    match storage.load() {
        Ok(entries) => {
            println!("Successfully loaded {} entries:\n", entries.len());
            for (i, entry) in entries.iter().enumerate() {
                println!("{}. Name: {}", i + 1, entry.name);
                println!("   Schedule: {}", entry.schedule);
                println!("   Command: {}", entry.command);
                println!("   Enabled: {}\n", entry.enabled);
            }

            // Check if file was created
            if cron_file.exists() {
                println!("✓ Local file created at: {}", cron_file.display());
            } else {
                println!("✗ Local file NOT created");
            }
        }
        Err(e) => {
            eprintln!("Error loading crontab: {}", e);
        }
    }
}
