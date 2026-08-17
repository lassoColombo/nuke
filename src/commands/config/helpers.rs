use kube::config::{Kubeconfig, NamedAuthInfo, NamedCluster, NamedContext};
use nu_protocol::{Span, Value};

use crate::conversions::json_to_nu;

pub fn kubeconfig_to_value(kc: &Kubeconfig, span: Span) -> Value {
    json_to_nu(&serde_json::to_value(kc).unwrap_or_default(), span)
}

pub fn context_to_value(nc: &NamedContext, span: Span) -> Value {
    json_to_nu(&serde_json::to_value(nc).unwrap_or_default(), span)
}

pub fn cluster_to_value(nc: &NamedCluster, span: Span) -> Value {
    json_to_nu(&serde_json::to_value(nc).unwrap_or_default(), span)
}

pub fn user_to_value(nc: &NamedAuthInfo, span: Span) -> Value {
    json_to_nu(&serde_json::to_value(nc).unwrap_or_default(), span)
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
