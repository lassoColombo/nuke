//! Shared extraction and conversion helpers for all resource formatters.
//!
//! # Design principles
//!
//! - Every function is pure and infallible (never panics, uses sensible defaults).
//! - Nushell types are used as precisely as possible:
//!     - plain string            → `Value::string`
//!     - RFC 3339 timestamp      → `Value::date`   (chrono `DateTime<FixedOffset>`)
//!     - k8s duration ("15m")    → `Value::duration` (nanoseconds i64)
//!     - memory quantity ("512Mi") → `Value::filesize` (bytes i64)
//!     - CPU quantity ("250m")   → `Value::int`    (millicores i64)
//!     - integer                 → `Value::int`
//!     - boolean                 → `Value::bool`
//!     - absent / unparseable    → `Value::nothing`
//!
//! # Layers
//!
//! 1. **JSON walkers** (`json_str`, `json_i64`, …) — replace the scattered
//!    `item.data["a"]["b"].as_str().unwrap_or_default()` pattern.
//!    The dot-path API (`"status.phase"`) is readable and uniform.
//!
//! 2. **Metadata helpers** (`meta_name`, `meta_namespace`, …) — typed `Value`
//!    wrappers around `kube::ResourceExt` for the fields every resource has.
//!
//! 3. **Quantity converters** (`parse_date`, `parse_duration`, `parse_memory`,
//!    `parse_cpu`, `pct`) — the single authoritative place that decides which
//!    nushell type a Kubernetes quantity becomes — and their path-reading
//!    forms (`json_date_val`, `json_memory_val`, `json_cpu_val`,
//!    `json_duration_val`, `json_int_or_string_val`).
//!
//! 4. **Spec/status extractors** (`meta_owner`, `object_ref`,
//!    `condition_status`, `label_selector`, `spec_selector`, `spec_strategy`)
//!    — structured sub-object helpers shared across multiple resource
//!    formatters.
//!
//! 5. **Container helpers** (`fmt_containers`, `fmt_images`) — extract
//!    container metadata from `.spec.template.spec.containers`.

use crate::conversions::json_to_nu;
use k8s_openapi::jiff;
use kube::ResourceExt;
use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

// ---------------------------------------------------------------------------
// 1. Low-level JSON walkers
// ---------------------------------------------------------------------------

/// Walk a **dot-separated path** into a `serde_json::Value`.
///
/// Returns `None` if any segment is missing or the final node is JSON null.
///
/// ```text
/// json_at(&item.data, "status.phase")
/// json_at(&container, "state.waiting.reason")
/// ```
pub fn json_at<'a, const N: usize>(root: &'a Json, segments: &[&str; N]) -> Option<&'a Json> {
    let mut cur = root;
    for seg in segments {
        cur = cur.get(seg)?;
    }
    if cur.is_null() { None } else { Some(cur) }
}

/// Extract a `&str` from a dot-path, falling back to `""`.
pub fn json_str<'a, const N: usize>(root: &'a Json, path: &[&str; N]) -> Option<&'a str> {
    json_at(root, path).and_then(|v| v.as_str())
}

/// Extract a `&str` from a dot-path → `Value::string`, or `Value::nothing`
/// when the field is absent or empty.
pub fn json_str_val<const N: usize>(root: &Json, path: &[&str; N], span: Span) -> Value {
    match json_str(root, path) {
        Some(s) if !s.is_empty() => Value::string(s, span),
        _ => Value::nothing(span),
    }
}

/// Extract an `i64` from a dot-path.
///
/// Returns `None` when the field is absent or not an integer, so the caller
/// can supply the appropriate default for the field:
///
/// ```rust
/// json_i64(data, &["spec", "replicas"]).unwrap_or(1)
/// json_i64(data, &["status", "replicas"]).unwrap_or(0)
/// ```
pub fn json_i64<const N: usize>(root: &Json, path: &[&str; N]) -> Option<i64> {
    json_at(root, path).and_then(|v| v.as_i64())
}

/// Extract an `i64` from a dot-path → `Value::int`, or `Value::nothing`
/// when the field is absent or not an integer.
pub fn json_i64_val<const N: usize>(root: &Json, path: &[&str; N], span: Span) -> Value {
    match json_i64(root, path) {
        Some(s) => Value::int(s, span),
        _ => Value::nothing(span),
    }
}

/// Extract a `bool` from a dot-path.
///
/// Returns `None` when the field is absent or not a boolean, so the caller
/// can supply the appropriate default for the field:
///
/// ```rust
/// json_bool(data, &["spec", "paused"]).unwrap_or(false)
/// json_bool(data, &["spec", "attachRequired"]).unwrap_or(true)
/// ```
pub fn json_bool<const N: usize>(root: &Json, path: &[&str; N]) -> Option<bool> {
    json_at(root, path).and_then(|v| v.as_bool())
}
/// Extract a `bool` from a dot-path → `Value::bool`, or `Value::nothing`
/// when the field is absent.
pub fn json_bool_val<const N: usize>(root: &Json, path: &[&str; N], span: Span) -> Value {
    match json_bool(root, path) {
        Some(b) => Value::bool(b, span),
        _ => Value::nothing(span),
    }
}

