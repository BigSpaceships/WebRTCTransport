#[macro_use]
extern crate rocket;
use std::sync::Arc;

use rocket::{
    http::Method,
    serde::{json::Json, Deserialize},
};
use rocket_cors::{AllowedOrigins, CorsOptions};
use tokio::task;
use webrtc::{
    api::{media_engine::MediaEngine, APIBuilder},
    ice_transport::{ice_server::RTCIceServer, ice_candidate::RTCIceCandidate},
    interceptor::registry::Registry,
    peer_connection::{configuration::RTCConfiguration, peer_connection_state::RTCPeerConnectionState, sdp::{session_description::RTCSessionDescription, sdp_type::RTCSdpType}}, data_channel::{RTCDataChannel, data_channel_message::DataChannelMessage}, ice::candidate,
};

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
async fn new_offer(offer: String) -> Option<String> {
    let mut m = MediaEngine::default();

    let _ = m.register_default_codecs();

    let mut registry = Registry::new();

    let api = APIBuilder::new()
        .with_media_engine(m)
        .with_interceptor_registry(registry)
        .build();

    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }],
        ..Default::default()
    };

    let pc = Arc::new(api.new_peer_connection(config).await.ok()?);

    let (done_tx, mut done_rx) = tokio::sync::mpsc::channel::<()>(1);

    pc.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
        println!("Peer Connection State has changed: {s}");

        if s == RTCPeerConnectionState::Failed {
            println!("Peer Connection has gone to failed exiting");
            let _ = done_tx.try_send(());
        }

        Box::pin(async {})
    }));

    pc.on_data_channel(Box::new(move |d: Arc<RTCDataChannel>| {
        let d_label = d.label().to_owned();
        let d_id = d.id();
        println!("New data channel {d_label} {d_id}");

        Box::pin(async move {
            let d2 = Arc::clone(&d);

            let d_label2 = d_label.clone();
            let d_id2 = d_id;

            d.on_close(Box::new(move || {
                println!("Data channel closed");
                Box::pin(async {})
            }));

            d.on_open(Box::new(move || {
                println!("Data channel {d_label} {d_id} opened");
                Box::pin(async {})
            }));

            d.on_message(Box::new(move |msg: DataChannelMessage| {
                let msg_str = String::from_utf8(msg.data.to_vec()).unwrap();
                println!("Message from data channel '{d_label2}': '{msg_str}'");
                Box::pin(async {})
            }));
        })
    }));

    let session_desc = RTCSessionDescription::offer(offer).ok()?;

    pc.set_remote_description(session_desc).await.ok()?;
    
    let answer = pc.create_answer(None).await.ok()?;
    
    pc.on_ice_candidate(Box::new(move |candidate: Option<RTCIceCandidate>| {
        Box::pin(async move {
            // add to state to send
        })
    }));

    Some(answer.sdp)
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

    rocket::build()
        .attach(cors.to_cors().unwrap())
        .mount("/", routes![index, new_offer])
}
