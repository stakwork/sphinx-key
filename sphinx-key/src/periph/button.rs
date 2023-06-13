use crate::status::Status;
use esp_idf_hal::gpio;
use esp_idf_hal::gpio::*;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const MILLIS: u16 = 10_000;

const PAUSE: u16 = 50;

pub fn button_loop(gpio8: gpio::Gpio8, tx: mpsc::Sender<Status>) {
    thread::spawn(move || {
        let mut button = PinDriver::input(gpio8).unwrap();
        button.set_pull(Pull::Up).unwrap();
        let mut pressed = false;
        let mut up_times = 0;
        let mut low_times = 0;
        let mut last_status = Status::Starting;
        loop {
            // we are using thread::sleep here to make sure the watchdog isn't triggered
            thread::sleep(Duration::from_millis(PAUSE.into()));
            if button.is_high() {
                if pressed {
                    pressed = false;
                    log::info!("=> Button let up!");
                    up_times = 0;
                }
                if !pressed {
                    up_times = up_times + 1;
                    if PAUSE * up_times > MILLIS {
                        // stayed up
                        if last_status == Status::Reset1 {
                            log::info!("send Status::Reset2!");
                            tx.send(Status::Reset2).unwrap();
                            last_status = Status::Reset2;
                        }
                    }
                }
            } else {
                if !pressed {
                    pressed = true;
                    log::info!("=> Button pressed!");
                    low_times = 0;
                }
                if pressed {
                    low_times = low_times + 1;
                    if PAUSE * low_times > MILLIS {
                        // stayed held down
                        if last_status == Status::Reset2 {
                            log::info!("send Status::Reset3!");
                            tx.send(Status::Reset3).unwrap();
                            last_status = Status::Reset3;
                        } else if last_status != Status::Reset1 && last_status != Status::Reset3 {
                            log::info!("send Status::Reset1!");
                            tx.send(Status::Reset1).unwrap();
                            last_status = Status::Reset1;
                        }
                    }
                }
            }
        }
    });
}