/// Extract a JSON array from a dot-path, falling back to an empty slice.
pub fn json_array<'a, const N: usize>(root: &'a Json, path: &[&str; N]) -> &'a [Json] {
    json_at(root, path)
        .and_then(|v| v.as_array())
        .map(|v| v.as_slice())
        .unwrap_or(&[])
}

/// Extract a JSON array at `path` as a `Value::list` of strings.
///
/// Non-string and null entries are silently dropped. Returns an empty list
/// when the field is absent or not an array.
pub fn json_str_list<const N: usize>(root: &Json, path: &[&str; N], span: Span) -> Value {
    Value::list(
        json_array(root, path)
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| Value::string(s, span))
            .collect(),
        span,
    )
}

/// A JSON object of strings at `path` → `Value::record` of string columns
/// (non-string values become `""`). Returns an empty record when the field is
/// absent or not an object.
pub fn json_str_map<const N: usize>(root: &Json, path: &[&str; N], span: Span) -> Value {
    let mut rec = Record::new();
    if let Some(map) = json_at(root, path).and_then(|v| v.as_object()) {
        for (k, v) in map {
            rec.push(k.clone(), Value::string(v.as_str().unwrap_or(""), span));
        }
    }
    Value::record(rec, span)
}

/// Count the keys in a JSON object at `path`.
///
/// Returns `0` when the field is absent or not an object.
pub fn json_obj_key_count<const N: usize>(root: &Json, path: &[&str; N]) -> i64 {
    json_at(root, path)
        .and_then(|v| v.as_object())
        .map(|m| m.len() as i64)
        .unwrap_or(0)
}

/// Return the keys of a JSON object at `path` as a `Value::list` of strings.
///
/// Returns an empty list when the field is absent or not an object.
pub fn json_obj_keys<const N: usize>(root: &Json, path: &[&str; N], span: Span) -> Value {
    let keys: Vec<Value> = json_at(root, path)
        .and_then(|v| v.as_object())
        .map(|m| m.keys().map(|k| Value::string(k.clone(), span)).collect())
        .unwrap_or_default();
    Value::list(keys, span)
}

// ---------------------------------------------------------------------------
// 2. Typed metadata helpers  (DynamicObject → Value)
// ---------------------------------------------------------------------------

/// `metadata.name` → `Value::string`.
pub fn meta_name(item: &DynamicObject, span: Span) -> Value {
    Value::string(item.name_any(), span)
}

/// `metadata.namespace` → `Value::string` (`""` for cluster-scoped resources).
pub fn meta_namespace(item: &DynamicObject, span: Span) -> Value {
    Value::string(item.namespace().unwrap_or_default(), span)
}

/// `metadata.creationTimestamp` → `Value::date`, or `Value::nothing`.
pub fn meta_created(item: &DynamicObject, span: Span) -> Value {
    let Some(ts) = item.creation_timestamp() else {
        return Value::nothing(span);
    };
    timestamp_to_value(ts.0, span)
}

/// `metadata.labels` → `Value::record` (one string column per label).
/// Returns an empty record when there are no labels.
pub fn meta_labels(item: &DynamicObject, span: Span) -> Value {
    let mut rec = Record::new();
    for (k, v) in item.labels() {
        rec.push(k.clone(), Value::string(v.clone(), span));
    }
    Value::record(rec, span)
}

/// `metadata.annotations` → `Value::record` (one string column per annotation).
/// Returns an empty record when there are no annotations.
pub fn meta_annotations(item: &DynamicObject, span: Span) -> Value {
    let mut rec = Record::new();
    for (k, v) in item.annotations() {
        rec.push(k.clone(), Value::string(v.clone(), span));
    }
    Value::record(rec, span)
}

/// One `metadata.annotations` entry, or `None` when absent.
///
/// Annotations live on `item.metadata`, never in `item.data` — a
/// `json_*(&item.data, &["metadata", ..])` lookup can never match.
pub fn meta_annotation<'a>(item: &'a DynamicObject, key: &str) -> Option<&'a str> {
    item.metadata
        .annotations
        .as_ref()?
        .get(key)
        .map(String::as_str)
}

/// One annotation parsed as an integer → `Value::int`, or `Value::nothing`.
pub fn meta_annotation_int(item: &DynamicObject, key: &str, span: Span) -> Value {
    match meta_annotation(item, key).and_then(|s| s.parse::<i64>().ok()) {
        Some(n) => Value::int(n, span),
        None => Value::nothing(span),
    }
}

