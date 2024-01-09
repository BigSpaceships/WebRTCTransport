use actix_web::{
    middleware::Logger,
    post,
    web::{self, Data, Json},
    App, HttpResponse, HttpServer, Responder,
};
use env_logger::Env;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use webrtc::{
    api::{media_engine::MediaEngine, APIBuilder},
    data_channel::{data_channel_message::DataChannelMessage, RTCDataChannel},
    ice_transport::{
        ice_candidate::{RTCIceCandidate, RTCIceCandidateInit},
        ice_server::RTCIceServer,
    },
    interceptor::registry::Registry,
    peer_connection::{
        configuration::RTCConfiguration, peer_connection_state::RTCPeerConnectionState,
        sdp::session_description::RTCSessionDescription, RTCPeerConnection,
    },
};

#[derive(Deserialize, Serialize)]
struct OfferDescription {
    sdp: String,
}

#[derive(Serialize)]
struct OfferResponse {
    sdp: String,
    id: u32,
}

struct RTCConnections {
    connections: Mutex<HashMap<u32, Arc<RTCPeerConnection>>>,
}

enum IceCandidateType {
    Local,
    Remote,
}

#[derive(Deserialize)]
struct IceCandidateCreation {
    candidate: String,
    sdp_mid: Option<String>,
    sdp_mline_index: Option<u16>,
    id: u32,
}

struct RTCIceCandidates {
    candidates: Mutex<HashMap<u32, Vec<(IceCandidateType, RTCIceCandidateInit)>>>,
}

// post new_offer
// returns answer string
#[post("/new_offer")]
async fn new_offer(
    offer: Json<OfferDescription>,
    open_connections: Data<RTCConnections>,
    candidates: Data<RTCIceCandidates>,
) -> Option<Json<OfferResponse>> {
    let mut m = MediaEngine::default(); // TODO: share this

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

    let id = 11; // TODO: new id

    pc.on_ice_candidate(Box::new(move |candidate: Option<RTCIceCandidate>| {
        let id2 = id.to_owned();

        println!("ice candidate2");

        Box::pin(async move {
            println!("ice candidate");
        })
    }));

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

    let session_desc = RTCSessionDescription::offer(offer.sdp.clone()).ok()?;

    pc.set_remote_description(session_desc).await.ok()?;

    let answer = pc.create_answer(None).await.ok()?;

    let offer = OfferResponse {
        sdp: answer.sdp,
        id,
    };

    open_connections.connections.lock().unwrap().insert(id, pc);
    candidates.candidates.lock().unwrap().insert(id, Vec::new());

    Some(Json(offer))
}

// post ice_candidate
#[post("/ice_candidate")]
async fn ice_candidate(
    data: Json<IceCandidateCreation>,
    candidates: Data<RTCIceCandidates>,
) -> impl Responder {
    let candidate = RTCIceCandidateInit {
        candidate: data.candidate.clone(),
        sdp_mid: data.sdp_mid.clone(),
        sdp_mline_index: data.sdp_mline_index,
        username_fragment: None,
    };

    let id = data.id;

    let mut candidates = candidates.candidates.lock().unwrap();

    if !candidates.contains_key(&id) {
        return HttpResponse::NotFound();
    }

    candidates
        .get_mut(&id)
        .unwrap()
        .push((IceCandidateType::Remote, candidate));

    HttpResponse::Ok()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("debug"));

    let connections = Data::new(RTCConnections {
        connections: Mutex::new(HashMap::new()),
    });

    let candidates = Data::new(RTCIceCandidates {
        candidates: Mutex::new(HashMap::new()),
    });

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(connections.clone())
            .app_data(candidates.clone())
            .service(new_offer)
            .service(ice_candidate)
    })
    .bind(("127.0.0.1", 3000))? //TODO: i think i can make a proxy better with this
    .run()
    .await
}
