use hyper::{Body, Response as HyperResponse, StatusCode};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Response {
    pub status: StatusCode,
    pub headers: HashMap<String, String>,
    pub body: Body,
}

impl Response {
    const SET_COOKIE_SEPARATOR: &'static str = "\n";

    pub fn new() -> Self {
        Response {
            status: StatusCode::OK,
            headers: HashMap::new(),
            body: Body::empty(),
        }
    }

    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    pub fn header(mut self, key: &str, value: &str) -> Self {
        if key.eq_ignore_ascii_case("set-cookie") {
            return self.set_cookie_raw(value);
        }
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn json<T: Serialize>(mut self, data: &T) -> Result<Self, serde_json::Error> {
        let json_str = serde_json::to_string(data)?;
        self.headers.insert("Content-Length".to_string(), json_str.len().to_string());
        self.headers.insert("Content-Type".to_string(), "application/json".to_string());
        self.body = Body::from(json_str);
        Ok(self)
    }

    pub fn text(mut self, text: &str) -> Self {
        self.headers.insert("Content-Length".to_string(), text.len().to_string());
        self.headers.insert("Content-Type".to_string(), "text/plain".to_string());
        self.body = Body::from(text.to_string());
        self
    }

    pub fn html(mut self, html: &str) -> Self {
        self.headers.insert("Content-Length".to_string(), html.len().to_string());
        self.headers.insert("Content-Type".to_string(), "text/html".to_string());
        self.body = Body::from(html.to_string());
        self
    }

    pub fn body(mut self, body: Body) -> Self {
        self.body = body;
        self
    }

    pub fn redirect(mut self, location: &str) -> Self {
        self.status = StatusCode::FOUND;
        self.headers.insert("Location".to_string(), location.to_string());
        self
    }

    /// 204 No Content — successful request with no response body
    pub fn no_content() -> Self {
        Response {
            status: StatusCode::NO_CONTENT,
            headers: HashMap::new(),
            body: Body::empty(),
        }
    }

    /// 301 Moved Permanently redirect
    pub fn redirect_permanent(mut self, location: &str) -> Self {
        self.status = StatusCode::MOVED_PERMANENTLY;
        self.headers.insert("Location".to_string(), location.to_string());
        self
    }

    /// Chunked transfer-encoded stream from an AsyncRead source
    pub fn stream<R>(mut self, reader: R) -> Self
    where
        R: tokio::io::AsyncRead + Send + 'static,
    {
        let stream = tokio_util::io::ReaderStream::new(reader);
        self.body = Body::wrap_stream(stream);
        self.headers.insert("Transfer-Encoding".to_string(), "chunked".to_string());
        self
    }

    /// Server-Sent Events stream from a tokio mpsc Receiver
    pub fn event_stream(mut self, mut rx: tokio::sync::mpsc::Receiver<String>) -> Self {
        let (mut sender, body) = Body::channel();
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let formatted = format!("data: {}\n\n", event);
                if sender.send_data(hyper::body::Bytes::from(formatted)).await.is_err() {
                    break;
                }
            }
        });
        self.body = body;
        self.headers.insert("Content-Type".to_string(), "text/event-stream".to_string());
        self.headers.insert("Cache-Control".to_string(), "no-cache".to_string());
        self.headers.insert("Connection".to_string(), "keep-alive".to_string());
        self
    }

    /// Set a cookie with common options
    pub fn set_cookie(self, name: &str, value: &str) -> Self {
        let cookie = format!(
            "{}={}; Path=/; HttpOnly; SameSite=Lax",
            name, value
        );
        self.set_cookie_raw(&cookie)
    }

    pub fn set_cookie_raw(mut self, cookie: &str) -> Self {
        if let Some(existing) = self.headers.get_mut("Set-Cookie") {
            if !existing.is_empty() {
                existing.push_str(Self::SET_COOKIE_SEPARATOR);
            }
            existing.push_str(cookie);
        } else {
            self.headers
                .insert("Set-Cookie".to_string(), cookie.to_string());
        }
        self
    }

    pub fn into_hyper(self) -> HyperResponse<Body> {
        let mut response = HyperResponse::builder().status(self.status);
        
        for (key, value) in self.headers {
            if key.eq_ignore_ascii_case("set-cookie") {
                for cookie in value.split(Self::SET_COOKIE_SEPARATOR) {
                    response = response.header("Set-Cookie", cookie);
                }
            } else {
                response = response.header(key, value);
            }
        }
        
        response.body(self.body).unwrap()
    }
}

impl Default for Response {
    fn default() -> Self {
        Self::new()
    }
}