/// Resolve the controlling `ownerReference` → `Value::string` (`"kind/name"`),
/// or `Value::nothing` when there is no controller owner.
///
/// Mirrors the Nushell `meta owner` helper: walks
/// `metadata.ownerReferences[]`, finds the entry where `controller == true`,
/// and formats it as `"<lowercase-kind>/<name>"` (e.g. `"replicaset/my-rs-xz4f"`).
/// Only the first controller reference is returned (there can be at most one
/// per the Kubernetes spec).
pub fn meta_owner(item: &DynamicObject, span: Span) -> Value {
    item.owner_references()
        .iter()
        .find(|r| r.controller.unwrap_or(false))
        .map(|r| Value::string(format!("{}/{}", r.kind.to_lowercase(), r.name), span))
        .unwrap_or_else(|| Value::nothing(span))
}

/// An object reference (`involvedObject`, `scaleTargetRef`, a Binding's
/// `target`, an address's `targetRef`, a `roleRef`) as `"kind/name"` with the
/// kind lowercased, like [`meta_owner`], or `Value::nothing` when the
/// reference is absent or carries neither.
pub fn object_ref(reference: Option<&Json>, span: Span) -> Value {
    let field = |key| reference.and_then(|r| json_str(r, &[key])).unwrap_or("");
    match (field("kind"), field("name")) {
        ("", "") => Value::nothing(span),
        (kind, name) => Value::string(format!("{}/{name}", kind.to_lowercase()), span),
    }
}

// ---------------------------------------------------------------------------
// 3. Quantity converters  (string → typed Value)
// ---------------------------------------------------------------------------

/// Parse an RFC 3339 / ISO 8601 timestamp string, or `None` on failure.
pub fn parse_timestamp(s: &str) -> Option<jiff::Timestamp> {
    s.parse().ok()
}

/// Parse an RFC 3339 / ISO 8601 timestamp string → `Value::date`.
/// Returns `Value::nothing` on failure.
pub fn parse_date(s: &str, span: Span) -> Value {
    match parse_timestamp(s) {
        Some(ts) => timestamp_to_value(ts, span),
        None => Value::nothing(span),
    }
}

/// Convert a `jiff::Timestamp` to a `Value::date` (chrono `DateTime<FixedOffset>`).
///
/// Jiff timestamps are always UTC, so we fix the offset to zero.
/// This is the single place that bridges jiff → chrono for nushell.
///
/// Splits the i128 nanosecond count with Euclidean division so pre-1970
/// instants keep a non-negative sub-second part, as chrono requires. (An
/// `i64` nanosecond count would only reach 1677–2262; this covers jiff's
/// whole range.)
fn timestamp_to_value(ts: jiff::Timestamp, span: Span) -> Value {
    const NANOS_PER_SEC: i128 = 1_000_000_000;
    let nanos = ts.as_nanosecond();
    let secs = nanos.div_euclid(NANOS_PER_SEC) as i64;
    let subsec = nanos.rem_euclid(NANOS_PER_SEC) as u32;
    match chrono::DateTime::from_timestamp(secs, subsec) {
        Some(dt) => Value::date(dt.fixed_offset(), span),
        None => Value::nothing(span),
    }
}

/// Parse a Kubernetes memory quantity ("256Mi", "1Gi", "512k", "2G")
/// → `Value::filesize` (bytes i64).
/// Returns `Value::nothing` for an empty or unparseable string;
/// `Value::filesize(0)` for a valid zero quantity.
pub fn parse_memory(s: &str, span: Span) -> Value {
    match memory_to_bytes(s) {
        Some(bytes) => Value::filesize(bytes as i64, span),
        None => Value::nothing(span),
    }
}

/// Parse a Kubernetes CPU quantity ("500m", "2", "0.5", "100u", "800n")
/// → `Value::int` (millicores i64).
/// Returns `Value::nothing` for an empty or unparseable string.
pub fn parse_cpu(s: &str, span: Span) -> Value {
    match cpu_to_millicores(s) {
        Some(millicores) => Value::int(millicores as i64, span),
        None => Value::nothing(span),
    }
}

/// Parse a Kubernetes / Go duration string ("8760h", "15m", "1h30m")
/// → `Value::duration` (nanoseconds i64).
///
/// Returns `Value::nothing` for an empty or unparseable string.
///
/// Supports the Go `time.Duration` format used by cert-manager and others:
/// hours (`h`), minutes (`m`), seconds (`s`), milliseconds (`ms`),
/// microseconds (`us`/`µs`), nanoseconds (`ns`).
pub fn parse_duration(s: &str, span: Span) -> Value {
    if s.is_empty() {
        return Value::nothing(span);
    }
    match duration_to_nanos(s) {
        Some(ns) => Value::duration(ns, span),
        None => Value::nothing(span),
    }
}

