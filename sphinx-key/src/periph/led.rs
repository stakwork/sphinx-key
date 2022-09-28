use crate::core::events::Status;
use embedded_hal::delay::blocking::DelayUs;
use esp_idf_hal::delay::Ets;
use esp_idf_hal::rmt::config::TransmitConfig;
use esp_idf_hal::rmt::{FixedLengthSignal, PinState, Pulse, Transmit};
use esp_idf_hal::{gpio, rmt};
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
    s.insert(Status::Starting, (0x000001, 100)); // Blue
    s.insert(Status::MountingSDCard, (0x000102, 100)); // Cyan
    s.insert(Status::SyncingTime, (0x000122, 100)); // Cyan
    s.insert(Status::WifiAccessPoint, (0x000100, 100)); // Green
    s.insert(Status::Configuring, (0x010000, 20)); // Red
    s.insert(Status::ConnectingToWifi, (0x010100, 350)); // Yellow
    s.insert(Status::ConnectingToMqtt, (0x010001, 100)); // Purple
    s.insert(Status::Connected, (0x000101, 400)); // Cyan
    s.insert(Status::Signing, (0x111111, 100)); // White
    s.insert(Status::Ota, (0xffa500, 100)); // Orange
    s
}

pub fn led_control_loop(
    gpio8: gpio::Gpio8<gpio::Unknown>,
    channel0: rmt::CHANNEL0,
    rx: mpsc::Receiver<Status>,
) {
    let led = gpio8.into_output().unwrap();
    let config = TransmitConfig::new().clock_divider(1);
    let transmit = Arc::new(Mutex::new(Transmit::new(led, channel0, &config).unwrap()));
    thread::spawn(move || {
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
    });
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

    pub fn blink(
        &mut self,
        transmit: Arc<Mutex<Transmit<gpio::Gpio8<gpio::Output>, rmt::CHANNEL0>>>,
    ) {
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
        Ets.delay_ms(self.blink_length).unwrap();
        // Set low
        let mut signal = FixedLengthSignal::<24>::new();
        for i in 0..24 {
            let bit = 2_u32.pow(i) & 0x000000 != 0;
            let (high_pulse, low_pulse) = if bit { (t1h, t1l) } else { (t0h, t0l) };
            signal.set(i as usize, &(high_pulse, low_pulse)).unwrap();
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
