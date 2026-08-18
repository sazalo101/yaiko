//! Typed RPC envelope primitives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RpcError {
    InvalidMethod,
    PayloadTooLarge,
    InvalidId,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RpcRequest {
    pub id: String,
    pub method: String,
    pub payload: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RpcResponse {
    Success {
        id: String,
        payload: Vec<u8>,
    },
    Failure {
        id: String,
        code: String,
        message: String,
    },
}
#[derive(Debug, Clone)]
pub struct RpcFacade {
    max_payload: usize,
}
impl RpcFacade {
    pub fn new(max_payload: usize) -> Self {
        Self { max_payload }
    }
    pub fn request(
        &self,
        id: impl Into<String>,
        method: impl Into<String>,
        payload: impl Into<Vec<u8>>,
    ) -> Result<RpcRequest, RpcError> {
        let id = id.into();
        let method = method.into();
        let payload = payload.into();
        if id.is_empty()
            || id.len() > 128
            || id.chars().any(|c| c.is_control() || c.is_whitespace())
        {
            return Err(RpcError::InvalidId);
        }
        if method.is_empty()
            || method.len() > 128
            || method.chars().any(|c| {
                c.is_control()
                    || c.is_whitespace()
                    || (!c.is_ascii_alphanumeric() && c != '.' && c != '_' && c != '-')
            })
        {
            return Err(RpcError::InvalidMethod);
        }
        if payload.len() > self.max_payload {
            return Err(RpcError::PayloadTooLarge);
        }
        Ok(RpcRequest {
            id,
            method,
            payload,
        })
    }
    pub fn success(
        &self,
        request: &RpcRequest,
        payload: impl Into<Vec<u8>>,
    ) -> Result<RpcResponse, RpcError> {
        let payload = payload.into();
        if payload.len() > self.max_payload {
            return Err(RpcError::PayloadTooLarge);
        }
        Ok(RpcResponse::Success {
            id: request.id.clone(),
            payload,
        })
    }
    pub fn failure(
        &self,
        request: &RpcRequest,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> RpcResponse {
        RpcResponse::Failure {
            id: request.id.clone(),
            code: code.into(),
            message: message.into(),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds_success_and_failure_responses() {
        let f = RpcFacade::new(8);
        let r = f.request("1", "media.render", b"x".to_vec()).unwrap();
        assert_eq!(
            f.success(&r, b"ok".to_vec()).unwrap(),
            RpcResponse::Success {
                id: "1".into(),
                payload: b"ok".to_vec()
            }
        );
        assert_eq!(
            f.failure(&r, "not_found", "missing"),
            RpcResponse::Failure {
                id: "1".into(),
                code: "not_found".into(),
                message: "missing".into()
            }
        )
    }
    #[test]
    fn validates_methods_ids_and_payloads() {
        let f = RpcFacade::new(1);
        assert!(f.request("1", "bad method", Vec::new()).is_err());
        assert!(f.request("1", "ok", b"xx".to_vec()).is_err());
        assert!(f.request("bad id", "ok", Vec::new()).is_err())
    }
}
