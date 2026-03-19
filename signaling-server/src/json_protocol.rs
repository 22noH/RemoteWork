use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::Message;

/// Top-level JSON envelope sent and received by the TypeScript viewer client.
///
/// Examples inbound:
///   {"type":"connect_request","payload":{...}}
///   {"type":"sdp_offer","payload":{...}}
///
/// Examples outbound:
///   {"type":"connect_response","payload":{...}}
///   {"type":"sdp_answer","payload":{...}}
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub payload: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse a raw JSON text string from the wire into a `JsonMessage`.
pub fn parse_json_message(text: &str) -> anyhow::Result<JsonMessage> {
    let msg: JsonMessage = serde_json::from_str(text)?;
    Ok(msg)
}

// ---------------------------------------------------------------------------
// Serialisation helpers — each builder returns a ready-to-send JSON string.
// All field names match exactly what the TypeScript viewer expects.
// ---------------------------------------------------------------------------

/// Wrap a serialisable value as a `{"type":…,"payload":…}` string.
fn build<T: Serialize>(msg_type: &str, payload: T) -> String {
    let msg = serde_json::json!({
        "type": msg_type,
        "payload": payload,
    });
    msg.to_string()
}

/// `{"type":"register_ack","payload":{"host_id":"…","success":true}}`
pub fn json_register_ack(host_id: String, success: bool) -> String {
    build("register_ack", serde_json::json!({
        "host_id": host_id,
        "success": success,
    }))
}

/// `{"type":"connect_response","payload":{"accepted":…,"session_token":"…","error_message":"…"}}`
pub fn json_connect_response(accepted: bool, session_token: String, error_message: String) -> String {
    build("connect_response", serde_json::json!({
        "accepted": accepted,
        "session_token": session_token,
        "error_message": error_message,
    }))
}

/// `{"type":"incoming_connection","payload":{"viewer_session_id":"…"}}`
pub fn json_incoming_connection(viewer_session_id: String) -> String {
    build("incoming_connection", serde_json::json!({
        "viewer_session_id": viewer_session_id,
    }))
}

/// `{"type":"sdp_offer","payload":{"sdp":"…","session_token":"…"}}`
pub fn json_sdp_offer(sdp: String, session_token: String) -> String {
    build("sdp_offer", serde_json::json!({
        "sdp": sdp,
        "session_token": session_token,
    }))
}

/// `{"type":"sdp_answer","payload":{"sdp":"…","session_token":"…"}}`
pub fn json_sdp_answer(sdp: String, session_token: String) -> String {
    build("sdp_answer", serde_json::json!({
        "sdp": sdp,
        "session_token": session_token,
    }))
}

/// `{"type":"ice_candidate","payload":{"candidate":"…","sdp_mid":"…","sdp_mline_index":0,"session_token":"…"}}`
pub fn json_ice_candidate(
    candidate: String,
    sdp_mid: String,
    sdp_mline_index: i32,
    session_token: String,
) -> String {
    build("ice_candidate", serde_json::json!({
        "candidate": candidate,
        "sdp_mid": sdp_mid,
        "sdp_mline_index": sdp_mline_index,
        "session_token": session_token,
    }))
}

/// `{"type":"pong","payload":{"timestamp_ms":…}}`
pub fn json_pong(timestamp_ms: u64) -> String {
    build("pong", serde_json::json!({
        "timestamp_ms": timestamp_ms,
    }))
}

/// `{"type":"error","payload":{"code":"…","message":"…"}}`
pub fn json_error(code: String, message: String) -> String {
    build("error", serde_json::json!({
        "code": code,
        "message": message,
    }))
}

// ---------------------------------------------------------------------------
// Wire helpers
// ---------------------------------------------------------------------------

/// Serialise a `JsonMessage` (already constructed) into a WebSocket text frame.
pub fn to_ws_message(msg: JsonMessage) -> Message {
    let text = serde_json::to_string(&msg)
        .unwrap_or_else(|_| r#"{"type":"error","payload":{"code":"SERIALISE","message":"internal"}}"#.to_string());
    Message::Text(text)
}

/// Convert a pre-built JSON string directly into a WebSocket text frame.
pub fn str_to_ws_message(json: String) -> Message {
    Message::Text(json)
}
