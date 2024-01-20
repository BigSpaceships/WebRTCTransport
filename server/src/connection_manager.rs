use std::sync::Arc;

use rand::Rng;
use tokio::sync::{mpsc::Sender, oneshot};
use webrtc::{
    api::API,
    data_channel::{data_channel_message::DataChannelMessage, RTCDataChannel},
    ice_transport::{
        ice_candidate::{RTCIceCandidate, RTCIceCandidateInit},
        ice_gatherer_state::RTCIceGathererState,
        ice_server::RTCIceServer,
    },
    peer_connection::{
        configuration::RTCConfiguration,
        peer_connection_state::RTCPeerConnectionState,
        policy::{
            bundle_policy::RTCBundlePolicy, ice_transport_policy::RTCIceTransportPolicy,
            rtcp_mux_policy::RTCRtcpMuxPolicy,
        },
        sdp::session_description::RTCSessionDescription,
        signaling_state::RTCSignalingState,
        RTCPeerConnection,
    },
};

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
    Cleanup,
}

fn get_id() -> u32 {
    return get_id_with_count(0);
}

fn get_id_with_count(iteration: u8) -> u32 {
    static IDS:Vec<u32> = Vec::new();

    if iteration >= 100 {
        panic!("Deer god what happened we guessed the id too many times");
    }

    let id = rand::thread_rng().gen_range(0..=u32::MAX);

    if IDS.contains(&id) {
        return get_id_with_count(iteration + 1);
    }
    
    return id;
}

pub async fn new_connection(
    api: &API,
    tx: Sender<ConnectionMessage>,
    offer: String,
) -> Option<(Arc<RTCPeerConnection>, u32, String)> {
    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }],
        ice_transport_policy: RTCIceTransportPolicy::All,
        bundle_policy: RTCBundlePolicy::Balanced,
        rtcp_mux_policy: RTCRtcpMuxPolicy::Require,
        ..Default::default()
    };

    let pc = api.new_peer_connection(config).await.ok();

    if pc.is_none() {
        return None;
    }

    let id = get_id();

    let pc = Arc::new(pc.unwrap());

    pc.on_ice_candidate(Box::new(move |candidate: Option<RTCIceCandidate>| {
        let tx2 = tx.clone();

        Box::pin(async move {
            if let Some(candidate) = candidate {
                if let Ok(candidate_json) = candidate.to_json() {
                    let (resp_tx, resp_rx) = oneshot::channel();

                    let _ = tx2
                        .send(ConnectionMessage::AddLocalCandidate {
                            id,
                            candidate: candidate_json,
                            resp: resp_tx,
                        })
                        .await;

                    let _ = resp_rx.await;
                }
            }
        })
    }));

    pc.on_signaling_state_change(Box::new(move |s: RTCSignalingState| {
        println!("Signaling State has changed: {s}");
        Box::pin(async {})
    }));

    pc.on_ice_gathering_state_change(Box::new(move |s: RTCIceGathererState| {
        println!("Ice Gathering State has changed: {s}");

        Box::pin(async {})
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

    let session_desc = RTCSessionDescription::offer(offer.clone()).ok();

    if session_desc.is_none() {
        return None;
    }

    let session_desc = session_desc.unwrap();

    let _ = pc.set_remote_description(session_desc).await.ok();

    let answer = pc.create_answer(None).await.ok();

    if answer.is_none() {
        return None;
    }

    let answer = answer.unwrap();

    let _ = pc.set_local_description(answer.clone()).await; // dammit

    return Some((pc, id, answer.sdp));
}
