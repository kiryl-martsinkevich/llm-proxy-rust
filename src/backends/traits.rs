use crate::types::Result;
use async_trait::async_trait;
use bytes::Bytes;
use http::HeaderMap;

#[async_trait]
pub trait Backend: Send + Sync {
    async fn send_request(
        &self,
        headers: HeaderMap,
        body: Bytes,
    ) -> Result<(u16, HeaderMap, Bytes)>;
}
