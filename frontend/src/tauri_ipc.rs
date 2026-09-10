use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["__TAURI__", "core"], catch)]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

pub async fn call_tauri<T: Serialize, R: DeserializeOwned>(cmd: &str, args: &T) -> Result<R, String> {
    let js_val = serde_wasm_bindgen::to_value(args).map_err(|e| e.to_string())?;
    let response = invoke(cmd, js_val)
        .await
        .map_err(|e| format!("{:?}", e))?;
    serde_wasm_bindgen::from_value(response).map_err(|e| e.to_string())
}
