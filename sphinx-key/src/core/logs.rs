use crate::conn::mqtt::QOS;
use embedded_svc::mqtt::client::utils::ConnState;
use embedded_svc::mqtt::client::{MessageImpl, Publish};
use esp_idf_svc::mqtt::client::*;
use esp_idf_sys::EspError;
use log::*;
use sphinx_signer::sphinx_glyph::topics;
use std::sync::{Arc, Mutex};

struct MyLogger {
    filter: LevelFilter,
    mqtt: Option<Arc<Mutex<EspMqttClient<ConnState<MessageImpl, EspError>>>>>,
}

impl Log for MyLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.filter
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let lg = format!("{} {} {}", record.level(), record.target(), record.args());
            if let Some(mqtt_) = &self.mqtt {
                let mut mqtt = mqtt_.lock().unwrap();
                mqtt.publish(topics::ERROR, QOS, false, lg.as_bytes())
                    .expect("could not publish VLS error");
            } else {
                println!("{}", &lg);
            }
        }
    }

    fn flush(&self) {}
}

pub fn setup_logs(mqtt: Arc<Mutex<EspMqttClient<ConnState<MessageImpl, EspError>>>>) {
    let elog1: Box<dyn Log> = Box::new(MyLogger {
        filter: LevelFilter::Info,
        mqtt: None,
    });
    let elog2: Box<dyn Log> = Box::new(MyLogger {
        filter: LevelFilter::Info,
        mqtt: Some(mqtt),
    });
    fern::Dispatch::new()
        .level(LevelFilter::Warn)
        .level_for("lightning_signer", LevelFilter::Info)
        .chain(elog1) // Chaining two logs
        .chain(elog2)
        .apply()
        .expect("log config");
    debug!("debug");
    info!("info");
    info!(target: "lightning_signer", "info policy");
    warn!(target: "lightning_signer", "warn policy");
    warn!("warn");
}
