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
//!    nushell type a Kubernetes quantity becomes.
//!
//! 4. **Spec/status extractors** (`meta_owner`, `status_condition`,
//!    `spec_selector`, `spec_strategy`) — structured sub-object helpers shared
//!    across multiple resource formatters.
//!
//! 5. **Container helpers** (`fmt_containers`, `fmt_images`) — extract
//!    container metadata from `.spec.template.spec.containers`.

use k8s_openapi::jiff;
use kube::api::DynamicObject;
use kube::ResourceExt;
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
    if cur.is_null() {
        None
    } else {
        Some(cur)
    }
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

/// Resolve the controlling `ownerReference` → `Value::string` (`"kind/name"`),
/// or `Value::nothing` when there is no controller owner.
///
/// Mirrors the Nushell `meta owner` helper: walks
/// `metadata.ownerReferences[]`, finds the entry where `controller == true`,
/// and formats it as `"<lowercase-kind>/<name>"` (e.g. `"replicaset/my-rs-xz4f"`).
/// Only the first controller reference is returned (there can be at most one
/// per the Kubernetes spec).
pub fn meta_owner(item: &DynamicObject, span: Span) -> Value {
    let refs = match item.owner_references() {
        refs if !refs.is_empty() => refs,
        _ => return Value::nothing(span),
    };

    let controller = refs.iter().find(|r| r.controller.unwrap_or(false));

    controller
        .map(|r| Value::string(format!("{}/{}", r.kind.to_lowercase(), r.name), span))
        .unwrap_or_else(|| Value::nothing(span))
}

// ---------------------------------------------------------------------------
// 3. Quantity converters  (string → typed Value)
// ---------------------------------------------------------------------------

/// Parse an RFC 3339 / ISO 8601 timestamp string → `Value::date`.
/// Returns `Value::nothing` on failure.
pub fn parse_date(s: &str, span: Span) -> Value {
    match s.parse::<jiff::Timestamp>() {
        Ok(ts) => timestamp_to_value(ts, span),
        Err(_) => Value::nothing(span),
    }
}

/// Convert a `jiff::Timestamp` to a `Value::date` (chrono `DateTime<FixedOffset>`).
///
/// Jiff timestamps are always UTC, so we fix the offset to zero.
/// This is the single place that bridges jiff → chrono for nushell.
fn timestamp_to_value(ts: jiff::Timestamp, span: Span) -> Value {
    use chrono::TimeZone;
    let secs = ts.as_second();
    let nanos = ts.subsec_nanosecond();
    let (secs_adj, nanos_u32) = if nanos < 0 {
        (secs - 1, (1_000_000_000i32 + nanos) as u32)
    } else {
        (secs, nanos as u32)
    };
    match chrono::Utc.timestamp_opt(secs_adj, nanos_u32) {
        chrono::LocalResult::Single(dt) => Value::date(dt.fixed_offset(), span),
        _ => Value::nothing(span),
    }
}

/// Parse a Kubernetes memory quantity ("256Mi", "1Gi", "512k", "2G")
/// → `Value::filesize` (bytes i64).
/// Returns `Value::nothing` for an empty string; `Value::filesize(0)` for
/// a valid zero quantity.
pub fn parse_memory(s: &str, span: Span) -> Value {
    if s.is_empty() {
        return Value::nothing(span);
    }
    Value::filesize(memory_to_bytes(s) as i64, span)
}

