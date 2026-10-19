use kube::config::Kubeconfig;
use nu_protocol::{Span, Value};
use serde::Serialize;
use serde_json::Value as Json;

use crate::conversions::json_to_nu;

/// Any kubeconfig piece (the whole file, a named cluster/context/user) as a
/// nushell value, with credentials masked unless `raw`.
pub fn to_value<T: Serialize>(item: &T, raw: bool, span: Span) -> Value {
    let mut json = serde_json::to_value(item).unwrap_or_default();
    if !raw {
        redact(&mut json);
    }
    json_to_nu(&json, span)
}

/// Mask credentials the way `kubectl config view` does without `--raw`:
/// certificate data becomes `DATA+OMITTED`, keys and secrets `REDACTED`.
/// Auth-provider config entries (OIDC id/refresh tokens, client secrets) are
/// masked too. Exec plugin settings are left alone: they configure a command,
/// they do not hold its output.
fn redact(json: &mut Json) {
    match json {
        Json::Object(map) => {
            for (key, value) in map.iter_mut() {
                match key.as_str() {
                    "client-certificate-data" | "certificate-authority-data" if value.is_string() => {
                        *value = "DATA+OMITTED".into()
                    }
                    "client-key-data" | "token" | "password" if value.is_string() => {
                        *value = "REDACTED".into()
                    }
                    "auth-provider" => redact_auth_provider(value),
                    _ => redact(value),
                }
            }
        }
        Json::Array(items) => items.iter_mut().for_each(redact),
        _ => {}
    }
}

fn redact_auth_provider(provider: &mut Json) {
    let Some(config) = provider.get_mut("config").and_then(Json::as_object_mut) else { return };
    for (key, value) in config.iter_mut() {
        let key = key.to_lowercase();
        if key.contains("token") || key.contains("secret") {
            *value = "REDACTED".into();
        }
    }
}

/// Resolve a context name: use the explicit flag, or fall back to current-context.
pub fn resolve_context_name(
    kc: &Kubeconfig,
    context_flag: Option<String>,
) -> Result<String, nu_protocol::LabeledError> {
    context_flag
        .or_else(|| kc.current_context.clone())
        .ok_or_else(|| nu_protocol::LabeledError::new("no current context set"))
}
