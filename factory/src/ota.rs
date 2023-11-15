use crate::sdcard::Manager;
use crate::FactoryError;
use core::ptr;
use embedded_sdmmc::{Error::FileNotFound, Mode, VolumeIdx};
use esp_idf_svc::{
    ota::EspOta,
    sys::{esp, esp_ota_get_next_update_partition, esp_ota_set_boot_partition},
};

const FILE: &str = "update.bin";
const BUFFER_LEN: usize = 1024;

pub(crate) fn update_present(volume_mgr: &mut Manager) -> Result<bool, FactoryError> {
    let volume0 = volume_mgr
        .get_volume(VolumeIdx(0))
        .map_err(FactoryError::SdCard)?;
    let root_dir = volume_mgr
        .open_root_dir(&volume0)
        .map_err(FactoryError::SdCard)?;
    let ret = match volume_mgr.find_directory_entry(&volume0, &root_dir, FILE) {
        Ok(_) => Ok(true),
        Err(FileNotFound) => Ok(false),
        Err(e) => Err(FactoryError::SdCard(e)),
    };
    volume_mgr.close_dir(&volume0, root_dir);
    ret
}

pub(crate) fn write_update(volume_mgr: &mut Manager) -> Result<(), FactoryError> {
    let mut volume0 = volume_mgr
        .get_volume(VolumeIdx(0))
        .map_err(FactoryError::SdCard)?;
    let root_dir = volume_mgr
        .open_root_dir(&volume0)
        .map_err(FactoryError::SdCard)?;
    let mut my_file = volume_mgr
        .open_file_in_dir(&mut volume0, &root_dir, FILE, Mode::ReadOnly)
        .map_err(FactoryError::SdCard)?;

    let mut ota = EspOta::new().map_err(FactoryError::Ota)?;
    let mut ota = ota.initiate_update().map_err(FactoryError::Ota)?;

    let mut buffer = [0u8; BUFFER_LEN];
    while !my_file.eof() {
        let r = volume_mgr
            .read(&volume0, &mut my_file, &mut buffer)
            .map_err(FactoryError::SdCard)?;
        ota.write(&buffer[..r]).map_err(FactoryError::Ota)?;
    }

    ota.complete().map_err(FactoryError::Ota)?;

    volume_mgr
        .close_file(&volume0, my_file)
        .map_err(FactoryError::SdCard)?;
    volume_mgr.close_dir(&volume0, root_dir);
    Ok(())
}

pub(crate) fn set_boot_main_app() -> Result<(), FactoryError> {
    esp!(unsafe {
        let partition = esp_ota_get_next_update_partition(ptr::null());
        esp_ota_set_boot_partition(partition)
    })
    .map_err(FactoryError::Ota)
}