/// Parse a Kubernetes CPU quantity ("500m", "2", "0.5", "100u", "800n")
/// → `Value::int` (millicores i64).
/// Returns `Value::nothing` for an empty string.
pub fn parse_cpu(s: &str, span: Span) -> Value {
    if s.is_empty() {
        return Value::nothing(span);
    }
    Value::int(cpu_to_millicores(s) as i64, span)
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

// ---------------------------------------------------------------------------
// 4. Spec / status extractors
// ---------------------------------------------------------------------------

/// Find the first `.status.conditions[]` entry whose `type` field equals
/// `cond_type` → `Value::record` with all condition fields as string columns.
///
/// Returns an empty record (not `Value::nothing`) when no match is found,
/// mirroring the Nushell `status condition` helper behaviour so callers can
/// always do `$r.conditions.Available.status` without a null-check.
///
/// Typical usage:
/// ```text
/// status_condition(&item.data, "Available", span)
/// status_condition(&item.data, "Ready",     span)
/// ```
pub fn status_condition(data: &Json, cond_type: &str, span: Span) -> Value {
    let condition = json_array(data, &["status", "conditions"])
        .iter()
        .find(|c| json_str(c, &["type"]).unwrap_or("") == cond_type);

    let obj = if let Some(c) = condition {
        c
    } else {
        return Value::record(Record::new(), span);
    };

    // Materialise the condition object as a flat string record.
    // We handle the common fields explicitly so the column order is stable.
    let keys = ["type", "status", "reason", "message", "lastTransitionTime"];
    let rec = {
        let mut r = Record::new();
        keys.iter().for_each(|&key| {
            r.push(
                key,
                Value::string(json_str(obj, &[key]).unwrap_or(""), span),
            );
        });
        r
    };

    Value::record(rec, span)
}

/// Extract `.spec.selector` → `Value::record`.
///
/// For **Services**, `spec.selector` is a flat `{ label: value }` map — use
/// this function.  Returns an empty record when the field is absent.
pub fn spec_selector(data: &Json, span: Span) -> Value {
    let selector = json_at(data, &["spec", "selector"]).and_then(|v| v.as_object());

    let mut rec = Record::new();
    if let Some(map) = selector {
        for (k, v) in map {
            let s = v.as_str().unwrap_or("");
            rec.push(k.clone(), Value::string(s, span));
        }
    }
    Value::record(rec, span)
}

/// Extract `.spec.selector.matchLabels` → `Value::record` (flat string map).
///
/// Use this for workload resources (Deployment, DaemonSet, StatefulSet,
/// ReplicaSet, PVC) where `spec.selector` is a `LabelSelector` with a
/// `matchLabels` sub-object, not a flat map.  Returns an empty record when
/// the field is absent.
pub fn spec_matchlabels(data: &Json, span: Span) -> Value {
    let labels =
        json_at(data, &["spec", "selector", "matchLabels"]).and_then(|v| v.as_object());

    let mut rec = Record::new();
    if let Some(map) = labels {
        for (k, v) in map {
            let s = v.as_str().unwrap_or("");
            rec.push(k.clone(), Value::string(s, span));
        }
    }
    Value::record(rec, span)
}

/// Extract `.spec.strategy` → `Value::record { type, maxUnavailable, maxSurge }`.
///
/// Mirrors the Nushell `spec strategy` helper.  Fields that are absent in
/// the spec are stored as `Value::nothing` so wide formatters can display
/// them conditionally.
pub fn spec_strategy(data: &Json, span: Span) -> Value {
    let strategy_type = json_str(data, &["spec", "strategy", "type"]).unwrap_or("RollingUpdate");

    let max_unavailable = json_str(
        data,
        &["spec", "strategy", "rollingUpdate", "maxUnavailable"],
    )
    .map(|s| Value::string(s, span))
    .unwrap_or_else(|| Value::nothing(span));

    let max_surge = json_str(data, &["spec", "strategy", "rollingUpdate", "maxSurge"])
        .filter(|s| !s.is_empty())
        .map(|s| Value::string(s, span))
        .unwrap_or_else(|| Value::nothing(span));

    let mut rec = Record::new();
    rec.push("type", Value::string(strategy_type, span));
    rec.push("maxUnavailable", max_unavailable);
    rec.push("maxSurge", max_surge);
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
            let updated = match json_str(c, &["lastTransitionTime"]) {
                Some(s) if !s.is_empty() => parse_date(s, span),
                _ => Value::nothing(span),
            };
            let mut rec = Record::new();
            rec.push(
                "type",
                Value::string(json_str(c, &["type"]).unwrap_or(""), span),
            );
            rec.push(
                "status",
                Value::string(json_str(c, &["status"]).unwrap_or(""), span),
            );
            rec.push(
                "reason",
                Value::string(json_str(c, &["reason"]).unwrap_or(""), span),
            );
            rec.push(
                "message",
                Value::string(json_str(c, &["message"]).unwrap_or(""), span),
            );
            rec.push("updated", updated);
            Value::record(rec, span)
        })
        .collect();
    Value::list(rows, span)
}

// ---------------------------------------------------------------------------
// 5. Container helpers
// ---------------------------------------------------------------------------

/// Build a `Value::record` for a single container JSON object.
///
/// ```text
/// { name, image, requests?: { cpu, memory }, limits?: { cpu, memory } }
/// ```
///
/// `requests` and `limits` are omitted entirely (not `nothing`) when absent,
/// matching the Nushell `container base` helper.
pub fn container_base(c: &Json, span: Span) -> Value {
    let mut rec = Record::new();
    rec.push(
        "name",
        Value::string(json_str(c, &["name"]).unwrap_or(""), span),
    );
    rec.push(
        "image",
        Value::string(json_str(c, &["image"]).unwrap_or(""), span),
    );

    // resources — only include sub-record when the field is present
    if let Some(resources) = json_at(c, &["resources"]) {
        if let Some(requests) = json_at(resources, &["requests"]) {
            rec.push("requests", resources_record(requests, span));
        }
        if let Some(limits) = json_at(resources, &["limits"]) {
            rec.push("limits", resources_record(limits, span));
        }
    }

    Value::record(rec, span)
}

