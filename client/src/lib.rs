use gloo_utils::format::JsValueSerdeExt;
use js_sys::{
    Object, Reflect,
    JSON::{self, stringify}, Array,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::{convert::FromWasmAbi, prelude::*};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    MessageEvent, Request, RequestInit, RequestMode, Response, RtcDataChannelEvent,
    RtcPeerConnection, RtcPeerConnectionIceEvent, RtcSdpType, RtcSessionDescriptionInit, RtcConfiguration, RtcIceServer,
};

macro_rules! console_log {
    //($($t:tt)*) => (log(&format_args!($($t)*).to_string()))

    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}
macro_rules! console_warn {
    ($($t:tt)*) => (warn(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    #[wasm_bindgen(js_namespace = console)]
    fn warn(s: &str);

    fn alert(s: &str);
}

#[derive(Serialize, Deserialize)]
struct IceCandidateInit<'a> {
    candidate: &'a str,
    sdp_mid: &'a str,
    sdp_mline_index: u16,
}

#[derive(Serialize, Deserialize)]
struct OfferDescription<'a> {
    sdp: &'a str,
}

#[wasm_bindgen]
pub async fn start() -> Result<(), JsValue> {
    let mut config = RtcConfiguration::new();

    let ice_servers = Array::new();
    let ice_server_urls = Array::new();
    ice_server_urls.push(&JsValue::from_str("stun:stun.l.google.com:19302"));
    ice_servers.push(RtcIceServer::new().urls(&ice_server_urls));

    config.ice_servers(&ice_servers);

    let pc = RtcPeerConnection::new_with_configuration(&config)?;
     console_log!("{:?}", pc.get_configuration());

    let on_ice_candidate_candidate_callback =
        Closure::<dyn FnMut(_)>::new(move |ev: RtcPeerConnectionIceEvent| {
            if let Some(candidate) = ev.candidate() {
                let mut opts = RequestInit::new();
                opts.method("POST");

                let candidate_str = candidate.candidate();
                let sdp_mid = candidate.sdp_mid().unwrap_or(String::from(""));
                let sdp_mline_index = candidate.sdp_m_line_index().unwrap_or(0);

                let candidate_obj = IceCandidateInit {
                    candidate: candidate_str.as_str(),
                    sdp_mid: sdp_mid.as_str(),
                    sdp_mline_index,
                };

                let a = JsValue::from_serde(&candidate_obj).unwrap();

                opts.body(JSON::stringify(&a).ok().map(|s| s.into()).as_ref());

                let url = "/api/ice_candidate";

                let window = web_sys::window().unwrap();

                let request = Request::new_with_str_and_init(&url, &opts);

                if request.is_err() {
                    return;
                }

                let request = request.unwrap();

                let _ = request.headers().set("Content-Type", "application/json");
                let _ = window.fetch_with_request(&request);
            }
        });

    pc.set_onicecandidate(Some(
        on_ice_candidate_candidate_callback.as_ref().unchecked_ref(),
    ));

    on_ice_candidate_candidate_callback.forget();
    let dc = pc.create_data_channel("data");
    console_log!("data channel created: label {:?}", dc.label());

    let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |ev: MessageEvent| {
        if let Some(message) = ev.data().as_string() {
            console_log!("{:?}", message);
        }
    });

    dc.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();

    let offer = JsFuture::from(pc.create_offer()).await?;
    let offer_sdp = Reflect::get(&offer, &JsValue::from_str("sdp"))?
        .as_string()
        .unwrap();
    // console_log!("pc: offer {:?}", offer_sdp);

    let mut offer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
    offer_obj.sdp(&offer_sdp);
    let sld_promise = pc.set_local_description(&offer_obj);
    JsFuture::from(sld_promise).await?;
    // console_log!("pc: state {:?}", pc.signaling_state());

    let mut opts = RequestInit::new();
    opts.method("POST");

    let offer_description = OfferDescription { sdp: &offer_sdp };

    let offer_description_js_value = JsValue::from_serde(&offer_description).unwrap();

    opts.body(
        JSON::stringify(&offer_description_js_value)
            .ok()
            .map(|s| s.into())
            .as_ref(),
    );

    let url = "/api/new_offer";

    let request = Request::new_with_str_and_init(&url, &opts)?;
    let _ = request.headers().set("Content-Type", "application/json");

    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;

    // `resp_value` is a `Response` object.
    assert!(resp_value.is_instance_of::<Response>());
    let resp: Response = resp_value.dyn_into().unwrap();

    let json = JsFuture::from(resp.json()?).await?;

    let sdp = Reflect::get(&json, &"sdp".into())?.as_string().unwrap();

    let mut answer = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
    answer.sdp(&sdp);

    let set_answer_promise = pc.set_remote_description(&answer);
    JsFuture::from(set_answer_promise).await?;

    Ok(())
}
