use crate::status::Status;
use crate::FlashPersister;
use anyhow::Result;
use esp_idf_hal::gpio;
use esp_idf_hal::gpio::*;
use sphinx_signer::sphinx_glyph::control::ControlPersist;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

const MILLIS: u16 = 10_000;

const PAUSE: u16 = 50;

// progression is waiting -> *starting -> reset1a -> reset1 -> reset2a -> reset2 -> reset3
// state machine initialized at starting
pub fn button_loop(
    gpio9: gpio::Gpio9,
    tx: mpsc::Sender<Status>,
    flash_arc: Arc<Mutex<FlashPersister>>,
) -> Result<()> {
    let builder = thread::Builder::new().stack_size(2500);
    builder.spawn(move || {
        let mut button = PinDriver::input(gpio9).unwrap();
        button.set_pull(Pull::Up).unwrap();
        let mut pressed = false;
        let mut up_times = 0;
        let mut low_times = 0;
        let mut machine = Machine::new(tx, Status::Starting);
        loop {
            // we are using thread::sleep here to make sure the watchdog isn't triggered
            if machine.state == Status::Reset3 {
                let mut flash = flash_arc.lock().unwrap();
                if let Err(_e) = flash.remove_config() {
                    log::error!("could not clear wifi config");
                    drop(flash);
                } else {
                    log::info!("restarting esp!");
                    drop(flash);
                    unsafe { esp_idf_sys::esp_restart() };
                }
            }
            thread::sleep(Duration::from_millis(PAUSE.into()));
            if button.is_high() {
                if pressed {
                    pressed = false;
                    log::info!("=> Button let up!");
                    up_times = 0;
                }
                if !pressed {
                    up_times = up_times + 1;

                    // back to start after waiting for a button release
                    if machine.state == Status::Waiting {
                        machine.update_status(Status::Starting);
                    }

                    // if button release while in reset2, reset
                    if machine.state == Status::Reset2 {
                        machine.update_status(Status::Starting);
                    }

                    // advance
                    if machine.state == Status::Reset1a {
                        machine.update_status(Status::Reset1);
                    }

                    if machine.state == Status::Reset3a {
                        machine.update_status(Status::Reset3);
                    }

                    // if stayed up, advance
                    if PAUSE * up_times > MILLIS {
                        if machine.state == Status::Reset1 {
                            machine.update_status(Status::Reset2a);
                        }
                    }

                    // if stays up for much longer, reset
                    if PAUSE * up_times > 2 * MILLIS {
                        machine.update_status(Status::Starting);
                        up_times = 0;
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

                    // if button press while in reset1, wait for a release, and reset
                    if machine.state == Status::Reset1 {
                        machine.update_status(Status::Waiting);
                    }

                    // advance
                    if machine.state == Status::Reset2a {
                        machine.update_status(Status::Reset2);
                    }

                    // if stayed held down, advance
                    if PAUSE * low_times > MILLIS {
                        if machine.state == Status::Reset2 {
                            machine.update_status(Status::Reset3a);
                        } else if machine.state == Status::Starting {
                            machine.update_status(Status::Reset1a);
                        }
                    }

                    // if stayed held down for much longer, wait for a release, and reset
                    if PAUSE * low_times > 2 * MILLIS {
                        machine.update_status(Status::Waiting);
                        low_times = 0;
                    }
                }
            }
        }
    })?;
    Ok(())
}

struct Machine {
    tx: mpsc::Sender<Status>,
    state: Status,
}

impl Machine {
    fn new(tx: mpsc::Sender<Status>, state: Status) -> Machine {
        log::info!("send {:?}", state);
        tx.send(state).unwrap();
        Self { tx, state }
    }
    fn update_status(&mut self, new_state: Status) {
        if self.state != new_state {
            log::info!("send {:?}", new_state);
            self.tx.send(new_state).unwrap();
            self.state = new_state;
        }
    }
}