/// Compute `used / total * 100` → `Value::float`.
/// Returns `Value::nothing` when `total` is zero.
pub fn pct(used: u64, total: u64, span: Span) -> Value {
    if total == 0 {
        Value::nothing(span)
    } else {
        Value::float((used as f64 / total as f64) * 100.0, span)
    }
}

/// A timestamp string at `path` → `Value::date`, or `Value::nothing` when
/// absent or unparseable.
pub fn json_date_val<const N: usize>(root: &Json, path: &[&str; N], span: Span) -> Value {
    parse_date(json_str(root, path).unwrap_or(""), span)
}

/// A memory quantity at `path` → `Value::filesize`, or `Value::nothing`.
pub fn json_memory_val<const N: usize>(root: &Json, path: &[&str; N], span: Span) -> Value {
    parse_memory(json_str(root, path).unwrap_or(""), span)
}

/// A CPU quantity at `path` → `Value::int` (millicores), or `Value::nothing`.
pub fn json_cpu_val<const N: usize>(root: &Json, path: &[&str; N], span: Span) -> Value {
    parse_cpu(json_str(root, path).unwrap_or(""), span)
}

/// A Go duration at `path` → `Value::duration`, or `Value::nothing`.
pub fn json_duration_val<const N: usize>(root: &Json, path: &[&str; N], span: Span) -> Value {
    parse_duration(json_str(root, path).unwrap_or(""), span)
}

/// A Kubernetes `IntOrString` at `path` (a Service `targetPort`, a rolling
/// update's `maxUnavailable`, a PDB's `minAvailable`, …) → `Value::int` or
/// `Value::string` as the server stored it, or `Value::nothing` when absent.
pub fn json_int_or_string_val<const N: usize>(root: &Json, path: &[&str; N], span: Span) -> Value {
    match json_at(root, path) {
        Some(Json::Number(n)) => n
            .as_i64()
            .map_or_else(|| Value::nothing(span), |i| Value::int(i, span)),
        Some(Json::String(s)) => Value::string(s, span),
        _ => Value::nothing(span),
    }
}

// ---------------------------------------------------------------------------
// 4. Spec / status extractors
// ---------------------------------------------------------------------------

/// The `status` string (`"True"`, `"False"`, `"Unknown"`) of the first
/// `.status.conditions[]` entry of type `cond_type`, or `None` when absent.
pub fn condition_status<'a>(data: &'a Json, cond_type: &str) -> Option<&'a str> {
    condition_status_in(json_array(data, &["status", "conditions"]), cond_type)
}

/// [`condition_status`] over any conditions array — for objects that keep
/// conditions somewhere else, such as a route's `status.parents[].conditions`.
pub fn condition_status_in<'a>(conditions: &'a [Json], cond_type: &str) -> Option<&'a str> {
    conditions
        .iter()
        .find(|c| json_str(c, &["type"]) == Some(cond_type))
        .and_then(|c| json_str(c, &["status"]))
}

/// Whether the condition of type `cond_type` exists with status `"True"`.
pub fn condition_is_true(data: &Json, cond_type: &str) -> bool {
    condition_status(data, cond_type) == Some("True")
}

/// A condition as a tri-state → `Value::bool` for `"True"`/`"False"`, or
/// `Value::nothing` when it is `"Unknown"` or absent (not yet reconciled).
pub fn condition_bool(data: &Json, cond_type: &str, span: Span) -> Value {
    condition_bool_in(json_array(data, &["status", "conditions"]), cond_type, span)
}

/// [`condition_bool`] over any conditions array.
pub fn condition_bool_in(conditions: &[Json], cond_type: &str, span: Span) -> Value {
    match condition_status_in(conditions, cond_type) {
        Some("True") => Value::bool(true, span),
        Some("False") => Value::bool(false, span),
        _ => Value::nothing(span),
    }
}

/// Extract `.spec.selector` → `Value::record`.
///
/// For **Services**, `spec.selector` is a flat `{ label: value }` map — use
/// this function.  Returns an empty record when the field is absent.
pub fn spec_selector(data: &Json, span: Span) -> Value {
    json_str_map(data, &["spec", "selector"], span)
}

/// A Kubernetes `LabelSelector` node (a workload's `spec.selector`, a PDB's,
/// a Cilium `EndpointSelector`, a CSIStorageCapacity's `nodeTopology`) →
/// `{ matchLabels: record, matchExpressions: [ { key, operator, values } ] }`.
///
/// Always returns both columns (empty record / empty list when absent) so
/// callers can query `$x.selector.matchLabels.app` without a null-check, and
/// a selector written with `matchExpressions` only is not mistaken for one
/// that matches everything.
pub fn label_selector(node: Option<&Json>, span: Span) -> Value {
    let match_labels = match node {
        Some(n) => json_str_map(n, &["matchLabels"], span),
        None => Value::record(Record::new(), span),
    };

    let match_expressions: Vec<Value> = node
        .map(|n| json_array(n, &["matchExpressions"]))
        .unwrap_or_default()
        .iter()
        .map(|e| {
            let mut er = Record::new();
            er.push("key", json_str_val(e, &["key"], span));
            er.push("operator", json_str_val(e, &["operator"], span));
            er.push("values", json_str_list(e, &["values"], span));
            Value::record(er, span)
        })
        .collect();

    let mut rec = Record::new();
    rec.push("matchLabels", match_labels);
    rec.push("matchExpressions", Value::list(match_expressions, span));
    Value::record(rec, span)
}

