use crate::ChannelRequest;

use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::*;
use tokio::sync::mpsc;

pub async fn launch_rocket(
    tx: mpsc::Sender<ChannelRequest>,
) -> std::result::Result<Rocket<Ignite>, rocket::Error> {
    let rz = routes![tester];
    println!("LAUNCH ROCKET!");
    rocket::build()
        .mount("/api/", rz)
        .attach(CORS)
        .manage(tx)
        .launch()
        .await
}

#[get("/tester")]
pub async fn tester() -> Result<String> {
    //
    Ok("hi".to_string())
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

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),
    #[error("not found")]
    NotFound,
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
