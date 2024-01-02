use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    MessageEvent, RequestInit, RequestMode, RtcDataChannelEvent, RtcPeerConnection,
    RtcPeerConnectionIceEvent, RtcSdpType, RtcSessionDescriptionInit, Request, Response,
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

#[wasm_bindgen]
pub async fn start() -> Result<(), JsValue> {
    let mut opts = RequestInit::new();
    opts.method("GET");

    let request = Request::new_with_str_and_init("https://symmetrical-winner-j4px56p667539j4-3000.app.github.dev/", &opts)?;

    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;


    // `resp_value` is a `Response` object.
    assert!(resp_value.is_instance_of::<Response>());
    let resp: Response = resp_value.dyn_into().unwrap();

    console_log!("{:?}",resp);


    Ok(())
}
