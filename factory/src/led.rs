use embedded_hal::blocking::delay::DelayMs;
use esp_idf_hal::delay::Ets;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::rmt::config::TransmitConfig;
use esp_idf_hal::rmt::*;
use std::time::Duration;

pub fn set_ota_led() -> anyhow::Result<()> {
    let peripherals = Peripherals::take().unwrap();
    let led = peripherals.pins.gpio0;
    let channel = peripherals.rmt.channel0;
    let config = TransmitConfig::new().clock_divider(1);
    let mut tx = TxRmtDriver::new(channel, led, &config).unwrap();

    neopixel(
        RGB {
            r: 255,
            g: 55,
            b: 00,
        },
        &mut tx,
    )?;
    FreeRtos::delay_ms(10);

    Ok(())
}

struct RGB {
    r: u8,
    g: u8,
    b: u8,
}

fn ns(nanos: u64) -> Duration {
    Duration::from_nanos(nanos)
}

fn rotate_rgb(rgb: u32) -> u32 {
    let b_mask: u32 = 0xff;
    let blue = (rgb & b_mask) << 16;
    blue | (rgb >> 8)
}

fn neopixel(rgb: RGB, tx: &mut TxRmtDriver) -> anyhow::Result<()> {
    // e.g. rgb: (1,2,4)
    // G        R        B
    // 7      0 7      0 7      0
    // 00000010 00000001 00000100
    let color: u32 = ((rgb.g as u32) << 16) | ((rgb.r as u32) << 8) | rgb.b as u32;
    let ticks_hz = tx.counter_clock()?;
    let t0h = Pulse::new_with_duration(ticks_hz, PinState::High, &ns(350))?;
    let t0l = Pulse::new_with_duration(ticks_hz, PinState::Low, &ns(800))?;
    let t1h = Pulse::new_with_duration(ticks_hz, PinState::High, &ns(700))?;
    let t1l = Pulse::new_with_duration(ticks_hz, PinState::Low, &ns(600))?;
    let mut signal = FixedLengthSignal::<24>::new();
    for i in (0..24).rev() {
        let p = 2_u32.pow(i);
        let bit = p & color != 0;
        let (high_pulse, low_pulse) = if bit { (t1h, t1l) } else { (t0h, t0l) };
        signal.set(23 - i as usize, &(high_pulse, low_pulse))?;
    }
    tx.start_blocking(&signal)?;

    Ok(())
}
