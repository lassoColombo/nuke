pub mod api_resources;
pub mod api_versions;
pub mod config;
pub mod get;
pub mod http_get;
pub mod rollout_status;
pub mod top;

use kube::Client;
use nu_protocol::LabeledError;
use serde_json::Value as Json;

/// GET `path` (`/api/v1/namespaces/x/pods/web`) from the apiserver as
/// JSON: every command reads objects in this one representation, the one
/// `nuke get` and the column layer read.
pub async fn get_json(client: &Client, path: &str) -> anyhow::Result<Json> {
    let request = http::Request::get(path).body(Vec::new())?;
    Ok(client.request(request).await?)
}

/// Render an error for the user: every cause in the chain, outermost first,
/// each said once.
///
/// kube's `Error::Api` prints a Debug dump of the whole `Status` after the
/// message; show the server's own `message: reason` instead. Causes whose text
/// the previous one already includes (thiserror's `{0}` source embedding) are
/// skipped.
pub fn to_labeled(e: anyhow::Error) -> LabeledError {
    let mut parts: Vec<String> = Vec::new();
    for cause in e.chain() {
        let text = match cause.downcast_ref::<kube::Error>() {
            Some(kube::Error::Api(status)) => status.to_string(),
            _ => cause.to_string(),
        };
        if !parts.last().is_some_and(|prev| prev.contains(&text)) {
            parts.push(text);
        }
        if matches!(
            cause.downcast_ref::<kube::Error>(),
            Some(kube::Error::Api(_))
        ) {
            break;
        }
    }
    LabeledError::new(parts.join(": "))
}