/// Build a typed `{ cpu, memory }` record from a resources map.
fn resources_record(r: &Json, span: Span) -> Value {
    let mut rec = Record::new();
    rec.push("cpu", parse_cpu(json_str(r, &["cpu"]).unwrap_or(""), span));
    rec.push(
        "memory",
        parse_memory(json_str(r, &["memory"]).unwrap_or(""), span),
    );
    Value::record(rec, span)
}

/// Format a slice of container JSON objects → `Value::list` of
/// `container_base` records.
///
/// The caller is responsible for locating the right array with `json_array`:
///
/// ```rust
/// // Standard workload (Deployment, DaemonSet, StatefulSet, ReplicaSet, Job)
/// fmt_containers(json_array(data, "spec.template.spec.containers"), span)
///
/// // CronJob — template nested one level deeper
/// fmt_containers(json_array(data, "spec.jobTemplate.spec.template.spec.containers"), span)
/// ```
pub fn fmt_containers(containers: &[Json], span: Span) -> Value {
    Value::list(
        containers.iter().map(|c| container_base(c, span)).collect(),
        span,
    )
}

/// Format a slice of container JSON objects → `Value::list` of image strings.
///
/// Same calling convention as `fmt_containers`:
///
/// ```rust
/// fmt_images(json_array(data, "spec.template.spec.containers"), span)
/// ```
pub fn fmt_images(containers: &[Json], span: Span) -> Value {
    Value::list(
        containers
            .iter()
            .map(|c| Value::string(json_str(c, &["image"]).unwrap_or(""), span))
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
/// from `m` quantities round down to zero.
pub(crate) fn memory_to_bytes(s: &str) -> u64 {
    // Binary suffixes (IEC) — try float parse to support e.g. "1.5Gi"
    for (suffix, shift) in [
        ("Ki", 10u32),
        ("Mi", 20),
        ("Gi", 30),
        ("Ti", 40),
        ("Pi", 50),
        ("Ei", 60),
    ] {
        if let Some(v) = s
            .strip_suffix(suffix)
            .and_then(|n| n.trim().parse::<f64>().ok())
        {
            return (v * (1u64 << shift) as f64) as u64;
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
            if let Ok(v) = n.trim().parse::<f64>() {
                return (v * factor as f64) as u64;
            }
        }
    }
    // Millibytes — sub-byte quantity, rounds down (1000m == 1 byte)
    if let Some(n) = s.strip_suffix('m') {
        if let Ok(v) = n.trim().parse::<f64>() {
            return (v / 1_000.0) as u64;
        }
    }
    // Plain integer or float bytes
    s.trim().parse::<f64>().map(|v| v as u64).unwrap_or(0)
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
pub(crate) fn cpu_to_millicores(s: &str) -> u64 {
    // millicores — float to support "0.5m" (sub-millicore, truncates to 0)
    if let Some(n) = s.strip_suffix('m') {
        return n.trim().parse::<f64>().map(|v| v as u64).unwrap_or(0);
    }
    // microcores → millicores (÷ 1 000, truncates)
    if let Some(n) = s.strip_suffix('u') {
        return n
            .trim()
            .parse::<f64>()
            .map(|v| (v / 1_000.0) as u64)
            .unwrap_or(0);
    }
    // nanocores → millicores (÷ 1 000 000, truncates)
    if let Some(n) = s.strip_suffix('n') {
        return n
            .trim()
            .parse::<f64>()
            .map(|v| (v / 1_000_000.0) as u64)
            .unwrap_or(0);
    }
    // Decimal large-unit suffixes (rare but spec-valid)
    for (suffix, factor) in [
        ("k", 1_000_000u64),
        ("M", 1_000_000_000),
        ("G", 1_000_000_000_000),
    ] {
        if let Some(n) = s.strip_suffix(suffix) {
            if let Ok(v) = n.trim().parse::<f64>() {
                return (v * factor as f64) as u64;
            }
        }
    }
    // Bare float/int — whole cores, multiply by 1 000
    s.trim()
        .parse::<f64>()
        .map(|f| (f * 1_000.0) as u64)
        .unwrap_or(0)
}

/// Parse a Go `time.Duration` string into nanoseconds.
///
/// The format is a sequence of decimal numbers each with a unit suffix:
/// `"1h30m"`, `"500ms"`, `"2h0m0s"`, `"8760h"`.
///
/// Recognised units: `ns`, `us`/`µs`, `ms`, `s`, `m`, `h`.
fn duration_to_nanos(s: &str) -> Option<i64> {
    let s = s.trim();
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
