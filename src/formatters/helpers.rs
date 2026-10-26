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
//!    `item.data["a"]["b"].as_str().unwrap_or_default()` pattern with a
//!    path of segments: `json_str(&item.data, &["status", "phase"])`.
//!
//! 2. **Metadata helpers** (`meta_name`, `meta_namespace`, …) — typed `Value`
//!    wrappers around `kube::ResourceExt` for the fields every resource has.
//!
//! 3. **Quantity converters** (`parse_duration`, `parse_memory`, `parse_cpu`,
//!    `quantity_val`, `typed_quantity`, `pct`) — the single authoritative
//!    place that decides which nushell type a Kubernetes quantity becomes —
//!    and their path-reading forms (`json_date_val`, `json_memory_val`,
//!    `json_quantity_val`, `json_duration_val`, `json_int_or_string_val`).
//!
//! 4. **Spec/status extractors** (`meta_owner`, `object_ref`,
//!    `condition_status`, `label_selector`, `spec_selector`, `spec_strategy`)
//!    — structured sub-object helpers shared across multiple resource
//!    formatters.
//!
//! 5. **Container helpers** (`fmt_containers`, `fmt_images`,
//!    `resources_record`) — container specs and resource maps as records.

use crate::conversions::json_to_nu;
use k8s_openapi::jiff;
use kube::ResourceExt;
use kube::api::DynamicObject;
use nu_protocol::{Record, Span, Value};
use serde_json::Value as Json;

// ---------------------------------------------------------------------------
// 1. Low-level JSON walkers
// ---------------------------------------------------------------------------

/// Walk a path of keys into a `serde_json::Value`.
///
/// Returns `None` if any segment is missing or the final node is JSON null.
///
/// ```text
/// json_at(&item.data, &["status", "phase"])
/// json_at(&container, &["state", "waiting", "reason"])
/// ```
pub fn json_at<'a, const N: usize>(root: &'a Json, segments: &[&str; N]) -> Option<&'a Json> {
    let mut cur = root;
    for seg in segments {
        cur = cur.get(seg)?;
    }
    if cur.is_null() { None } else { Some(cur) }
}

/// The string at `path`, or `None` when absent or not a string.
pub fn json_str<'a, const N: usize>(root: &'a Json, path: &[&str; N]) -> Option<&'a str> {
    json_at(root, path).and_then(|v| v.as_str())
}

/// The string at `path` → `Value::string`, or `Value::nothing` when the
/// field is absent or empty.
///
/// Folding `""` into nothing is right for most fields, whose Go type has
/// `omitempty`. Where the API gives `""` its own meaning (a claim's
/// `storageClassName: ""` disables dynamic provisioning), use
/// [`json_str_exact_val`].
pub fn json_str_val<const N: usize>(root: &Json, path: &[&str; N], span: Span) -> Value {
    match json_str(root, path) {
        Some(s) if !s.is_empty() => Value::string(s, span),
        _ => Value::nothing(span),
    }
}

/// The string at `path` → `Value::string`, kept even when empty, or
/// `Value::nothing` when absent: for the fields where `""` and unset differ.
pub fn json_str_exact_val<const N: usize>(root: &Json, path: &[&str; N], span: Span) -> Value {
    json_str(root, path).map_or_else(|| Value::nothing(span), |s| Value::string(s, span))
}

/// The string at `path`, or `default` when absent → `Value::string`. For
/// fields the API defaults (`spec.type` → `ClusterIP`, `protocol` → `TCP`).
pub fn json_str_or<const N: usize>(
    root: &Json,
    path: &[&str; N],
    default: &str,
    span: Span,
) -> Value {
    Value::string(json_str(root, path).unwrap_or(default), span)
}

/// The integer at `path`.
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

/// The integer at `path` → `Value::int`, or `Value::nothing` when the field
/// is absent or not an integer.
pub fn json_i64_val<const N: usize>(root: &Json, path: &[&str; N], span: Span) -> Value {
    match json_i64(root, path) {
        Some(s) => Value::int(s, span),
        _ => Value::nothing(span),
    }
}

/// The integer at `path`, or `default` when absent → `Value::int`. For
/// fields the API defaults (`spec.replicas` → 1) and status counters that
/// are 0 until first reconciled.
pub fn json_i64_or<const N: usize>(
    root: &Json,
    path: &[&str; N],
    default: i64,
    span: Span,
) -> Value {
    Value::int(json_i64(root, path).unwrap_or(default), span)
}

