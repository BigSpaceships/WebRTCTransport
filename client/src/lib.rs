use gloo_utils::format::JsValueSerdeExt;
use js_sys::{
    Object, Reflect,
    JSON::{self, stringify},
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::{convert::FromWasmAbi, prelude::*};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    MessageEvent, Request, RequestInit, RequestMode, Response, RtcDataChannelEvent,
    RtcPeerConnection, RtcPeerConnectionIceEvent, RtcSdpType, RtcSessionDescriptionInit,
};

macro_rules! console_log {
    //($($t:tt)*) => (log(&format_args!($($t)*).to_string()))

    ($($t:tt)*) => (alert(&format_args!($($t)*).to_string()))
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
    sdp_mline_index: &'a str,
}

#[wasm_bindgen]
pub async fn start() -> Result<(), JsValue> {
    let pc = RtcPeerConnection::new()?;
    // console_log!("pc created: state {:?}", pc.signaling_state());

    let dc = pc.create_data_channel("data");
    // console_log!("data channel created: label {:?}", dc.label());

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

    let on_ice_candidate_candidate_callback =
        Closure::<dyn FnMut(_)>::new(move |ev: RtcPeerConnectionIceEvent| {
            if let Some(candidate) = ev.candidate() {
                let mut opts = RequestInit::new();
                opts.method("POST");

                let candidate_str = candidate.candidate();
                let sdp_mid = candidate.sdp_mid().unwrap_or(String::from(""));
                let sdp_mline_index = candidate.sdp_m_line_index().unwrap_or(0).to_string();

                let candidate_obj = IceCandidateInit {
                    candidate: candidate_str.as_str(),
                    sdp_mid: sdp_mid.as_str(),
                    sdp_mline_index: sdp_mline_index.as_str(),
                };

                let a = JsValue::from_serde(&candidate_obj).unwrap();

                let b = JSON::stringify(&a).ok();

                opts.body(JSON::stringify(&a).ok().map(|s| s.into()).as_ref());

                console_log!("{:?}", b);

                //opts.body();

                let url = "/api/ice_candidate";

                let window = web_sys::window().unwrap();

                if let Ok(request) = Request::new_with_str_and_init(&url, &opts) {
                    let _ = window.fetch_with_request(&request);
                }
            }
        });

    pc.set_onicecandidate(Some(
        on_ice_candidate_candidate_callback.as_ref().unchecked_ref(),
    ));

    on_ice_candidate_candidate_callback.forget();

    let mut opts = RequestInit::new();
    opts.method("POST");

    opts.body(Some(&(offer_sdp.into())));

    let url = "/api/new_offer";

    let request = Request::new_with_str_and_init(&url, &opts)?;

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
