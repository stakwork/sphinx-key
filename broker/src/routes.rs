use crate::ChannelRequest;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::tokio::sync::{mpsc::Sender, oneshot};
use rocket::*;
use rocket::{Request, Response};

pub type Result<T> = std::result::Result<T, Error>;

#[post("/control?<msg>")]
pub async fn yo(sender: &State<Sender<ChannelRequest>>, msg: &str) -> Result<String> {
    let message = hex::decode(msg)?;
    // FIXME validate?
    if message.len() < 65 {
        return Err(Error::Fail);
    }
    let (reply_tx, reply_rx) = oneshot::channel();
    let request = ChannelRequest { message, reply_tx };
    // send to ESP
    let _ = sender.send(request).await.map_err(|_| Error::Fail)?;
    // wait for reply
    let reply = reply_rx.await.map_err(|_| Error::Fail)?;
    Ok(hex::encode(reply.reply).to_string())
}

pub fn launch_rocket(tx: Sender<ChannelRequest>) -> Rocket<Build> {
    rocket::build()
        .mount("/api/", routes![yo])
        .attach(CORS)
        .manage(tx)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed")]
    Fail,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("hex error: {0}")]
    Hex(#[from] hex::FromHexError),
}

use rocket::http::Status;
use rocket::response::{self, Responder};
impl<'r, 'o: 'r> Responder<'r, 'o> for Error {
    fn respond_to(self, req: &'r rocket::Request<'_>) -> response::Result<'o> {
        // log `self` to your favored error tracker, e.g.
        // sentry::capture_error(&self);
        match self {
            // in our simplistic example, we're happy to respond with the default 500 responder in all cases
            _ => Status::InternalServerError.respond_to(req),
        }
    }
}

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
