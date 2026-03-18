//! Formatter for `core/v1 Pod` resources.

use kube::api::DynamicObject;
use nu_protocol::{Span, Value};

use crate::formatters::helpers::{
    json_array, json_bool, json_i64, json_str, meta_created, meta_name, meta_namespace,
};
use crate::formatters::ResourceFormatter;

pub struct PodFormatter;

// ---------------------------------------------------------------------------
// Effective status  (mirrors kubectl's pod status derivation)
// ---------------------------------------------------------------------------

fn effective_status(item: &DynamicObject) -> String {
    // 1. Terminating — pod has been deleted but not yet gone.
    if item.metadata.deletion_timestamp.is_some() {
        return "Terminating".to_string();
    }

    // 2. Init containers — surface any non-completed init state.
    let init_statuses = json_array(&item.data, "status.initContainerStatuses");
    let init_specs = json_array(&item.data, "spec.initContainers");
    let init_total = init_specs.len();

    for (i, cs) in init_statuses.iter().enumerate() {
        if json_bool(cs, "ready") {
            continue;
        }
        // Waiting with a reason
        let waiting_reason = json_str(cs, "state.waiting.reason");
        if !waiting_reason.is_empty() && waiting_reason != "PodInitializing" {
            return format!("Init:{}", waiting_reason);
        }
        // Terminated with non-zero exit
        let term_reason = json_str(cs, "state.terminated.reason");
        if !term_reason.is_empty() && term_reason != "Completed" {
            return format!("Init:{}", term_reason);
        }
        // Still running init containers — show progress
        return format!("Init:{}/{}", i, init_total);
    }

    // 3. Regular containers — pick the first non-running / non-ready state.
    let container_statuses = json_array(&item.data, "status.containerStatuses");
    for cs in container_statuses {
        // Waiting: surface the reason (CrashLoopBackOff, ImagePullBackOff, …)
        let waiting_reason = json_str(cs, "state.waiting.reason");
        if !waiting_reason.is_empty() {
            return waiting_reason.to_string();
        }
        // Terminated with a reason
        let term_reason = json_str(cs, "state.terminated.reason");
        if !term_reason.is_empty() {
            return term_reason.to_string();
        }
        // Terminated with a non-zero exit code (no reason string)
        if json_at_exists(cs, "state.terminated") {
            let exit = json_i64(cs, "state.terminated.exitCode");
            if exit != 0 {
                return format!("Error");
            }
        }
    }

    // 4. Fall back to phase.
    let phase = json_str(&item.data, "status.phase");
    if phase.is_empty() {
        "Unknown".to_string()
    } else {
        phase.to_string()
    }
}

/// Returns true if the dot-path exists and is not null.
/// Avoids importing json_at publicly just for a boolean check.
fn json_at_exists(root: &serde_json::Value, path: &str) -> bool {
    let mut cur = root;
    for seg in path.split('.') {
        match cur.get(seg) {
            Some(v) if !v.is_null() => cur = v,
            _ => return false,
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Ready / total containers
// ---------------------------------------------------------------------------

/// Count of containers whose `ready` field is true.
fn ready_count(item: &DynamicObject) -> i64 {
    json_array(&item.data, "status.containerStatuses")
        .iter()
        .filter(|c| json_bool(c, "ready"))
        .count() as i64
}

/// Total number of containers defined in the spec.
/// We use the spec (not containerStatuses) so the number is stable even before
/// the kubelet has populated all statuses.
fn total_containers(item: &DynamicObject) -> i64 {
    json_array(&item.data, "spec.containers").len() as i64
}

// ---------------------------------------------------------------------------
// Restart count
// ---------------------------------------------------------------------------

/// Sum of restartCount across all containers.
fn total_restarts(item: &DynamicObject) -> i64 {
    json_array(&item.data, "status.containerStatuses")
        .iter()
        .map(|c| json_i64(c, "restartCount"))
        .sum()
}

// ---------------------------------------------------------------------------
// Per-container detail record  (wide only)
// ---------------------------------------------------------------------------

/// Human-readable container state: "running", "waiting:<reason>",
/// "terminated:<reason>" or "terminated:exit=<N>".
fn container_state(cs: &serde_json::Value) -> String {
    if json_at_exists(cs, "state.running") {
        return "running".to_string();
    }
    let waiting = json_str(cs, "state.waiting.reason");
    if !waiting.is_empty() {
        return format!("waiting:{}", waiting);
    }
    if json_at_exists(cs, "state.terminated") {
        let reason = json_str(cs, "state.terminated.reason");
        if !reason.is_empty() {
            return format!("terminated:{}", reason);
        }
        let exit = json_i64(cs, "state.terminated.exitCode");
        return format!("terminated:exit={}", exit);
    }
    "unknown".to_string()
}

/// Build the per-container detail list for the wide format.
///
/// Cross-references `spec.containers` (for the image) with
/// `status.containerStatuses` (for runtime state).
fn containers_value(item: &DynamicObject, span: Span) -> Value {
    let specs = json_array(&item.data, "spec.containers");
    let statuses = json_array(&item.data, "status.containerStatuses");

    let rows: Vec<Value> = specs
        .iter()
        .map(|spec| {
            let name = json_str(spec, "name");
            let image = json_str(spec, "image");

            // Find the matching status entry by container name.
            let status = statuses.iter().find(|s| json_str(s, "name") == name);

            let (ready, restarts, state) = match status {
                Some(s) => (
                    json_bool(s, "ready"),
                    json_i64(s, "restartCount"),
                    container_state(s),
                ),
                None => (false, 0, "unknown".to_string()),
            };

            let mut rec = nu_protocol::Record::new();
            rec.push("name", Value::string(name, span));
            rec.push("image", Value::string(image, span));
            rec.push("ready", Value::bool(ready, span));
            rec.push("restarts", Value::int(restarts, span));
            rec.push("state", Value::string(state, span));
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
        let mut rec = nu_protocol::Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("ready", Value::int(ready_count(item), span));
        rec.push("total", Value::int(total_containers(item), span));
        rec.push("status", Value::string(effective_status(item), span));
        rec.push("restarts", Value::int(total_restarts(item), span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let mut rec = nu_protocol::Record::new();
        // --- compact columns first, in the same order ---
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("ready", Value::int(ready_count(item), span));
        rec.push("total", Value::int(total_containers(item), span));
        rec.push("status", Value::string(effective_status(item), span));
        rec.push("restarts", Value::int(total_restarts(item), span));
        rec.push("created", meta_created(item, span));
        // --- wide-only columns ---
        rec.push(
            "node",
            Value::string(json_str(&item.data, "spec.nodeName"), span),
        );
        rec.push(
            "pod_ip",
            Value::string(json_str(&item.data, "status.podIP"), span),
        );
        rec.push("containers", containers_value(item, span));
        Value::record(rec, span)
    }
}