/// The boolean at `path`.
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
/// The boolean at `path` → `Value::bool`, or `Value::nothing` when the field
/// is absent.
pub fn json_bool_val<const N: usize>(root: &Json, path: &[&str; N], span: Span) -> Value {
    match json_bool(root, path) {
        Some(b) => Value::bool(b, span),
        _ => Value::nothing(span),
    }
}

/// The boolean at `path`, or `default` when absent → `Value::bool`. For
/// fields the API defaults (`spec.paused` → false, `attachRequired` → true).
pub fn json_bool_or<const N: usize>(
    root: &Json,
    path: &[&str; N],
    default: bool,
    span: Span,
) -> Value {
    Value::bool(json_bool(root, path).unwrap_or(default), span)
}

/// The array at `path`, or an empty slice when absent or not an array.
pub fn json_array<'a, const N: usize>(root: &'a Json, path: &[&str; N]) -> &'a [Json] {
    json_at(root, path)
        .and_then(|v| v.as_array())
        .map(|v| v.as_slice())
        .unwrap_or(&[])
}

/// The length of the array at `path` → `Value::int`, 0 when absent: the
/// `<thing>Count` compact column of an object list.
pub fn json_count<const N: usize>(root: &Json, path: &[&str; N], span: Span) -> Value {
    Value::int(json_array(root, path).len() as i64, span)
}

/// The `name` of each element of the object array at `path` → `Value::list`
/// of strings (a ServiceAccount's `secrets`, a CRD's `spec.versions`).
pub fn json_name_list<const N: usize>(root: &Json, path: &[&str; N], span: Span) -> Value {
    Value::list(
        json_array(root, path)
            .iter()
            .filter_map(|v| json_str(v, &["name"]))
            .map(|s| Value::string(s, span))
            .collect(),
        span,
    )
}

/// The array at `path` as a `Value::list` of strings.
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

