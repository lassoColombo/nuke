//! Shared helpers for `admissionregistration.k8s.io/v1` formatters.

use nu_protocol::IntoValue;
use serde_json::Value as Json;

use crate::formatters::helpers::{
    json_array, json_at, json_count, json_i64, json_str_list, json_str_nonempty,
};

/// A webhook's `clientConfig.service`: `port` defaults to 443 as the API
/// does.
#[derive(IntoValue)]
struct ServiceReference<'a> {
    namespace: Option<&'a str>,
    name: Option<&'a str>,
    port: i64,
    path: Option<&'a str>,
}

/// A webhook's `clientConfig` → `{ url, service }`: the API sets exactly
/// one. The CA bundle is left out.
#[derive(IntoValue)]
struct ClientConfig<'a> {
    url: Option<&'a str>,
    service: Option<ServiceReference<'a>>,
}

fn client_config(cfg: &Json) -> ClientConfig<'_> {
    ClientConfig {
        url: json_str_nonempty(cfg, &["url"]),
        service: json_at(cfg, &["service"]).map(|svc| ServiceReference {
            namespace: json_str_nonempty(svc, &["namespace"]),
            name: json_str_nonempty(svc, &["name"]),
            port: json_i64(svc, &["port"]).unwrap_or(443),
            path: json_str_nonempty(svc, &["path"]),
        }),
    }
}

/// One `.webhooks[]` entry; `ruleCount` counts the rules array.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
pub struct Webhook<'a> {
    name: Option<&'a str>,
    client_config: Option<ClientConfig<'a>>,
    rule_count: i64,
    admission_review_versions: Vec<&'a str>,
    side_effects: Option<&'a str>,
    failure_policy: Option<&'a str>,
    timeout_seconds: Option<i64>,
}

/// Every `.webhooks[]` entry as a [`Webhook`].
pub fn webhooks_list(data: &Json) -> Vec<Webhook<'_>> {
    json_array(data, &["webhooks"])
        .iter()
        .map(|w| Webhook {
            name: json_str_nonempty(w, &["name"]),
            client_config: json_at(w, &["clientConfig"]).map(client_config),
            rule_count: json_count(w, &["rules"]),
            admission_review_versions: json_str_list(w, &["admissionReviewVersions"]),
            side_effects: json_str_nonempty(w, &["sideEffects"]),
            failure_policy: json_str_nonempty(w, &["failurePolicy"]),
            timeout_seconds: json_i64(w, &["timeoutSeconds"]),
        })
        .collect()
}
