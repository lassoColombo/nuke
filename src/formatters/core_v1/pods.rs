//! Formatter for `core/v1 Pod` resources.

use k8s_openapi::jiff::Timestamp;
use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    condition_is_true, container_base, json_array, json_at, json_bool, json_i64, json_str,
    json_str_val, meta_created, meta_name, meta_namespace, meta_owner, parse_timestamp,
    timestamp_val,
};

pub struct PodFormatter;

// ---------------------------------------------------------------------------
// kubectl's READY / STATUS / RESTARTS columns
// ---------------------------------------------------------------------------

/// The compact columns kubectl derives in one pass over the container
/// statuses.
struct Summary {
    /// kubectl's STATUS; empty when the pod reports no phase at all.
    status: String,
    ready: i64,
    total: i64,
    restarts: i64,
    /// When the most recently restarted container last terminated: the
    /// `(3m ago)` kubectl appends to RESTARTS.
    last_restart: Option<Timestamp>,
}

/// A port of kubectl's `printPod` (`pkg/printers/internalversion/printers.go`):
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
///
/// `ready`/`total` count sidecars like kubectl does. `restarts` is kubectl's
/// sum over the containers it walks (the init containers up to the one
/// still initializing, else the sidecars plus the regular containers), and
/// `last_restart` the latest `lastState.terminated.finishedAt` among them.
fn summary(item: &DynamicObject) -> Summary {
    let data = &item.data;
    let phase = json_str(data, &["status", "phase"]).unwrap_or("");
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
    let restartable = |name: Option<&str>| {
        init_specs
            .iter()
            .find(|c| json_str(c, &["name"]) == name)
            .is_some_and(|c| json_str(c, &["restartPolicy"]) == Some("Always"))
    };
    let restart_count = |c: &Json| json_i64(c, &["restartCount"]).unwrap_or(0);
    let last_terminated = |c: &Json| {
        json_str(c, &["lastState", "terminated", "finishedAt"]).and_then(parse_timestamp)
    };

    let total = json_array(data, &["spec", "containers"]).len() as i64
        + init_specs
            .iter()
            .filter(|c| json_str(c, &["restartPolicy"]) == Some("Always"))
            .count() as i64;
    let mut ready = 0;
    let (mut init_restarts, mut init_last) = (0, None);
    let (mut sidecar_restarts, mut sidecar_last) = (0, None);

    let mut initializing = false;
    for (i, st) in json_array(data, &["status", "initContainerStatuses"])
        .iter()
        .enumerate()
    {
        let sidecar = restartable(json_str(st, &["name"]));
        init_restarts += restart_count(st);
        init_last = init_last.max(last_terminated(st));
        if sidecar {
            sidecar_restarts += restart_count(st);
            sidecar_last = sidecar_last.max(last_terminated(st));
        }
        let terminated = json_at(st, &["state", "terminated"]);
        let started = json_bool(st, &["started"]).unwrap_or(false);

        reason = match (terminated, waiting_reason(st)) {
            (Some(t), _) if json_i64(t, &["exitCode"]).unwrap_or(0) == 0 => continue,
            _ if sidecar && started => {
                if json_bool(st, &["ready"]).unwrap_or(false) {
                    ready += 1;
                }
                continue;
            }
            (Some(t), _) => format!("Init:{}", terminated_reason(t)),
            (None, Some(w)) if w != "PodInitializing" => format!("Init:{w}"),
            _ => format!("Init:{i}/{}", init_specs.len()),
        };
        initializing = true;
        break;
    }

    let (mut restarts, mut last_restart) = (init_restarts, init_last);
    if !initializing || condition_is_true(data, "Initialized") {
        (restarts, last_restart) = (sidecar_restarts, sidecar_last);
        let mut has_running = false;
        for cs in json_array(data, &["status", "containerStatuses"])
            .iter()
            .rev()
        {
            restarts += restart_count(cs);
            last_restart = last_restart.max(last_terminated(cs));
            if let Some(w) = waiting_reason(cs) {
                reason = w.to_string();
            } else if let Some(t) = json_at(cs, &["state", "terminated"]) {
                reason = terminated_reason(t);
            } else if json_bool(cs, &["ready"]).unwrap_or(false)
                && json_at(cs, &["state", "running"]).is_some()
            {
                has_running = true;
                ready += 1;
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

    Summary {
        status: reason,
        ready,
        total,
        restarts,
        last_restart,
    }
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
// Per-container detail record  (wide only)
// ---------------------------------------------------------------------------

/// Human-readable container state: `Running`, the terminated reason (or
/// `Terminated`), the waiting reason (or `Waiting`), or `None` when no
/// state is reported yet.
fn container_state_str(cs: &Json) -> Option<&str> {
    let reason = |state| json_str(cs, &["state", state, "reason"]).filter(|r| !r.is_empty());
    if json_at(cs, &["state", "running"]).is_some() {
        Some("Running")
    } else if json_at(cs, &["state", "terminated"]).is_some() {
        Some(reason("terminated").unwrap_or("Terminated"))
    } else if json_at(cs, &["state", "waiting"]).is_some() {
        Some(reason("waiting").unwrap_or("Waiting"))
    } else {
        None
    }
}

/// The per-container detail list for the wide format: `spec.<specs>`
/// (name, image, resources) cross-referenced by name with
/// `status.<statuses>` (ready, restartCount, state), one
/// `{ name, image, requests, limits, ready, restarts, state }` row each.
fn containers_value(item: &DynamicObject, specs: &str, statuses: &str, span: Span) -> Value {
    let specs = json_array(&item.data, &["spec", specs]);
    let statuses = json_array(&item.data, &["status", statuses]);

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
                state.map_or_else(|| Value::nothing(span), |s| Value::string(s, span)),
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
        let s = summary(item);
        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("ready", Value::int(s.ready, span));
        rec.push("total", Value::int(s.total, span));
        rec.push(
            "status",
            match s.status.is_empty() {
                true => Value::nothing(span),
                false => Value::string(s.status, span),
            },
        );
        rec.push("restarts", Value::int(s.restarts, span));
        rec.push("lastRestart", timestamp_val(s.last_restart, span));
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
            "podIPs",
            Value::list(
                json_array(&item.data, &["status", "podIPs"])
                    .iter()
                    .filter_map(|ip| json_str(ip, &["ip"]))
                    .map(|ip| Value::string(ip, span))
                    .collect(),
                span,
            ),
        );
        rec.push(
            "nominatedNode",
            json_str_val(&item.data, &["status", "nominatedNodeName"], span),
        );
        rec.push(
            "qos",
            json_str_val(&item.data, &["status", "qosClass"], span),
        );
        rec.push(
            "containers",
            containers_value(item, "containers", "containerStatuses", span),
        );
        rec.push(
            "initContainers",
            containers_value(item, "initContainers", "initContainerStatuses", span),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn status_of(v: serde_json::Value) -> String {
        summary(&serde_json::from_value(v).unwrap()).status
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

    /// kubectl's READY counts sidecars; RESTARTS sums the init containers
    /// while initializing and the sidecars plus regular containers
    /// afterwards, and remembers the latest restart.
    #[test]
    fn counts_follow_kubectl() {
        let of = |v| summary(&serde_json::from_value(v).unwrap());
        let crashing_init = of(json!({"metadata": {"name": "p"},
            "spec": {"initContainers": [{"name": "a"}], "containers": [{"name": "c"}]},
            "status": {"phase": "Pending",
                "initContainerStatuses": [{"name": "a", "restartCount": 7, "state": {"waiting": {"reason": "CrashLoopBackOff"}}}],
                "containerStatuses": [{"name": "c", "restartCount": 0, "state": {"waiting": {"reason": "PodInitializing"}}}]}}));
        assert_eq!(crashing_init.status, "Init:CrashLoopBackOff");
        assert_eq!(
            (
                crashing_init.ready,
                crashing_init.total,
                crashing_init.restarts
            ),
            (0, 1, 7)
        );

        let sidecar = of(json!({"metadata": {"name": "p"},
            "spec": {"initContainers": [{"name": "s", "restartPolicy": "Always"}], "containers": [{"name": "c"}]},
            "status": {"phase": "Running", "conditions": [{"type": "Initialized", "status": "True"}],
                "initContainerStatuses": [{"name": "s", "started": true, "ready": true, "restartCount": 4, "state": {"running": {}},
                    "lastState": {"terminated": {"finishedAt": "2026-01-01T00:00:00Z"}}}],
                "containerStatuses": [{"name": "c", "ready": true, "restartCount": 1, "state": {"running": {}},
                    "lastState": {"terminated": {"finishedAt": "2026-01-02T00:00:00Z"}}}]}}));
        assert_eq!(sidecar.status, "Running");
        assert_eq!((sidecar.ready, sidecar.total, sidecar.restarts), (2, 2, 5));
        assert_eq!(
            sidecar.last_restart,
            "2026-01-02T00:00:00Z".parse::<Timestamp>().ok()
        );
        assert_eq!(crashing_init.last_restart, None);

        assert_eq!(
            of(json!({"metadata": {"name": "p"}, "spec": {}})).status,
            ""
        );
    }
}
