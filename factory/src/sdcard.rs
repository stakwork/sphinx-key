use crate::FactoryError;
use embedded_sdmmc::{SdCard, TimeSource, Timestamp, VolumeManager};
use esp_idf_svc::hal::{
    delay::Ets,
    gpio::{Gpio10, Gpio2, Gpio6, Gpio7, Output, PinDriver},
    spi::{
        config::{DriverConfig, Duplex},
        SpiConfig, SpiDeviceDriver, SpiDriver, SPI2,
    },
    units::FromValueType,
};

pub(crate) type Manager<'a> = VolumeManager<
    SdCard<SpiDeviceDriver<'a, SpiDriver<'a>>, PinDriver<'a, Gpio10, Output>, Ets>,
    SdMmcClock,
>;

pub(crate) struct Peripherals {
    pub spi: SPI2,
    pub sck: Gpio6,
    pub mosi: Gpio7,
    pub miso: Gpio2,
    pub cs: Gpio10,
}

pub(crate) struct SdMmcClock;

impl TimeSource for SdMmcClock {
    fn get_timestamp(&self) -> Timestamp {
        Timestamp {
            year_since_1970: 0,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}

pub(crate) fn setup(peripherals: Peripherals) -> Result<Manager<'static>, FactoryError> {
    let driver = SpiDriver::new(
        peripherals.spi,
        peripherals.sck,
        peripherals.mosi,
        Some(peripherals.miso),
        &DriverConfig::default(),
    )
    .map_err(FactoryError::Esp)?;
    let mut spi_config = SpiConfig::new();
    spi_config.duplex = Duplex::Full;
    spi_config = spi_config.baudrate(24.MHz().into());
    let spi = SpiDeviceDriver::new(driver, Option::<Gpio10>::None, &spi_config)
        .map_err(FactoryError::Esp)?;
    let sdmmc_cs = PinDriver::output(peripherals.cs).map_err(FactoryError::Esp)?;
    let sdcard = SdCard::new(spi, sdmmc_cs, Ets {});
    let volume_mgr = VolumeManager::new(sdcard, SdMmcClock {});
    Ok(volume_mgr)
}
