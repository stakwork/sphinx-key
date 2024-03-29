use crate::conn::{current_conns, ChannelRequest};
use crate::util::Settings;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::response::stream::{Event, EventStream};
use rocket::tokio::select;
use rocket::tokio::sync::{
    broadcast::{self, error::RecvError},
    mpsc::Sender,
};
use rocket::*;
use sphinx_signer::sphinx_glyph::{error::Error as GlyphError, topics};
use std::net::IpAddr::V4;
use std::net::Ipv4Addr;

pub type Result<T> = std::result::Result<T, Error>;

#[get("/clients")]
pub async fn get_clients() -> Result<String> {
    Ok(serde_json::to_string(&current_conns())?)
}

#[post("/control?<msg>&<cid>")]
pub async fn control(
    sender: &State<Sender<ChannelRequest>>,
    msg: &str,
    cid: &str,
) -> Result<String> {
    let message = hex::decode(msg)?;
    // FIXME validate? and auth here?
    if message.len() < 65 {
        return Err(Error::Fail);
    }
    let (request, reply_rx) = ChannelRequest::new(cid, topics::CONTROL, message);
    // send to ESP
    sender.send(request).await.map_err(|_| Error::Fail)?;
    // wait for reply
    let reply = reply_rx.await.map_err(|_| Error::Fail)?;
    if reply.is_empty() {
        return Err(Error::Fail);
    }
    Ok(hex::encode(reply.reply))
}

#[get("/errors")]
async fn errors(error_tx: &State<broadcast::Sender<Vec<u8>>>, mut end: Shutdown) -> EventStream![] {
    let mut rx = error_tx.subscribe();
    EventStream! {
        loop {
            let msg = select! {
                msg = rx.recv() => match msg {
                    Ok(msg) => GlyphError::from_slice(&msg[..]),
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(_)) => continue,
                },
                _ = &mut end => break,
            };

            yield Event::json(&msg);
        }
    }
}

pub fn launch_rocket(
    tx: Sender<ChannelRequest>,
    error_tx: broadcast::Sender<Vec<u8>>,
    settings: Settings,
) -> Rocket<Build> {
    let config = Config {
        address: V4(Ipv4Addr::UNSPECIFIED),
        port: settings.http_port,
        ..Config::debug_default()
    };
    rocket::build()
        .configure(config)
        .mount("/api/", routes![control, errors, get_clients])
        .attach(CORS)
        .manage(tx)
        .manage(error_tx)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed")]
    Fail,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("hex error: {0}")]
    Hex(#[from] hex::FromHexError),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

use rocket::http::Status;
use rocket::response::{self, Responder};
impl<'r, 'o: 'r> Responder<'r, 'o> for Error {
    fn respond_to(self, req: &'r rocket::Request<'_>) -> response::Result<'o> {
        // log `self` to your favored error tracker, e.g.
        // sentry::capture_error(&self);
        println!("ERROR {:?}", self);
        // in our simplistic example, we're happy to respond with the default 500 responder in all cases
        Status::InternalServerError.respond_to(req)
    }
}

#[allow(clippy::upper_case_acronyms)]
pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "POST, GET, PATCH, OPTIONS",
        ));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}
