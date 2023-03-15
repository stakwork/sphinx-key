use bitcoin::Network;
use std::default::Default;
use std::env;
use std::fs;
use std::str::FromStr;
use toml::Value;

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub http_port: u16,
    pub mqtt_port: u16,
    pub network: Network,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            http_port: 8000,
            mqtt_port: 1883,
            network: Network::Regtest,
        }
    }
}

pub fn read_broker_config(config_path: &str) -> Settings {
    let mut settings = Settings::default();
    if let Ok(set) = fs::read_to_string(config_path) {
        let table = Value::from_str(&set)
            .expect("Couldn't read broker.conf make sure it follows the toml format");
        if let Some(network) = read_network_setting(&table) {
            settings.network = network;
        }
        if let Some(mqtt_port) = read_mqtt_port_setting(&table) {
            settings.mqtt_port = mqtt_port;
        }
        if let Some(http_port) = read_http_port_setting(&table) {
            settings.http_port = http_port;
        }
    } else {
        log::info!("File broker.conf not found, using default settings");
    }
    if let Ok(env_net) = env::var("BROKER_NETWORK") {
        if let Ok(net) = Network::from_str(&env_net) {
            settings.network = net;
        }
    }
    if let Ok(env_port) = env::var("BROKER_MQTT_PORT") {
        if let Ok(mqtt_port) = env_port.parse::<u16>() {
            if mqtt_port > 1023 {
                settings.mqtt_port = mqtt_port;
            }
        }
    }
    if let Ok(env_port) = env::var("BROKER_HTTP_PORT") {
        if let Ok(http_port) = env_port.parse::<u16>() {
            if http_port > 1023 {
                settings.http_port = http_port;
            }
        }
    }
    settings
}

pub fn setup_logging(who: &str, level_arg: &str) {
    use fern::colors::{Color, ColoredLevelConfig};
    let colors = ColoredLevelConfig::new()
        .info(Color::Green)
        .error(Color::Red)
        .warn(Color::Yellow);
    let level = env::var("RUST_LOG").unwrap_or(level_arg.to_string());
    let who = who.to_string();
    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{} {}/{} {}] {}",
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f"),
                who,
                record.target(),
                colors.color(record.level()),
                message
            ))
        })
        .level(log::LevelFilter::from_str(&level).expect("level"))
        .level_for("h2", log::LevelFilter::Info)
        .level_for("sled", log::LevelFilter::Info)
        // .level_for("rumqttd", log::LevelFilter::Warn)
        .level_for("rocket", log::LevelFilter::Warn)
        .level_for("tracing", log::LevelFilter::Warn)
        .level_for("_", log::LevelFilter::Warn)
        .chain(std::io::stdout())
        // .chain(fern::log_file("/tmp/output.log")?)
        .apply()
        .expect("log config");
}

fn read_network_setting(table: &Value) -> Option<Network> {
    if let None = table.get("network") {
        log::info!("Network not specified, setting to default regtest");
        None
    } else {
        if !table["network"].is_str()
            || table["network"].as_str().unwrap() != "bitcoin"
                && table["network"].as_str().unwrap() != "regtest"
        {
            panic!("The network must be set to either 'bitcoin' or 'regtest'");
        }
        log::info!(
            "Read network setting: {}",
            table["network"].as_str().unwrap()
        );
        Some(Network::from_str(table["network"].as_str().unwrap()).unwrap())
    }
}

fn read_mqtt_port_setting(table: &Value) -> Option<u16> {
    if let None = table.get("mqtt_port") {
        log::info!("Broker mqtt port not specified, setting to default 1883");
        None
    } else {
        let temp = table["mqtt_port"]
            .as_integer()
            .expect("The mqtt port number is not an integer greater than 1023");
        if temp <= 1023 {
            panic!("The mqtt port number is not an integer greater than 1023")
        }
        let max: i64 = u16::MAX.into();
        if temp > max {
            panic!("The mqtt port number is way too big!")
        }
        log::info!("Read broker mqtt port setting: {}", temp);
        Some(temp.try_into().unwrap())
    }
}

fn read_http_port_setting(table: &Value) -> Option<u16> {
    if let None = table.get("http_port") {
        log::info!("Broker http port not specified, setting to default 8000");
        None
    } else {
        let temp = table["http_port"]
            .as_integer()
            .expect("The http port number is not an integer greater than 1023");
        if temp <= 1023 {
            panic!("The http port number is not an integer greater than 1023")
        }
        let max: i64 = u16::MAX.into();
        if temp > max {
            panic!("The http port number is way too big!")
        }
        log::info!("Read broker http port setting: {}", temp);
        Some(temp.try_into().unwrap())
    }
}
