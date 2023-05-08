use crate::core::events::Status;
use esp_idf_hal::gpio;
use esp_idf_hal::gpio::*;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const MILLIS: u16 = 10_000;

const PAUSE: u16 = 15;

pub fn button_loop(gpio8: gpio::Gpio8, tx: mpsc::Sender<Status>) {
    thread::spawn(move || {
        let mut button = PinDriver::input(gpio8).unwrap();
        button.set_pull(Pull::Down).unwrap();
        let mut high = false;
        let mut high_times = 0;
        let mut low_times = 0;
        loop {
            // we are using thread::sleep here to make sure the watchdog isn't triggered
            thread::sleep(Duration::from_millis(PAUSE.into()));
            if button.is_high() {
                if !high {
                    high = true;
                    log::info!("=> GPIO8 HIGH!");
                    high_times = 0;
                }
                if high {
                    high_times = high_times + 1;
                    if PAUSE * high_times > MILLIS {
                        // stayed held down?
                    }
                }
            } else {
                if high {
                    high = false;
                    log::info!("=> GPIO8 LOW!");
                    tx.send(Status::Reset1).unwrap();
                    low_times = 0;
                }
                if !high {
                    low_times = low_times + 1;
                    if PAUSE * low_times > MILLIS {
                        // stayed not held down?
                    }
                }
            }
        }
    });
}
