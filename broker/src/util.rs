use bitcoin::Network;
use std::default::Default;
use std::env;
use std::fs;
use std::str::FromStr;
use toml::Value;

pub struct Settings {
    pub network: Network,
    pub port: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            network: Network::Regtest,
            port: 1883,
        }
    }
}

pub fn read_broker_config(config_path: &str) -> Settings {
    let mut settings = Settings::default();
    if let Ok(set) = fs::read_to_string(config_path) {
        let table = Value::from_str(&set)
            .expect("Couldn't read broker.conf make sure it follows the toml format");
        log::info!("Read broker.conf");
        if let Some(network) = read_network_setting(&table) {
            settings.network = network
        }
        if let Some(port) = read_port_setting(&table) {
            settings.port = port
        }
    } else {
        log::info!("File broker.conf not found, using default settings");
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
        .level_for(
            "librumqttd::rumqttlog::router::router",
            log::LevelFilter::Warn,
        )
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

fn read_port_setting(table: &Value) -> Option<u16> {
    if let None = table.get("port") {
        log::info!("Broker port not specified, setting to default 1883");
        None
    } else {
        let temp = table["port"]
            .as_integer()
            .expect("The port number is not an integer greater than 1023");
        if temp <= 1023 {
            panic!("The port number is not an integer greater than 1023")
        }
        if temp > u16::MAX.into() {
            panic!("The port number is way too big!")
        }
        log::info!("Read broker port setting: {}", temp);
        Some(temp.try_into().unwrap())
    }
}
