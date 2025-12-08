use cron_manager::cron_entry::CronEntry;
use cron_manager::storage::Storage;

fn main() {
    println!("=== Testing System Scheduler Integration ===\n");

    // 1. Load current entries
    println!("1. Loading current system scheduler...");
    let mut storage = Storage::with_system_scheduler();
    let mut entries = storage.load().expect("Failed to load scheduler");
    println!("   Current entries: {}\n", entries.len());

    // 2. Add a new test entry
    println!("2. Adding test entry...");
    let test_entry = CronEntry::new(
        "Test Entry from App".to_string(),
        "0 * * * *".to_string(), // Every hour (simplified for cross-platform compatibility)
        "/usr/bin/test_command.sh".to_string(),
    );
    entries.push(test_entry);
    storage.save(&entries).expect("Failed to save");
    println!("   Added: 'Test Entry from App'\n");

    // 3. Verify it was saved to system
    println!("3. Verifying system scheduler was updated...");
    // Note: Verification depends on the scheduler backend being used
    println!("   (Verification step - check system scheduler manually)");

    // 4. Reload and verify
    println!("\n4. Reloading to verify...");
    let reloaded = storage.load().expect("Failed to reload");
    println!("   Entries after reload: {}", reloaded.len());

    let found = reloaded.iter().find(|e| e.name == "Test Entry from App");
    if found.is_some() {
        println!("   ✓ Test entry found after reload");
    } else {
        println!("   ✗ Test entry NOT found after reload");
    }

    // 5. Remove the test entry
    println!("\n5. Removing test entry...");
    let entries: Vec<_> = reloaded
        .into_iter()
        .filter(|e| e.name != "Test Entry from App")
        .collect();
    storage.save(&entries).expect("Failed to save after removal");
    println!("   Removed: 'Test Entry from App'\n");

    // 6. Final verification
    println!("6. Final verification...");
    let final_entries = storage.load().expect("Failed to final load");
    println!("   Final entry count: {}", final_entries.len());

    let still_there = final_entries.iter().find(|e| e.name == "Test Entry from App");
    if still_there.is_none() {
        println!("   ✓ Test entry successfully removed");
    } else {
        println!("   ✗ Test entry still present");
    }

    println!("\n=== Test Complete ===");
}
