use crate::status::Status;
use anyhow::Result;
use esp_idf_svc::hal::rmt::config::TransmitConfig;
use esp_idf_svc::hal::rmt::{FixedLengthSignal, PinState, Pulse, TxRmtDriver};
use esp_idf_svc::hal::{gpio, rmt};
use std::collections::BTreeMap;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

type Color = u32;
type Time = u32;

pub struct Led {
    brg: Color,
    blink_length: Time,
}

fn states() -> BTreeMap<Status, (Color, Time)> {
    let mut s = BTreeMap::new();
    s.insert(Status::MountingSDCard, (0x000102, 100)); // Cyan
    s.insert(Status::SyncingTime, (0x000122, 100)); // Cyan
    s.insert(Status::WifiAccessPoint, (0x000100, 100)); // Green
    s.insert(Status::Configuring, (0x010000, 20)); // Red
    s.insert(Status::ConnectingToWifi, (0x010100, 350)); // Yellow
    s.insert(Status::ConnectingToMqtt, (0x010001, 100)); // Purple
    s.insert(Status::Connected, (0x000101, 400)); // Cyan
    s.insert(Status::Signing, (0x111111, 100)); // White
    s.insert(Status::Ota, (0xffa500, 100)); // Orange
    s.insert(Status::Waiting, (0x000001, 100)); // Blue
    s.insert(Status::Starting, (0x000001, 100)); // Blue
    s.insert(Status::Reset1a, (0x017700, 100)); // yellow
    s.insert(Status::Reset1, (0x017700, 100)); // yellow
    s.insert(Status::Reset2a, (0xffa500, 100)); // orange
    s.insert(Status::Reset2, (0xffa500, 100)); // orange
    s.insert(Status::Reset3a, (0x010000, 100)); // Red
    s.insert(Status::Reset3, (0x010000, 100)); // Red
    s
}

pub fn led_control_loop(
    gpio0: gpio::Gpio0,
    channel0: rmt::CHANNEL0,
    rx: mpsc::Receiver<Status>,
) -> Result<()> {
    let config = TransmitConfig::new().clock_divider(1);
    let transmit = Arc::new(Mutex::new(
        TxRmtDriver::new(channel0, gpio0, &config).unwrap(),
    ));
    let builder = thread::Builder::new().stack_size(2500);
    builder.spawn(move || {
        let mut led = Led::new(0x000001, 100);
        let states = states();
        loop {
            if let Ok(status) = rx.try_recv() {
                log::info!("LED STATUS: {:?}", status);
                if let Some(s) = states.get(&status) {
                    led.set(s.0, s.1);
                }
            }
            led.blink(transmit.clone());
            thread::sleep(Duration::from_millis(400));
        }
    })?;
    Ok(())
}

impl Led {
    pub fn new(rgb: Color, blink_length: Time) -> Led {
        Led {
            brg: rotate_rgb(rgb),
            blink_length,
        }
    }

    pub fn set(&mut self, rgb: Color, blink_length: Time) {
        self.brg = rotate_rgb(rgb);
        self.blink_length = blink_length;
    }

    pub fn blink(&mut self, transmit: Arc<Mutex<TxRmtDriver>>) {
        // Prepare signal
        let mut tx = transmit.lock().unwrap();
        let ticks_hz = tx.counter_clock().unwrap();
        let t0h = Pulse::new_with_duration(ticks_hz, PinState::High, &ns(350)).unwrap();
        let t0l = Pulse::new_with_duration(ticks_hz, PinState::Low, &ns(800)).unwrap();
        let t1h = Pulse::new_with_duration(ticks_hz, PinState::High, &ns(700)).unwrap();
        let t1l = Pulse::new_with_duration(ticks_hz, PinState::Low, &ns(600)).unwrap();
        // Set led color
        let mut signal = FixedLengthSignal::<24>::new();
        for i in 0..24 {
            let bit = 2_u32.pow(i) & self.brg != 0;
            let (high_pulse, low_pulse) = if bit { (t1h, t1l) } else { (t0h, t0l) };
            signal.set(i as usize, &(high_pulse, low_pulse)).unwrap();
        }
        // Set high and wait
        tx.start_blocking(&signal).unwrap();
        // FreeRtos::delay_ms(self.blink_length);
        thread::sleep(Duration::from_millis(self.blink_length.into()));
        // Set low
        let mut signal = FixedLengthSignal::<24>::new();
        for i in 0..24 {
            signal.set(i as usize, &(t0h, t0l)).unwrap();
        }
        tx.start_blocking(&signal).unwrap();
    }
}

fn ns(nanos: u64) -> Duration {
    Duration::from_nanos(nanos)
}

fn rotate_rgb(rgb: u32) -> u32 {
    let b_mask: u32 = 0xff;
    let blue = (rgb & b_mask) << 16;
    blue | (rgb >> 8)
}
