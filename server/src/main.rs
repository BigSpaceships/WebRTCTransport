#[macro_use]
extern crate rocket;
use rocket::{http::Method, serde::{Deserialize, json::Json}};
use rocket_cors::{AllowedOrigins, CorsOptions};
use tokio::task;

#[get("/")]
fn index() -> &'static str {
    task::spawn(async {
        println!("woah");
    });
    "Hello, world!"
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct OfferDescription<'a> {
    sdp: &'a str,
}

#[post("/new_offer", data = "<offer>")]
fn new_offer(offer: String) -> &'static str {
    println!("{}", offer);
    "bo"
}

#[launch]
fn rocket() -> _ {
    let cors = CorsOptions::default()
        .allowed_origins(AllowedOrigins::all())
        .allowed_methods(
            vec![Method::Get, Method::Post, Method::Patch]
                .into_iter()
                .map(From::from)
                .collect(),
        )
        .allow_credentials(true);

    rocket::build().attach(cors.to_cors().unwrap()).mount("/", routes![index, new_offer])
}