/// [`label_selector`] over `.spec.selector`, for the workload resources
/// (Deployment, DaemonSet, StatefulSet, ReplicaSet, PVC, PDB) whose selector
/// is a `LabelSelector` rather than the flat map of [`spec_selector`].
pub fn spec_label_selector(data: &Json, span: Span) -> Value {
    label_selector(json_at(data, &["spec", "selector"]), span)
}

/// A rolling-update strategy node (a Deployment's `spec.strategy`, a
/// DaemonSet's `spec.updateStrategy`) →
/// `Value::record { type, maxUnavailable, maxSurge }`.
///
/// `type` defaults to `RollingUpdate` as the API does. `maxUnavailable` and
/// `maxSurge` are `IntOrString`: an int or a percentage string as stored, or
/// `Value::nothing` when absent.
pub fn spec_strategy(node: Option<&Json>, span: Span) -> Value {
    let strategy_type = node
        .and_then(|n| json_str(n, &["type"]))
        .unwrap_or("RollingUpdate");
    let field = |key| match node {
        Some(n) => json_int_or_string_val(n, &["rollingUpdate", key], span),
        None => Value::nothing(span),
    };

    let mut rec = Record::new();
    rec.push("type", Value::string(strategy_type, span));
    rec.push("maxUnavailable", field("maxUnavailable"));
    rec.push("maxSurge", field("maxSurge"));
    Value::record(rec, span)
}

/// Return all `.status.conditions[]` as a `Value::list` of records.
///
/// Each record has the shape: `{ type, status, reason, message, updated }`.
/// The `updated` field is parsed from `lastTransitionTime` → `Value::date`,
/// or `Value::nothing` when absent.
pub fn status_conditions_list(data: &Json, span: Span) -> Value {
    let rows: Vec<Value> = json_array(data, &["status", "conditions"])
        .iter()
        .map(|c| {
            let mut rec = Record::new();
            rec.push("type", json_str_val(c, &["type"], span));
            rec.push("status", json_str_val(c, &["status"], span));
            rec.push("reason", json_str_val(c, &["reason"], span));
            rec.push("message", json_str_val(c, &["message"], span));
            rec.push("updated", json_date_val(c, &["lastTransitionTime"], span));
            Value::record(rec, span)
        })
        .collect();
    Value::list(rows, span)
}

/// The first key in `candidates` present under `.spec` → `Value::string`, or
/// `Value::nothing` when none match.
///
/// Surfaces which variant of a union-typed spec is configured (e.g. a Traefik
/// `Middleware` is a `stripPrefix` *or* a `headers` *or* …; a Hub
/// `AccessControlPolicy` is `jwt` *or* `oidc* *or* …) as one queryable column,
/// instead of forcing callers to probe every key.
pub fn detected_type(data: &Json, candidates: &[&str], span: Span) -> Value {
    let Some(spec) = json_at(data, &["spec"]) else {
        return Value::nothing(span);
    };
    for &key in candidates {
        if spec.get(key).is_some() {
            return Value::string(key, span);
        }
    }
    Value::nothing(span)
}

// ---------------------------------------------------------------------------
// 5. Container helpers
// ---------------------------------------------------------------------------

/// The record for a single container JSON object, which callers may extend:
///
/// ```text
/// { name, image, requests?: { cpu, memory }, limits?: { cpu, memory } }
/// ```
///
/// `requests` and `limits` are omitted entirely (not `nothing`) when absent.
pub fn container_base(c: &Json, span: Span) -> Record {
    let mut rec = Record::new();
    rec.push("name", json_str_val(c, &["name"], span));
    rec.push("image", json_str_val(c, &["image"], span));

    // resources — only include sub-record when the field is present
    if let Some(resources) = json_at(c, &["resources"]) {
        if let Some(requests) = json_at(resources, &["requests"]) {
            rec.push("requests", resources_record(requests, span));
        }
        if let Some(limits) = json_at(resources, &["limits"]) {
            rec.push("limits", resources_record(limits, span));
        }
    }

    rec
}

