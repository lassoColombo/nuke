//! Shared helpers for `admissionregistration.k8s.io/v1` formatters.

use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

use crate::formatters::helpers::{
    json_array, json_at, json_i64, json_i64_val, json_str, json_str_list, json_str_val,
};

/// A webhook's `clientConfig` as one string: the explicit `url`, or the
/// service reference as `service://namespace/name[:port][path]` (the port
/// only when it is not the default 443; the path only when set), or
/// `Value::nothing` when it names neither.
fn client_config(cfg: Option<&Json>, span: Span) -> Value {
    let Some(cfg) = cfg else {
        return Value::nothing(span);
    };
    if let Some(url) = json_str(cfg, &["url"]) {
        return Value::string(url, span);
    }
    let Some(service) = json_at(cfg, &["service"]) else {
        return Value::nothing(span);
    };
    let field = |key| json_str(service, &[key]).unwrap_or("");
    let port = match json_i64(service, &["port"]) {
        Some(port) if port != 443 => format!(":{port}"),
        _ => String::new(),
    };
    Value::string(
        format!(
            "service://{}/{}{port}{}",
            field("namespace"),
            field("name"),
            field("path")
        ),
        span,
    )
}

/// Build a summary list of webhook records from `.webhooks[]`.
///
/// Each record: `{ name, clientConfig, rules, admissionReviewVersions,
/// sideEffects, failurePolicy, timeoutSeconds }`; `ruleCount` counts the
/// rules array.
pub fn webhooks_list(data: &Json, span: Span) -> Value {
    Value::list(
        json_array(data, &["webhooks"])
            .iter()
            .map(|w| {
                let rules_count = json_array(w, &["rules"]).len() as i64;

                let mut rec = Record::new();
                rec.push("name", json_str_val(w, &["name"], span));
                rec.push(
                    "clientConfig",
                    client_config(json_at(w, &["clientConfig"]), span),
                );
                rec.push("ruleCount", Value::int(rules_count, span));
                rec.push(
                    "admissionReviewVersions",
                    json_str_list(w, &["admissionReviewVersions"], span),
                );
                rec.push("sideEffects", json_str_val(w, &["sideEffects"], span));
                rec.push("failurePolicy", json_str_val(w, &["failurePolicy"], span));
                rec.push("timeoutSeconds", json_i64_val(w, &["timeoutSeconds"], span));
                Value::record(rec, span)
            })
            .collect(),
        span,
    )
}
