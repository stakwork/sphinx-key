use rocket::tokio;
use std::io::Write;
use std::{env, fs};

const DEFAULT_ERROR_LOG_PATH: &str = "/root/.lightning/broker_errors.log";

pub fn log_errors(
    mut error_rx: tokio::sync::broadcast::Receiver<Vec<u8>>,
    task_set: &mut tokio::task::JoinSet<()>,
) {
    // collect errors
    task_set.spawn(async move {
        let err_log_path =
            env::var("BROKER_ERROR_LOG_PATH").unwrap_or(DEFAULT_ERROR_LOG_PATH.to_string());
        if let Ok(mut file) = fs::OpenOptions::new()
            .create(true) // create if doesn't exist
            .append(true)
            .open(err_log_path)
        {
            while let Ok(err_msg) = error_rx.recv().await {
                let mut log = format!("[{}]: ", chrono::Utc::now()).as_bytes().to_vec();
                log.extend_from_slice(&err_msg);
                log.extend_from_slice(b"\n");
                if let Err(e) = file.write_all(&log) {
                    log::warn!("failed to write error to log {:?}", e);
                }
            }
        } else {
            log::warn!("FAILED TO OPEN ERROR LOG FILE");
        }
    });
}