/// Build a typed `{ cpu, memory }` record from a resources map (a
/// container's `requests`/`limits`, a RuntimeClass's `overhead.podFixed`).
pub fn resources_record(r: &Json, span: Span) -> Value {
    let mut rec = Record::new();
    rec.push("cpu", json_cpu_val(r, &["cpu"], span));
    rec.push("memory", json_memory_val(r, &["memory"], span));
    Value::record(rec, span)
}

/// Format a slice of container JSON objects → `Value::list` of
/// `container_base` records.
///
/// The caller is responsible for locating the right array with `json_array`:
///
/// ```rust
/// // Standard workload (Deployment, DaemonSet, StatefulSet, ReplicaSet, Job)
/// fmt_containers(json_array(data, &["spec", "template", "spec", "containers"]), span)
/// ```
pub fn fmt_containers(containers: &[Json], span: Span) -> Value {
    Value::list(
        containers
            .iter()
            .map(|c| Value::record(container_base(c, span), span))
            .collect(),
        span,
    )
}

/// Format a slice of container JSON objects → `Value::list` of image strings.
///
/// Same calling convention as `fmt_containers`.
pub fn fmt_images(containers: &[Json], span: Span) -> Value {
    Value::list(
        containers
            .iter()
            .map(|c| json_str_val(c, &["image"], span))
            .collect(),
        span,
    )
}

// ---------------------------------------------------------------------------
// Internal quantity parsers (consumed by the public wrappers above)
// ---------------------------------------------------------------------------

/// Parse a Kubernetes memory quantity string to bytes.
///
/// Handles all suffixes defined in the Kubernetes quantity spec:
///
/// | suffix | unit        | base               |
/// |--------|-------------|--------------------|
/// | `Ki`   | kibibytes   | 2^10               |
/// | `Mi`   | mebibytes   | 2^20               |
/// | `Gi`   | gibibytes   | 2^30               |
/// | `Ti`   | tebibytes   | 2^40               |
/// | `Pi`   | pebibytes   | 2^50               |
/// | `Ei`   | exbibytes   | 2^60               |
/// | `k`    | kilobytes   | 10^3               |
/// | `M`    | megabytes   | 10^6               |
/// | `G`    | gigabytes   | 10^9               |
/// | `T`    | terabytes   | 10^12              |
/// | `P`    | petabytes   | 10^15              |
/// | `E`    | exabytes    | 10^18              |
/// | `m`    | millibytes  | 10^-3 (rounds down)|
/// | *(none)* | bytes     | 1                  |
///
/// Fractional values (e.g. `"1.5Gi"`) are supported; sub-byte results
/// from `m` quantities round down to zero. Returns `None` for an empty or
/// unparseable string.
pub(crate) fn memory_to_bytes(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    // Binary suffixes (IEC) — try float parse to support e.g. "1.5Gi"
    for (suffix, shift) in [
        ("Ki", 10u32),
        ("Mi", 20),
        ("Gi", 30),
        ("Ti", 40),
        ("Pi", 50),
        ("Ei", 60),
    ] {
        if let Some(n) = s.strip_suffix(suffix) {
            return number(n).map(|v| (v * (1u64 << shift) as f64) as u64);
        }
    }
    // Decimal suffixes (SI) — float parse for fractional support
    for (suffix, factor) in [
        ("k", 1_000u64),
        ("M", 1_000_000),
        ("G", 1_000_000_000),
        ("T", 1_000_000_000_000),
        ("P", 1_000_000_000_000_000),
        ("E", 1_000_000_000_000_000_000),
    ] {
        if let Some(n) = s.strip_suffix(suffix) {
            return number(n).map(|v| (v * factor as f64) as u64);
        }
    }
    // Millibytes — sub-byte quantity, rounds down (1000m == 1 byte)
    if let Some(n) = s.strip_suffix('m') {
        return number(n).map(|v| (v / 1_000.0) as u64);
    }
    // Plain integer or float bytes
    number(s).map(|v| v as u64)
}

/// The numeric part of a quantity: a non-negative decimal, possibly with an
/// exponent (`"1e3"`), as Kubernetes' `Quantity` grammar allows.
fn number(s: &str) -> Option<f64> {
    s.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && *v >= 0.0)
}

