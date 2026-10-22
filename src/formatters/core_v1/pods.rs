//! Formatter for `core/v1 Pod` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    condition_is_true, container_base, json_array, json_at, json_bool, json_i64, json_str,
    json_str_val, meta_created, meta_name, meta_namespace, meta_owner,
};

pub struct PodFormatter;

// ---------------------------------------------------------------------------
// Effective status — kubectl's STATUS column
// ---------------------------------------------------------------------------

/// The pod status string, a port of kubectl's `printPod`
/// (`pkg/printers/internalversion/printers.go`):
///
/// 1. `status.reason`, else `status.phase`.
/// 2. `SchedulingGated` when the `PodScheduled` condition says so.
/// 3. The first init container status that is not done decides
///    (`Init:<reason>`, `Init:Signal:N`, `Init:ExitCode:N` or `Init:i/n`);
///    a restartable (sidecar) init container counts as done once started.
/// 4. Unless still initializing, the regular container statuses are walked
///    in reverse, so the *first* container's waiting/terminated reason
///    wins; `Completed` with another container still running and ready
///    becomes `Running` (or `NotReady` without the Ready condition).
/// 5. A pod being deleted shows `Terminating`, or `Unknown` when its node
///    was lost, unless its phase is already terminal.
fn effective_status(item: &DynamicObject) -> String {
    let data = &item.data;
    let phase = json_str(data, &["status", "phase"]).unwrap_or("Unknown");
    let pod_reason = json_str(data, &["status", "reason"]).filter(|r| !r.is_empty());
    let mut reason = pod_reason.unwrap_or(phase).to_string();

    let gated = json_array(data, &["status", "conditions"]).iter().any(|c| {
        json_str(c, &["type"]) == Some("PodScheduled")
            && json_str(c, &["reason"]) == Some("SchedulingGated")
    });
    if gated {
        reason = "SchedulingGated".to_string();
    }

    let init_specs = json_array(data, &["spec", "initContainers"]);
    let mut initializing = false;
    for (i, st) in json_array(data, &["status", "initContainerStatuses"])
        .iter()
        .enumerate()
    {
        let terminated = json_at(st, &["state", "terminated"]);
        let waiting_reason = waiting_reason(st);
        let restartable = init_specs
            .iter()
            .find(|c| json_str(c, &["name"]) == json_str(st, &["name"]))
            .is_some_and(|c| json_str(c, &["restartPolicy"]) == Some("Always"));
        let started = json_bool(st, &["started"]).unwrap_or(false);

        reason = match (terminated, waiting_reason) {
            (Some(t), _) if json_i64(t, &["exitCode"]).unwrap_or(0) == 0 => continue,
            _ if restartable && started => continue,
            (Some(t), _) => format!("Init:{}", terminated_reason(t)),
            (None, Some(w)) if w != "PodInitializing" => format!("Init:{w}"),
            _ => format!("Init:{i}/{}", init_specs.len()),
        };
        initializing = true;
        break;
    }

    if !initializing || condition_is_true(data, "Initialized") {
        let mut has_running = false;
        for cs in json_array(data, &["status", "containerStatuses"])
            .iter()
            .rev()
        {
            if let Some(w) = waiting_reason(cs) {
                reason = w.to_string();
            } else if let Some(t) = json_at(cs, &["state", "terminated"]) {
                reason = terminated_reason(t);
            } else if json_bool(cs, &["ready"]).unwrap_or(false)
                && json_at(cs, &["state", "running"]).is_some()
            {
                has_running = true;
            }
        }
        if reason == "Completed" && has_running {
            reason = match condition_is_true(data, "Ready") {
                true => "Running",
                false => "NotReady",
            }
            .to_string();
        }
    }

    if item.metadata.deletion_timestamp.is_some() {
        if pod_reason == Some("NodeLost") {
            reason = "Unknown".to_string();
        } else if !matches!(phase, "Succeeded" | "Failed") {
            reason = "Terminating".to_string();
        }
    }

    reason
}

/// A non-empty `state.waiting.reason`.
fn waiting_reason(cs: &Json) -> Option<&str> {
    json_str(cs, &["state", "waiting", "reason"]).filter(|r| !r.is_empty())
}

/// kubectl's name for a terminated state: its reason, else `Signal:N`,
/// else `ExitCode:N`.
fn terminated_reason(term: &Json) -> String {
    match json_str(term, &["reason"]).filter(|r| !r.is_empty()) {
        Some(reason) => reason.to_string(),
        None => match json_i64(term, &["signal"]).unwrap_or(0) {
            0 => format!("ExitCode:{}", json_i64(term, &["exitCode"]).unwrap_or(0)),
            signal => format!("Signal:{signal}"),
        },
    }
}

// ---------------------------------------------------------------------------
// Ready / total / restarts
// ---------------------------------------------------------------------------

/// Count of regular containers whose `ready` field is `true`.
///
/// Only `status.containerStatuses`: unlike kubectl, sidecars (restartable
/// init containers) are not counted.
fn ready_count(item: &DynamicObject) -> i64 {
    json_array(&item.data, &["status", "containerStatuses"])
        .iter()
        .filter(|c| json_bool(c, &["ready"]).unwrap_or(false))
        .count() as i64
}

/// Total containers defined in `spec.containers` (stable before kubelet
/// populates statuses). Unlike kubectl, sidecars are not counted.
fn total_containers(item: &DynamicObject) -> i64 {
    json_array(&item.data, &["spec", "containers"]).len() as i64
}

