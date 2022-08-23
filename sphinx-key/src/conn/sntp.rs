use esp_idf_svc::sntp::EspSntp;
use esp_idf_svc::sntp::SyncStatus::Completed;
use std::time::Duration;
use std::thread;

pub fn sync_time() {
    let sntp = EspSntp::new_default().unwrap();
    println!("SNTP initialized");
    while sntp.get_sync_status() != Completed {
            println!("Waiting for sntp sync...");
            thread::sleep(Duration::from_secs(1));
    }
}
