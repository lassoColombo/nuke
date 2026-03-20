//! Formatter for `core/v1 ReplicationController` resources.

use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};

use crate::formatters::helpers::{
    fmt_containers, json_array, json_at, meta_created, meta_name, meta_namespace, meta_owner,
};
use crate::formatters::ResourceFormatter;

pub struct ReplicationControllerFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

struct Replicas {
    desired: i64,
    current: i64,
    ready: i64,
    available: i64,
}

fn replicas(item: &DynamicObject) -> Replicas {
    let int = |path: &str, default: i64| -> i64 {
        item.data
            .pointer(&format!("/{}", path.replace('.', "/")))
            .and_then(|v| v.as_i64())
            .unwrap_or(default)
    };

    Replicas {
        desired: int("spec.replicas", 1),
        current: int("status.replicas", 0),
        ready: int("status.readyReplicas", 0),
        available: int("status.availableReplicas", 0),
    }
}

fn effective_status(r: &Replicas) -> &'static str {
    if r.current < r.desired {
        "Scaling"
    } else if r.ready < r.desired {
        "NotReady"
    } else {
        "Ready"
    }
}

/// `spec.selector` → `Value::record` of strings, or empty record when absent.
///
/// ReplicationController uses a plain label map (not the `matchLabels`
/// wrapper that Deployments use), so we read it directly rather than via
/// `spec_selector` which targets `spec.selector`.  The two are equivalent
/// here but this makes the intent explicit.
fn selector(item: &DynamicObject, span: Span) -> Value {
    let mut rec = Record::new();
    if let Some(obj) = json_at(&item.data, &vec!["spec", "selector"]).and_then(|v| v.as_object()) {
        for (k, v) in obj {
            rec.push(k.clone(), Value::string(v.as_str().unwrap_or(""), span));
        }
    }
    Value::record(rec, span)
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

impl ResourceFormatter for ReplicationControllerFormatter {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value {
        let r = replicas(item);
        let status = effective_status(&r);

        let mut rec = Record::new();
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("status", Value::string(status, span));
        rec.push("desired", Value::int(r.desired, span));
        rec.push("current", Value::int(r.current, span));
        rec.push("ready", Value::int(r.ready, span));
        rec.push("available", Value::int(r.available, span));
        rec.push("created", meta_created(item, span));
        Value::record(rec, span)
    }

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        let r = replicas(item);
        let status = effective_status(&r);

        let mut rec = Record::new();

        // Compact columns.
        rec.push("name", meta_name(item, span));
        rec.push("namespace", meta_namespace(item, span));
        rec.push("status", Value::string(status, span));
        rec.push("desired", Value::int(r.desired, span));
        rec.push("current", Value::int(r.current, span));
        rec.push("ready", Value::int(r.ready, span));
        rec.push("available", Value::int(r.available, span));
        rec.push("created", meta_created(item, span));

        // Wide-only columns.
        rec.push("selector", selector(item, span));
        rec.push(
            "containers",
            fmt_containers(
                json_array(&item.data, &vec!["spec", "template", "spec", "containers"]),
                span,
            ),
        );
        rec.push("owner", meta_owner(item, span));

        Value::record(rec, span)
    }
}
