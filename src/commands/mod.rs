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

/// The `items` of the list at `path`.
pub async fn list_json(client: &Client, path: &str) -> anyhow::Result<Vec<Json>> {
    Ok(crate::values::take_items(
        &mut get_json(client, path).await?,
    ))
}

/// Render an error for the user: every cause in the chain, outermost first,
/// each said once.
///
/// See [`describe`].
pub fn to_labeled(e: anyhow::Error) -> LabeledError {
    LabeledError::new(describe(&e))
}

/// An error as one line for the user, for a command's error and a
/// warning alike: every cause in the chain, outermost first, each said
/// once and trimmed.
///
/// kube's `Error::Api` prints a Debug dump of the whole `Status` after the
/// message; show the server's own `message: reason` instead. Causes whose text
/// the previous one already includes (thiserror's `{0}` source embedding) are
/// skipped.
pub fn describe(e: &anyhow::Error) -> String {
    let mut parts: Vec<String> = Vec::new();
    for cause in e.chain() {
        // The server's status ends the chain: what kube wraps it in only
        // repeats it.
        let (text, last) = match cause.downcast_ref::<kube::Error>() {
            Some(kube::Error::Api(status)) => {
                let message = status.message.trim();
                let text = match status.reason.trim() {
                    "" => message.to_string(),
                    reason => format!("{message}: {reason}"),
                };
                (text, true)
            }
            _ => (cause.to_string().trim().to_string(), false),
        };
        if !parts.last().is_some_and(|prev| prev.contains(&text)) {
            parts.push(text);
        }
        if last {
            break;
        }
    }
    parts.join(": ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A server status is its trimmed message and reason, not kube's
    /// Debug dump, and ends the chain; context before it stays.
    #[test]
    fn an_api_error_reads_as_the_servers_words() {
        let status = kube::core::Status {
            code: 403,
            message: "Forbidden\n".into(),
            reason: "Failed to parse error data".into(),
            ..Default::default()
        };
        let e = anyhow::Error::new(kube::Error::Api(status.boxed())).context("listing pods");
        assert_eq!(
            describe(&e),
            "listing pods: Forbidden: Failed to parse error data"
        );
        let e = anyhow::anyhow!("plain\n");
        assert_eq!(describe(&e), "plain");
    }
}