/// Parse a Kubernetes CPU quantity string to millicores.
///
/// Handles all suffixes defined in the Kubernetes quantity spec:
///
/// | suffix   | unit        | conversion              |
/// |----------|-------------|-------------------------|
/// | `m`      | millicores  | value as-is             |
/// | *(none)* | cores       | × 1 000                 |
/// | `k`      | kilocores   | × 1 000 000             |
/// | `M`      | megacores   | × 1 000 000 000         |
/// | `G`      | gigacores   | × 1 000 000 000 000     |
/// | `u`      | microcores  | ÷ 1 000 (truncates)     |
/// | `n`      | nanocores   | ÷ 1 000 000 (truncates) |
///
/// Fractional values (e.g. `"1.5"`, `"0.5m"`) are supported.
/// Sub-millicore results from `u`/`n` quantities truncate toward zero.
/// Returns `None` for an empty or unparseable string.
pub(crate) fn cpu_to_millicores(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    // millicores — float to support "0.5m" (sub-millicore, truncates to 0)
    if let Some(n) = s.strip_suffix('m') {
        return number(n).map(|v| v as u64);
    }
    // microcores → millicores (÷ 1 000, truncates)
    if let Some(n) = s.strip_suffix('u') {
        return number(n).map(|v| (v / 1_000.0) as u64);
    }
    // nanocores → millicores (÷ 1 000 000, truncates)
    if let Some(n) = s.strip_suffix('n') {
        return number(n).map(|v| (v / 1_000_000.0) as u64);
    }
    // Decimal large-unit suffixes (rare but spec-valid)
    for (suffix, factor) in [
        ("k", 1_000_000u64),
        ("M", 1_000_000_000),
        ("G", 1_000_000_000_000),
    ] {
        if let Some(n) = s.strip_suffix(suffix) {
            return number(n).map(|v| (v * factor as f64) as u64);
        }
    }
    // Bare float/int — whole cores, multiply by 1 000
    number(s).map(|f| (f * 1_000.0) as u64)
}

/// Parse a Go `time.Duration` string into nanoseconds.
///
/// The format is a sequence of decimal numbers each with a unit suffix:
/// `"1h30m"`, `"500ms"`, `"2h0m0s"`, `"8760h"`.
///
/// Recognised units: `ns`, `us`/`µs`, `ms`, `s`, `m`, `h`; a leading `-`
/// negates the whole duration, as in Go.
fn duration_to_nanos(s: &str) -> Option<i64> {
    let s = s.trim();
    if let Some(positive) = s.strip_prefix('-') {
        return duration_to_nanos(positive)?.checked_neg();
    }
    // Go accepts a bare zero without a unit.
    if s == "0" {
        return Some(0);
    }
    if s.is_empty() {
        return None;
    }
    let mut total: i64 = 0;
    let mut remaining = s;

    while !remaining.is_empty() {
        // Parse the numeric part (integer or float).
        let num_end = remaining
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .unwrap_or(remaining.len());
        if num_end == 0 {
            return None; // no digits before unit
        }
        let num: f64 = remaining[..num_end].parse().ok()?;
        remaining = &remaining[num_end..];

        // Parse the unit suffix.
        let (nanos_per_unit, unit_len) = if remaining.starts_with("ns") {
            (1i64, 2)
        } else if remaining.starts_with("us") || remaining.starts_with("µs") {
            (1_000i64, if remaining.starts_with("µ") { 3 } else { 2 })
        } else if remaining.starts_with("ms") {
            (1_000_000i64, 2)
        } else if remaining.starts_with('s') {
            (1_000_000_000i64, 1)
        } else if remaining.starts_with('m') {
            (60_000_000_000i64, 1)
        } else if remaining.starts_with('h') {
            (3_600_000_000_000i64, 1)
        } else {
            return None; // unknown unit
        };

        total = total.checked_add((num * nanos_per_unit as f64) as i64)?;
        remaining = &remaining[unit_len..];
    }

    Some(total)
}

// ---------------------------------------------------------------------------
// 6. Native JSON passthrough
// ---------------------------------------------------------------------------
//
// Some CRD specs (Cilium BGP instances, Traefik routes/middlewares, envoy
// services, …) are deeply nested and version-churny.  Rather than hand-type
// every field, these helpers pass the whole sub-tree through `json_to_nu`,
// preserving its native structure (records/lists with typed scalars) so it
// stays fully queryable without flattening anything into composite strings.

/// A JSON node → native nushell value, or an empty **list** when absent.
/// Use for array-shaped fields.
pub fn native_list(node: Option<&Json>, span: Span) -> Value {
    node.map_or_else(|| Value::list(vec![], span), |v| json_to_nu(v, span))
}

