use embedded_hal::blocking::delay::DelayMs;
use esp_idf_hal::delay::Ets;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::rmt::config::TransmitConfig;
use esp_idf_hal::rmt::{FixedLengthSignal, PinState, Pulse, Transmit};
use std::time::Duration;

pub fn set_ota_led() {
    let peripherals = Peripherals::take().unwrap();
    let led = peripherals.pins.gpio8.into_output().unwrap();
    let channel = peripherals.rmt.channel0;
    let config = TransmitConfig::new().clock_divider(1);
    let mut tx = Transmit::new(led, channel, &config).unwrap();

    let rgb = 0xffa500; // Orange

    let ticks_hz = tx.counter_clock().unwrap();
    let t0h = Pulse::new_with_duration(ticks_hz, PinState::High, &ns(350)).unwrap();
    let t0l = Pulse::new_with_duration(ticks_hz, PinState::Low, &ns(800)).unwrap();
    let t1h = Pulse::new_with_duration(ticks_hz, PinState::High, &ns(700)).unwrap();
    let t1l = Pulse::new_with_duration(ticks_hz, PinState::Low, &ns(600)).unwrap();

    let mut signal = FixedLengthSignal::<24>::new();
    for i in 0..24 {
        let bit = 2_u32.pow(i) & rotate_rgb(rgb) != 0;
        let (high_pulse, low_pulse) = if bit { (t1h, t1l) } else { (t0h, t0l) };
        signal.set(i as usize, &(high_pulse, low_pulse)).unwrap();
    }
    tx.start_blocking(&signal).unwrap();
    Ets.delay_ms(10u8);
}

fn ns(nanos: u64) -> Duration {
    Duration::from_nanos(nanos)
}

fn rotate_rgb(rgb: u32) -> u32 {
    let b_mask: u32 = 0xff;
    let blue = (rgb & b_mask) << 16;
    blue | (rgb >> 8)
}
