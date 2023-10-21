use crate::{colors::*, FactoryError};
use core::time::Duration;
use esp_idf_svc::hal::{
    delay::FreeRtos,
    gpio::Gpio0,
    rmt::{config::TransmitConfig, FixedLengthSignal, PinState, Pulse, TxRmtDriver, CHANNEL0},
    sys::EspError,
};

pub(crate) struct Peripherals {
    pub led: Gpio0,
    pub channel: CHANNEL0,
}

pub(crate) fn setup(peripherals: Peripherals) -> Result<TxRmtDriver<'static>, FactoryError> {
    let led = peripherals.led;
    let channel = peripherals.channel;
    let config = TransmitConfig::new().clock_divider(1);
    let tx = TxRmtDriver::new(channel, led, &config).map_err(|e| FactoryError::EspError(e))?;
    Ok(tx)
}

pub(crate) fn setup_complete(led_tx: &mut TxRmtDriver) -> Result<(), FactoryError> {
    neopixel(BLUE, led_tx).map_err(|e| FactoryError::EspError(e))?;
    FreeRtos::delay_ms(10);
    Ok(())
}

pub(crate) fn update_launch(led_tx: &mut TxRmtDriver) -> Result<(), FactoryError> {
    neopixel(ORANGE, led_tx).map_err(|e| FactoryError::EspError(e))?;
    FreeRtos::delay_ms(10);
    Ok(())
}

pub(crate) fn update_complete(led_tx: &mut TxRmtDriver) -> Result<(), FactoryError> {
    neopixel(GREEN, led_tx).map_err(|e| FactoryError::EspError(e))?;
    FreeRtos::delay_ms(10);
    Ok(())
}

pub(crate) fn main_app_launch(led_tx: &mut TxRmtDriver) -> Result<(), FactoryError> {
    neopixel(WHITE, led_tx).map_err(|e| FactoryError::EspError(e))?;
    FreeRtos::delay_ms(10);
    Ok(())
}

fn ns(nanos: u64) -> Duration {
    Duration::from_nanos(nanos)
}

fn neopixel(rgb: RGB, tx: &mut TxRmtDriver) -> Result<(), EspError> {
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