/// Restart count: the **maximum** restartCount across regular containers —
/// how often the worst container restarted.
///
/// kubectl's RESTARTS column is the *sum* across regular containers (plus
/// sidecars), so the two differ for multi-container pods.
fn max_restarts(item: &DynamicObject) -> i64 {
    json_array(&item.data, &["status", "containerStatuses"])
        .iter()
        .map(|c| json_i64(c, &["restartCount"]).unwrap_or(0))
        .max()
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Per-container detail record  (wide only)
// ---------------------------------------------------------------------------

/// Human-readable container state: `Running`, the terminated reason (or
/// `Terminated`), the waiting reason (or `Waiting`), or `None` when no
/// state is reported yet.
fn container_state_str(cs: &Json) -> Option<String> {
    if json_at(cs, &["state", "running"]).is_some() {
        return Some("Running".to_string());
    }
    if let Some(term) = json_at(cs, &["state", "terminated"]) {
        let r = json_str(term, &["reason"]).unwrap_or("");
        return Some(if r.is_empty() {
            "Terminated".to_string()
        } else {
            r.to_string()
        });
    }
    if let Some(wait) = json_at(cs, &["state", "waiting"]) {
        let r = json_str(wait, &["reason"]).unwrap_or("");
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
/// Each row is `{ name, image, requests, limits, ready, restarts, state }`.
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

            let (ready, restarts, state) = match cstat {
                Some(s) => (
                    json_bool(s, &["ready"]).unwrap_or(false),
                    json_i64(s, &["restartCount"]).unwrap_or(0),
                    container_state_str(s),
                ),
                _ => (false, 0, None),
            };

            // The shared static columns (name, image, resources), then the
            // runtime ones.
            let mut rec = container_base(spec, span);
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
    fn compact(&self, item: &DynamicObject, span: Span) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("ready", Value::int(ready_count(item), span));
        rec.push("total", Value::int(total_containers(item), span));
        rec.push("status", Value::string(effective_status(item), span));
        rec.push("restarts", Value::int(max_restarts(item), span));
        rec.push("created", meta_created(item, span));
        rec
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        rec.push("owner", meta_owner(item, span));
        rec.push(
            "node",
            json_str_val(&item.data, &["spec", "nodeName"], span),
        );

        rec.push(
            "pod_ip",
            json_str_val(&item.data, &["status", "podIP"], span),
        );

        rec.push(
            "nominated_node",
            json_str_val(&item.data, &["status", "nominatedNodeName"], span),
        );

        rec.push(
            "qos",
            json_str_val(&item.data, &["status", "qosClass"], span),
        );
        rec.push("containers", containers_value(item, span));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn status_of(v: serde_json::Value) -> String {
        effective_status(&serde_json::from_value(v).unwrap())
    }

    #[test]
    fn statuses_follow_kubectl() {
        // Unschedulable: init containers in the spec, no statuses yet.
        assert_eq!(
            status_of(
                json!({"metadata": {"name": "p"}, "spec": {"initContainers": [{"name": "i"}]},
                "status": {"phase": "Pending"}})
            ),
            "Pending"
        );
        // Rejected by the kubelet: no statuses, pod-level reason.
        assert_eq!(
            status_of(
                json!({"metadata": {"name": "p"}, "spec": {"initContainers": [{"name": "i"}]},
                "status": {"phase": "Failed", "reason": "OutOfcpu"}})
            ),
            "OutOfcpu"
        );
        // Init container still waiting.
        assert_eq!(
            status_of(
                json!({"metadata": {"name": "p"}, "spec": {"initContainers": [{"name": "a"}, {"name": "b"}]},
                "status": {"phase": "Pending", "initContainerStatuses": [
                    {"name": "a", "state": {"terminated": {"exitCode": 0}}},
                    {"name": "b", "state": {"waiting": {"reason": "PodInitializing"}}}]}})
            ),
            "Init:1/2"
        );
        assert_eq!(
            status_of(
                json!({"metadata": {"name": "p"}, "spec": {"initContainers": [{"name": "a"}]},
                "status": {"phase": "Pending", "initContainerStatuses": [
                    {"name": "a", "state": {"terminated": {"exitCode": 1}}}]}})
            ),
            "Init:ExitCode:1"
        );
        // Deleted during init is still Terminating.
        assert_eq!(
            status_of(
                json!({"metadata": {"name": "p", "deletionTimestamp": "2026-01-01T00:00:00Z"},
                "spec": {"initContainers": [{"name": "a"}]},
                "status": {"phase": "Pending", "initContainerStatuses": [
                    {"name": "a", "state": {"waiting": {"reason": "ImagePullBackOff"}}}]}})
            ),
            "Terminating"
        );
        // First container's reason wins; Completed next to a running one is Running.
        assert_eq!(
            status_of(json!({"metadata": {"name": "p"}, "spec": {},
                "status": {"phase": "Running", "conditions": [{"type": "Ready", "status": "True"}],
                    "containerStatuses": [
                        {"name": "a", "ready": true, "state": {"running": {}}},
                        {"name": "b", "state": {"terminated": {"exitCode": 0, "reason": "Completed"}}}]}})),
            "Running"
        );
        assert_eq!(
            status_of(json!({"metadata": {"name": "p"}, "spec": {},
                "status": {"phase": "Running", "containerStatuses": [
                    {"name": "a", "state": {"waiting": {"reason": "CrashLoopBackOff"}}},
                    {"name": "b", "state": {"terminated": {"exitCode": 1}}}]}})),
            "CrashLoopBackOff"
        );
        assert_eq!(
            status_of(
                json!({"metadata": {"name": "p", "deletionTimestamp": "2026-01-01T00:00:00Z"},
                "spec": {}, "status": {"phase": "Running", "reason": "NodeLost"}})
            ),
            "Unknown"
        );
    }
}
