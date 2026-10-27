use kube::config::{Context, Kubeconfig};
use nu_plugin::EvaluatedCall;
use nu_protocol::{IntoValue, LabeledError, PipelineData, Span, Value};
use serde::Serialize;
use serde_json::Value as Json;

use crate::kube_env::KubeEnv;

/// Any kubeconfig piece (the whole file, a named cluster/context/user) as a
/// nushell value, with credentials masked unless `raw`.
pub fn to_value<T: Serialize>(item: &T, raw: bool, span: Span) -> Value {
    let mut json = serde_json::to_value(item).unwrap_or_default();
    if !raw {
        redact(&mut json);
    }
    json.into_value(span)
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
                    "client-certificate-data" | "certificate-authority-data"
                        if value.is_string() =>
                    {
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
    let Some(config) = provider.get_mut("config").and_then(Json::as_object_mut) else {
        return;
    };
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
) -> Result<String, LabeledError> {
    context_flag
        .or_else(|| kc.current_context.clone())
        .ok_or_else(|| LabeledError::new("no current context set"))
}

/// A named section of the kubeconfig that contexts point into — users
/// (`auth_infos`) or clusters — as `get-users`/`get-clusters` list it.
pub struct NamedSection<T: 'static> {
    /// `"user"` / `"cluster"`, for error messages.
    pub noun: &'static str,
    /// The section's entries.
    pub all: fn(&Kubeconfig) -> &[T],
    /// An entry's name.
    pub name: fn(&T) -> &str,
    /// The entry a context points at.
    pub of_context: fn(&Context) -> Option<&str>,
}

impl<T: Serialize + 'static> NamedSection<T> {
    /// With `--current` or `--context`, the entry that context points at;
    /// otherwise every entry. Credentials masked unless `--raw`.
    pub fn run(&self, env: &KubeEnv, call: &EvaluatedCall) -> Result<PipelineData, LabeledError> {
        let span = call.head;
        let raw = call.has_flag("raw")?;
        let context_flag: Option<String> = call.get_flag("context")?;
        let kc = env.read_kubeconfig().map_err(crate::commands::to_labeled)?;
        let all = (self.all)(&kc);

        if call.has_flag("current")? || context_flag.is_some() {
            let ctx_name = resolve_context_name(&kc, context_flag)?;
            let not_found =
                |what: &str, name: &str| LabeledError::new(format!("{what} '{name}' not found"));
            let name = kc
                .contexts
                .iter()
                .find(|c| c.name == ctx_name)
                .ok_or_else(|| not_found("context", &ctx_name))?
                .context
                .as_ref()
                .and_then(|c| (self.of_context)(c))
                .ok_or_else(|| {
                    LabeledError::new(format!("context '{ctx_name}' has no {}", self.noun))
                })?;
            let item = all
                .iter()
                .find(|item| (self.name)(item) == name)
                .ok_or_else(|| not_found(self.noun, name))?;
            return Ok(PipelineData::Value(to_value(item, raw, span), None));
        }

        let rows = all.iter().map(|item| to_value(item, raw, span)).collect();
        Ok(PipelineData::Value(Value::list(rows, span), None))
    }
}
