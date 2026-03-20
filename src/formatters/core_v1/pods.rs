//! Formatter for `core/v1 Pod` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

use crate::formatters::helpers::{
    container_base, json_array, json_bool, json_i64, json_str, meta_created, meta_name,
    meta_namespace, meta_owner,
};
use crate::formatters::ResourceFormatter;

pub struct PodFormatter;

// ---------------------------------------------------------------------------
// Effective status  (mirrors kubectl / Nushell `fmt pod-phase`)
// ---------------------------------------------------------------------------

/// Derive the human-readable pod status string.
///
/// The logic follows the Nushell `fmt pod-phase` helper exactly:
///
/// 1. Start from `.status.phase` (default `"Unknown"`).
/// 2. Override with `.status.reason` when present.
/// 3. Override with `"SchedulingGated"` when the `PodScheduled` condition
///    carries `reason == "SchedulingGated"`.
/// 4. Walk `spec.initContainers` in order; for the first non-completed one,
///    derive an `"Init:…"` string and **return immediately** — regular
///    containers are not inspected while init is still running.
/// 5. Walk `status.containerStatuses`; for each container surface waiting
///    reason, terminated reason/signal, or `"Completed"`.
/// 6. Final overrides: `"Failed"` with no better reason → `"Error"`;
///    `deletionTimestamp` on a non-terminal pod → `"Terminating"`.
fn effective_status(item: &DynamicObject) -> String {
    let data = &item.data;

    // ------------------------------------------------------------------ 1 & 2
    // Base reason: phase, then pod-level reason if present.
    let mut reason = {
        let phase = json_str(data, &["status", "phase"]);
        if phase.is_empty() {
            "Unknown".to_string()
        } else {
            phase.to_string()
        }
    };

    if let Some(pod_reason) = data
        .pointer("/status/reason")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
    {
        reason = pod_reason.to_string();
    }

    // ------------------------------------------------------------------ 3
    // SchedulingGated: a PodScheduled condition with reason == SchedulingGated.
    let scheduling_gated = json_array(data, &["status", "conditions"]).iter().any(|c| {
        json_str(c, &["type"]) == "PodScheduled" && json_str(c, &["reason"]) == "SchedulingGated"
    });
    if scheduling_gated {
        reason = "SchedulingGated".to_string();
    }

    // ------------------------------------------------------------------ 4
    // Init containers: iterate spec order, look up status by name.
    let init_specs = json_array(data, &["spec", "initContainers"]);
    let init_statuses = json_array(data, &["status", "initContainerStatuses"]);
    let init_count = init_specs.len();

    if init_count > 0 {
        for (i, spec) in init_specs.iter().enumerate() {
            let name = json_str(spec, &["name"]);
            let st = init_statuses
                .iter()
                .find(|s| json_str(s, &["name"]) == name);

            // No status entry for this init container yet.
            let st = if let Some(s) = st {
                s
            } else {
                return format!("Init:{}/{}", i, init_count);
            };

            let terminated = st.pointer("/state/terminated");
            let waiting = st.pointer("/state/waiting");

            if let Some(term) = terminated {
                let exit_code = term.get("exitCode").and_then(|v| v.as_i64()).unwrap_or(0);

                if exit_code == 0 {
                    // Completed successfully — continue to the next init container.
                    continue;
                }

                // Non-zero exit: reason > signal > "Error".
                let r = init_term_reason(term);
                return format!("Init:{}", r);
            } else if let Some(wait) = waiting {
                let wr = wait.get("reason").and_then(|v| v.as_str()).unwrap_or("");

                if !wr.is_empty() && wr != "PodInitializing" {
                    return format!("Init:{}", wr);
                } else {
                    return format!("Init:{}/{}", i, init_count);
                }
            } else {
                // Running but not terminated — still in progress.
                return format!("Init:{}/{}", i, init_count);
            }
        }
        // All init containers completed — fall through to regular containers.
    }

    // ------------------------------------------------------------------ 5
    // Regular containers.
    for cs in json_array(data, &["status", "containerStatuses"]) {
        if let Some(wait) = cs.pointer("/state/waiting") {
            let wr = wait.get("reason").and_then(|v| v.as_str()).unwrap_or("");
            if !wr.is_empty() {
                reason = wr.to_string();
            }
        } else if let Some(term) = cs.pointer("/state/terminated") {
            let exit_code = term.get("exitCode").and_then(|v| v.as_i64()).unwrap_or(0);
            let tr = term.get("reason").and_then(|v| v.as_str()).unwrap_or("");
            let signal = term.get("signal").and_then(|v| v.as_i64()).unwrap_or(0);

            if !tr.is_empty() {
                reason = tr.to_string();
            } else if signal != 0 {
                reason = format!("Signal:{}", signal);
            } else if exit_code != 0 {
                reason = "Error".to_string();
            } else {
                reason = "Completed".to_string();
            }
        }
        // Running containers do not override the reason.
    }

    // ------------------------------------------------------------------ 6
    // Final overrides.

    // "Failed" phase with nothing better to show → "Error".
    if json_str(data, &["status", "phase"]) == "Failed" && reason == "Failed" {
        reason = "Error".to_string();
    }

    // Terminating: deletionTimestamp is set and pod is not already terminal.
    let phase = json_str(data, &["status", "phase"]);
    let is_terminal = phase == "Succeeded" || phase == "Failed";
    if item.metadata.deletion_timestamp.is_some() && !is_terminal {
        reason = "Terminating".to_string();
    }

    reason
}

