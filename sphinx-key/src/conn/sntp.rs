use esp_idf_svc::sntp::EspSntp;
use esp_idf_svc::sntp::SyncStatus::Completed;
use std::thread;
use std::time::Duration;

pub fn sync_time() {
    let sntp = EspSntp::new_default().unwrap();
    loop {
        let status = sntp.get_sync_status();
        println!("SNTP status {:?}", status);
        if status == Completed {
            break;
        }
        thread::sleep(Duration::from_secs(1));
    }
}
