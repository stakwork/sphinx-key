use anyhow::{anyhow, Result};
use esp_idf_svc::sntp::EspSntp;
use esp_idf_svc::sntp::SyncStatus::Completed;
use std::thread;
use std::time::Duration;

pub fn sync_time_timeout() -> Result<()> {
    let mut counter = 0;
    let sntp = EspSntp::new_default()?;
    loop {
        let status = sntp.get_sync_status();
        println!("SNTP status {:?}", status);
        if status == Completed {
            break Ok(());
        } else if counter == 30 {
            break Err(anyhow!("SNTP setup timed out"));
        }
        counter = counter + 1;
        thread::sleep(Duration::from_secs(1));
    }
}