/// Derive the init-container failure reason from a terminated state object.
/// Priority: reason string > signal > "Error".
fn init_term_reason(term: &Json) -> String {
    let r = term.get("reason").and_then(|v| v.as_str()).unwrap_or("");
    if !r.is_empty() {
        return r.to_string();
    }
    let signal = term.get("signal").and_then(|v| v.as_i64()).unwrap_or(0);
    if signal != 0 {
        return format!("Signal:{}", signal);
    }
    "Error".to_string()
}

// ---------------------------------------------------------------------------
// Ready / total / restarts
// ---------------------------------------------------------------------------

/// Count of containers whose `ready` field is `true`.
fn ready_count(item: &DynamicObject) -> i64 {
    json_array(&item.data, &["status", "containerStatuses"])
        .iter()
        .filter(|c| json_bool(c, &["ready"]))
        .count() as i64
}

/// Total containers defined in `spec.containers` (stable before kubelet
/// populates statuses).
fn total_containers(item: &DynamicObject) -> i64 {
    json_array(&item.data, &["spec", "containers"]).len() as i64
}

/// Restart count: the **maximum** restartCount across all containers.
///
/// This matches kubectl's display: it shows the single container that has
/// restarted most often, not the sum across all containers.
fn max_restarts(item: &DynamicObject) -> i64 {
    json_array(&item.data, &["status", "containerStatuses"])
        .iter()
        .map(|c| json_i64(c, &["restartCount"]))
        .max()
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Per-container detail record  (wide only)
// ---------------------------------------------------------------------------

/// Human-readable container state string.
///
/// Mirrors the Nushell wide container map:
/// - `"Running"`
/// - terminated reason or `"Terminated"` fallback
/// - waiting reason or `"Waiting"` fallback
/// - `null` (→ `Value::nothing`) when no state is available yet
fn container_state_str(cs: &Json) -> Option<String> {
    if cs.pointer("/state/running").is_some() {
        return Some("Running".to_string());
    }
    if let Some(term) = cs.pointer("/state/terminated") {
        let r = term.get("reason").and_then(|v| v.as_str()).unwrap_or("");
        return Some(if r.is_empty() {
            "Terminated".to_string()
        } else {
            r.to_string()
        });
    }
    if let Some(wait) = cs.pointer("/state/waiting") {
        let r = wait.get("reason").and_then(|v| v.as_str()).unwrap_or("");
        return Some(if r.is_empty() {
            "Waiting".to_string()
        } else {
            r.to_string()
        });
    }
    None
}

/// Build the per-container detail list for the wide format.
///
/// Each row is `{ name, image, ready, restarts, state, requests?, limits? }`.
/// Cross-references `spec.containers` (image, resources) with
/// `status.containerStatuses` (ready, restartCount, state).
fn containers_value(item: &DynamicObject, span: Span) -> Value {
    let specs = json_array(&item.data, &["spec", "containers"]);
    let statuses = json_array(&item.data, &["status", "containerStatuses"]);

    let rows: Vec<Value> = specs
        .iter()
        .map(|spec| {
            let name = json_str(spec, &["name"]);

            // Match the runtime status by container name.
            let cstat = statuses.iter().find(|s| json_str(s, &["name"]) == name);

            let (ready, restarts, state) = if let Some(s) = cstat {
                (
                    json_bool(s, &["ready"]),
                    json_i64(s, &["restartCount"]),
                    container_state_str(s),
                )
            } else {
                (false, 0, None)
            };

            // Start from the shared container_base (name, image, resources).
            let mut rec = match container_base(spec, span) {
                Value::Record { val, .. } => val.into_owned(),
                _ => Record::new(),
            };

            // Insert runtime fields after the static ones.
            rec.push("ready", Value::bool(ready, span));
            rec.push("restarts", Value::int(restarts, span));
            rec.push(
                "state",
                state
                    .map(|s| Value::string(s, span))
                    .unwrap_or_else(|| Value::nothing(span)),
            );

            Value::record(rec, span)
        })
        .collect();

    Value::list(rows, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for PodFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("ready", Value::int(ready_count(item), span));
        rec.push("total", Value::int(total_containers(item), span));
        rec.push("status", Value::string(effective_status(item), span));
        rec.push("restarts", Value::int(max_restarts(item), span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = Record::new();

        // Compact columns — same order as format_compact.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("ready", Value::int(ready_count(item), span));
        rec.push("total", Value::int(total_containers(item), span));
        rec.push("status", Value::string(effective_status(item), span));
        rec.push("restarts", Value::int(max_restarts(item), span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "node",
            Value::string(json_str(&item.data, &["spec", "nodeName"]), span),
        );
        rec.push(
            "pod_ip",
            Value::string(json_str(&item.data, &["status", "podIP"]), span),
        );
        rec.push(
            "nominated_node",
            Value::string(json_str(&item.data, &["status", "nominatedNodeName"]), span),
        );
        rec.push(
            "qos",
            Value::string(json_str(&item.data, &["status", "qosClass"]), span),
        );
        rec.push("containers", containers_value(item, span));

        Value::record(rec, span)
    }
}
