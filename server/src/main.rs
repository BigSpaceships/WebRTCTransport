#[macro_use]
extern crate rocket;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use rocket::{
    http::{Cookie, CookieJar, Method, SameSite},
    serde::{json::Json, Deserialize, Serialize},
    State,
};
use rocket_cors::{AllowedOrigins, CorsOptions};
use tokio::task;
use webrtc::{
    api::{media_engine::MediaEngine, APIBuilder},
    ice_transport::{ice_server::RTCIceServer, ice_candidate::RTCIceCandidate},
    data_channel::{data_channel_message::DataChannelMessage, RTCDataChannel},
    interceptor::registry::Registry,
    peer_connection::{
        configuration::RTCConfiguration,
        peer_connection_state::RTCPeerConnectionState,
        sdp::{sdp_type::RTCSdpType, session_description::RTCSessionDescription},
        RTCPeerConnection,
    },
};

#[get("/")]
fn index() -> &'static str {
    task::spawn(async {
        println!("woah");
    });
    "Hello, world!"
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct OfferDescription {
    sdp: String,
}

struct RTCConnections {
    connections: HashMap<u32, Arc<RTCPeerConnection>>,
}

#[post("/new_offer", data = "<offer>")]
async fn new_offer(
    offer: String,
    open_connections_state: &State<tokio::sync::Mutex<RTCConnections>>,
    cookies: &CookieJar<'_>,
) -> Option<Json<OfferDescription>> {
    let mut m = MediaEngine::default();

    let _ = m.register_default_codecs();

    let registry = Registry::new();

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

    // TODO: new ids
    // TODO: use private (i think this probably means i should set up a proxy so they're coming
    // from the same domain) 
    let id = cookies
        .get("id")
        .map(|id| id.value().parse::<u32>().ok())
        .flatten()
        .unwrap_or_else(|| {
            let new_id = 11;

            let mut cookie = Cookie::new("id", new_id.to_string());
            cookie.set_same_site(SameSite::Lax);
            cookies.add(cookie);

            new_id
        });

    open_connections_state.lock().await.connections.remove(&id);

    pc.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
        println!("Peer Connection State has changed: {s}");

        if s == RTCPeerConnectionState::Failed {
            println!("Peer Connection has gone to failed exiting");
        }

        Box::pin(async {})
    }));

    pc.on_data_channel(Box::new(move |d: Arc<RTCDataChannel>| {
        let d_label = d.label().to_owned();
        let d_id = d.id();
        println!("New data channel {d_label} {d_id}");

        Box::pin(async move {
            let d_label2 = d_label.clone();

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
        let id2 = id.to_owned();

        println!("ice candidate2");

        Box::pin(async move {
            println!("ice candidate");
        })
    }));

    let offer = OfferDescription { sdp: answer.sdp };
    open_connections_state
        .lock()
        .await
        .connections
        .insert(id, pc);

    Some(Json(offer))
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
        .manage(tokio::sync::Mutex::new(RTCConnections {
            connections: HashMap::new(),
        }))
}
