use actix_web::{
    get, post,
    web::{self, Json},
    App, HttpResponse, HttpServer, Responder,
};
use serde::Deserialize;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use webrtc::{
    ice_transport::ice_candidate::RTCIceCandidateInit,
    peer_connection::{sdp::session_description::RTCSessionDescription, RTCPeerConnection},
};

#[derive(Deserialize, Debug)]
struct OfferDescription {
    sdp: String,
}

struct RTCConnections {
    connections: HashMap<u32, Arc<RTCPeerConnection>>,
}

enum IceCandidateType {
    Local,
    Remote,
}

struct RTCIceCandidates {
    candidates: HashMap<u32, Vec<(IceCandidateType, RTCIceCandidateInit)>>,
}

// post new_offer
// returns answer string
#[post("/new_offer")]
async fn new_offer(offer: Json<OfferDescription>) -> impl Responder {
    println!("{:?}", offer);
    HttpResponse::Ok()
}

// post ice_candidate

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(new_offer))
        .bind(("127.0.0.1", 3000))? //TODO: i think i can make a proxy better with this
        .run()
        .await
}
