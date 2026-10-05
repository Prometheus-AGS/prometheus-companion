use eventsource_stream::Eventsource;
use futures::StreamExt;
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use std::pin::Pin;
use tracing::debug;
use url::Url;

use crate::error::ClientError;
use crate::types::{AgUiEvent, SkillResult, SyncStatus};

/// Client for a running sovereign-sync node.
pub struct SovereignClient {
    base_url: Url,
    http: Client,
}

impl SovereignClient {
    /// Create a client pointing at the given base URL (e.g. "http://127.0.0.1:7892").
    pub fn new(base_url: &str) -> Result<Self, ClientError> {
        Ok(Self {
            base_url: Url::parse(base_url)?,
            http: Client::new(),
        })
    }

    /// Connect to the same-user control socket without proxy or redirect fallback.
    #[cfg(unix)]
    pub fn unix_socket(path: &std::path::Path) -> Result<Self, ClientError> {
        Ok(Self {
            base_url: Url::parse("http://localhost")?,
            http: Client::builder()
                .unix_socket(path)
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(std::time::Duration::from_secs(10))
                .build()?,
        })
    }

    fn url(&self, path: &str) -> Result<Url, ClientError> {
        Ok(self.base_url.join(path)?)
    }

    // -----------------------------------------------------------------------
    // REST API
    // -----------------------------------------------------------------------

    /// Check node health.
    pub async fn health(&self) -> Result<Value, ClientError> {
        let url = self.url("/health")?;
        debug!("GET {}", url);
        let resp = self.http.get(url).send().await?.error_for_status()?;
        Ok(resp.json().await?)
    }

