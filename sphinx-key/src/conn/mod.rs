pub mod wifi;
pub mod http;
mod html;

use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Deserialize)]
pub struct Params {
    pub config: String
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub broker: String,
    pub ssid: String,
    pub pass: String,
}

/*
52.91.253.115:1883

curl -X POST 192.168.71.1/config?config=%7B%22ssid%22%3A%22apples%26acorns%22%2C%22pass%22%3A%2242flutes%22%2C%22broker%22%3A%2252.91.253.115%3A1883%22%7D

arp -a

http://192.168.71.1/?broker=52.91.253.115%3A1883
*/