use std::env;
use std::fs;
use std::str::FromStr;
use toml::map::Map;
use toml::Value;

pub fn read_broker_config(config_path: &str) -> Value {
    let mut ret = Value::Table(Map::new());
    if let Ok(set) = fs::read_to_string(config_path) {
        ret = Value::from_str(&set)
            .expect("Couldn't read broker.conf make sure it follows the toml format");
        log::info!("Read broker.conf");
    } else {
        log::info!("File broker.conf not found, using default settings");
    }
    validate_network_setting(&mut ret);
    validate_port_setting(&mut ret);
    ret
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

fn validate_network_setting(settings: &mut Value) {
    if let None = settings.get("network") {
        log::info!("Network not specified, setting to default regtest");
        settings
            .as_table_mut()
            .unwrap()
            .insert("network".to_string(), Value::String("regtest".to_string()));
    } else {
        if !settings["network"].is_str()
            || settings["network"].as_str().unwrap() != "bitcoin"
                && settings["network"].as_str().unwrap() != "regtest"
        {
            panic!("The network must be set to either 'bitcoin' or 'regtest'");
        }
        log::info!(
            "Read network setting: {}",
            settings["network"].as_str().unwrap()
        );
    }
}

fn validate_port_setting(settings: &mut Value) {
    if let None = settings.get("port") {
        log::info!("Broker port not specified, setting to default 1883");
        settings
            .as_table_mut()
            .unwrap()
            .insert("port".to_string(), Value::Integer(1883));
    } else {
        let temp = settings["port"]
            .as_integer()
            .expect("The port number is not an integer greater than 1023");
        if temp <= 1023 {
            panic!("The port number is not an integer greater than 1023")
        }
        if temp > u16::MAX.into() {
            panic!("The port number is way too big!")
        }
        log::info!("Read broker port setting: {}", temp);
    }
}
