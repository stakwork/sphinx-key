use bitflags::bitflags;
use esp_idf_sys::c_types::c_char;
use esp_idf_sys::{
    esp, esp_vfs_fat_sdmmc_mount_config_t, esp_vfs_fat_sdspi_mount, gpio_num_t, sdmmc_card_t,
    sdmmc_host_t, sdspi_device_config_t, spi_bus_config_t, spi_bus_initialize, spi_host_device_t,
    spi_host_device_t_SPI2_HOST,
};
use std::fs;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Read;
use std::io::Write;
use std::ptr;
use std::string::String;

const C_MOUNT_POINT: &'static [u8] = b"/sdcard\0";

const SPI_HOST_SLOT: spi_host_device_t = spi_host_device_t_SPI2_HOST;
const SPI_GPIO_MOSI: gpio_num_t = 4;
const SPI_GPIO_CLK: gpio_num_t = 5;
const SPI_GPIO_MISO: gpio_num_t = 6;
const SPI_GPIO_CS: gpio_num_t = 1;

bitflags! {
    struct SDMMCHostFlag: u32 {
        /// host supports 1-line SD and MMC protocol
        const BIT1 = 1 << 0;
        /// host supports 4-line SD and MMC protocol
        const BIT4 = 1 << 1;
        /// host supports 8-line MMC protocol
        const BIT8 = 1 << 2;
        /// host supports SPI protocol
        const SPI = 1 << 3;
        /// host supports DDR mode for SD/MMC
        const DDR = 1 << 4;
        /// host `deinit` function called with the slot argument
        const DEINIT_ARG = 1 << 5;
    }
}

#[allow(dead_code)]
enum SDMMCFreq {
    /// SD/MMC Default speed (limited by clock divider)
    Default = 20000,
    /// SD High speed (limited by clock divider)
    HighSPeed = 40000,
    /// SD/MMC probing speed
    Probing = 400,
    /// MMC 52MHz speed
    _52M = 52000,
    /// MMC 26MHz speed
    _26M = 26000,
}

pub fn sd_card() {
    let mount_config = esp_vfs_fat_sdmmc_mount_config_t {
        format_if_mount_failed: false,
        max_files: 5,
        allocation_unit_size: 16 * 1024,
    };

    let mut card: *mut sdmmc_card_t = ptr::null_mut();

    let bus_cfg = spi_bus_config_t {
        __bindgen_anon_1: esp_idf_sys::spi_bus_config_t__bindgen_ty_1 {
            mosi_io_num: SPI_GPIO_MOSI,
        },
        __bindgen_anon_2: esp_idf_sys::spi_bus_config_t__bindgen_ty_2 {
            miso_io_num: SPI_GPIO_MISO,
        },
        sclk_io_num: SPI_GPIO_CLK,
        __bindgen_anon_3: esp_idf_sys::spi_bus_config_t__bindgen_ty_3 { quadwp_io_num: -1 },
        __bindgen_anon_4: esp_idf_sys::spi_bus_config_t__bindgen_ty_4 { quadhd_io_num: -1 },
        data4_io_num: -1,
        data5_io_num: -1,
        data6_io_num: -1,
        data7_io_num: -1,
        max_transfer_sz: 4000,
        flags: 0,
        intr_flags: 0,
    };

    let res = esp!(unsafe {
        spi_bus_initialize(
            SPI_HOST_SLOT as u32,
            &bus_cfg,
            esp_idf_sys::spi_common_dma_t_SPI_DMA_CH_AUTO,
        )
    });

    match res {
        Ok(_) => (),
        Err(e) => {
            println!("Failed to initialize SPI Bus: {}", e);
        }
    }

    println!("Initialized SPI BUS!");

    let slot_config = sdspi_device_config_t {
        host_id: SPI_HOST_SLOT,
        gpio_cs: SPI_GPIO_CS,
        gpio_cd: -1,
        gpio_wp: -1,
        gpio_int: -1,
    };

    let host = sdmmc_host_t {
        flags: (SDMMCHostFlag::SPI | SDMMCHostFlag::DEINIT_ARG).bits, //SDMMC_HOST_FLAG_SPI | SDMMC_HOST_FLAG_DEINIT_ARG,
        slot: SPI_HOST_SLOT as i32,
        max_freq_khz: SDMMCFreq::Default as i32, //SDMMC_FREQ_DEFAULT,
        io_voltage: 3.3f32,
        init: Some(esp_idf_sys::sdspi_host_init),
        set_bus_width: None,
        get_bus_width: None,
        set_bus_ddr_mode: None,
        set_card_clk: Some(esp_idf_sys::sdspi_host_set_card_clk),
        do_transaction: Some(esp_idf_sys::sdspi_host_do_transaction),
        __bindgen_anon_1: esp_idf_sys::sdmmc_host_t__bindgen_ty_1 {
            deinit_p: Some(esp_idf_sys::sdspi_host_remove_device),
        },
        io_int_enable: Some(esp_idf_sys::sdspi_host_io_int_enable),
        io_int_wait: Some(esp_idf_sys::sdspi_host_io_int_wait),
        command_timeout_ms: 0,
    };

    let res = esp!(unsafe {
        esp_vfs_fat_sdspi_mount(
            C_MOUNT_POINT.as_ptr() as *const c_char,
            &host,
            &slot_config,
            &mount_config,
            &mut card as *mut *mut sdmmc_card_t,
        )
    });

    match res {
        Ok(_) => (),
        Err(e) => {
            println!("Failed to mount filesystem: {}", e);
        }
    }
}

pub fn simple_fs_test() {
    // Create and write a file, append a file, read a file, delete a file, create a directory, write a file

    let file = fs::File::create("/sdcard/sphinx.txt").expect("create failed");
    let mut writer = BufWriter::new(file);
    writer
        .write_all("Hello World".as_bytes())
        .expect("write failed");
    writer
        .write_all("\nWelcome to your own network".as_bytes())
        .expect("write failed");
    writer.flush().expect("Problem flushing");
    drop(writer);
    println!("Data written to file");

    let file = fs::OpenOptions::new()
        .append(true)
        .open("/sdcard/sphinx.txt")
        .expect("cannot open file");
    let mut writer = BufWriter::new(file);
    writer
        .write_all("\nDepeche Mode".as_bytes())
        .expect("write failed");
    writer
        .write_all("\nWelcome to my World".as_bytes())
        .expect("write failed");
    writer.flush().expect("Problem flushing");
    drop(writer);
    println!("File append success");

    let file = fs::File::open("/sdcard/sphinx.txt").expect("Cannot open file");
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader
        .read_to_string(&mut contents)
        .expect("could not read file");
    drop(reader);

    fs::remove_file("/sdcard/sphinx.txt").expect("could not remove file");
    println!("File is removed");

    let _ = fs::create_dir("/sdcard/data");

    let file = fs::File::create("/sdcard/data/sphinx.txt").expect("Cannot open file");
    let mut writer = BufWriter::new(file);
    writer
        .write_all(contents.as_bytes())
        .expect("Could not write");
    writer.flush().expect("Problem flushing");
    drop(writer);
    println!("Wrote file in data directory");
    println!("Simple fs test complete!");
}
