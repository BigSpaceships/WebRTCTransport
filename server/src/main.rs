mod connection_manager;

use actix_session::{storage::CookieSessionStore, Session, SessionMiddleware};
use actix_web::{
    cookie::Key,
    get,
    middleware::Logger,
    post,
    web::{Data, Json},
    App, HttpResponse, HttpServer, Responder,
};
use env_logger::Env;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{
    mpsc::{self, Sender},
    oneshot,
};
use webrtc::{
    api::{media_engine::MediaEngine, APIBuilder},
    ice_transport::ice_candidate::RTCIceCandidateInit,
    interceptor::registry::Registry,
    peer_connection::RTCPeerConnection,
};

use crate::connection_manager::ConnectionMessage;

#[derive(Deserialize, Serialize)]
struct OfferDescription {
    sdp: String,
}

#[derive(Serialize)]
struct OfferResponse {
    sdp: String,
}

#[derive(Serialize, Debug)]
struct IceCandidates {
    candidates: Vec<RTCIceCandidateInit>,
}


// post new_offer
// returns answer string
#[post("/new_offer")]
async fn new_offer(
    offer: Json<OfferDescription>,
    channel: Data<Sender<ConnectionMessage>>,
    session: Session,
) -> Option<Json<OfferResponse>> {
    let _ = session.remove("id");

    let (resp_tx, resp_rx) = oneshot::channel();

    let _ = channel // TODO: i should maybe handle this one
        .send(ConnectionMessage::NewConnection {
            offer: offer.sdp.clone(),
            tx: channel.get_ref().clone(),
            resp: resp_tx,
        })
        .await;

    let res = resp_rx.await.ok().flatten();

    if res.is_none() {
        return None;
    }

    let (answer, id) = res.unwrap();
    session.insert("id", &id).ok()?;

    let offer = OfferResponse { sdp: answer };

    Some(Json(offer))
}

// post ice_candidate
#[post("/ice_candidate")]
async fn ice_candidate(
    data: Json<RTCIceCandidateInit>,
    session: Session,
    channel: Data<Sender<ConnectionMessage>>,
) -> impl Responder {
    let candidate = data.0;

    let id = session.get::<u32>("id");

    if id.is_err() {
        return HttpResponse::Unauthorized();
    }

    let id = id.unwrap();

    if id.is_none() {
        return HttpResponse::Unauthorized();
    }

    let id = id.unwrap();

    let (resp_tx, resp_rx) = oneshot::channel();

    let message = channel
        .send(ConnectionMessage::AddRemoteCandidate {
            id,
            candidate,
            resp: resp_tx,
        })
        .await;

    if message.is_err() {
        return HttpResponse::InternalServerError();
    }

    let resp = resp_rx.await;

    if resp.is_err() {
        return HttpResponse::InternalServerError();
    }

    HttpResponse::Ok()
}

#[get("/ice_candidate")]
async fn get_ice_candidates(session: Session, data: Data<Sender<ConnectionMessage>>) -> Option<Json<Vec<RTCIceCandidateInit>>> {
    let (resp_tx, resp_rx) = oneshot::channel();

    let id = session.get("id").ok().flatten()?;

    let _ = data.send(ConnectionMessage::GetIceCandidates { id, resp: resp_tx }).await;

    let resp = resp_rx.await;

    let candidates = resp.ok().flatten();

    return candidates.map(|c| Json(c));
}

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel::<ConnectionMessage>(32);

    let stop_tx = tx.clone();

    let server = tokio::spawn(async move {
        env_logger::init_from_env(Env::default().default_filter_or("info"));

        let channel = Data::new(tx);

        let _ = HttpServer::new(move || {
            App::new()
                .wrap(Logger::default())
                .wrap(
                    SessionMiddleware::builder(CookieSessionStore::default(), Key::from(&[0; 64]))
                        .cookie_secure(false)
                        .build(),
                )
                .app_data(channel.clone())
                .service(new_offer)
                .service(ice_candidate)
                .service(get_ice_candidates)
        })
        .bind(("127.0.0.1", 3000))
        .unwrap() //TODO: i think i can make a proxy better with this
        .run()
        .await;
    });

    let mut m = MediaEngine::default();

    let _ = m.register_default_codecs();

    let registry = Registry::new();

    let api = APIBuilder::new()
        .with_media_engine(m)
        .with_interceptor_registry(registry)
        .build();

    let mut connections: HashMap<u32, Arc<RTCPeerConnection>> = HashMap::new();

    let mut candidates: HashMap<u32, Vec<RTCIceCandidateInit>> = HashMap::new();

    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;

        let _ = stop_tx.send(ConnectionMessage::Cleanup).await;
    });

    while let Some(message) = rx.recv().await {
        match message {
            ConnectionMessage::NewConnection { offer, tx, resp } => {

                let connection = connection_manager::new_connection(&api, tx.clone(), offer).await;

                if connection.is_none() {
                    let _ = resp.send(None);
                    continue;
                }

                let (pc, id, answer) = connection.unwrap();

                connections.insert(id, pc);

                let _ = resp.send(Some((answer, id)));
            }
            ConnectionMessage::AddRemoteCandidate {
                id,
                candidate,
                resp,
            } => {
                if let Some(connection) = connections.get(&id) {
                    let _ = connection.add_ice_candidate(candidate).await;

                    let _ = resp.send(());
                }
            }
            ConnectionMessage::AddLocalCandidate {
                id,
                candidate,
                resp,
            } => {
                if !candidates.contains_key(&id) {
                    candidates.insert(id, Vec::new());
                }

                candidates.get_mut(&id).unwrap().push(candidate);
                let _ = resp.send(());
            }
            ConnectionMessage::GetIceCandidates { id, resp } => {
                let pc_candidates = candidates.remove(&id);

                println!("{:?}", pc_candidates);
                let _ = resp.send(pc_candidates);
            }
            ConnectionMessage::Cleanup => {
                for (_, value) in connections.iter() { 
                    let _ = value.close().await;
                }

                connections.clear();
            }
        }
    }


    server.await.unwrap();
}
