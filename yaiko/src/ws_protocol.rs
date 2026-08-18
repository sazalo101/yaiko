//! Bounded, typed WebSocket message protocol helpers.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WsEnvelope {
    pub kind: String,
    pub request_id: Option<String>,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WsProtocolError {
    Empty,
    TooLarge,
    MalformedJson,
    MissingKind,
    InvalidKind,
    MissingPayload,
}

impl WsProtocolError {
    pub fn close_code(&self) -> u16 {
        match self {
            Self::TooLarge => 1009,
            Self::MalformedJson
            | Self::MissingKind
            | Self::InvalidKind
            | Self::MissingPayload
            | Self::Empty => 1003,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WsProtocol {
    max_frame_bytes: usize,
    allowed_kinds: Vec<String>,
}

impl WsProtocol {
    pub fn new(
        max_frame_bytes: usize,
        allowed_kinds: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            max_frame_bytes,
            allowed_kinds: allowed_kinds.into_iter().map(Into::into).collect(),
        }
    }
    pub fn decode(&self, bytes: &[u8]) -> Result<WsEnvelope, WsProtocolError> {
        if bytes.is_empty() {
            return Err(WsProtocolError::Empty);
        }
        if bytes.len() > self.max_frame_bytes {
            return Err(WsProtocolError::TooLarge);
        }
        let envelope: WsEnvelope =
            serde_json::from_slice(bytes).map_err(|_| WsProtocolError::MalformedJson)?;
        if envelope.kind.trim().is_empty() {
            return Err(WsProtocolError::MissingKind);
        }
        if !self.allowed_kinds.is_empty()
            && !self.allowed_kinds.iter().any(|kind| kind == &envelope.kind)
        {
            return Err(WsProtocolError::InvalidKind);
        }
        if envelope.payload.is_null() {
            return Err(WsProtocolError::MissingPayload);
        }
        Ok(envelope)
    }
    pub fn encode(&self, envelope: &WsEnvelope) -> Result<Vec<u8>, WsProtocolError> {
        let bytes = serde_json::to_vec(envelope).map_err(|_| WsProtocolError::MalformedJson)?;
        if bytes.len() > self.max_frame_bytes {
            return Err(WsProtocolError::TooLarge);
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn protocol() -> WsProtocol {
        WsProtocol::new(128, ["chat", "typing"])
    }

    #[test]
    fn encodes_and_decodes_typed_envelopes() {
        let protocol = protocol();
        let envelope = WsEnvelope {
            kind: "chat".into(),
            request_id: Some("r1".into()),
            payload: serde_json::json!({"text":"hello"}),
        };
        assert_eq!(
            protocol
                .decode(&protocol.encode(&envelope).unwrap())
                .unwrap(),
            envelope
        );
    }

    #[test]
    fn rejects_malformed_oversized_and_invalid_messages() {
        let protocol = protocol();
        assert_eq!(protocol.decode(b""), Err(WsProtocolError::Empty));
        assert_eq!(
            protocol.decode(b"not-json"),
            Err(WsProtocolError::MalformedJson)
        );
        let invalid = serde_json::json!({"kind":"unknown","payload":{}});
        assert_eq!(
            protocol.decode(&serde_json::to_vec(&invalid).unwrap()),
            Err(WsProtocolError::InvalidKind)
        );
        assert_eq!(WsProtocolError::TooLarge.close_code(), 1009);
    }

    #[test]
    fn enforces_frame_size_and_required_fields() {
        let protocol = WsProtocol::new(16, std::iter::empty::<String>());
        let payload = serde_json::json!({"kind":"chat","payload":{"value":"large"}});
        assert_eq!(
            protocol.decode(&serde_json::to_vec(&payload).unwrap()),
            Err(WsProtocolError::TooLarge)
        );
        let missing = serde_json::json!({"kind":"chat","payload":null});
        let permissive = WsProtocol::new(128, std::iter::empty::<String>());
        assert_eq!(
            permissive.decode(&serde_json::to_vec(&missing).unwrap()),
            Err(WsProtocolError::MissingPayload)
        );
    }
}