/// A JSON node → native nushell value, or `Value::nothing` when absent.
/// Use for object-shaped fields.
pub fn native_or_nothing(node: Option<&Json>, span: Span) -> Value {
    node.map_or_else(|| Value::nothing(span), |v| json_to_nu(v, span))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn durations() {
        assert_eq!(duration_to_nanos("0"), Some(0));
        assert_eq!(duration_to_nanos("1h30m"), Some(5_400_000_000_000));
        assert_eq!(duration_to_nanos("500ms"), Some(500_000_000));
        assert_eq!(duration_to_nanos("1.5s"), Some(1_500_000_000));
        assert_eq!(duration_to_nanos("2µs"), Some(2_000));
        assert_eq!(duration_to_nanos("-1h"), Some(-3_600_000_000_000));
        assert_eq!(duration_to_nanos("10"), None);
        assert_eq!(duration_to_nanos("h"), None);
    }

    #[test]
    fn int_or_string() {
        let span = Span::test_data();
        let v = |j: Json| json_int_or_string_val(&j, &["x"], span);
        assert_eq!(v(json!({"x": 1})), Value::int(1, span));
        assert_eq!(v(json!({"x": "25%"})), Value::string("25%", span));
        assert_eq!(v(json!({"x": null})), Value::nothing(span));
        assert_eq!(v(json!({})), Value::nothing(span));
    }

    #[test]
    fn object_refs() {
        let span = Span::test_data();
        let r = |v: Json| object_ref(Some(&v), span);
        assert_eq!(object_ref(None, span), Value::nothing(span));
        assert_eq!(r(json!({})), Value::nothing(span));
        assert_eq!(
            r(json!({"kind": "Pod", "name": "web-1"})),
            Value::string("pod/web-1", span)
        );
        // One side alone still renders.
        assert_eq!(r(json!({"kind": "Node"})), Value::string("node/", span));
        assert_eq!(r(json!({"name": "x"})), Value::string("/x", span));
    }

    #[test]
    fn label_selectors() {
        let span = Span::test_data();
        let sel = json!({"matchExpressions": [{"key": "k", "operator": "Exists"}]});
        let Value::Record { val, .. } = label_selector(Some(&sel), span) else {
            panic!("not a record");
        };
        assert_eq!(
            val.get("matchLabels")
                .and_then(|v| v.as_record().ok().map(|r| r.len())),
            Some(0)
        );
        assert_eq!(
            val.get("matchExpressions")
                .and_then(|v| v.as_list().ok().map(|l| l.len())),
            Some(1)
        );
    }

    #[test]
    fn strategies() {
        let span = Span::test_data();
        let s = json!({"type": "RollingUpdate", "rollingUpdate": {"maxUnavailable": 1, "maxSurge": "25%"}});
        let Value::Record { val, .. } = spec_strategy(Some(&s), span) else {
            panic!("not a record");
        };
        assert_eq!(val.get("maxUnavailable"), Some(&Value::int(1, span)));
        assert_eq!(val.get("maxSurge"), Some(&Value::string("25%", span)));
        let Value::Record { val, .. } = spec_strategy(None, span) else {
            panic!("not a record");
        };
        assert_eq!(val.get("type"), Some(&Value::string("RollingUpdate", span)));
        assert_eq!(val.get("maxSurge"), Some(&Value::nothing(span)));
    }

    #[test]
    fn dates() {
        let span = Span::test_data();
        let date = |s: &str| parse_date(s, span).as_date().ok();
        assert_eq!(
            date("1969-12-31T23:59:59.5Z"),
            chrono::DateTime::from_timestamp(-1, 500_000_000).map(|d| d.fixed_offset())
        );
        assert_eq!(
            date("2026-01-02T03:04:05.123456789Z"),
            chrono::DateTime::from_timestamp(1_767_323_045, 123_456_789).map(|d| d.fixed_offset())
        );
        // Beyond the i64-nanosecond range (1677–2262).
        assert_eq!(
            date("9000-01-01T00:00:00Z"),
            chrono::DateTime::from_timestamp(221_845_392_000, 0).map(|d| d.fixed_offset())
        );
        assert_eq!(
            date("1500-01-01T00:00:00.25Z"),
            chrono::DateTime::from_timestamp(-14_831_769_600, 250_000_000)
                .map(|d| d.fixed_offset())
        );
        assert_eq!(date("not a date"), None);
    }

    #[test]
    fn memory_quantities() {
        assert_eq!(memory_to_bytes("512Mi"), Some(512 << 20));
        assert_eq!(memory_to_bytes("1.5Gi"), Some(3 << 29));
        assert_eq!(memory_to_bytes("2G"), Some(2_000_000_000));
        assert_eq!(memory_to_bytes("1000m"), Some(1));
        assert_eq!(memory_to_bytes("123"), Some(123));
        assert_eq!(memory_to_bytes("1e3"), Some(1_000));
        assert_eq!(memory_to_bytes("0"), Some(0));
        assert_eq!(memory_to_bytes(""), None);
        assert_eq!(memory_to_bytes("garbage"), None);
        assert_eq!(memory_to_bytes("-1Gi"), None);
    }

    #[test]
    fn cpu_quantities() {
        assert_eq!(cpu_to_millicores("250m"), Some(250));
        assert_eq!(cpu_to_millicores("2"), Some(2_000));
        assert_eq!(cpu_to_millicores("0.5"), Some(500));
        assert_eq!(cpu_to_millicores("1500000n"), Some(1));
        assert_eq!(cpu_to_millicores("3k"), Some(3_000_000));
        assert_eq!(cpu_to_millicores(""), None);
        assert_eq!(cpu_to_millicores("abc"), None);
    }
}
