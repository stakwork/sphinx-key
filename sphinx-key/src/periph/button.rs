use crate::core::events::Status;
use esp_idf_hal::gpio;
use esp_idf_hal::gpio::*;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub fn button_loop(gpio8: gpio::Gpio8, tx: mpsc::Sender<Status>) {
    thread::spawn(move || {
        let mut button = PinDriver::input(gpio8).unwrap();
        button.set_pull(Pull::Down).unwrap();
        loop {
            // we are using thread::sleep here to make sure the watchdog isn't triggered
            thread::sleep(Duration::from_millis(10));
            if button.is_high() {
                log::info!("=> GPIO8 HIGH!");
                tx.send(Status::Reset1).unwrap();
            } else {
                log::info!("=> GPIO8 LOW!");
            }
        }
    });
}
