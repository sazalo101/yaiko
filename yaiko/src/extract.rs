//! Typed request extraction primitives for Yaiko handlers.

use crate::{AppError, AppResult, Request};
use serde::de::DeserializeOwned;

/// Trait implemented by values that can be extracted from an inbound request.
#[async_trait::async_trait]
pub trait FromRequest: Sized {
    async fn from_request(request: &mut Request) -> AppResult<Self>;
}

/// Extract and deserialize a JSON request body.
pub struct Json<T>(pub T);

#[async_trait::async_trait]
impl<T> FromRequest for Json<T>
where
    T: DeserializeOwned + Send,
{
    async fn from_request(request: &mut Request) -> AppResult<Self> {
        let value = request.json().await.map_err(AppError::from)?;
        serde_json::from_value(value)
            .map(Json)
            .map_err(AppError::from)
    }
}

/// Extract and deserialize query parameters.
pub struct Query<T>(pub T);

#[async_trait::async_trait]
impl<T> FromRequest for Query<T>
where
    T: DeserializeOwned + Send,
{
    async fn from_request(request: &mut Request) -> AppResult<Self> {
        serde_json::to_value(&request.query)
            .map_err(AppError::from)
            .and_then(|value| {
                serde_json::from_value(value)
                    .map(Query)
                    .map_err(AppError::from)
            })
    }
}

/// Extract and deserialize route path parameters.
pub struct Path<T>(pub T);

#[async_trait::async_trait]
impl<T> FromRequest for Path<T>
where
    T: DeserializeOwned + Send,
{
    async fn from_request(request: &mut Request) -> AppResult<Self> {
        serde_json::to_value(&request.params)
            .map_err(AppError::from)
            .and_then(|value| {
                serde_json::from_value(value)
                    .map(Path)
                    .map_err(AppError::from)
            })
    }
}

/// Extract URL-encoded form fields.
pub struct Form<T>(pub T);

#[async_trait::async_trait]
impl<T> FromRequest for Form<T>
where
    T: DeserializeOwned + Send,
{
    async fn from_request(request: &mut Request) -> AppResult<Self> {
        let fields = request.form_data().await.map_err(AppError::from)?;
        serde_json::to_value(fields)
            .map_err(AppError::from)
            .and_then(|value| {
                serde_json::from_value(value)
                    .map(Form)
                    .map_err(AppError::from)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::Body;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Payload {
        title: String,
    }

    #[tokio::test]
    async fn json_extractor_deserializes_request_body() {
        let hyper_request = hyper::Request::builder()
            .method("POST")
            .uri("/items")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"title":"hello"}"#))
            .unwrap();
        let mut request = Request::from_hyper(hyper_request).await.unwrap();

        let Json(payload) = Json::<Payload>::from_request(&mut request).await.unwrap();
        assert_eq!(
            payload,
            Payload {
                title: "hello".into()
            }
        );
    }

    #[tokio::test]
    async fn query_extractor_deserializes_query_map() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Filters {
            page: String,
        }

        let hyper_request = hyper::Request::builder()
            .uri("/items?page=2")
            .body(Body::empty())
            .unwrap();
        let mut request = Request::from_hyper(hyper_request).await.unwrap();
        let Query(filters) = Query::<Filters>::from_request(&mut request).await.unwrap();
        assert_eq!(filters, Filters { page: "2".into() });
    }
}
