use std::sync::Arc;


use tokio::sync::{
    mpsc::{Sender, self},
    oneshot,
};
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
    Error,
};

use super::manager::{get_id, ConnectionMessage};

pub struct Connection {
    pc: Arc<RTCPeerConnection>,
    tx: Sender<ConnectionMessage>,
    pub id: u32,
}

impl Connection {
    pub async fn new(api: &API, tx: Sender<ConnectionMessage>) -> Result<Connection, Error> {
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

        let pc = api.new_peer_connection(config).await;

        let id = get_id();

        let pc = Arc::new(pc.unwrap());

        let tx2 = tx.clone();
        pc.on_ice_candidate(Box::new(move |candidate: Option<RTCIceCandidate>| {
            let tx2 = tx2.clone();

            Box::pin(async move {
                if let Some(candidate) = candidate {
                    if let Ok(candidate_json) = candidate.to_json() {
                        let (resp_tx, resp_rx) = oneshot::channel();

                        let _ = tx2.send(ConnectionMessage::AddLocalCandidate {
                            id,
                            candidate: candidate_json,
                            resp: resp_tx,
                        }).await;

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

        let tx_close = tx.clone();
        pc.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
            println!("Peer Connection State has changed: {s}");

            if s == RTCPeerConnectionState::Failed {
                println!("Peer Connection has gone to failed exiting");
            }

            let tx_close = tx_close.clone();

            Box::pin(async move {
                if s == RTCPeerConnectionState::Disconnected {
                    let _ = tx_close.send(ConnectionMessage::ConnectionClosed { id: id.clone() }).await;
                }
            })
        }));


        Ok(Connection { pc, tx, id })
    }

    pub async fn connect(&self, offer: String) -> Option<String> {
        let (data_tx, mut data_rx) = mpsc::channel::<String>(1);

        self.pc.on_data_channel(Box::new(move |d: Arc<RTCDataChannel>| {
            let d_label = d.label().to_owned();
            let d_id = d.id();
            println!("New data channel {d_label} {d_id}");

            let data_tx = data_tx.clone();

            Box::pin(async move {
                let d2 = Arc::clone(&d);
                let d_label2 = d_label.clone();

                d.on_open(Box::new(move || {
                    println!("Data channel {d_label} {d_id} opened");

                    Box::pin(async move {
                        let _ = data_tx.send("HII".to_string()).await;
                        let _ = d2.send_text("WAAA").await;
                    })
                }));

                d.on_message(Box::new(move |msg: DataChannelMessage| {
                    let msg_str = String::from_utf8(msg.data.to_vec()).unwrap();

                    println!("Message from data channel '{d_label2}': '{msg_str}'");

                    Box::pin(async {
                    })
                }));
            })
        }));

        let session_desc = RTCSessionDescription::offer(offer.clone()).ok()?;

        let _ = self.pc.set_remote_description(session_desc).await.ok()?;

        let answer = self.pc.create_answer(None).await.ok()?;

        let _ = self.pc.set_local_description(answer.clone()).await; // dammit
        
        if let Some(dc) = data_rx.recv().await {
            println!("dc");
        }

        Some(answer.sdp)
    }

    pub async fn add_ice_candidate(&self, candidate: RTCIceCandidateInit) -> Result<(), Error> {
        self.pc.add_ice_candidate(candidate).await
    }

    pub async fn close(&self) {
        let _ = self.pc.close().await;
    }
}
