//! Shared helpers for `admissionregistration.k8s.io/v1` formatters.

use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

use crate::formatters::helpers::{
    json_array, json_at, json_i64_or, json_i64_val, json_str_list, json_str_val,
};

/// A webhook's `clientConfig` → `{ url, service }`: the API sets exactly one.
/// `service` is `{ namespace, name, port, path }`, `port` defaulting to 443 as
/// the API does; the CA bundle is left out. Nothing when the config is
/// absent.
fn client_config(cfg: Option<&Json>, span: Span) -> Value {
    let Some(cfg) = cfg else {
        return Value::nothing(span);
    };
    let service = match json_at(cfg, &["service"]) {
        Some(svc) => {
            let mut rec = Record::new();
            rec.push("namespace", json_str_val(svc, &["namespace"], span));
            rec.push("name", json_str_val(svc, &["name"], span));
            rec.push("port", json_i64_or(svc, &["port"], 443, span));
            rec.push("path", json_str_val(svc, &["path"], span));
            Value::record(rec, span)
        }
        None => Value::nothing(span),
    };
    let mut rec = Record::new();
    rec.push("url", json_str_val(cfg, &["url"], span));
    rec.push("service", service);
    Value::record(rec, span)
}

/// Build a summary list of webhook records from `.webhooks[]`.
///
/// Each record: `{ name, clientConfig, ruleCount, admissionReviewVersions,
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
