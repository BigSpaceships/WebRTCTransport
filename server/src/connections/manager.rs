use std::{collections::HashMap, sync::Arc};

use bytes::Bytes;
use rand::Rng;
use tokio::sync::{
    mpsc::{Receiver, Sender},
    oneshot,
};
use webrtc::{
    api::{media_engine::MediaEngine, APIBuilder},
    ice_transport::ice_candidate::RTCIceCandidateInit, interceptor::registry::Registry, data::data_channel::DataChannel, data_channel::RTCDataChannel,
};

use super::connection::Connection;

pub enum ConnectionMessage {
    NewConnection {
        offer: String,
        tx: Sender<ConnectionMessage>,
        resp: oneshot::Sender<Option<(String, u32)>>,
    },
    AddRemoteCandidate {
        id: u32,
        candidate: RTCIceCandidateInit,
        resp: oneshot::Sender<()>,
    },
    AddLocalCandidate {
        id: u32,
        candidate: RTCIceCandidateInit,
        resp: oneshot::Sender<()>,
    },
    GetIceCandidates {
        id: u32,
        resp: oneshot::Sender<Option<Vec<RTCIceCandidateInit>>>,
    },
    ConnectionClosed {
        id: u32,
    },
    Cleanup,
    DataChannel {
        id: u32,
        dc: Arc<RTCDataChannel>,
    },
    DataMessage {
        id: u32,
        message: Bytes,
    },
}

pub fn get_id() -> u32 {
    return get_id_with_count(0);
}

fn get_id_with_count(iteration: u8) -> u32 {
    static IDS: Vec<u32> = Vec::new();

    if iteration >= 100 {
        panic!("Deer god what happened we guessed the id too many times");
    }

    let id = rand::thread_rng().gen_range(0..=u32::MAX);

    if IDS.contains(&id) {
        return get_id_with_count(iteration + 1);
    }

    return id;
}

pub async fn start_message_manager(mut rx: Receiver<ConnectionMessage>) {
    let mut m = MediaEngine::default();

    let _ = m.register_default_codecs();

    let registry = Registry::new();

    let api = APIBuilder::new()
        .with_media_engine(m)
        .with_interceptor_registry(registry)
        .build();

    let mut connections: HashMap<u32, Connection> = HashMap::new();

    let mut candidates: HashMap<u32, Vec<RTCIceCandidateInit>> = HashMap::new();

    while let Some(message) = rx.recv().await {
        match message {
            ConnectionMessage::NewConnection { offer, tx, resp } => {
                let connection = Connection::new(&api, tx).await;

                if connection.is_err() {
                    let _ = resp.send(None);
                    continue;
                }

                let connection = connection.unwrap();


                let answer = connection.connect(offer).await;

                if answer.is_none() {
                    let _ = resp.send(None);
                    continue;
                }

                let answer_sdp = answer.unwrap();

                let _ = resp.send(Some((answer_sdp, connection.id)));
                connections.insert(connection.id, connection);
            }
            ConnectionMessage::AddRemoteCandidate {
                id,
                candidate,
                resp,
            } => {
                if let Some(connection) = connections.get(&id) {
                    let result = connection.add_ice_candidate(candidate).await;

                    if result.is_err() {
                        println!("Error adding candidate: {:?}", result.unwrap_err());
                    };

                }
                let _ = resp.send(());
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
            ConnectionMessage::ConnectionClosed { id } => {
                let connection = connections.get(&id);

                if connection.is_none() { 
                    continue;
                }

                connection.unwrap().close().await;

                connections.remove(&id);
            }
            ConnectionMessage::Cleanup => {
                for (_, value) in connections.iter() {
                    value.close().await;
                }

                connections.clear();
            }
            ConnectionMessage::DataChannel { id, dc } => {
                // data channel
            }
            ConnectionMessage::DataMessage { id, message } => {
                println!("Message from {id}: {:?}", message);
            }
        }
    }
}