    /// Search skills on the remote node.
    pub async fn search_skills(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SkillResult>, ClientError> {
        let url = self.url("/api/v1/skills/search")?;
        let resp = self
            .http
            .get(url)
            .query(&[("q", query), ("limit", &limit.to_string())])
            .send()
            .await?
            .error_for_status()?;
        let body: Value = resp.json().await?;
        let results = body
            .get("results")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();
        Ok(results)
    }

    /// Get current sync status from the remote node.
    pub async fn sync_status(&self) -> Result<SyncStatus, ClientError> {
        let url = self.url("/api/v1/sync/status")?;
        let resp = self.http.get(url).send().await?.error_for_status()?;
        Ok(resp.json().await?)
    }

    /// Preserve the current host's complete transport/domain status schema.
    pub async fn sync_status_value(&self) -> Result<Value, ClientError> {
        let response = self.http.get(self.url("/api/v1/sync/status")?)
            .send().await?.error_for_status()?;
        Ok(response.json().await?)
    }

    pub async fn sync_peers(&self) -> Result<Value, ClientError> {
        let response = self.http.get(self.url("/api/v1/sync/peers")?)
            .send().await?.error_for_status()?;
        Ok(response.json().await?)
    }

    /// Submit an already signed v2 request once. The caller retains the exact
    /// request for receipt lookup/replay if transport or response decoding fails.
    /// HTTP errors retain their structured body rather than losing a request ID.
    pub async fn sync_push_signed<T: Serialize + ?Sized>(
        &self,
        request: &T,
    ) -> Result<(u16, Value), ClientError> {
        let response = self.http.post(self.url("/api/v2/sync/pushes")?)
            .json(request).send().await?;
        let status = response.status().as_u16();
        Ok((status, response.json().await?))
    }

    pub async fn sync_push_receipt(&self, request_id: &str) -> Result<(u16, Value), ClientError> {
        let mut url = self.url("/api/v2/sync/pushes/")?;
        url.path_segments_mut()
            .map_err(|_| ClientError::Api("control URL cannot accept path segments".into()))?
            .pop_if_empty().push(request_id);
        let response = self.http.get(url).send().await?;
        let status = response.status().as_u16();
        Ok((status, response.json().await?))
    }

    /// Deprecated unsigned v1 helper; same-user Unix transport only. Prefer
    /// `sync_push_signed` and keep its request identity through uncertainty.
    pub async fn sync_push(&self, domain: &str) -> Result<Value, ClientError> {
        let url = self.url("/api/v1/sync/push")?;
        let resp = self
            .http
            .post(url)
            .json(&serde_json::json!({ "domain": domain }))
            .send()
            .await?
            .error_for_status()?;
        Ok(resp.json().await?)
    }

    pub async fn kbd_status(
        &self,
        project_id: &str,
    ) -> Result<kbd_runtime::KbdStateV2, ClientError> {
        let url = self.url(&format!("/api/v1/kbd/projects/{project_id}/status"))?;
        let resp = self.http.get(url).send().await?.error_for_status()?;
        Ok(resp.json().await?)
    }

    pub async fn kbd_projects(&self) -> Result<Value, ClientError> {
        let url = self.url("/api/v1/kbd/projects")?;
        let resp = self.http.get(url).send().await?.error_for_status()?;
        Ok(resp.json().await?)
    }

    pub async fn kbd_replicas(&self, project_id: &str) -> Result<Value, ClientError> {
        let url = self.url(&format!("/api/v1/kbd/projects/{project_id}/replicas"))?;
        let resp = self.http.get(url).send().await?.error_for_status()?;
        Ok(resp.json().await?)
    }

    pub async fn plan_kbd_adoption(
        &self,
        path: &str,
        into_project_id: &str,
    ) -> Result<Value, ClientError> {
        self.kbd_adoption(path, into_project_id, false).await
    }

    pub async fn apply_kbd_adoption(
        &self,
        path: &str,
        into_project_id: &str,
    ) -> Result<Value, ClientError> {
        self.kbd_adoption(path, into_project_id, true).await
    }

    async fn kbd_adoption(
        &self,
        path: &str,
        into_project_id: &str,
        apply: bool,
    ) -> Result<Value, ClientError> {
        let url = self.url("/api/v1/kbd/projects/adopt")?;
        let resp = self
            .http
            .post(url)
            .json(&serde_json::json!({
                "path": path,
                "intoProjectId": into_project_id,
                "apply": apply
            }))
            .send()
            .await?
            .error_for_status()?;
        Ok(resp.json().await?)
    }

    pub async fn submit_kbd_command(
        &self,
        project_id: &str,
        command: &kbd_runtime::SignedCommandEnvelope,
    ) -> Result<kbd_runtime::CommandResult, ClientError> {
        let url = self.url(&format!("/api/v1/kbd/projects/{project_id}/commands"))?;
        let resp = self
            .http
            .post(url)
            .json(command)
            .send()
            .await?
            .error_for_status()?;
        let body: Value = resp.json().await?;
        serde_json::from_value(body).map_err(ClientError::Json)
    }

    pub async fn kbd_claims(&self, project_id: &str) -> Result<Value, ClientError> {
        let url = self.url(&format!("/api/v1/kbd/projects/{project_id}/claims"))?;
        let resp = self.http.get(url).send().await?.error_for_status()?;
        Ok(resp.json().await?)
    }

    pub async fn kbd_conflicts(&self, project_id: &str) -> Result<Value, ClientError> {
        let url = self.url(&format!("/api/v1/kbd/projects/{project_id}/conflicts"))?;
        let resp = self.http.get(url).send().await?.error_for_status()?;
        Ok(resp.json().await?)
    }

    pub async fn kbd_submodules(&self, project_id: &str) -> Result<Value, ClientError> {
        let url = self.url(&format!("/api/v1/kbd/projects/{project_id}/submodules"))?;
        let resp = self.http.get(url).send().await?.error_for_status()?;
        Ok(resp.json().await?)
    }

    pub async fn export_kbd_audit(&self, project_id: &str) -> Result<Vec<u8>, ClientError> {
        let url = self.url(&format!("/api/v1/kbd/projects/{project_id}/audit"))?;
        let resp = self.http.get(url).send().await?.error_for_status()?;
        Ok(resp.bytes().await?.to_vec())
    }

    // -----------------------------------------------------------------------
    // AG-UI SSE stream
    // -----------------------------------------------------------------------

    /// Submit an A2UI task and stream AG-UI events until completion.
    ///
    /// Returns a stream of `AgUiEvent`s. The caller drives it with `.next()`.
    pub async fn stream_task(
        &self,
        task: Value,
    ) -> Result<
        Pin<Box<dyn futures::Stream<Item = Result<AgUiEvent, ClientError>> + Send>>,
        ClientError,
    > {
        let url = self.url("/api/v1/stream")?;
        let resp = self
            .http
            .post(url)
            .json(&task)
            .send()
            .await?
            .error_for_status()?;

        let byte_stream = resp.bytes_stream();
        let event_stream = byte_stream.eventsource().map(|result| match result {
            Ok(event) => serde_json::from_str::<AgUiEvent>(&event.data).map_err(ClientError::Json),
            Err(e) => Err(ClientError::Stream(e.to_string())),
        });

        Ok(Box::pin(event_stream))
    }

    /// Subscribe to continuous typed KBD/AG-UI operational events.
    pub async fn stream_events(
        &self,
    ) -> Result<
        Pin<Box<dyn futures::Stream<Item = Result<AgUiEvent, ClientError>> + Send>>,
        ClientError,
    > {
        let url = self.url("/api/v1/events")?;
        let resp = self.http.get(url).send().await?.error_for_status()?;
        let event_stream = resp
            .bytes_stream()
            .eventsource()
            .map(|result| match result {
                Ok(event) => {
                    serde_json::from_str::<AgUiEvent>(&event.data).map_err(ClientError::Json)
                }
                Err(error) => Err(ClientError::Stream(error.to_string())),
            });
        Ok(Box::pin(event_stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_construction_with_valid_url() {
        let client = SovereignClient::new("http://127.0.0.1:7892");
        assert!(client.is_ok());
    }

    #[test]
    fn client_construction_with_invalid_url() {
        let client = SovereignClient::new("not-a-url");
        assert!(client.is_err());
    }

    #[test]
    fn url_joining_works() {
        let client = SovereignClient::new("http://127.0.0.1:7892").unwrap();
        let url = client.url("/health").unwrap();
        assert_eq!(url.as_str(), "http://127.0.0.1:7892/health");
    }
}