/// `metadata.namespace` → `Value::string`, or `Value::nothing` for a
/// cluster-scoped object.
pub fn meta_namespace(item: &DynamicObject, span: Span) -> Value {
    match item.namespace() {
        Some(ns) => Value::string(ns, span),
        None => Value::nothing(span),
    }
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
fn parse_date(s: &str, span: Span) -> Value {
    match parse_timestamp(s) {
        Some(ts) => timestamp_to_value(ts, span),
        None => Value::nothing(span),
    }
}

/// A parsed timestamp → `Value::date`, or `Value::nothing` for `None`.
pub fn timestamp_val(ts: Option<jiff::Timestamp>, span: Span) -> Value {
    ts.map_or_else(|| Value::nothing(span), |ts| timestamp_to_value(ts, span))
}

/// The first of `dates` that is not nothing, else nothing: for a date that
/// several fields may carry (an event's `lastTimestamp`, else `eventTime`).
pub fn first_date<const N: usize>(dates: [Value; N], span: Span) -> Value {
    dates
        .into_iter()
        .find(|v| !v.is_nothing())
        .unwrap_or_else(|| Value::nothing(span))
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

/// A quantity node as text: the string as stored, or a bare JSON number
/// rendered (CRDs typed int-or-string keep a number a number).
fn quantity_text(node: Option<&Json>) -> String {
    match node {
        Some(Json::String(s)) => s.clone(),
        Some(Json::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

/// [`quantity_text`] of the quantity at `path`.
fn json_quantity_str<const N: usize>(root: &Json, path: &[&str; N]) -> Option<String> {
    json_at(root, path).map(|node| quantity_text(Some(node)))
}

/// A quantity typed by the resource it measures: `cpu` → millicores;
/// `memory`, `storage`, `ephemeral-storage`, `hugepages-*` → filesize;
/// anything else (object counts, extended resources such as
/// `nvidia.com/gpu`, ratios) → [`quantity_val`]. Nothing when `raw` is
/// absent or unparseable.
///
/// The resource name's last path segment decides, so a quota's
/// `gold.storageclass.storage.k8s.io/requests.storage` is a size while
/// `gold.storageclass.storage.k8s.io/persistentvolumeclaims` is a count,
/// and a `requests.`/`limits.` prefix is transparent. This is the one
/// classifier behind [`resources_record`], quotas and limit ranges.
pub fn typed_quantity(resource: &str, raw: Option<&Json>, span: Span) -> Value {
    let leaf = resource.rsplit('/').next().unwrap_or(resource);
    let leaf = leaf
        .strip_prefix("requests.")
        .or_else(|| leaf.strip_prefix("limits."))
        .unwrap_or(leaf);
    let text = quantity_text(raw);
    match leaf {
        "cpu" => parse_cpu(&text, span),
        "memory" | "storage" | "ephemeral-storage" => parse_memory(&text, span),
        _ if leaf.starts_with("hugepages-") => parse_memory(&text, span),
        _ => quantity_val(&text, span),
    }
}

/// A memory quantity at `path` → `Value::filesize`, or `Value::nothing`.
pub fn json_memory_val<const N: usize>(root: &Json, path: &[&str; N], span: Span) -> Value {
    parse_memory(&json_quantity_str(root, path).unwrap_or_default(), span)
}

/// A dimensionless quantity (a count, a ratio) → `Value::int` when integral,
/// `Value::float` otherwise, or `Value::nothing` when unparseable.
///
/// The apiserver canonicalises quantities, so a count of `1000` is read back
/// as `"1k"` and a ratio of `1.5` as `"1500m"`; this turns them back into
/// numbers so a column never mixes ints and strings.
pub fn quantity_val(raw: &str, span: Span) -> Value {
    match parse_quantity(raw) {
        Some(v) if v.fract() == 0.0 && v.abs() < (1u64 << 53) as f64 => Value::int(v as i64, span),
        Some(v) => Value::float(v, span),
        None => Value::nothing(span),
    }
}

/// [`quantity_val`] of the quantity at `path`.
pub fn json_quantity_val<const N: usize>(root: &Json, path: &[&str; N], span: Span) -> Value {
    quantity_val(&json_quantity_str(root, path).unwrap_or_default(), span)
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

/// The first condition of type `cond_type` in `conditions`.
pub fn condition<'a>(conditions: &'a [Json], cond_type: &str) -> Option<&'a Json> {
    conditions
        .iter()
        .find(|c| json_str(c, &["type"]) == Some(cond_type))
}

/// The `status` string (`"True"`, `"False"`, `"Unknown"`) of the first
/// `.status.conditions[]` entry of type `cond_type`, or `None` when absent.
pub fn condition_status<'a>(data: &'a Json, cond_type: &str) -> Option<&'a str> {
    condition_status_in(json_array(data, &["status", "conditions"]), cond_type)
}

/// [`condition_status`] over any conditions array — for objects that keep
/// conditions somewhere else, such as a route's `status.parents[].conditions`.
fn condition_status_in<'a>(conditions: &'a [Json], cond_type: &str) -> Option<&'a str> {
    condition(conditions, cond_type).and_then(|c| json_str(c, &["status"]))
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
/// A present selector always has both columns (an empty record / list when
/// unset), so a selector written with `matchExpressions` only is not
/// mistaken for one that matches everything. An absent selector is
/// `Value::nothing`: what it means (no objects, every object, not
/// applicable) is the field's business, never an invented `{}`.
pub fn label_selector(node: Option<&Json>, span: Span) -> Value {
    let Some(node) = node else {
        return Value::nothing(span);
    };
    let match_labels = json_str_map(node, &["matchLabels"], span);
    let match_expressions: Vec<Value> = json_array(node, &["matchExpressions"])
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
/// { name, image, requests: { cpu, memory, … }, limits: { cpu, memory, … } }
/// ```
///
/// `requests` and `limits` are always present (`nothing` when unset) so
/// `get containers.requests.cpu` never errors on a container without them.
pub fn container_base(c: &Json, span: Span) -> Record {
    let mut rec = Record::new();
    rec.push("name", json_str_val(c, &["name"], span));
    rec.push("image", json_str_val(c, &["image"], span));
    rec.push(
        "requests",
        json_resources_val(c, &["resources", "requests"], span),
    );
    rec.push(
        "limits",
        json_resources_val(c, &["resources", "limits"], span),
    );
    rec
}

/// A resource map (a container's `requests`/`limits`, a Node's `capacity`,
/// a RuntimeClass's `overhead.podFixed`) → record with one column per key,
/// each typed by [`typed_quantity`]. `cpu` and `memory` are always present.
pub fn resources_record(r: &Json, span: Span) -> Value {
    let map = r.as_object();
    let at = |key: &str| map.and_then(|m| m.get(key));
    let mut rec = Record::new();
    rec.push("cpu", typed_quantity("cpu", at("cpu"), span));
    rec.push("memory", typed_quantity("memory", at("memory"), span));
    for (key, raw) in map.into_iter().flatten() {
        if key != "cpu" && key != "memory" {
            rec.push(key, typed_quantity(key, Some(raw), span));
        }
    }
    Value::record(rec, span)
}

/// [`resources_record`] of the map at `path`, or `Value::nothing` when
/// absent.
pub fn json_resources_val<const N: usize>(root: &Json, path: &[&str; N], span: Span) -> Value {
    match json_at(root, path) {
        Some(amounts) => resources_record(amounts, span),
        None => Value::nothing(span),
    }
}

/// Compact columns of a plain replica controller — `apps/v1 ReplicaSet` and
/// its predecessor `core/v1 ReplicationController` count and report replicas
/// identically: `{ status, desired, current, ready, available }`.
pub fn replica_controller_compact(item: &DynamicObject, rec: &mut Record, span: Span) {
    let data = &item.data;
    // spec.replicas defaults to 1 when absent (Kubernetes default)
    let desired = json_i64(data, &["spec", "replicas"]).unwrap_or(1);
    let current = json_i64(data, &["status", "replicas"]).unwrap_or(0);
    let ready = json_i64(data, &["status", "readyReplicas"]).unwrap_or(0);
    let available = json_i64(data, &["status", "availableReplicas"]).unwrap_or(0);

    // Scaling either way: a scale-down is not Ready until the extra pods go.
    let status = if current != desired {
        "Scaling"
    } else if ready < desired {
        "NotReady"
    } else {
        "Ready"
    };

    rec.push("status", Value::string(status, span));
    rec.push("desired", Value::int(desired, span));
    rec.push("current", Value::int(current, span));
    rec.push("ready", Value::int(ready, span));
    rec.push("available", Value::int(available, span));
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

/// Parse a Kubernetes `Quantity` string to its value in base units
/// (bytes, cores, or a plain count).
///
/// Handles every suffix of the quantity grammar — binary (`Ki`..`Ei`),
/// decimal SI (`n`, `u`, `m`, `k`, `M`, `G`, `T`, `P`, `E`) and a decimal
/// exponent (`1e3`) — and fractional numbers (`"1.5Gi"`). Returns `None` for
/// an empty, negative or unparseable string.
pub(crate) fn parse_quantity(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    for (suffix, shift) in [
        ("Ki", 10u32),
        ("Mi", 20),
        ("Gi", 30),
        ("Ti", 40),
        ("Pi", 50),
        ("Ei", 60),
    ] {
        if let Some(n) = s.strip_suffix(suffix) {
            return number(n).map(|v| v * (1u64 << shift) as f64);
        }
    }
    for (suffix, factor) in [
        ("n", 1e-9),
        ("u", 1e-6),
        ("m", 1e-3),
        ("k", 1e3),
        ("M", 1e6),
        ("G", 1e9),
        ("T", 1e12),
        ("P", 1e15),
        ("E", 1e18),
    ] {
        if let Some(n) = s.strip_suffix(suffix) {
            return number(n).map(|v| v * factor);
        }
    }
    // A plain number, with or without an exponent.
    number(s)
}

/// The numeric part of a quantity: a non-negative decimal, possibly with an
/// exponent (`"1e3"`), as Kubernetes' `Quantity` grammar allows.
fn number(s: &str) -> Option<f64> {
    s.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && *v >= 0.0)
}

/// A memory quantity (`"512Mi"`, `"2G"`, `"1000m"`) in whole bytes; a
/// sub-byte remainder rounds down.
pub(crate) fn memory_to_bytes(s: &str) -> Option<u64> {
    parse_quantity(s).map(|bytes| bytes as u64)
}

/// A CPU quantity (`"250m"`, `"2"`, `"0.5"`, `"1500000n"`) in millicores.
///
/// A fraction of a millicore rounds **up**, as `Quantity.MilliValue()` does
/// (and so `kubectl top`): `999999n` is 1 millicore, not 0. The product is
/// rounded to a nanocore first so `1000000n` stays exactly 1.
pub(crate) fn cpu_to_millicores(s: &str) -> Option<u64> {
    let millis = parse_quantity(s)? * 1_000.0;
    Some(((millis * 1e6).round() / 1e6).ceil() as u64)
}

/// Parse a Go `time.Duration` string into nanoseconds.
///
/// The format is a sequence of decimal numbers each with a unit suffix:
/// `"1h30m"`, `"500ms"`, `"2h0m0s"`, `"8760h"`.
///
/// Recognised units: `ns`, `us`/`µs`, `ms`, `s`, `m`, `h`, plus the `d`,
/// `w` and `y` (365 days) of Prometheus durations — a strict superset of
/// Go's; a leading `-` negates the whole duration and a leading `+` is
/// allowed, as in Go.
fn duration_to_nanos(s: &str) -> Option<i64> {
    let s = s.trim();
    if let Some(positive) = s.strip_prefix('-') {
        return duration_to_nanos(positive)?.checked_neg();
    }
    let s = s.strip_prefix('+').unwrap_or(s);
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
        } else if remaining.starts_with('d') {
            (86_400_000_000_000i64, 1)
        } else if remaining.starts_with('w') {
            (604_800_000_000_000i64, 1)
        } else if remaining.starts_with('y') {
            (31_536_000_000_000_000i64, 1)
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
        assert_eq!(duration_to_nanos("+1s"), Some(1_000_000_000));
        assert_eq!(duration_to_nanos("1d"), Some(86_400_000_000_000));
        assert_eq!(duration_to_nanos("2w"), Some(1_209_600_000_000_000));
        assert_eq!(duration_to_nanos("1y"), Some(31_536_000_000_000_000));
        assert_eq!(duration_to_nanos("10"), None);
        assert_eq!(duration_to_nanos("h"), None);
    }

    #[test]
    fn quantities() {
        let span = Span::test_data();
        let q = |s: &str| quantity_val(s, span);
        assert_eq!(q("1k"), Value::int(1_000, span));
        assert_eq!(q("2000"), Value::int(2_000, span));
        assert_eq!(q("1500m"), Value::float(1.5, span));
        assert_eq!(q("4"), Value::int(4, span));
        assert_eq!(q("1Ki"), Value::int(1_024, span));
        assert_eq!(q(""), Value::nothing(span));
        assert_eq!(q("x"), Value::nothing(span));
        // A bare JSON number is a quantity too.
        let j = serde_json::json!({"storage": 10737418240u64, "pods": 110});
        assert_eq!(
            json_memory_val(&j, &["storage"], span),
            Value::filesize(10_737_418_240i64, span)
        );
        assert_eq!(
            json_quantity_val(&j, &["pods"], span),
            Value::int(110, span)
        );
    }

    #[test]
    fn resource_maps() {
        let span = Span::test_data();
        let r = serde_json::json!({"cpu": "250m", "pods": "110", "hugepages-2Mi": "0", "ephemeral-storage": "10Gi", "nvidia.com/gpu": "2"});
        let Value::Record { val, .. } = resources_record(&r, span) else {
            panic!("not a record");
        };
        assert_eq!(val.get("cpu"), Some(&Value::int(250, span)));
        assert_eq!(val.get("memory"), Some(&Value::nothing(span)));
        assert_eq!(val.get("pods"), Some(&Value::int(110, span)));
        assert_eq!(
            val.get("ephemeral-storage"),
            Some(&Value::filesize(10i64 << 30, span))
        );
        assert_eq!(val.get("nvidia.com/gpu"), Some(&Value::int(2, span)));
        let bare = container_base(&serde_json::json!({"name": "c"}), span);
        assert_eq!(bare.get("requests"), Some(&Value::nothing(span)));
    }

    #[test]
    fn quantities_are_typed_by_resource() {
        let span = Span::test_data();
        let q = |resource: &str, raw: &str| typed_quantity(resource, Some(&json!(raw)), span);
        assert_eq!(q("requests.cpu", "500m"), Value::int(500, span));
        assert_eq!(q("limits.memory", "1Gi"), Value::filesize(1i64 << 30, span));
        assert_eq!(
            q("gold.storageclass.storage.k8s.io/requests.storage", "50Gi"),
            Value::filesize(50i64 << 30, span)
        );
        assert_eq!(
            q(
                "gold.storageclass.storage.k8s.io/persistentvolumeclaims",
                "5"
            ),
            Value::int(5, span)
        );
        assert_eq!(q("hugepages-2Mi", "1Gi"), Value::filesize(1i64 << 30, span));
        assert_eq!(q("count/deployments.apps", "3"), Value::int(3, span));
        assert_eq!(q("requests.nvidia.com/gpu", "2"), Value::int(2, span));
        assert_eq!(q("pods", ""), Value::nothing(span));
        assert_eq!(q("pods", "1k"), Value::int(1_000, span));
        assert_eq!(typed_quantity("pods", None, span), Value::nothing(span));
        // A bare JSON number is a quantity too.
        assert_eq!(
            typed_quantity("cpu", Some(&json!(2)), span),
            Value::int(2_000, span)
        );
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
        assert_eq!(label_selector(None, span), Value::nothing(span));
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
        assert_eq!(cpu_to_millicores("1500000n"), Some(2));
        assert_eq!(cpu_to_millicores("999999n"), Some(1));
        assert_eq!(cpu_to_millicores("1000000n"), Some(1));
        assert_eq!(cpu_to_millicores("0.5m"), Some(1));
        assert_eq!(cpu_to_millicores("3k"), Some(3_000_000));
        assert_eq!(cpu_to_millicores(""), None);
        assert_eq!(cpu_to_millicores("abc"), None);
    }
}
