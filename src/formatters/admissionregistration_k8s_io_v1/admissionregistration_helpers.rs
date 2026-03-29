//! Shared helpers for `admissionregistration.k8s.io/v1` formatters.

use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

use crate::formatters::helpers::{json_array, json_i64_val, json_str, json_str_list};

/// Count webhooks in a `.webhooks[]` array.
pub fn webhooks_count(data: &Json) -> i64 {
    json_array(data, &["webhooks"]).len() as i64
}

/// Format a `clientConfig` sub-object as a compact string.
///
/// - Service reference → `"service://namespace/name<path>"`
/// - Explicit URL → the URL string
/// - Absent → `""`
pub fn client_config_str(cfg: &Json) -> String {
    if let Some(url) = json_str(cfg, &["url"]) {
        return url.to_string();
    }
    let ns = json_str(cfg, &["service", "namespace"]).unwrap_or("");
    let name = json_str(cfg, &["service", "name"]).unwrap_or("");
    if ns.is_empty() && name.is_empty() {
        return String::new();
    }
    let path = json_str(cfg, &["service", "path"]).unwrap_or("/");
    format!("service://{}/{}{}", ns, name, path)
}

/// Build a summary list of webhook records from `.webhooks[]`.
///
/// Each record: `{ name, clientConfig, rules, sideEffects, failurePolicy }`.
/// `rules` is an integer count of the rules array.
pub fn webhooks_list(data: &Json, span: Span) -> Value {
    Value::list(
        json_array(data, &["webhooks"])
            .iter()
            .map(|w| {
                let name = json_str(w, &["name"]).unwrap_or("");
                let rules_count = json_array(w, &["rules"]).len() as i64;
                let client_cfg =
                    json_str(w, &["clientConfig"])
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| {
                            w.get("clientConfig")
                                .map(client_config_str)
                                .unwrap_or_default()
                        });

                let mut rec = Record::new();
                rec.push("name", Value::string(name, span));
                rec.push("clientConfig", Value::string(client_cfg, span));
                rec.push("rules", Value::int(rules_count, span));
                rec.push(
                    "admissionReviewVersions",
                    json_str_list(w, &["admissionReviewVersions"], span),
                );
                rec.push(
                    "sideEffects",
                    Value::string(json_str(w, &["sideEffects"]).unwrap_or(""), span),
                );
                rec.push(
                    "failurePolicy",
                    Value::string(json_str(w, &["failurePolicy"]).unwrap_or(""), span),
                );
                rec.push(
                    "timeoutSeconds",
                    json_i64_val(w, &["timeoutSeconds"], span),
                );
                Value::record(rec, span)
            })
            .collect(),
        span,
    )
}
