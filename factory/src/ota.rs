use anyhow::Result;
use embedded_svc::io::Write;
use embedded_svc::ota::Ota;
use embedded_svc::ota::OtaUpdate;
use esp_idf_svc::ota::EspOta;
use esp_idf_sys::{esp, esp_ota_get_next_update_partition, esp_ota_set_boot_partition};
use log::info;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::ptr;

pub const UPDATE_BIN_PATH: &str = "/sdcard/update.bin";
const BUFFER_LEN: usize = 1024;

pub fn run_sdcard_ota_update() -> Result<()> {
    let f = File::open(UPDATE_BIN_PATH)?;
    let mut reader = BufReader::with_capacity(BUFFER_LEN, f);

    let mut ota = EspOta::new()?;
    let mut ota = ota.initiate_update()?;

    let mut buf = [0_u8; BUFFER_LEN];
    let mut read_tot: usize = 0;
    let mut write_tot: usize = 0;
    let mut i = 0;
    loop {
        let r = reader.read(&mut buf)?;
        if r == 0 {
            break;
        }
        let w = ota.write(&buf[..r])?;
        read_tot += r;
        write_tot += w;
        i += 1;
        if i % 20 == 0 {
            info!("Cumulative bytes read: {}", read_tot);
            info!("Cumulative bytes written: {}", write_tot);
        }
    }
    info!("TOTAL read: {}", read_tot);
    info!("TOTAL write: {}", write_tot);
    ota.complete()?;
    Ok(())
}

pub fn set_boot_main_app() {
    let partition = unsafe { esp_ota_get_next_update_partition(ptr::null()) };
    esp!(unsafe { esp_ota_set_boot_partition(partition) })
        .expect("Couldn't set next boot partition...");
}
